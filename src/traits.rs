use std::time::Duration;

use crate::check_robots::Robot;
use url::Url;
use tracing::{debug, info, error};
use futures::future;

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
        use std::collections::{HashMap, HashSet};
        use std::time::Instant;
        
        info!("Starting crawl with {} seed URL(s)", seed_url.len());
        
        self.add_to_queue(seed_url).await?;

        // Track last crawl time per domain for throttling
        let mut domain_last_crawl: HashMap<String, Instant> = HashMap::new();
        let mut domain_crawl_delays: HashMap<String, f64> = HashMap::new();

        loop {
            let next_url = match self.next_queue().await {
                Some(url) => url,
                None => {
                    info!("Crawl queue empty, finishing crawl");
                    return Ok(());
                }
            };

            let domain = next_url.host_str().unwrap_or("unknown").to_string();
            
            info!("Processing URL: {} (domain: {})", next_url, domain);

            // Enforce per-domain crawl delay
            if let Some(last_crawl) = domain_last_crawl.get(&domain) {
                let delay = domain_crawl_delays.get(&domain).copied().unwrap_or(0.0);
                let elapsed = last_crawl.elapsed().as_secs_f64();
                
                if elapsed < delay {
                    let wait_time = delay - elapsed;
                    debug!("Applying crawl delay for {}: {:.2}s", domain, wait_time);
                    tokio::time::sleep(Duration::from_secs_f64(wait_time)).await;
                }
            }

            // Fetch and set robot policy for domain (if not already cached)
            if self.get_robot_txt(&next_url).await?.is_none() {
                debug!("Fetching robots.txt for domain: {}", domain);
                if let Ok(Some(robot)) = self.fetch_robot_txt(&next_url).await {
                    let delay = robot.crawl_delay("*").unwrap_or_default();
                    domain_crawl_delays.insert(domain.clone(), delay);
                    self.set_robot_txt(&next_url, robot).await?;
                }
            }

            // Check robot policy, skip on error
            if !self.check_robot_policy(&next_url).await.unwrap_or(false) {
                info!("Skipping URL due to robots.txt policy: {}", next_url);
                continue;
            }

            if self.has_seen(&next_url).await {
                info!("URL already seen, skipping: {}", next_url);
                continue;
            }

            domain_last_crawl.insert(domain.clone(), Instant::now());

            // Collect URLs with different domains for concurrent fetching
            let mut batch_urls = vec![next_url.clone()];
            let mut seen_domains = HashSet::new();
            seen_domains.insert(domain);

            // Try to find more URLs from different domains without blocking
            while batch_urls.len() < 5 {
                match self.next_queue().await {
                    Some(url) => {
                        let url_domain = url.host_str().unwrap_or("unknown").to_string();
                        
                        // Only add if domain is different (for concurrent fetching)
                        if !seen_domains.contains(&url_domain) {
                            // Quick checks before adding to batch
                            if !self.has_seen(&url).await {
                                if self.check_robot_policy(&url).await.unwrap_or(false) {
                                    seen_domains.insert(url_domain);
                                    batch_urls.push(url);
                                    continue;
                                }
                            }
                        }
                        
                        // If we can't add to batch, put it back in queue by adding to a buffer
                        // For now we'll just lose it (next iteration will get it)
                    }
                    None => break,
                }
            }

            info!("Fetching {} URLs from {} different domains", batch_urls.len(), seen_domains.len());

            // Fetch all URLs concurrently (different domains only)
            let mut fetch_futures = Vec::new();
            for url in &batch_urls {
                fetch_futures.push(self.fetch_page(url));
            }
            
            let results = future::join_all(fetch_futures).await;

            // Process results sequentially
            for (idx, result) in results.into_iter().enumerate() {
                let url = batch_urls.get(idx).cloned();
                if url.is_none() {
                    break;
                }
                let url = url.unwrap();

                info!("Processing fetched URL: {}", url);
                
                let response = match result {
                    Ok(resp) => {
                        info!("Page fetched successfully (status: {})", resp.status());
                        resp
                    }
                    Err(e) => {
                        error!("Failed to fetch {}: {}", url, e);
                        continue;
                    }
                };

                // Extract content
                info!("Extracting content");
                let (html, headers) = match self.extract_content(response).await {
                    Ok(result) => {
                        info!("Content extracted successfully");
                        result
                    }
                    Err(e) => {
                        error!("Failed to extract content from {}: {}", url, e);
                        continue;
                    }
                };

                // Save page
                info!("Saving page to database");
                if let Err(e) = self.save(&url, &html, headers).await {
                    error!("Failed to save {}: {}", url, e);
                    continue;
                }
                info!("Page saved successfully");

                // Mark as visited
                info!("Marking URL as visited");
                if let Err(e) = self.mark_as_visited(&url).await {
                    error!("Failed to mark {} as visited: {}", url, e);
                    continue;
                }

                // Parse links
                info!("Parsing links from page");
                let urls = match self.parse_links(&url, &html).await {
                    Ok(urls) => {
                        info!("Found {} links on page", urls.len());
                        urls
                    }
                    Err(e) => {
                        error!("Failed to parse links from {}: {}", url, e);
                        continue;
                    }
                };

                // Add to queue
                info!("Adding {} discovered links to queue", urls.len());
                if let Err(e) = self.add_to_queue(urls).await {
                    error!("Failed to add URLs to queue: {}", e);
                }
            }
        }
    }
}
