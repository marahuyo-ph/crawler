use std::collections::{VecDeque, HashSet};
use std::sync::Mutex;
use std::time::Duration;
use clap::Args;

use anyhow::anyhow;
use reqwest::StatusCode;
use scraper::Selector;
use url::Url;
use tracing::{debug, warn, error};

use crate::check_robots::Robot;
use crate::extract_links::ExtractLinks;
use crate::traits::IAsyncCrawler;

#[derive(Debug,Clone,Args)]
pub struct StdOutCrawlerOptions {
  pub url:Url,
}

pub struct StdOutCrawler {
    client: reqwest::Client,
    queue: VecDeque<Url>,
    visited: HashSet<String>,
    robot:Option<Robot>,
    max_retries: i32,
    max_redirects: i32,
    retry_delay: Duration,
}

impl StdOutCrawler {
    pub fn new(client: &reqwest::Client) -> Self {
        Self {
            client: client.clone(),
            queue: VecDeque::new(),
            visited: HashSet::new(),
            robot:None,
            max_retries: 3,
            max_redirects: 5,
            retry_delay: Duration::from_millis(100),
        }
    }
}

impl IAsyncCrawler for StdOutCrawler {
    async fn check_robot_policy(&self, url: &url::Url) -> anyhow::Result<bool> {
        match &self.robot {
            Some(robot) => Ok(robot.allow(url.as_str(), "Marahuyo")),
            None => Ok(true), // Allow by default if no robots.txt loaded
        }
    }

    async fn fetch_page(&self, url: &url::Url) -> anyhow::Result<reqwest::Response> {
        let mut current_url = url.clone();
        let mut retry_count = 0;
        let mut redirect_count = 0;
        let mut max_redirects = self.max_redirects;
        let mut retry_delay = self.retry_delay;

        debug!("Starting fetch for URL: {}", url);

        loop {
            if max_redirects == 0 {
                error!("Too many redirects for URL: {}", url);
                return Err(anyhow!("Too many redirects"));
            }

            debug!(
                "Attempting to fetch URL: {} (retry: {}/{})",
                current_url, retry_count, self.max_retries
            );

            let response = match self.client.get(current_url.clone()).send().await {
                Ok(resp) => {
                    debug!(
                        "Received response from: {}, status: {}",
                        current_url,
                        resp.status()
                    );
                    resp
                }
                Err(e) => {
                    if retry_count < self.max_retries {
                        retry_count += 1;
                        warn!(
                            error = %e,
                            retry = retry_count,
                            max_retries = self.max_retries,
                            delay_ms = retry_delay.as_millis(),
                            "Network error, retrying..."
                        );
                        tokio::time::sleep(retry_delay).await;
                        retry_delay = Duration::from_millis(retry_delay.as_millis() as u64 * 2);
                        continue;
                    } else {
                        error!(
                            error = %e,
                            max_retries = self.max_retries,
                            "Failed to fetch after max retries"
                        );
                        return Err(anyhow!(
                            "Failed to fetch after {} retries: {}",
                            self.max_retries,
                            e
                        ));
                    }
                }
            };

            match response.status() {
                StatusCode::OK => {
                    debug!("Received valid HTML response from: {}", current_url);
                    
                    if let Some(content_type) = response.headers().get("Content-Type") {
                        if !content_type.to_str()?.contains("text/html") {
                            return Err(anyhow!("Response is not HTML"));
                        }
                    }
                    
                    return Ok(response);
                }
                StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND => {
                    let location = response
                        .headers()
                        .get("Location")
                        .ok_or(anyhow!("Redirect without Location header"))?
                        .to_str()?;

                    debug!(
                        status = response.status().as_u16(),
                        location = location,
                        "Following redirect"
                    );

                    current_url = Url::parse(location)
                        .or_else(|_| current_url.join(location))
                        .map_err(|_| anyhow!("Invalid redirect URL: {}", location))?;

                    max_redirects -= 1;
                    redirect_count += 1;
                    retry_count = 0;

                    debug!(
                        redirect_number = redirect_count,
                        remaining_redirects = max_redirects,
                        new_url = %current_url,
                        "Redirect processed"
                    );
                }
                StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                    if retry_count < self.max_retries {
                        retry_count += 1;
                        warn!(
                            status = response.status().as_u16(),
                            retry = retry_count,
                            max_retries = self.max_retries,
                            delay_ms = retry_delay.as_millis(),
                            "Server error, retrying..."
                        );
                        tokio::time::sleep(retry_delay).await;
                        retry_delay = Duration::from_millis(retry_delay.as_millis() as u64 * 2);
                        continue;
                    } else {
                        error!(
                            status = response.status().as_u16(),
                            max_retries = self.max_retries,
                            "Server error after max retries"
                        );
                        return Err(anyhow!(
                            "HTTP Error {} after {} retries: {}",
                            response.status(),
                            self.max_retries,
                            response.status().canonical_reason().unwrap_or("Unknown")
                        ));
                    }
                }
                _ => {
                    error!(
                        status = response.status().as_u16(),
                        reason = response.status().canonical_reason(),
                        "HTTP error"
                    );
                    return Err(anyhow!(
                        "HTTP Error {}: {}",
                        response.status(),
                        response.status().canonical_reason().unwrap_or("Unknown")
                    ));
                }
            }
        }
    }

    async fn parse_links(
        &self,
        url: &url::Url,
        html: &scraper::Html,
    ) -> anyhow::Result<Vec<url::Url>> {
        let extracted = ExtractLinks::extract(url, html)?;
        
        // Combine internal and external links
        let mut links = Vec::new();
        for link_info in extracted.internal.iter() {
            if let Ok(parsed_url) = Url::parse(&link_info.url) {
                links.push(parsed_url);
            }
        }
        
        Ok(links)
    }

    async fn extract_content(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<(scraper::Html, reqwest::header::HeaderMap)> {
        let headers = response.headers().to_owned();

        let text_html = response.text().await?;

        let html = scraper::Html::parse_document(&text_html);

        Ok((html, headers))
    }

    async fn add_to_queue(&mut self, urls: Vec<url::Url>) -> anyhow::Result<()> {
        for url in urls.into_iter() {
            let url_str = url.to_string();
            if !self.visited.contains(&url_str) {
              self.queue.push_back(url);
            }
        }
        Ok(())
    }

    async fn next_queue(&mut self) -> Option<url::Url> {
        self.queue.pop_front()
    }

    async fn has_seen(&self, url: &url::Url) -> bool {
        self.visited.contains(&url.to_string())
    }

    async fn mark_as_visited(&mut self, url: &url::Url) -> anyhow::Result<()> {
        self.visited.insert(url.to_string());
        Ok(())
    }

    async fn save(
        &self,
        url: &url::Url,
        html: &scraper::Html,
        headers: reqwest::header::HeaderMap,
    ) -> anyhow::Result<()> {
        // Extract title
        let title_selector = Selector::parse("title").unwrap();
        let title = html
            .select(&title_selector)
            .next()
            .and_then(|t| t.text().next())
            .unwrap_or("(No title)");

        // Extract meta description
        let meta_selector = Selector::parse("meta[name=\"description\"]").unwrap();
        let description = html
            .select(&meta_selector)
            .next()
            .and_then(|m| m.value().attr("content"))
            .unwrap_or("(No description)");

        // Count links
        let links_selector = Selector::parse("a[href]").unwrap();
        let link_count = html.select(&links_selector).count();

        // Get content type and length
        let content_type = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        let content_length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown");

        // Extract links to be queued
        let extracted = ExtractLinks::extract(url, html)?;
        let internal_links = &extracted.internal;

        println!("\n📄 URL: {}", url);
        println!("   Title: {}", title);
        println!("   Description: {}", description);
        println!("   Links found: {}", link_count);
        println!("   Content-Type: {}", content_type);
        println!("   Content-Length: {} bytes", content_length);
        
        if !internal_links.is_empty() {
            println!("   🔗 Internal links to queue ({}): ", internal_links.len());
            for (i, link) in internal_links.iter().take(5).enumerate() {
                println!("      {}. {}", i + 1, link.url);
            }
            if internal_links.len() > 5 {
                println!("      ... and {} more", internal_links.len() - 5);
            }
        }

        Ok(())
    }
    
    async fn fetch_robot_txt(&self, url: &Url) -> anyhow::Result<Option<Robot>> {
        let robots_url = match url.port() {
            Some(port) => format!(
                "{}://{}:{}/robots.txt",
                url.scheme(),
                url.host_str().unwrap_or("localhost"),
                port
            ),
            None => format!(
                "{}://{}/robots.txt",
                url.scheme(),
                url.host_str().unwrap_or("localhost")
            ),
        };

        match self.client.get(&robots_url).send().await {
            Ok(response) => {
                if response.status() == StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                
                let content = response.text().await?;
                let robot = Robot::new(content);
                Ok(Some(robot))
            }
            Err(_) => Ok(None), // If can't fetch, return None
        }
    }
    
    async fn set_robot_txt(&mut self,robot:Robot) -> anyhow::Result<()> {

        self.robot = Some(robot);

        Ok(())
    }
}
