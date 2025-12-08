use std::time::Duration;

use reqwest::ClientBuilder;

use crate::{commands::Commands, extract_links::ExtractLinks, fetch::FetchedPage};

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
                .danger_accept_invalid_certs(false)
                .build()?;

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
                    println!(
                        "├─ Content-Type:         {}",
                        page.content_type.as_deref().unwrap_or("unknown")
                    );
                    println!("├─ Content Size:         {} bytes", page.html_content.len());
                    println!("├─ Fetch Duration:       {} ms", page.fetched_duration_ms);
                    println!("├─ Timestamp:            {}", page.timestamp.to_rfc3339());
                    println!("╰─────────────────────────────────────────────────────────────");
                }
            }
        }
        Commands::ExtractLinks {
            url,
            user_agent,
            timeout,
            rate_limit: _,
            internal_only,
            external_only,
            output_format,
        } => {
            let client = ClientBuilder::new()
                .user_agent(user_agent)
                .timeout(Duration::from_secs(timeout as u64))
                .danger_accept_invalid_certs(false)
                .build()?;

            let page = FetchedPage::fetch(&client, &url).await?;

            if let Some(document) = page.parsed_html {
                let links = ExtractLinks::extract(&page.final_url, &document)?;

                match output_format {
                    crate::commands::OutputFormat::Json => {
                        let json_output = if internal_only {
                            serde_json::json!({
                                "url": page.final_url.to_string(),
                                "type": "internal_links",
                                "count": links.internal.len(),
                                "links": links.internal
                            })
                        } else if external_only {
                            serde_json::json!({
                                "url": page.final_url.to_string(),
                                "type": "external_links",
                                "count": links.external.len(),
                                "links": links.external
                            })
                        } else {
                            serde_json::json!({
                                "url": page.final_url.to_string(),
                                "internal": {
                                    "count": links.internal.len(),
                                    "links": links.internal
                                },
                                "external": {
                                    "count": links.external.len(),
                                    "links": links.external
                                },
                                "mailto": {
                                    "count": links.mailto.len(),
                                    "links": links.mailto
                                },
                                "phone": {
                                    "count": links.phone.len(),
                                    "links": links.phone
                                },
                                "anchor": {
                                    "count": links.anchor.len(),
                                    "links": links.anchor
                                },
                                "javascript": {
                                    "count": links.javascript.len(),
                                    "links": links.javascript
                                }
                            })
                        };
                        println!("{}", serde_json::to_string_pretty(&json_output)?);
                    }
                    crate::commands::OutputFormat::Text => {
                        if internal_only {
                            println!("╭─ Internal Links ───────────────────────────────────────────");
                            println!("├─ URL:    {}", page.final_url);
                            println!("├─ Count:  {}", links.internal.len());
                            println!("├─ Links:");
                            for link in &links.internal {
                                println!("│  ├─ URL:  {}", link.url);
                                if !link.text.is_empty() {
                                    println!("│  ├─ Text: {}", link.text);
                                }
                                if let Some(title) = &link.title {
                                    println!("│  ├─ Title: {}", title);
                                }
                                if let Some(rel) = &link.rel {
                                    println!("│  ├─ Rel:  {}", rel);
                                }
                                if let Some(target) = &link.target {
                                    println!("│  └─ Target: {}", target);
                                }
                            }
                            println!("╰─────────────────────────────────────────────────────────────");
                        } else if external_only {
                            println!("╭─ External Links ───────────────────────────────────────────");
                            println!("├─ URL:    {}", page.final_url);
                            println!("├─ Count:  {}", links.external.len());
                            println!("├─ Links:");
                            for link in &links.external {
                                println!("│  ├─ URL:  {}", link.url);
                                if !link.text.is_empty() {
                                    println!("│  ├─ Text: {}", link.text);
                                }
                                if let Some(title) = &link.title {
                                    println!("│  ├─ Title: {}", title);
                                }
                                if let Some(rel) = &link.rel {
                                    println!("│  ├─ Rel:  {}", rel);
                                }
                                if let Some(target) = &link.target {
                                    println!("│  └─ Target: {}", target);
                                }
                            }
                            println!("╰─────────────────────────────────────────────────────────────");
                        } else {
                            println!("╭─ All Links ────────────────────────────────────────────────");
                            println!("├─ URL:    {}", page.final_url);
                            println!("│");
                            
                            if !links.internal.is_empty() {
                                println!("├─ Internal Links ({}):", links.internal.len());
                                for link in &links.internal {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                                println!("│");
                            }

                            if !links.external.is_empty() {
                                println!("├─ External Links ({}):", links.external.len());
                                for link in &links.external {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                                println!("│");
                            }

                            if !links.mailto.is_empty() {
                                println!("├─ Email Links ({}):", links.mailto.len());
                                for link in &links.mailto {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                                println!("│");
                            }

                            if !links.phone.is_empty() {
                                println!("├─ Phone Links ({}):", links.phone.len());
                                for link in &links.phone {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                                println!("│");
                            }

                            if !links.anchor.is_empty() {
                                println!("├─ Anchor Links ({}):", links.anchor.len());
                                for link in &links.anchor {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                                println!("│");
                            }

                            if !links.javascript.is_empty() {
                                println!("├─ JavaScript Links ({}):", links.javascript.len());
                                for link in &links.javascript {
                                    println!("│  ├─ URL:  {}", link.url);
                                    if !link.text.is_empty() {
                                        println!("│  │  └─ Text: {}", link.text);
                                    }
                                }
                            }
                            println!("╰─────────────────────────────────────────────────────────────");
                        }
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "HTML parsing failed: unable to parse content from {}",
                    page.final_url
                ));
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
