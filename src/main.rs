use clap::Parser;

use crate::{cli::execute_commands, commands::Cli};

mod cli;
mod commands;
mod extract_links;
mod fetch;
mod utils;
mod extract_metadata;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    execute_commands(cli.command).await
}
