use clap::Args;

use crate::traits::IAsyncCrawler;

#[derive(Debug,Clone,Args)]
pub struct SqliteCrawlerOptions {

}

pub struct SqliteCrawler {}

impl SqliteCrawler {
  pub fn new() -> Self {
    todo!("")
  }
}

impl IAsyncCrawler for SqliteCrawler {
    async fn check_robot_policy(&self, url: &url::Url) -> anyhow::Result<bool> {
        todo!()
    }

    async fn fetch_robot_txt(&self, url: &url::Url) -> anyhow::Result<Option<crate::check_robots::Robot>> {
        todo!()
    }

    async fn set_robot_txt(&mut self,robot:crate::check_robots::Robot) -> anyhow::Result<()> {
        todo!()
    }

    async fn fetch_page(&self, url: &url::Url) -> anyhow::Result<reqwest::Response> {
        todo!()
    }

    async fn parse_links(&self, url: &url::Url, html: &scraper::Html) -> anyhow::Result<Vec<url::Url>> {
        todo!()
    }

    async fn extract_content(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<(scraper::Html, reqwest::header::HeaderMap)> {
        todo!()
    }

    async fn add_to_queue(&mut self, urls: Vec<url::Url>) -> anyhow::Result<()> {
        todo!()
    }

    async fn next_queue(&mut self) -> Option<url::Url> {
        todo!()
    }

    async fn has_seen(&self, url: &url::Url) -> bool {
        todo!()
    }

    async fn mark_as_visited(&mut self, url: &url::Url) -> anyhow::Result<()> {
        todo!()
    }

    async fn save(
        &self,
        url: &url::Url,
        html: &scraper::Html,
        header: reqwest::header::HeaderMap,
    ) -> anyhow::Result<()> {
        todo!()
    }
    
    async fn get_robot_txt(&self) -> anyhow::Result<Option<crate::check_robots::Robot>> {
        todo!()
    }
}