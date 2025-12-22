use std::time::Duration;

use crate::check_robots::Robot;
use url::Url;
use tracing::{debug, info, error};

pub trait IAsyncCrawler {
    // Setup & Pre-flight
    async fn check_robot_policy(&self, url: &Url) -> anyhow::Result<bool>;
    async fn fetch_robot_txt(&self, url: &Url) -> anyhow::Result<Option<Robot>>;
    async fn set_robot_txt(&mut self, url: &Url, robot: Robot) -> anyhow::Result<()>;
    async fn get_robot_txt(&self, url: &Url) -> anyhow::Result<Option<Robot>>;

    // Fetching Logic
    async fn fetch_page(&self, url: &Url) -> anyhow::Result<reqwest::Response>;

    // Extraction Logic
    async fn parse_links(&self, url: &Url, html: &scraper::Html) -> anyhow::Result<Vec<Url>>;
    async fn extract_content(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<(scraper::Html, reqwest::header::HeaderMap)>;

    // State Management
    async fn add_to_queue(&mut self, urls: Vec<Url>) -> anyhow::Result<()>;
    async fn next_queue(&mut self) -> Option<Url>;
    async fn has_seen(&self, url: &Url) -> bool;
    async fn mark_as_visited(&mut self, url: &Url) -> anyhow::Result<()>;

    // Persistence
    async fn save(
        &self,
        url: &Url,
        html: &scraper::Html,
        header: reqwest::header::HeaderMap,
    ) -> anyhow::Result<()>;

    async fn start(&mut self, seed_url: Vec<Url>) -> anyhow::Result<()> {
        info!("Starting crawl with {} seed URL(s)", seed_url.len());
        
        self.add_to_queue(seed_url).await?;

        loop {
            let next_url = match self.next_queue().await {
                Some(url) => url,
                None => {
                    info!("Crawl queue empty, finishing crawl");
                    break Ok(());
                }
            };

            info!("Processing URL: {}", next_url);

            debug!("Fetching robots.txt for domain");
            let robot = self.fetch_robot_txt(&next_url).await?;

            if let Some(robot) = robot {
                self.set_robot_txt(&next_url, robot).await?;
            }   

            // Check robot policy, skip on error
            if !self.check_robot_policy(&next_url).await.unwrap_or(false) {
                info!("Skipping URL due to robots.txt policy");
                continue;
            }

            if let Some(robot) = self.get_robot_txt(&next_url).await? {
                let delay = robot.crawl_delay("*").unwrap_or_default();
                if delay > 0.0 {
                    info!("Applying crawl delay: {:.2}s", delay);
                    tokio::time::sleep(Duration::from_secs_f64(delay)).await;
                }
            }

            if self.has_seen(&next_url).await {
                info!("URL already seen, skipping");
                continue;
            }

            // Fetch page, log error and continue on failure
            info!("Fetching page");
            let response = match self.fetch_page(&next_url).await {
                Ok(resp) => {
                    info!("Page fetched successfully (status: {})", resp.status());
                    resp
                }
                Err(e) => {
                    error!("Failed to fetch {}: {}", next_url, e);
                    continue;
                }
            };

            // Extract content, log error and continue on failure
            info!("Extracting content");
            let (html, headers) = match self.extract_content(response).await {
                Ok(result) => {
                    info!("Content extracted successfully");
                    result
                }
                Err(e) => {
                    error!("Failed to extract content from {}: {}", next_url, e);
                    continue;
                }
            };

            // Save page, log error and continue on failure
            info!("Saving page to database");
            if let Err(e) = self.save(&next_url, &html, headers).await {
                error!("Failed to save {}: {}", next_url, e);
                continue;
            }
            info!("Page saved successfully");

            // Mark as visited, log error and continue on failure
            info!("Marking URL as visited");
            if let Err(e) = self.mark_as_visited(&next_url).await {
                error!("Failed to mark {} as visited: {}", next_url, e);
                continue;
            }

            // Parse links, log error and continue on failure
            info!("Parsing links from page");
            let urls = match self.parse_links(&next_url, &html).await {
                Ok(urls) => {
                    info!("Found {} links on page", urls.len());
                    urls
                }
                Err(e) => {
                    error!("Failed to parse links from {}: {}", next_url, e);
                    continue;
                }
            };

            // Add to queue, log error and continue on failure
            info!("Adding {} links to queue", urls.len());
            if let Err(e) = self.add_to_queue(urls).await {
                error!("Failed to add URLs to queue: {}", e);
                continue;
            }
        }
    }
}
