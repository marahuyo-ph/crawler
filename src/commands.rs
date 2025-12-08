use clap::{Parser, Subcommand, ValueEnum};
use url::Url;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Fetch {
        /// Target URL to fetch (required)
        url: Url,
        /// Custom User-Agent string (default: "Marahuyo-Crawler/0.1.0")
        #[arg(long, default_value = "Marahuyo-Crawler/0.1.0")]
        user_agent: String,
        /// HTTP request timeout in seconds (default: 30)
        #[arg(long, default_value_t = 30)]
        timeout: i64,
        /// Max requests per minute per domain (default: 10, resets per CLI invocation)
        #[arg(long, default_value_t = 10)]
        rate_limit: i64,
        /// Output format: json or default text (default: text)
        #[arg(long, value_parser, default_value = "text")]
        output_format: OutputFormat,
    },
    ExtractLinks {
        /// Target URL to fetch (required)
        url: Url,
        /// Custom User-Agent string (default: "Marahuyo-Crawler/0.1.0")
        #[arg(long, default_value = "Marahuyo-Crawler/0.1.0")]
        user_agent: String,
        /// HTTP request timeout in seconds (default: 30)
        #[arg(long, default_value_t = 30)]
        timeout: i64,
        /// Max requests per minute per domain (default: 10, resets per CLI invocation)
        #[arg(long, default_value_t = 10)]
        rate_limit: i64,
        /// Only return internal links
        #[arg(long)]
        internal_only: bool,
        /// Only return external links
        #[arg(long)]
        external_only: bool,
        /// Output format: json or default text (default: text)
        #[arg(long, value_parser, default_value = "text")]
        output_format: OutputFormat,
    },
}

#[derive(Parser, Debug)]
#[command(author,version,about,long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
