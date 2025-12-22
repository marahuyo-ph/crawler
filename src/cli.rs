use std::time::Duration;

use reqwest::ClientBuilder;
use tracing::{debug, error, warn};

use crate::{
    check_robots::Robot, commands::{Cli, Commands}, crawlers::stdout::StdOutCrawler, extract_links::ExtractLinks,  traits::IAsyncCrawler
};

pub async fn execute_commands(cli: Cli) -> anyhow::Result<()> {

    let client = ClientBuilder::new()
        .build()?;

    match cli.command {
        Commands::Crawl(options) => {
            let mut crawler = StdOutCrawler::new(&client);
            crawler.start(options.url).await?;
        }
    }

    Ok(())
}