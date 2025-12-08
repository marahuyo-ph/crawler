use std::time::{Duration, SystemTime};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use tracing::{debug, info, warn, error};
use url::Url;

#[derive(Clone, Debug, Deserialize)]
pub struct FetchedPage {
    pub url: Url,
    pub final_url: Url,
    #[serde(skip)]
    pub status_code: u16,
    pub content_type: Option<String>,
    pub html_content: String,
    #[serde(skip)]
    pub parsed_html: Option<scraper::Html>,
    pub fetched_duration_ms: u128,
    pub timestamp: DateTime<Utc>,
}

impl FetchedPage {

  pub async fn fetch(client:&Client,url:&Url) -> anyhow::Result<Self> {
    let mut current_url = url.clone();
    let mut max_redirects = 5;
    let max_retries = 3;
    let mut retry_count = 0;
    let mut retry_delay = Duration::from_millis(500);
    let mut redirect_count = 0;
    
    let now = SystemTime::now();

    debug!("Starting fetch for URL: {}", url);

    loop {
      if max_redirects == 0 {
        error!("Too many redirects for URL: {}", url);
        return Err(anyhow!("Too many redirects"));
      }

      debug!("Attempting to fetch URL: {} (retry: {}/{})", current_url, retry_count, max_retries);

      let response = match client.get(current_url.clone()).send().await {
        Ok(resp) => {
          debug!("Received response from: {}, status: {}", current_url, resp.status());
          resp
        }
        Err(e) => {
          if retry_count < max_retries {
            retry_count += 1;
            warn!(
              error = %e,
              retry = retry_count,
              max_retries = max_retries,
              delay_ms = retry_delay.as_millis(),
              "Network error, retrying..."
            );
            tokio::time::sleep(retry_delay).await;
            retry_delay = Duration::from_millis(retry_delay.as_millis() as u64 * 2);
            continue;
          } else {
            error!(
              error = %e,
              max_retries = max_retries,
              "Failed to fetch after max retries"
            );
            return Err(anyhow!("Failed to fetch after {} retries: {}", max_retries, e));
          }
        }
      };

      // HTTP Status Codes: 200 OK, 301/302 Redirects, 404 Not Found, 500 Internal Server Error, 503 Service Unavailable
      match response.status() {
        StatusCode::OK => {
          info!("Successfully fetched URL: {}", current_url);
          
          let duration = now.elapsed()?;
          debug!("Total fetch duration: {}ms", duration.as_millis());

          let content_type = response
              .headers()
              .get("Content-Type")
              .ok_or(anyhow!("Unable to get Content-Type"))?
              .to_str()?
              .to_string();

          debug!("Content-Type: {}", content_type);

          let status_code = response.status();

          let html = response.text().await?;
          debug!("Parsed HTML content, size: {} bytes", html.len());

          let timestamp = Utc::now();

          let html_document = scraper::Html::parse_document(&html);

          info!(
            url = %current_url,
            duration_ms = duration.as_millis(),
            content_size_bytes = html.len(),
            "Fetch completed successfully"
          );

          return Ok(FetchedPage {
              url: url.clone(),
              final_url: current_url.clone(),
              status_code:status_code.as_u16(),
              content_type: Some(content_type),
              html_content: html.clone(),
              parsed_html: Some(html_document),
              fetched_duration_ms: duration.as_millis(),
              timestamp,
          });
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
          if retry_count < max_retries {
            retry_count += 1;
            warn!(
              status = response.status().as_u16(),
              retry = retry_count,
              max_retries = max_retries,
              delay_ms = retry_delay.as_millis(),
              "Server error, retrying..."
            );
            tokio::time::sleep(retry_delay).await;
            retry_delay = Duration::from_millis(retry_delay.as_millis() as u64 * 2);
            continue;
          } else {
            error!(
              status = response.status().as_u16(),
              max_retries = max_retries,
              "Server error after max retries"
            );
            return Err(anyhow!(
                "HTTP Error {} after {} retries: {}",
                response.status(),
                max_retries,
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
}
