use std::time::Duration;

use reqwest::ClientBuilder;
use serde_json::json;

use crate::{commands::Commands, fetch::FetchedPage};

pub async fn execute_commands(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Fetch {
            url,
            user_agent,
            timeout,
            rate_limit: _,
            output_format,
        } => {

            let client = ClientBuilder::new()
              .user_agent(user_agent)
              .timeout(Duration::from_secs(timeout as u64))
              .build()?;

            // note:
            // per domain rate limit
            let page = FetchedPage::fetch(&client, &url).await?;

            match output_format {
              crate::commands::OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "url": page.url.to_string(),
                    "final_url": page.final_url.to_string(),
                    "status_code": page.status_code,
                    "content_type": page.content_type,
                    "content_length": page.html_content.len(),
                    "html":page.html_content,
                    "fetched_duration_ms": page.fetched_duration_ms,
                    "timestamp": page.timestamp.to_rfc3339(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output)?);
              }
              crate::commands::OutputFormat::Text => {
                println!("╭─ Fetch Results ────────────────────────────────────────────");
                println!("├─ URL:                  {}", page.url);
                println!("├─ Final URL:            {}", page.final_url);
                println!("├─ Status Code:          {}", page.status_code);
                println!("├─ Content-Type:         {}", page.content_type.as_deref().unwrap_or("unknown"));
                println!("├─ Content Size:         {} bytes", page.html_content.len());
                println!("├─ Fetch Duration:       {} ms", page.fetched_duration_ms);
                println!("├─ Timestamp:            {}", page.timestamp.to_rfc3339());
                println!("╰─────────────────────────────────────────────────────────────");
              }
            }
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
