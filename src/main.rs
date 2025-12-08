use clap::Parser;

use crate::{cli::execute_commands, commands::Cli};

mod cli;
mod commands;
mod fetch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    execute_commands(cli.command).await
}
