use std::time::Duration;

use reqwest::ClientBuilder;
use tracing::{debug, warn, error};

use crate::{
    check_robots::Robot,
    commands::Commands,
    extract_links::ExtractLinks,
    extract_metadata::PageMetadata,
    fetch::fetch_page,
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
                            println!(
                                "╭─ Internal Links ───────────────────────────────────────────"
                            );
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
                            println!(
                                "╰─────────────────────────────────────────────────────────────"
                            );
                        } else if external_only {
                            println!(
                                "╭─ External Links ───────────────────────────────────────────"
                            );
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
                            println!(
                                "╰─────────────────────────────────────────────────────────────"
                            );
                        } else {
                            println!(
                                "╭─ All Links ────────────────────────────────────────────────"
                            );
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
                            println!(
                                "╰─────────────────────────────────────────────────────────────"
                            );
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
                        println!("╭─ Metadata ─────────────────────────────────────────────────");
                        println!("├─ URL: {}", page.final_url);
                        println!("│");

                        // Basic Metadata
                        println!("├─ Basic Metadata:");
                        if let Some(title) = &metadata.basic.title {
                            println!("│  ├─ Title:       {}", title);
                        }
                        if let Some(desc) = &metadata.basic.description {
                            println!("│  ├─ Description: {}", desc);
                        }
                        if let Some(keywords) = &metadata.basic.keywords {
                            println!("│  ├─ Keywords:    {}", keywords.join(", "));
                        }
                        if let Some(charset) = &metadata.basic.charset {
                            println!("│  ├─ Charset:     {}", charset);
                        }
                        if let Some(lang) = &metadata.basic.language {
                            println!("│  └─ Language:    {}", lang);
                        }
                        println!("│");

                        // SEO Metadata
                        println!("├─ SEO Metadata:");
                        if let Some(robots) = &metadata.seo.robots {
                            println!("│  ├─ Robots:      {}", robots);
                        }
                        if let Some(canonical) = &metadata.seo.canonical {
                            println!("│  ├─ Canonical:   {}", canonical);
                        }
                        if let Some(author) = &metadata.seo.author {
                            println!("│  ├─ Author:      {}", author);
                        }
                        if let Some(publisher) = &metadata.seo.publisher {
                            println!("│  ├─ Publisher:   {}", publisher);
                        }
                        if let Some(creator) = &metadata.seo.creator {
                            println!("│  └─ Creator:     {}", creator);
                        }
                        println!("│");

                        // Open Graph
                        if metadata.open_graph.og_type.is_some()
                            || metadata.open_graph.og_title.is_some()
                            || metadata.open_graph.og_description.is_some()
                        {
                            println!("├─ Open Graph:");
                            if let Some(og_type) = &metadata.open_graph.og_type {
                                println!("│  ├─ Type:        {}", og_type);
                            }
                            if let Some(og_title) = &metadata.open_graph.og_title {
                                println!("│  ├─ Title:       {}", og_title);
                            }
                            if let Some(og_desc) = &metadata.open_graph.og_description {
                                println!("│  ├─ Description: {}", og_desc);
                            }
                            if let Some(og_url) = &metadata.open_graph.og_url {
                                println!("│  ├─ URL:         {}", og_url);
                            }
                            if let Some(og_image) = &metadata.open_graph.og_image {
                                println!("│  ├─ Image:       {}", og_image);
                            }
                            if let Some(og_site) = &metadata.open_graph.og_site_name {
                                println!("│  ├─ Site Name:   {}", og_site);
                            }
                            if let Some(og_locale) = &metadata.open_graph.og_locale {
                                println!("│  └─ Locale:      {}", og_locale);
                            }
                            println!("│");
                        }

                        // Twitter Card
                        if metadata.twitter_card.twitter_card.is_some()
                            || metadata.twitter_card.twitter_title.is_some()
                            || metadata.twitter_card.twitter_description.is_some()
                        {
                            println!("├─ Twitter Card:");
                            if let Some(card) = &metadata.twitter_card.twitter_card {
                                println!("│  ├─ Card Type:   {}", card);
                            }
                            if let Some(tw_title) = &metadata.twitter_card.twitter_title {
                                println!("│  ├─ Title:       {}", tw_title);
                            }
                            if let Some(tw_desc) = &metadata.twitter_card.twitter_description {
                                println!("│  ├─ Description: {}", tw_desc);
                            }
                            if let Some(tw_url) = &metadata.twitter_card.twitter_url {
                                println!("│  ├─ URL:         {}", tw_url);
                            }
                            if let Some(tw_image) = &metadata.twitter_card.twitter_image {
                                println!("│  └─ Image:       {}", tw_image);
                            }
                            println!("│");
                        }

                        // Viewport
                        if metadata.viewport.viewport.is_some()
                            || metadata.viewport.theme_color.is_some()
                            || metadata.viewport.apple_mobile_web_app_capable.is_some()
                        {
                            println!("├─ Viewport & Mobile:");
                            if let Some(vp) = &metadata.viewport.viewport {
                                println!("│  ├─ Viewport:                {}", vp);
                            }
                            if let Some(theme) = &metadata.viewport.theme_color {
                                println!("│  ├─ Theme Color:             {}", theme);
                            }
                            if let Some(capable) = metadata.viewport.apple_mobile_web_app_capable {
                                println!("│  ├─ Mobile Web App Capable:  {}", capable);
                            }
                            if let Some(status) =
                                &metadata.viewport.apple_mobile_web_app_status_bar_style
                            {
                                println!("│  └─ Status Bar Style:        {}", status);
                            }
                            println!("│");
                        }

                        // Link Metadata (if requested)
                        if include.contains(&"links".to_string())
                            || include.iter().any(|i| {
                                i.to_lowercase() == "hreflang"
                                    || i.to_lowercase() == "canonical"
                                    || i.to_lowercase() == "author"
                                    || i.to_lowercase() == "publisher"
                            })
                        {
                            println!("├─ Links:");
                            if let Some(canonical) = &metadata.links.canonical {
                                println!("│  ├─ Canonical:        {}", canonical);
                            }
                            if !metadata.links.alternate_languages.is_empty() {
                                println!("│  ├─ Alternate Languages:");
                                for (lang, url) in &metadata.links.alternate_languages {
                                    println!("│  │  ├─ {}: {}", lang, url);
                                }
                            }
                            if let Some(prev) = &metadata.links.prev {
                                println!("│  ├─ Previous:         {}", prev);
                            }
                            if let Some(next) = &metadata.links.next {
                                println!("│  ├─ Next:             {}", next);
                            }
                            if let Some(icon) = &metadata.links.icon {
                                println!("│  ├─ Icon:             {}", icon);
                            }
                            if let Some(apple_icon) = &metadata.links.apple_touch_icon {
                                println!("│  └─ Apple Touch Icon: {}", apple_icon);
                            }
                        }

                        println!("╰─────────────────────────────────────────────────────────────");
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
                            println!("╭─ Robots.txt Check ────────────────────────────────────────");
                            println!("├─ URL:            {}", url);
                            println!("├─ User-Agent:     {}", user_agent);
                            println!("├─ Status:         ⚠️  FORBIDDEN (403)");
                            println!("├─ Behavior:       All paths are DISALLOWED (conservative)");
                            println!("├─ Reason:         robots.txt returned 403 Forbidden");
                            println!("╰─────────────────────────────────────────────────────────────");
                        }
                        Some(robot_text) => {
                            let robot = Robot::new(robot_text);
                            let group_info = robot.get_group_info(&user_agent);
                            
                            println!("╭─ Robots.txt Check ────────────────────────────────────────");
                            println!("├─ URL:            {}", url);
                            println!("├─ User-Agent:     {}", user_agent);
                            println!("├─ Status:         ✓ OK");
                            
                            if let Some(info) = group_info {
                                println!("├─ Matched Group:  {:?}", info.user_agents);
                                println!("├─ Rules Found:    {} total ({} allow, {} disallow)", 
                                    info.rule_count, info.allow_count, info.disallow_count);
                                
                                if let Some(delay) = info.crawl_delay {
                                    println!("├─ Crawl-Delay:    {} seconds", delay);
                                }
                                if let Some(rate) = info.request_rate {
                                    println!("├─ Request-Rate:   {} requests/second", rate);
                                }
                            } else {
                                println!("├─ Matched Group:  * (wildcard)");
                                println!("├─ Rules Found:    No specific rules for this user-agent");
                            }
                            
                            let sitemaps = robot.sitemaps();
                            if !sitemaps.is_empty() {
                                println!("├─ Sitemaps:       {} found", sitemaps.len());
                                for sitemap in sitemaps {
                                    println!("│  ├─ {}", sitemap);
                                }
                            }
                            println!("╰─────────────────────────────────────────────────────────────");
                        }
                        None => {
                            println!("╭─ Robots.txt Check ────────────────────────────────────────");
                            println!("├─ URL:            {}", url);
                            println!("├─ User-Agent:     {}", user_agent);
                            println!("├─ Status:         ℹ️  NOT FOUND (404)");
                            println!("├─ Behavior:       All paths are ALLOWED");
                            println!("├─ Reason:         robots.txt not found, default is permissive");
                            println!("├─ Crawl-Delay:    (not specified)");
                            println!("├─ Request-Rate:   (not specified)");
                            println!("├─ Sitemaps:       (none found)");
                            println!("╰─────────────────────────────────────────────────────────────");
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
