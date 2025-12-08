use scraper::Selector;
use tracing::{debug, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct ExtractLinks {
    /// Same domain as source URL
    pub internal: Vec<Url>,
    /// Different domain
    pub external: Vec<Url>,
    /// Email links
    pub mailto: Vec<Url>,
    /// Phone Number Links
    pub phone: Vec<Url>,
    /// Fragment links (#section)
    pub anchor: Vec<Url>,
    /// javascript: protocol
    pub javascript: Vec<Url>,
}

impl ExtractLinks {
    pub fn extract(url: &Url, document: &scraper::Html) -> anyhow::Result<Self> {
        let mut internal = Vec::new();
        let mut external = Vec::new();
        let mut mailto = Vec::new();
        let mut phone = Vec::new();
        let mut anchor = Vec::new();
        let mut javascript = Vec::new();

        let href_selector = Selector::parse("*[href]").unwrap();
        let source_domain = url.domain().unwrap_or("");

        debug!("Extracting links from: {}", url);

        for element in document.select(&href_selector) {
            if let Some(href) = element.value().attr("href") {
                // Skip empty hrefs
                if href.is_empty() {
                    continue;
                }

                // Categorize by protocol/type
                if href.starts_with("javascript:") {
                    debug!("Found javascript link: {}", href);
                    javascript.push(
                        Url::parse(href)
                            .unwrap_or_else(|_| Url::parse("javascript:void(0)").unwrap()),
                    );
                } else if href.starts_with("mailto:") {
                    debug!("Found mailto link: {}", href);
                    mailto.push(
                        Url::parse(href).unwrap_or_else(|_| Url::parse("mailto:invalid").unwrap()),
                    );
                } else if href.starts_with("tel:") {
                    debug!("Found phone link: {}", href);
                    phone.push(Url::parse(href).unwrap_or_else(|_| Url::parse("tel:+0").unwrap()));
                } else if href.starts_with("#") {
                    // Anchor/fragment link
                    debug!("Found anchor link: {}", href);
                    if let Ok(anchor_url) = url.join(href) {
                        anchor.push(anchor_url);
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
                            debug!("Found internal link: {}", parsed);
                            internal.push(parsed);
                        } else if !link_domain.is_empty() {
                            debug!("Found external link: {}", parsed);
                            external.push(parsed);
                        } else if href.starts_with('/')
                            || href.starts_with("./")
                            || href.starts_with("../")
                        {
                            // Relative path without domain - treat as internal
                            debug!("Found relative internal link: {}", parsed);
                            internal.push(parsed);
                        }
                    } else {
                        warn!("Failed to parse href: {}", href);
                    }
                }
            }
        }

        debug!(
            internal_count = internal.len(),
            external_count = external.len(),
            mailto_count = mailto.len(),
            phone_count = phone.len(),
            anchor_count = anchor.len(),
            javascript_count = javascript.len(),
            "Link extraction complete"
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
