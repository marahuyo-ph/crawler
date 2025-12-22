
use reqwest::ClientBuilder;
use sea_orm::{ActiveModelTrait, Database};

use crate::{
    commands::{Cli, Commands},
    crawlers::sqlite::SqliteCrawler,
    models::prelude::*,
    traits::IAsyncCrawler,
};

pub async fn execute_commands(cli: Cli) -> anyhow::Result<()> {
    let client = ClientBuilder::new().build()?;

    match cli.command {
        Commands::Crawl(options) => {
            let database = Database::connect(&options.database_url).await?;
            
            // migrate
            SqliteCrawler::migrate(&database).await?;

            // Create or retrieve crawl session
            let session_id = if let Some(existing_id) = options.crawl_session_id {
                // Resume existing session
                existing_id
            } else {
                // Create a new crawl session
                let start_url = options
                    .urls
                    .first()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let crawl_session = crate::models::crawl_sessions::ActiveModel {
                    start_url: sea_orm::Set(start_url),
                    status: sea_orm::Set("running".to_string()),
                    pages_crawled: sea_orm::Set(0),
                    errors_encountered: sea_orm::Set(0),
                    ..Default::default()
                };

                let session = crawl_session.insert(&database).await?;
                session.id
            };

            let mut crawler = SqliteCrawler::new(&client, database, session_id);

            crawler.start(options.urls).await?;
        }
    }

    Ok(())
}
