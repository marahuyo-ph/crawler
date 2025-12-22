use clap::{Parser, Subcommand, ValueEnum};
use url::Url;

use crate::crawlers::stdout::StdOutCrawlerOptions;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Crawl(StdOutCrawlerOptions)
}

#[derive(Parser, Debug)]
#[command(author,version,about,long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
