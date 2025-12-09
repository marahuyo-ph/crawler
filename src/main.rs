use clap::Parser;

use crate::{cli::execute_commands, commands::Cli};

mod check_robots;
mod cli;
mod commands;
mod extract_links;
mod extract_metadata;
mod fetch;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    execute_commands(cli.command).await
}
