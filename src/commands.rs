use clap::{Parser, Subcommand, ValueEnum};

use crate::crawlers::{sqlite::SqliteCrawlerOptions, stdout::StdOutCrawlerOptions};

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Crawl(SqliteCrawlerOptions),
}

#[derive(Parser, Debug)]
#[command(author,version,about,long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
