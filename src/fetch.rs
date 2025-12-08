use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use url::Url;

use crate::commands::Commands;

#[derive(Clone, Debug)]
pub struct FetchedPage {
    pub url: Url,
    pub final_url: Url,
    pub status_code: StatusCode,
    pub content_type: Option<String>,
    pub html_content: String,
    pub parsed_html: Option<String>,
    pub fetched_duration_ms: u128,
    pub timestamp: DateTime<Utc>,
}

impl FetchedPage {

  pub async fn fetch(client:&Client,url:&Url) -> anyhow::Result<Self> {

    let now = SystemTime::now();

    let response = client.get(url.clone()).send().await?;

    let duration = now.elapsed()?;

    // HTTP Status Codes: 200 OK, 301/302 Redirects, 404 Not Found, 500 Internal Server Error, 503 Service Unavailable
    if response.status() != StatusCode::OK {
        todo!("handle error here")
    }

    let content_type = response
        .headers()
        .get("Content-Type")
        .ok_or(anyhow!("Unable to get Content-Type"))?
        .to_str()?
        .to_string();

    let status_code = response.status();

    let html = response.text().await?;

    let timestamp = Utc::now();

    Ok(FetchedPage {
        url: url.clone(),
        final_url: url.clone(),
        status_code,
        content_type: Some(content_type),
        html_content: html.clone(),
        parsed_html: Some(html),
        fetched_duration_ms: duration.as_millis(),
        timestamp,
    })
  }
}