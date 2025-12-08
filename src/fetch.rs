use chrono::{DateTime, Utc};
use url::Url;

use crate::commands::Commands;

#[derive(Clone, Debug)]
pub struct FetchedPage {
    pub url: Url,
    pub final_url: Url,
    pub status_code: i16,
    pub content_type: Option<String>,
    pub html_content: String,
    pub parsed_html: Option<String>,
    pub fetched_duration_ms: u16,
    pub timestamp: DateTime<Utc>,
}

pub async fn execute_fetch(
    Commands::Fetch {
        url,
        user_agent,
        timeout,
        rate_limit,
        output_format,
    }: Commands,
) -> anyhow::Result<()> {
    println!("trying to fetch {}", url);

    Ok(())
}
