use url::Url;
use crate::check_robots::Robot;

pub trait IAsyncCrawler {
    // Setup & Pre-flight
    async fn check_robot_policy(&self, url: &Url) -> anyhow::Result<bool>;
    async fn fetch_robot_txt(&self, url: &Url) -> anyhow::Result<Option<Robot>>;
    async fn set_robot_txt(&mut self,robot:Robot) -> anyhow::Result<()>;

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

    async fn start(&mut self, seed_url: Url) -> anyhow::Result<()> {
        self.add_to_queue(vec![seed_url]).await?;

        loop {
            let next_url = match self.next_queue().await {
                Some(url) => url,
                None => break Ok(()),
            };

            // Check robot policy, skip on error
            if !self.check_robot_policy(&next_url).await.unwrap_or(false) {
                continue;
            }

            if self.has_seen(&next_url).await {
                continue;
            }

            // Fetch page, log error and continue on failure
            let response = match self.fetch_page(&next_url).await {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("Failed to fetch {}: {}", next_url, e);
                    continue;
                }
            };

            // Extract content, log error and continue on failure
            let (html, headers) = match self.extract_content(response).await {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("Failed to extract content from {}: {}", next_url, e);
                    continue;
                }
            };

            // Save page, log error and continue on failure
            if let Err(e) = self.save(&next_url, &html, headers).await {
                eprintln!("Failed to save {}: {}", next_url, e);
                continue;
            }

            // Mark as visited, log error and continue on failure
            if let Err(e) = self.mark_as_visited(&next_url).await {
                eprintln!("Failed to mark {} as visited: {}", next_url, e);
                continue;
            }

            // Parse links, log error and continue on failure
            let urls = match self.parse_links(&next_url, &html).await {
                Ok(urls) => urls,
                Err(e) => {
                    eprintln!("Failed to parse links from {}: {}", next_url, e);
                    continue;
                }
            };

            // Add to queue, log error and continue on failure
            if let Err(e) = self.add_to_queue(urls).await {
                eprintln!("Failed to add URLs to queue: {}", e);
                continue;
            }
        }
    }
}
