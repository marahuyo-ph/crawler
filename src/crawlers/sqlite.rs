use clap::Args;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use std::collections::HashMap;
use url::Url;
use sha2::{Sha256, Digest};
use tracing::debug;

use crate::extract_links::ExtractLinks;
use crate::models::prelude::*;
use crate::traits::IAsyncCrawler;

#[derive(Debug, Clone, Args)]
pub struct SqliteCrawlerOptions {
    pub urls:Vec<Url>,
    #[arg(long, default_value = "crawler.db")]
    pub database_url: String,
    #[arg(long)]
    pub crawl_session_id: Option<i64>,
}

pub struct SqliteCrawler {
    client:reqwest::Client,
    database: DatabaseConnection,
    crawl_session_id: i64,
    robots_cache: HashMap<String, crate::check_robots::Robot>,
}

impl SqliteCrawler {
    pub fn new(client:&reqwest::Client,database: DatabaseConnection, crawl_session_id: i64) -> Self {
        Self {
            client:client.clone(),
            database,
            crawl_session_id,
            robots_cache: HashMap::new(),
        }
    }

    pub async fn migrate(database:&DatabaseConnection) -> anyhow::Result<()> {

        let pool = database.get_sqlite_connection_pool();

        let migrator =sqlx::migrate!("./migrations");

        migrator.run(pool).await?;

        Ok(())
    }

}

impl IAsyncCrawler for SqliteCrawler {
    async fn check_robot_policy(&self, url: &Url) -> anyhow::Result<bool> {
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;

        // Get domain configuration from database
        let domain_record = Domains::find()
            .filter(crate::models::domains::Column::Domain.eq(domain))
            .one(&self.database)
            .await?;

        match domain_record {
            Some(record) => Ok(record.allow_crawl),
            None => {
                // If no record exists, create one with default allow_crawl = true
                let new_domain = crate::models::domains::ActiveModel {
                    domain: sea_orm::Set(domain.to_string()),
                    allow_crawl: sea_orm::Set(true),
                    ..Default::default()
                };
                new_domain.insert(&self.database).await?;
                Ok(true)
            }
        }
    }

    async fn fetch_robot_txt(
        &self,
        url: &Url,
    ) -> anyhow::Result<Option<crate::check_robots::Robot>> {
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;

        // Try to get from database first
        let domain_record = Domains::find()
            .filter(crate::models::domains::Column::Domain.eq(domain))
            .one(&self.database)
            .await?;

        if let Some(record) = domain_record {
            if let Some(robots_txt) = record.robots_txt {
                return Ok(Some(crate::check_robots::Robot::new(robots_txt)));
            }
        }

        // If not in database, fetch from the web
        let robots_url = format!("{}://{}:/robots.txt", url.scheme(), domain);
        match reqwest::get(&robots_url).await {
            Ok(resp) => match resp.text().await {
                Ok(text) => Ok(Some(crate::check_robots::Robot::new(text))),
                Err(_) => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }

    async fn set_robot_txt(&mut self, url: &Url, robot: crate::check_robots::Robot) -> anyhow::Result<()> {
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;

        self.robots_cache.insert(domain.to_string(), robot);
        Ok(())
    }

    async fn get_robot_txt(&self, url: &Url) -> anyhow::Result<Option<crate::check_robots::Robot>> {
        let domain = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid URL: no host"))?;

        Ok(self.robots_cache.get(domain).cloned())
    }

    async fn fetch_page(&self, url: &Url) -> anyhow::Result<reqwest::Response> {
        debug!("Fetching page: {}", url);
        let response = reqwest::get(url.to_string()).await?;
        debug!("Received response from: {} (status: {})", url, response.status());
        Ok(response)
    }

    async fn parse_links(&self, url: &Url, html: &scraper::Html) -> anyhow::Result<Vec<Url>> {
        let extracted = ExtractLinks::extract(url, html)?;

        // Combine internal and external links
        let mut links = Vec::new();
        
        for link_info in extracted.internal.iter().chain(extracted.external.iter()) {
            if let Ok(parsed_url) = Url::parse(&link_info.url) {
                links.push(parsed_url);
            }
        }

        debug!("Found {} links (internal: {}, external: {})", links.len(), extracted.internal.len(), extracted.external.len());
        Ok(links)
    }

    async fn extract_content(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<(scraper::Html, reqwest::header::HeaderMap)> {
        debug!("Extracting content...");
        let headers = response.headers().clone();
        let text = response.text().await?;
        let html = scraper::Html::parse_document(&text);
        debug!("Content extracted ({} bytes)", text.len());
        Ok((html, headers))
    }

    async fn add_to_queue(&mut self, urls: Vec<Url>) -> anyhow::Result<()> {
        let mut added = 0;
        let mut skipped = 0;
        
        for url in urls {
            let url_str = url.to_string();

            // Check if URL already exists in queue
            let exists = crate::models::url_queue::Entity::find()
                .filter(crate::models::url_queue::Column::Url.eq(&url_str))
                .filter(crate::models::url_queue::Column::CrawlSessionId.eq(self.crawl_session_id))
                .count(&self.database)
                .await?
                > 0;

            if !exists {
                let queue_item = crate::models::url_queue::ActiveModel {
                    crawl_session_id: sea_orm::Set(self.crawl_session_id),
                    url: sea_orm::Set(url_str),
                    priority: sea_orm::Set(0),
                    retry_count: sea_orm::Set(0),
                    status: sea_orm::Set("pending".to_string()),
                    ..Default::default()
                };

                queue_item.insert(&self.database).await?;
                added += 1;
            } else {
                skipped += 1;
            }
        }

        debug!("Added {} URLs to queue (skipped {} duplicates)", added, skipped);
        Ok(())
    }

    async fn next_queue(&mut self) -> Option<Url> {
        let next_item = crate::models::url_queue::Entity::find()
            .filter(crate::models::url_queue::Column::CrawlSessionId.eq(self.crawl_session_id))
            .filter(crate::models::url_queue::Column::Status.eq("pending"))
            .order_by_desc(crate::models::url_queue::Column::Priority)
            .one(&self.database)
            .await
            .ok()
            .flatten()?;

        // Update status to processing
        let mut item = next_item.into_active_model();
        item.status = sea_orm::Set("processing".to_string());
        let new_item = item.update(&self.database).await.ok();

        let url = new_item.and_then(|n| Url::parse(&n.url).ok());
        if let Some(ref u) = url {
            debug!("Processing next URL from queue: {}", u);
        }
        url
    }

    async fn has_seen(&self, url: &Url) -> bool {
        let url_str = url.to_string();
        Pages::find()
            .filter(crate::models::pages::Column::Url.eq(&url_str))
            .filter(crate::models::pages::Column::CrawlSessionId.eq(self.crawl_session_id))
            .one(&self.database)
            .await
            .ok()
            .flatten()
            .is_some()
    }

    async fn mark_as_visited(&mut self, url: &Url) -> anyhow::Result<()> {
        let url_str = url.to_string();

        let queue_item = crate::models::url_queue::Entity::find()
            .filter(crate::models::url_queue::Column::Url.eq(&url_str))
            .filter(crate::models::url_queue::Column::CrawlSessionId.eq(self.crawl_session_id))
            .one(&self.database)
            .await?;

        if let Some(item) = queue_item {
            let mut active_item = item.into_active_model();
            active_item.status = sea_orm::Set("completed".to_string());
            active_item.update(&self.database).await?;
            debug!("Marked as visited: {}", url);
        }

        Ok(())
    }

    async fn save(
        &self,
        url: &Url,
        html: &scraper::Html,
        header: reqwest::header::HeaderMap,
    ) -> anyhow::Result<()> {
        use scraper::Selector;

        let url_str = url.to_string();
        debug!("Saving page: {}", url);

        // Get the full HTML content as string
        let html_content = html.html();
        
        // Calculate SHA256 hash of HTML content
        let mut hasher = Sha256::new();
        hasher.update(html_content.as_bytes());
        let content_hash = format!("{:x}", hasher.finalize());

        // Extract title
        let title = html
            .select(&Selector::parse("title").unwrap())
            .next()
            .and_then(|el| el.text().next())
            .map(|s| s.to_string());

        // Extract description from meta tag
        let selector = Selector::parse("meta[name='description']").unwrap();
        let description = html
            .select(&selector)
            .next()
            .and_then(|el| el.value().attr("content"))
            .map(|s| s.to_string());

        // Get content type and length from headers
        let content_type = header
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_length = header
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok());

        // Check if page already exists
        let existing_page = Pages::find()
            .filter(crate::models::pages::Column::Url.eq(&url_str))
            .filter(crate::models::pages::Column::CrawlSessionId.eq(self.crawl_session_id))
            .one(&self.database)
            .await?;

        let inserted_page = if let Some(page) = existing_page {
            // Update existing page
            let mut active_page = page.into_active_model();
            active_page.title = sea_orm::Set(title.clone());
            active_page.description = sea_orm::Set(description);
            active_page.content_type = sea_orm::Set(content_type);
            active_page.content_length = sea_orm::Set(content_length);
            active_page.content_hash = sea_orm::Set(Some(content_hash.clone()));
            active_page.html_content = sea_orm::Set(Some(html_content.clone()));
            active_page.update(&self.database).await?
        } else {
            // Insert new page
            let page = crate::models::pages::ActiveModel {
                crawl_session_id: sea_orm::Set(self.crawl_session_id),
                url: sea_orm::Set(url_str.clone()),
                title: sea_orm::Set(title.clone()),
                description: sea_orm::Set(description),
                content_type: sea_orm::Set(content_type),
                content_length: sea_orm::Set(content_length),
                content_hash: sea_orm::Set(Some(content_hash)),
                html_content: sea_orm::Set(Some(html_content)),
                status_code: sea_orm::Set(Some(200)),
                ..Default::default()
            };

            page.insert(&self.database).await?
        };
        debug!("Page saved with title: {}", title.unwrap_or_else(|| "(No title)".to_string()));

        // Parse links from HTML and save them to database
        let selector = Selector::parse("a[href]").unwrap();

        let mut links = vec![];

        for element in html.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                let link_text = element.text().collect::<Vec<_>>().join("");
                let link = crate::models::links::ActiveModel {
                    source_page_id: sea_orm::Set(inserted_page.id),
                    target_url: sea_orm::Set(href.to_string()),
                    link_text: sea_orm::Set(if link_text.is_empty() {
                        None
                    } else {
                        Some(link_text)
                    }),
                    link_type: sea_orm::Set(Some("internal".to_string())),
                    ..Default::default()
                };

                links.push(link);
            }
        }
        
        crate::models::links::Entity::insert_many(links).exec(&self.database).await?;

        Ok(())
    }
}
