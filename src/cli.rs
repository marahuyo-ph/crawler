use std::time::Duration;

use reqwest::ClientBuilder;
use tracing::{debug, warn, error};

use crate::{
    check_robots::Robot,
    commands::Commands,
    extract_links::ExtractLinks,
    extract_metadata::PageMetadata,
    fetch::fetch_page,
    printer::pretty_printer,
};

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

            let page = fetch_page(&client, &url, 5, 3, Duration::from_secs(1)).await?;

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
                    let json_output = serde_json::json!({
                        "Fetch Results": {
                            "URL": page.url.to_string(),
                            "Final URL": page.final_url.to_string(),
                            "Status Code": page.status_code,
                            "Content-Type": page.content_type.as_deref().unwrap_or("unknown"),
                            "Content Size": format!("{} bytes", page.html_content.len()),
                            "Fetch Duration": format!("{} ms", page.fetched_duration_ms),
                            "Timestamp": page.timestamp.to_rfc3339(),
                        }
                    });
                    println!("{}", pretty_printer(json_output)?);
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

            let page = fetch_page(&client, &url, 5, 3, Duration::from_secs(1)).await?;

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
                            let mut links_array = Vec::new();
                            for link in &links.internal {
                                let mut link_obj = serde_json::json!({
                                    "URL": link.url.to_string(),
                                });
                                if !link.text.is_empty() {
                                    link_obj["Text"] = serde_json::json!(link.text);
                                }
                                if let Some(title) = &link.title {
                                    link_obj["Title"] = serde_json::json!(title);
                                }
                                if let Some(rel) = &link.rel {
                                    link_obj["Rel"] = serde_json::json!(rel);
                                }
                                if let Some(target) = &link.target {
                                    link_obj["Target"] = serde_json::json!(target);
                                }
                                links_array.push(link_obj);
                            }
                            let json_output = serde_json::json!({
                                "Internal Links": {
                                    "URL": page.final_url.to_string(),
                                    "Count": links.internal.len(),
                                    "Links": links_array
                                }
                            });
                            println!("{}", pretty_printer(json_output)?);
                        } else if external_only {
                            let mut links_array = Vec::new();
                            for link in &links.external {
                                let mut link_obj = serde_json::json!({
                                    "URL": link.url.to_string(),
                                });
                                if !link.text.is_empty() {
                                    link_obj["Text"] = serde_json::json!(link.text);
                                }
                                if let Some(title) = &link.title {
                                    link_obj["Title"] = serde_json::json!(title);
                                }
                                if let Some(rel) = &link.rel {
                                    link_obj["Rel"] = serde_json::json!(rel);
                                }
                                if let Some(target) = &link.target {
                                    link_obj["Target"] = serde_json::json!(target);
                                }
                                links_array.push(link_obj);
                            }
                            let json_output = serde_json::json!({
                                "External Links": {
                                    "URL": page.final_url.to_string(),
                                    "Count": links.external.len(),
                                    "Links": links_array
                                }
                            });
                            println!("{}", pretty_printer(json_output)?);
                        } else {
                            let mut all_links = serde_json::json!({
                                "URL": page.final_url.to_string(),
                            });

                            if !links.internal.is_empty() {
                                let mut internal_array = Vec::new();
                                for link in &links.internal {
                                    internal_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["Internal Links"] = serde_json::json!({
                                    "Count": links.internal.len(),
                                    "Links": internal_array
                                });
                            }

                            if !links.external.is_empty() {
                                let mut external_array = Vec::new();
                                for link in &links.external {
                                    external_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["External Links"] = serde_json::json!({
                                    "Count": links.external.len(),
                                    "Links": external_array
                                });
                            }

                            if !links.mailto.is_empty() {
                                let mut mailto_array = Vec::new();
                                for link in &links.mailto {
                                    mailto_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["Email Links"] = serde_json::json!({
                                    "Count": links.mailto.len(),
                                    "Links": mailto_array
                                });
                            }

                            if !links.phone.is_empty() {
                                let mut phone_array = Vec::new();
                                for link in &links.phone {
                                    phone_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["Phone Links"] = serde_json::json!({
                                    "Count": links.phone.len(),
                                    "Links": phone_array
                                });
                            }

                            if !links.anchor.is_empty() {
                                let mut anchor_array = Vec::new();
                                for link in &links.anchor {
                                    anchor_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["Anchor Links"] = serde_json::json!({
                                    "Count": links.anchor.len(),
                                    "Links": anchor_array
                                });
                            }

                            if !links.javascript.is_empty() {
                                let mut js_array = Vec::new();
                                for link in &links.javascript {
                                    js_array.push(serde_json::json!({
                                        "URL": link.url.to_string(),
                                        "Text": link.text,
                                    }));
                                }
                                all_links["JavaScript Links"] = serde_json::json!({
                                    "Count": links.javascript.len(),
                                    "Links": js_array
                                });
                            }

                            let json_output = serde_json::json!({
                                "All Links": all_links
                            });
                            println!("{}", pretty_printer(json_output)?);
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
        Commands::ExtractMetadata {
            url,
            user_agent,
            timeout,
            rate_limit: _,
            include,
            output_format,
        } => {
            let client = ClientBuilder::new()
                .user_agent(user_agent)
                .timeout(Duration::from_secs(timeout as u64))
                .danger_accept_invalid_certs(false)
                .build()?;

            let page = fetch_page(&client, &url, 5, 3, Duration::from_secs(1)).await?;

            if let Some(document) = page.parsed_html {
                let metadata = PageMetadata::extract(&document)?;

                match output_format {
                    crate::commands::OutputFormat::Json => {
                        let mut json_output = serde_json::json!({
                            "url": page.final_url.to_string(),
                            "basic": {
                                "title": metadata.basic.title,
                                "description": metadata.basic.description,
                                "keywords": metadata.basic.keywords,
                                "charset": metadata.basic.charset,
                                "language": metadata.basic.language,
                            },
                            "seo": {
                                "robots": metadata.seo.robots,
                                "canonical": metadata.seo.canonical.map(|u| u.to_string()),
                                "author": metadata.seo.author,
                                "publisher": metadata.seo.publisher,
                                "creator": metadata.seo.creator,
                            },
                            "open_graph": {
                                "og_type": metadata.open_graph.og_type,
                                "og_title": metadata.open_graph.og_title,
                                "og_description": metadata.open_graph.og_description,
                                "og_url": metadata.open_graph.og_url.map(|u| u.to_string()),
                                "og_image": metadata.open_graph.og_image.map(|u| u.to_string()),
                                "og_site_name": metadata.open_graph.og_site_name,
                                "og_locale": metadata.open_graph.og_locale,
                            },
                            "twitter_card": {
                                "twitter_card": metadata.twitter_card.twitter_card,
                                "twitter_title": metadata.twitter_card.twitter_title,
                                "twitter_description": metadata.twitter_card.twitter_description,
                                "twitter_url": metadata.twitter_card.twitter_url.map(|u| u.to_string()),
                                "twitter_image": metadata.twitter_card.twitter_image.map(|u| u.to_string()),
                            },
                            "viewport": {
                                "viewport": metadata.viewport.viewport,
                                "theme_color": metadata.viewport.theme_color,
                                "apple_mobile_web_app_capable": metadata.viewport.apple_mobile_web_app_capable,
                                "apple_mobile_web_app_status_bar_style": metadata.viewport.apple_mobile_web_app_status_bar_style,
                            },
                        });

                        // Add optional fields if requested
                        if include
                            .iter()
                            .any(|i| i.to_lowercase() == "hreflang" || i.to_lowercase() == "links")
                        {
                            let hreflang: std::collections::HashMap<String, String> = metadata
                                .links
                                .alternate_languages
                                .iter()
                                .map(|(k, v)| (k.clone(), v.to_string()))
                                .collect();
                            json_output["links"]["alternate_languages"] =
                                serde_json::to_value(hreflang)?;
                        }

                        if include
                            .iter()
                            .any(|i| i.to_lowercase() == "canonical" || i.to_lowercase() == "links")
                        {
                            json_output["links"]["canonical"] = serde_json::to_value(
                                metadata.links.canonical.map(|u| u.to_string()),
                            )?;
                        }

                        if include
                            .iter()
                            .any(|i| i.to_lowercase() == "author" || i.to_lowercase() == "links")
                        {
                            json_output["links"]["prev"] =
                                serde_json::to_value(metadata.links.prev.map(|u| u.to_string()))?;
                            json_output["links"]["next"] =
                                serde_json::to_value(metadata.links.next.map(|u| u.to_string()))?;
                        }

                        if include
                            .iter()
                            .any(|i| i.to_lowercase() == "publisher" || i.to_lowercase() == "links")
                        {
                            json_output["links"]["icon"] =
                                serde_json::to_value(metadata.links.icon.map(|u| u.to_string()))?;
                            json_output["links"]["apple_touch_icon"] = serde_json::to_value(
                                metadata.links.apple_touch_icon.map(|u| u.to_string()),
                            )?;
                        }

                        println!("{}", serde_json::to_string_pretty(&json_output)?);
                    }
                    crate::commands::OutputFormat::Text => {
                        let mut metadata_obj = serde_json::json!({
                            "URL": page.final_url.to_string(),
                            "Basic Metadata": {
                                "Title": metadata.basic.title,
                                "Description": metadata.basic.description,
                                "Keywords": metadata.basic.keywords.as_ref().map(|k| k.join(", ")),
                                "Charset": metadata.basic.charset,
                                "Language": metadata.basic.language,
                            },
                            "SEO Metadata": {
                                "Robots": metadata.seo.robots,
                                "Canonical": metadata.seo.canonical.map(|u| u.to_string()),
                                "Author": metadata.seo.author,
                                "Publisher": metadata.seo.publisher,
                                "Creator": metadata.seo.creator,
                            },
                        });

                        if metadata.open_graph.og_type.is_some()
                            || metadata.open_graph.og_title.is_some()
                            || metadata.open_graph.og_description.is_some()
                        {
                            metadata_obj["Open Graph"] = serde_json::json!({
                                "Type": metadata.open_graph.og_type,
                                "Title": metadata.open_graph.og_title,
                                "Description": metadata.open_graph.og_description,
                                "URL": metadata.open_graph.og_url.map(|u| u.to_string()),
                                "Image": metadata.open_graph.og_image.map(|u| u.to_string()),
                                "Site Name": metadata.open_graph.og_site_name,
                                "Locale": metadata.open_graph.og_locale,
                            });
                        }

                        if metadata.twitter_card.twitter_card.is_some()
                            || metadata.twitter_card.twitter_title.is_some()
                            || metadata.twitter_card.twitter_description.is_some()
                        {
                            metadata_obj["Twitter Card"] = serde_json::json!({
                                "Card Type": metadata.twitter_card.twitter_card,
                                "Title": metadata.twitter_card.twitter_title,
                                "Description": metadata.twitter_card.twitter_description,
                                "URL": metadata.twitter_card.twitter_url.map(|u| u.to_string()),
                                "Image": metadata.twitter_card.twitter_image.map(|u| u.to_string()),
                            });
                        }

                        if metadata.viewport.viewport.is_some()
                            || metadata.viewport.theme_color.is_some()
                            || metadata.viewport.apple_mobile_web_app_capable.is_some()
                        {
                            metadata_obj["Viewport & Mobile"] = serde_json::json!({
                                "Viewport": metadata.viewport.viewport,
                                "Theme Color": metadata.viewport.theme_color,
                                "Mobile Web App Capable": metadata.viewport.apple_mobile_web_app_capable,
                                "Status Bar Style": metadata.viewport.apple_mobile_web_app_status_bar_style,
                            });
                        }

                        if include.contains(&"links".to_string())
                            || include.iter().any(|i| {
                                i.to_lowercase() == "hreflang"
                                    || i.to_lowercase() == "canonical"
                                    || i.to_lowercase() == "author"
                                    || i.to_lowercase() == "publisher"
                            })
                        {
                            let mut links_obj = serde_json::json!({});
                            if let Some(canonical) = &metadata.links.canonical {
                                links_obj["Canonical"] = serde_json::json!(canonical.to_string());
                            }
                            if !metadata.links.alternate_languages.is_empty() {
                                let mut alt_langs = serde_json::Map::new();
                                for (lang, url) in &metadata.links.alternate_languages {
                                    alt_langs.insert(lang.clone(), serde_json::json!(url.to_string()));
                                }
                                links_obj["Alternate Languages"] = serde_json::Value::Object(alt_langs);
                            }
                            if let Some(prev) = &metadata.links.prev {
                                links_obj["Previous"] = serde_json::json!(prev.to_string());
                            }
                            if let Some(next) = &metadata.links.next {
                                links_obj["Next"] = serde_json::json!(next.to_string());
                            }
                            if let Some(icon) = &metadata.links.icon {
                                links_obj["Icon"] = serde_json::json!(icon.to_string());
                            }
                            if let Some(apple_icon) = &metadata.links.apple_touch_icon {
                                links_obj["Apple Touch Icon"] = serde_json::json!(apple_icon.to_string());
                            }
                            metadata_obj["Links"] = links_obj;
                        }

                        let json_output = serde_json::json!({
                            "Metadata": metadata_obj
                        });
                        println!("{}", pretty_printer(json_output)?);
                    }
                }
            } else {
                return Err(anyhow::anyhow!(
                    "HTML parsing failed: unable to parse content from {}",
                    page.final_url
                ));
            }
        },
        Commands::CheckRobot { url, user_agent, timeout, output_format } => {
            
            let client = ClientBuilder::new()
                .user_agent(&user_agent)
                .timeout(Duration::from_secs(timeout as u64))
                .danger_accept_invalid_certs(false)
                .build()?;

            let robots_url = url.join("robots.txt")?;
            debug!("Fetching robots.txt from: {}", robots_url);
            
            let response = match client.get(robots_url.clone()).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to fetch robots.txt: {}", e);
                    return Err(anyhow::anyhow!("Failed to fetch robots.txt: {}", e));
                }
            };

            let status = response.status();
            debug!("robots.txt response status: {}", status);

            // Handle different HTTP status codes per RFC 9309
            let robot = match status.as_u16() {
                404 => {
                    warn!("robots.txt not found (404) - treating as all paths allowed");
                    None
                }
                403 => {
                    warn!("robots.txt forbidden (403) - treating as all paths disallowed (conservative)");
                    // Return a special marker - we'll handle this in output
                    Some("FORBIDDEN".to_string())
                }
                200 => {
                    match response.text().await {
                        Ok(text) => {
                            debug!("Successfully fetched robots.txt (size: {} bytes)", text.len());
                            Some(text)
                        }
                        Err(e) => {
                            error!("Failed to read robots.txt response body: {}", e);
                            return Err(anyhow::anyhow!("Failed to read robots.txt: {}", e));
                        }
                    }
                }
                code => {
                    warn!("Unexpected robots.txt status code: {} - treating as all paths allowed", code);
                    None
                }
            };

            match output_format {
                crate::commands::OutputFormat::Json => {
                    match robot {
                        Some(robot_text) if robot_text == "FORBIDDEN" => {
                            let json_output = serde_json::json!({
                                "url": url.to_string(),
                                "user_agent": user_agent,
                                "status": "forbidden",
                                "message": "robots.txt returned 403 Forbidden - treating all paths as disallowed",
                                "crawl_delay": serde_json::Value::Null,
                                "request_rate": serde_json::Value::Null,
                                "sitemaps": Vec::<String>::new(),
                                "groups": Vec::<serde_json::Value>::new(),
                            });
                            println!("{}", serde_json::to_string_pretty(&json_output)?);
                        }
                        Some(robot_text) => {
                            let robot = Robot::new(robot_text);
                            let group_info = robot.get_group_info(&user_agent);
                            
                            let json_output = serde_json::json!({
                                "url": url.to_string(),
                                "user_agent": user_agent,
                                "status": "ok",
                                "matched_group": group_info.as_ref().map(|g| &g.user_agents),
                                "rule_count": group_info.as_ref().map(|g| g.rule_count).unwrap_or(0),
                                "allow_rules": group_info.as_ref().map(|g| g.allow_count).unwrap_or(0),
                                "disallow_rules": group_info.as_ref().map(|g| g.disallow_count).unwrap_or(0),
                                "crawl_delay": robot.crawl_delay(&user_agent),
                                "request_rate": robot.request_rate(&user_agent),
                                "sitemaps": robot.sitemaps(),
                            });
                            println!("{}", serde_json::to_string_pretty(&json_output)?);
                        }
                        None => {
                            let json_output = serde_json::json!({
                                "url": url.to_string(),
                                "user_agent": user_agent,
                                "status": "not_found",
                                "message": "robots.txt not found (404) - treating as all paths allowed",
                                "crawl_delay": serde_json::Value::Null,
                                "request_rate": serde_json::Value::Null,
                                "sitemaps": Vec::<String>::new(),
                                "groups": Vec::<serde_json::Value>::new(),
                            });
                            println!("{}", serde_json::to_string_pretty(&json_output)?);
                        }
                    }
                }
                crate::commands::OutputFormat::Text => {
                    match robot {
                        Some(robot_text) if robot_text == "FORBIDDEN" => {
                            let json_output = serde_json::json!({
                                "Robots.txt Check": {
                                    "URL": url.to_string(),
                                    "User-Agent": user_agent,
                                    "Status": "⚠️  FORBIDDEN (403)",
                                    "Behavior": "All paths are DISALLOWED (conservative)",
                                    "Reason": "robots.txt returned 403 Forbidden",
                                }
                            });
                            println!("{}", pretty_printer(json_output)?);
                        }
                        Some(robot_text) => {
                            let robot = Robot::new(robot_text);
                            let group_info = robot.get_group_info(&user_agent);
                            
                            let mut check_obj = serde_json::json!({
                                "URL": url.to_string(),
                                "User-Agent": user_agent,
                                "Status": "✓ OK",
                            });

                            if let Some(info) = group_info {
                                check_obj["Matched Group"] = serde_json::json!(format!("{:?}", info.user_agents));
                                check_obj["Rules Found"] = serde_json::json!(format!(
                                    "{} total ({} allow, {} disallow)",
                                    info.rule_count, info.allow_count, info.disallow_count
                                ));
                                
                                if let Some(delay) = info.crawl_delay {
                                    check_obj["Crawl-Delay"] = serde_json::json!(format!("{} seconds", delay));
                                }
                                if let Some(rate) = info.request_rate {
                                    check_obj["Request-Rate"] = serde_json::json!(format!("{} requests/second", rate));
                                }
                            } else {
                                check_obj["Matched Group"] = serde_json::json!("* (wildcard)");
                                check_obj["Rules Found"] = serde_json::json!("No specific rules for this user-agent");
                            }
                            
                            let sitemaps = robot.sitemaps();
                            if !sitemaps.is_empty() {
                                check_obj["Sitemaps"] = serde_json::json!(sitemaps);
                            }

                            let json_output = serde_json::json!({
                                "Robots.txt Check": check_obj
                            });
                            println!("{}", pretty_printer(json_output)?);
                        }
                        None => {
                            let json_output = serde_json::json!({
                                "Robots.txt Check": {
                                    "URL": url.to_string(),
                                    "User-Agent": user_agent,
                                    "Status": "ℹ️  NOT FOUND (404)",
                                    "Behavior": "All paths are ALLOWED",
                                    "Reason": "robots.txt not found, default is permissive",
                                    "Crawl-Delay": "(not specified)",
                                    "Request-Rate": "(not specified)",
                                    "Sitemaps": "(none found)",
                                }
                            });
                            println!("{}", pretty_printer(json_output)?);
                        }
                    }
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
