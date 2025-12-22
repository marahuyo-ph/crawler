use clap::Parser;
use tracing_subscriber::{self,EnvFilter, fmt, prelude::*};

use crate::{
    cli::execute_commands, commands::Cli, crawlers::stdout::StdOutCrawler, traits::IAsyncCrawler,
};

mod check_robots;
mod cli;
mod commands;
mod crawlers;
mod extract_links;
mod models;
mod traits;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with env filter
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true),
        )
        .init();

    let cli = Cli::parse();

    execute_commands(cli).await
}
