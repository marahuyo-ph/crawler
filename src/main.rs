use clap::Parser;
use url::Url;

use crate::{cli::execute_commands, commands::Cli, crawlers::stdout::StdOutCrawler, traits::IAsyncCrawler};

mod check_robots;
mod cli;
mod commands;
mod crawlers;
mod extract_links;
mod extract_metadata;
mod fetch;
mod printer;
mod traits;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    execute_commands(cli).await
}
