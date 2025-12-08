use std::time::Duration;

use reqwest::ClientBuilder;

use crate::{commands::Commands, fetch::FetchedPage};

pub async fn execute_commands(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Fetch {
            url,
            user_agent,
            timeout,
            rate_limit,
            output_format,
        } => {

            let client = ClientBuilder::new()
              .user_agent(user_agent)
              .timeout(Duration::from_secs(timeout as u64))
              .build()?;

            // note:
            // per domain rate limit
            let page = FetchedPage::fetch(&client, &url).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use clap::Parser;

    use crate::cli::execute_commands;
    use crate::commands::Cli;
    use crate::utils::PythonServer;

    #[rstest::fixture]
    fn python_server() -> PythonServer {
        PythonServer::new(9001)
    }

    #[rstest::rstest]
    #[case("crawler fetch http://localhost:9001")]
    #[tokio::test]
    async fn test_cli(
        mut python_server: PythonServer,
        #[case] command: &str,
    ) -> anyhow::Result<()> {
        python_server.run()?;

        let args: Vec<&str> = command.split_whitespace().collect();
        let cli = Cli::parse_from(&args);

        if let Err(e) = execute_commands(cli.command).await {
            drop(python_server);
            return Err(e);
        }

        Ok(())
    }
}
