use scraper::Selector;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, warn};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct LinkInfo {
    pub url: String,
    pub text: String,
    pub title: Option<String>,
    pub rel: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtractLinks {
    /// Same domain as source URL
    pub internal: Vec<LinkInfo>,
    /// Different domain
    pub external: Vec<LinkInfo>,
    /// Email links
    pub mailto: Vec<LinkInfo>,
    /// Phone Number Links
    pub phone: Vec<LinkInfo>,
    /// Fragment links (#section)
    pub anchor: Vec<LinkInfo>,
    /// javascript: protocol
    pub javascript: Vec<LinkInfo>,
}

impl ExtractLinks {
    pub fn extract(url: &Url, document: &scraper::Html) -> anyhow::Result<Self> {
        let mut internal = Vec::new();
        let mut external = Vec::new();
        let mut mailto = Vec::new();
        let mut phone = Vec::new();
        let mut anchor = Vec::new();
        let mut javascript = Vec::new();

        let href_selector = Selector::parse("a[href]").unwrap();
        let source_domain = url.domain().unwrap_or("");

        debug!("Extracting links from: {}", url);

        for element in document.select(&href_selector) {
            if let Some(href) = element.value().attr("href") {
                // Skip empty hrefs
                if href.is_empty() {
                    continue;
                }

                // Extract link text and attributes
                let text = element
                    .text()
                    .collect::<String>()
                    .trim()
                    .chars()
                    .take(100)
                    .collect::<String>();

                let title = element.value().attr("title").map(|s| s.to_string());
                let rel = element.value().attr("rel").map(|s| s.to_string());
                let target = element.value().attr("target").map(|s| s.to_string());

                // Create LinkInfo helper
                let create_link_info = |url_str: String| -> LinkInfo {
                    LinkInfo {
                        url: url_str,
                        text: text.clone(),
                        title: title.clone(),
                        rel: rel.clone(),
                        target: target.clone(),
                    }
                };

                // Categorize by protocol/type
                if href.starts_with("javascript:") {
                    debug!("Found javascript link: {} (text: {})", href, text);
                    javascript.push(create_link_info(href.to_string()));
                } else if href.starts_with("mailto:") {
                    debug!("Found mailto link: {} (text: {})", href, text);
                    mailto.push(create_link_info(href.to_string()));
                } else if href.starts_with("tel:") {
                    debug!("Found phone link: {} (text: {})", href, text);
                    phone.push(create_link_info(href.to_string()));
                } else if href.starts_with("#") {
                    // Anchor/fragment link
                    debug!("Found anchor link: {} (text: {})", href, text);
                    if let Ok(anchor_url) = url.join(href) {
                        anchor.push(create_link_info(anchor_url.to_string()));
                    }
                } else {
                    // Try to parse as absolute or relative URL
                    let parsed_url = if href.starts_with("http://") || href.starts_with("https://")
                    {
                        Url::parse(href)
                    } else {
                        url.join(href)
                    };

                    if let Ok(parsed) = parsed_url {
                        let link_domain = parsed.domain().unwrap_or("");

                        if link_domain == source_domain && !source_domain.is_empty() {
                            debug!("Found internal link: {} (text: {})", parsed, text);
                            internal.push(create_link_info(parsed.to_string()));
                        } else if !link_domain.is_empty() {
                            debug!("Found external link: {} (text: {})", parsed, text);
                            external.push(create_link_info(parsed.to_string()));
                        } else if href.starts_with('/')
                            || href.starts_with("./")
                            || href.starts_with("../")
                        {
                            // Relative path without domain - treat as internal
                            debug!("Found relative internal link: {} (text: {})", parsed, text);
                            internal.push(create_link_info(parsed.to_string()));
                        }
                    } else {
                        warn!("Failed to parse href: {}", href);
                    }
                }
            }
        }

        // Track counts before deduplication
        let internal_before = internal.len();
        let external_before = external.len();
        let mailto_before = mailto.len();
        let phone_before = phone.len();
        let anchor_before = anchor.len();
        let javascript_before = javascript.len();

        // Deduplicate links using HashSet based on URL
        internal = internal
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        external = external
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        mailto = mailto
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        phone = phone
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        anchor = anchor
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        javascript = javascript
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Track counts after deduplication
        let internal_after = internal.len();
        let external_after = external.len();
        let mailto_after = mailto.len();
        let phone_after = phone.len();
        let anchor_after = anchor.len();
        let javascript_after = javascript.len();

        debug!(
            internal_count = %format!("{} -> {}", internal_before, internal_after),
            external_count = %format!("{} -> {}", external_before, external_after),
            mailto_count = %format!("{} -> {}", mailto_before, mailto_after),
            phone_count = %format!("{} -> {}", phone_before, phone_after),
            anchor_count = %format!("{} -> {}", anchor_before, anchor_after),
            javascript_count = %format!("{} -> {}", javascript_before, javascript_after),
            duplicates_removed = %format!("{} total", (internal_before - internal_after) + (external_before - external_after) + (mailto_before - mailto_after) + (phone_before - phone_after) + (anchor_before - anchor_after) + (javascript_before - javascript_after)),
            "Link extraction and deduplication complete"
        );

        Ok(ExtractLinks {
            internal,
            external,
            mailto,
            phone,
            anchor,
            javascript,
        })
    }
}
