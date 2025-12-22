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
    pub fn new() -> Self {
        ExtractLinks {
            internal: Vec::new(),
            external: Vec::new(),
            mailto: Vec::new(),
            phone: Vec::new(),
            anchor: Vec::new(),
            javascript: Vec::new(),
        }
    }

    pub fn extract(url: &Url, document: &scraper::Html) -> anyhow::Result<Self> {
        let mut extractor = Self::new();
        extractor.parse(url, document)?;
        extractor.deduplicate();
        Ok(extractor)
    }

    /// Parse all links from the HTML document
    pub fn parse(&mut self, url: &Url, document: &scraper::Html) -> anyhow::Result<()> {
        let href_selector = Selector::parse("a[href]").unwrap();
        let source_domain = url.domain().unwrap_or("");

        debug!("Extracting links from: {}", url);

        for element in document.select(&href_selector) {
            if let Some(href) = element.value().attr("href") {
                if href.is_empty() {
                    continue;
                }

                let link_info = self.create_link_info(&element, href);
                self.categorize_link(url, href, source_domain, link_info);
            }
        }

        Ok(())
    }

    /// Create LinkInfo from an anchor element
    fn create_link_info(&self, element: &scraper::element_ref::ElementRef, _href: &str) -> LinkInfo {
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

        LinkInfo {
            url: String::new(), // Will be set by categorize_link
            text,
            title,
            rel,
            target,
        }
    }

    /// Categorize link by type and add to appropriate vector
    fn categorize_link(
        &mut self,
        url: &Url,
        href: &str,
        source_domain: &str,
        mut link_info: LinkInfo,
    ) {
        if href.starts_with("javascript:") {
            debug!("Found javascript link: {} (text: {})", href, link_info.text);
            link_info.url = href.to_string();
            self.javascript.push(link_info);
        } else if href.starts_with("mailto:") {
            debug!("Found mailto link: {} (text: {})", href, link_info.text);
            link_info.url = href.to_string();
            self.mailto.push(link_info);
        } else if href.starts_with("tel:") {
            debug!("Found phone link: {} (text: {})", href, link_info.text);
            link_info.url = href.to_string();
            self.phone.push(link_info);
        } else if href.starts_with("#") {
            self.add_anchor_link(url, href, link_info);
        } else {
            self.add_web_link(url, href, source_domain, link_info);
        }
    }

    /// Handle anchor/fragment links
    fn add_anchor_link(&mut self, url: &Url, href: &str, mut link_info: LinkInfo) {
        debug!("Found anchor link: {} (text: {})", href, link_info.text);
        if let Ok(anchor_url) = url.join(href) {
            link_info.url = anchor_url.to_string();
            self.anchor.push(link_info);
        }
    }

    /// Handle HTTP/HTTPS web links, categorizing as internal or external
    fn add_web_link(
        &mut self,
        url: &Url,
        href: &str,
        source_domain: &str,
        mut link_info: LinkInfo,
    ) {
        let parsed_url = if href.starts_with("http://") || href.starts_with("https://") {
            Url::parse(href)
        } else {
            url.join(href)
        };

        if let Ok(parsed) = parsed_url {
            let link_domain = parsed.domain().unwrap_or("");

            link_info.url = parsed.to_string();

            if link_domain == source_domain && !source_domain.is_empty() {
                debug!("Found internal link: {} (text: {})", parsed, link_info.text);
                self.internal.push(link_info);
            } else if !link_domain.is_empty() {
                debug!("Found external link: {} (text: {})", parsed, link_info.text);
                self.external.push(link_info);
            } else if href.starts_with('/') || href.starts_with("./") || href.starts_with("../") {
                debug!("Found relative internal link: {} (text: {})", parsed, link_info.text);
                self.internal.push(link_info);
            }
        } else {
            warn!("Failed to parse href: {}", href);
        }
    }

    /// Remove duplicate links from all categories
    pub fn deduplicate(&mut self) {
        let internal_before = self.internal.len();
        let external_before = self.external.len();
        let mailto_before = self.mailto.len();
        let phone_before = self.phone.len();
        let anchor_before = self.anchor.len();
        let javascript_before = self.javascript.len();

        self.internal = self.deduplicate_vec(&self.internal);
        self.external = self.deduplicate_vec(&self.external);
        self.mailto = self.deduplicate_vec(&self.mailto);
        self.phone = self.deduplicate_vec(&self.phone);
        self.anchor = self.deduplicate_vec(&self.anchor);
        self.javascript = self.deduplicate_vec(&self.javascript);

        let internal_after = self.internal.len();
        let external_after = self.external.len();
        let mailto_after = self.mailto.len();
        let phone_after = self.phone.len();
        let anchor_after = self.anchor.len();
        let javascript_after = self.javascript.len();

        let total_removed = (internal_before - internal_after)
            + (external_before - external_after)
            + (mailto_before - mailto_after)
            + (phone_before - phone_after)
            + (anchor_before - anchor_after)
            + (javascript_before - javascript_after);

        debug!(
            internal_count = %format!("{} -> {}", internal_before, internal_after),
            external_count = %format!("{} -> {}", external_before, external_after),
            mailto_count = %format!("{} -> {}", mailto_before, mailto_after),
            phone_count = %format!("{} -> {}", phone_before, phone_after),
            anchor_count = %format!("{} -> {}", anchor_before, anchor_after),
            javascript_count = %format!("{} -> {}", javascript_before, javascript_after),
            duplicates_removed = %total_removed,
            "Link extraction and deduplication complete"
        );
    }

    /// Deduplicate a vector of LinkInfo using HashSet
    fn deduplicate_vec(&self, links: &[LinkInfo]) -> Vec<LinkInfo> {
        links
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect()
    }
}
