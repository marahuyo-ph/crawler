use clap::Parser;

use crate::{cli::execute_commands, commands::Cli, fetch::execute_fetch};

mod cli;
mod commands;
mod fetch;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    execute_commands(cli.command).await
}

#[cfg(test)]
mod test {

    // cli testing
    async fn setup() -> anyhow::Result<()> {
        // run python -m http.server at test-site
        Ok(())
    }

    async fn teardown() -> anyhow::Result<()> {
        // shutdown the python server
        Ok(())
    }
}
