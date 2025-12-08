use url::Url;
use std::collections::HashMap;
use tracing::debug;

/// Basic metadata about the page
#[derive(Debug, Clone, Default)]
pub struct BasicMetadata {
  /// The page title from the `<title>` tag
  pub title: Option<String>,
  /// The page description from the `description` meta tag
  pub description: Option<String>,
  /// Keywords associated with the page from the `keywords` meta tag
  pub keywords: Option<Vec<String>>,
  /// Character encoding of the page from the `charset` meta tag
  pub charset: Option<String>,
  /// The primary language of the page from the `language` meta tag
  pub language: Option<String>,
}

/// Crawler and SEO related metadata
#[derive(Debug, Clone, Default)]
pub struct SeoMetadata {
  /// Robots directive from the `robots` meta tag (e.g., "index, follow")
  pub robots: Option<String>,
  /// The canonical URL of the page to prevent duplicate content issues
  pub canonical: Option<Url>,
  /// The author of the page content
  pub author: Option<String>,
  /// The publisher of the page
  pub publisher: Option<String>,
  /// The creator of the page content
  pub creator: Option<String>,
}

/// Open Graph metadata for social media sharing
#[derive(Debug, Clone, Default)]
pub struct OpenGraphMetadata {
  /// The type of content (e.g., "website", "article")
  pub og_type: Option<String>,
  /// The title for sharing on social media
  pub og_title: Option<String>,
  /// The description for sharing on social media
  pub og_description: Option<String>,
  /// The canonical URL for the content
  pub og_url: Option<Url>,
  /// The image URL for preview when shared
  pub og_image: Option<Url>,
  /// The name of the website
  pub og_site_name: Option<String>,
  /// The locale of the content (e.g., "en_US")
  pub og_locale: Option<String>,
}

/// Twitter Card metadata for Twitter sharing
#[derive(Debug, Clone, Default)]
pub struct TwitterCardMetadata {
  /// The type of Twitter card (e.g., "summary", "summary_large_image")
  pub twitter_card: Option<String>,
  /// The title for Twitter sharing
  pub twitter_title: Option<String>,
  /// The description for Twitter sharing
  pub twitter_description: Option<String>,
  /// The URL associated with the Twitter card
  pub twitter_url: Option<Url>,
  /// The image URL for the Twitter card
  pub twitter_image: Option<Url>,
}

/// Viewport and mobile metadata
#[derive(Debug, Clone, Default)]
pub struct ViewportMetadata {
  /// Viewport settings for responsive design (e.g., "width=device-width, initial-scale=1.0")
  pub viewport: Option<String>,
  /// The theme color for browser UI on mobile devices
  pub theme_color: Option<String>,
  /// Whether the page is capable of being run as a web app on Apple devices
  pub apple_mobile_web_app_capable: Option<bool>,
  /// The style of the status bar on Apple devices
  pub apple_mobile_web_app_status_bar_style: Option<String>,
}

/// Link relationships
#[derive(Debug, Clone, Default)]
pub struct LinkMetadata {
  /// The canonical URL of the page to prevent duplicate content issues
  pub canonical: Option<Url>,
  /// Alternate language versions of the page mapped by language code
  pub alternate_languages: HashMap<String, Url>,
  /// The URL of the previous page in a series
  pub prev: Option<Url>,
  /// The URL of the next page in a series
  pub next: Option<Url>,
  /// The favicon URL for the page
  pub icon: Option<Url>,
  /// The Apple touch icon URL for iOS home screen shortcuts
  pub apple_touch_icon: Option<Url>,
}

/// Complete page metadata combining all metadata types
#[derive(Debug, Clone, Default)]
pub struct PageMetadata {
  /// Basic page information (title, description, language, etc.)
  pub basic: BasicMetadata,
  /// SEO and crawler-related metadata (robots, canonical, author, etc.)
  pub seo: SeoMetadata,
  /// Open Graph metadata for social media sharing (Facebook, LinkedIn, etc.)
  pub open_graph: OpenGraphMetadata,
  /// Twitter Card metadata for Twitter sharing
  pub twitter_card: TwitterCardMetadata,
  /// Viewport and mobile-related metadata
  pub viewport: ViewportMetadata,
  /// Link relationships and alternate versions
  pub links: LinkMetadata,
}

impl PageMetadata {
  /// Extracts metadata from an HTML document
  #[tracing::instrument(skip(document))]
  pub fn extract(document: &scraper::Html) -> anyhow::Result<Self> {
    debug!("Starting metadata extraction");
    let mut metadata = PageMetadata::default();

    // Extract basic metadata
    debug!("Extracting basic metadata");
    metadata.basic.title = Self::extract_title(document);
    metadata.basic.description = Self::extract_meta_content(document, "name", "description");
    metadata.basic.keywords = Self::extract_keywords(document);
    metadata.basic.charset = Self::extract_charset(document);
    metadata.basic.language = Self::extract_meta_content(document, "name", "language");

    // Extract SEO metadata
    debug!("Extracting SEO metadata");
    metadata.seo.robots = Self::extract_meta_content(document, "name", "robots");
    metadata.seo.canonical = Self::extract_canonical_from_link(document);
    metadata.seo.author = Self::extract_meta_content(document, "name", "author");
    metadata.seo.publisher = Self::extract_meta_content(document, "name", "publisher");
    metadata.seo.creator = Self::extract_meta_content(document, "name", "creator");

    // Extract Open Graph metadata
    debug!("Extracting Open Graph metadata");
    metadata.open_graph.og_type = Self::extract_meta_property(document, "og:type");
    metadata.open_graph.og_title = Self::extract_meta_property(document, "og:title");
    metadata.open_graph.og_description = Self::extract_meta_property(document, "og:description");
    metadata.open_graph.og_url = Self::extract_url_from_property(document, "og:url");
    metadata.open_graph.og_image = Self::extract_url_from_property(document, "og:image");
    metadata.open_graph.og_site_name = Self::extract_meta_property(document, "og:site_name");
    metadata.open_graph.og_locale = Self::extract_meta_property(document, "og:locale");

    // Extract Twitter Card metadata
    debug!("Extracting Twitter Card metadata");
    metadata.twitter_card.twitter_card = Self::extract_meta_content(document, "name", "twitter:card");
    metadata.twitter_card.twitter_title = Self::extract_meta_content(document, "name", "twitter:title");
    metadata.twitter_card.twitter_description = Self::extract_meta_content(document, "name", "twitter:description");
    metadata.twitter_card.twitter_url = Self::extract_url_from_meta(document, "name", "twitter:url");
    metadata.twitter_card.twitter_image = Self::extract_url_from_meta(document, "name", "twitter:image");

    // Extract viewport metadata
    debug!("Extracting viewport metadata");
    metadata.viewport.viewport = Self::extract_meta_content(document, "name", "viewport");
    metadata.viewport.theme_color = Self::extract_meta_content(document, "name", "theme-color");
    metadata.viewport.apple_mobile_web_app_capable = 
      Self::extract_meta_content(document, "name", "apple-mobile-web-app-capable")
        .map(|v| v.to_lowercase() == "yes");
    metadata.viewport.apple_mobile_web_app_status_bar_style = 
      Self::extract_meta_content(document, "name", "apple-mobile-web-app-status-bar-style");

    // Extract link metadata
    debug!("Extracting link metadata");
    metadata.links.canonical = Self::extract_canonical_from_link(document);
    metadata.links.alternate_languages = Self::extract_alternate_languages(document);
    metadata.links.prev = Self::extract_link_href(document, "prev");
    metadata.links.next = Self::extract_link_href(document, "next");
    metadata.links.icon = Self::extract_link_href(document, "icon");
    metadata.links.apple_touch_icon = Self::extract_link_href(document, "apple-touch-icon");

    debug!("Metadata extraction completed successfully");
    Ok(metadata)
  }

  /// Extracts the page title from the `<title>` tag
  #[tracing::instrument(skip(document))]
  fn extract_title(document: &scraper::Html) -> Option<String> {
    debug!("Extracting title from <title> tag");
    let selector = scraper::Selector::parse("title").ok()?;
    let result = document
      .select(&selector)
      .next()
      .map(|el| el.inner_html().trim().to_string());
    
    if result.is_some() {
      debug!("Title extracted successfully");
    } else {
      debug!("No title found in document");
    }
    
    result
  }

  /// Extracts meta tag content by name or property attribute
  #[tracing::instrument(skip(document))]
  fn extract_meta_content(document: &scraper::Html, attr: &str, value: &str) -> Option<String> {
    debug!(attr = %attr, value = %value, "Extracting meta content");
    let selector = scraper::Selector::parse(&format!("meta[{}=\"{}\"]", attr, value)).ok()?;
    let result = document
      .select(&selector)
      .next()
      .and_then(|el| el.value().attr("content"))
      .map(|s| s.trim().to_string());
    
    if result.is_some() {
      debug!(attr = %attr, value = %value, "Meta content found");
    } else {
      debug!(attr = %attr, value = %value, "Meta content not found");
    }
    
    result
  }

  /// Extracts meta property content (for Open Graph)
  #[tracing::instrument(skip(document))]
  fn extract_meta_property(document: &scraper::Html, property: &str) -> Option<String> {
    debug!(property = %property, "Extracting meta property");
    let selector = scraper::Selector::parse(&format!("meta[property=\"{}\"]", property)).ok()?;
    let result = document
      .select(&selector)
      .next()
      .and_then(|el| el.value().attr("content"))
      .map(|s| s.trim().to_string());
    
    if result.is_some() {
      debug!(property = %property, "Meta property found");
    } else {
      debug!(property = %property, "Meta property not found");
    }
    
    result
  }

  /// Extracts keywords as a vector of strings
  #[tracing::instrument(skip(document))]
  fn extract_keywords(document: &scraper::Html) -> Option<Vec<String>> {
    debug!("Extracting keywords");
    Self::extract_meta_content(document, "name", "keywords").map(|keywords| {
      let keyword_list: Vec<String> = keywords
        .split(',')
        .map(|k| k.trim().to_string())
        .collect();
      debug!(count = keyword_list.len(), "Keywords extracted");
      keyword_list
    })
  }

  /// Extracts the charset from meta tag
  #[tracing::instrument(skip(document))]
  fn extract_charset(document: &scraper::Html) -> Option<String> {
    debug!("Extracting charset");
    let selector = scraper::Selector::parse("meta[charset]").ok()?;
    let result = document
      .select(&selector)
      .next()
      .and_then(|el| el.value().attr("charset"))
      .map(|s| s.to_string());
    
    if let Some(ref charset) = result {
      debug!(charset = %charset, "Charset extracted");
    } else {
      debug!("No charset found");
    }
    
    result
  }

  /// Extracts canonical URL from link tag
  #[tracing::instrument(skip(document))]
  fn extract_canonical_from_link(document: &scraper::Html) -> Option<Url> {
    debug!("Extracting canonical URL from link tag");
    let selector = scraper::Selector::parse("link[rel=\"canonical\"]").ok()?;
    let result = document
      .select(&selector)
      .next()
      .and_then(|el| el.value().attr("href"))
      .and_then(|href| {
        match Url::parse(href) {
          Ok(url) => {
            debug!(url = %url, "Canonical URL parsed successfully");
            Some(url)
          }
          Err(e) => {
            debug!(href = %href, error = %e, "Failed to parse canonical URL");
            None
          }
        }
      });
    
    result
  }

  /// Extracts URL from meta property
  #[tracing::instrument(skip(document))]
  fn extract_url_from_property(document: &scraper::Html, property: &str) -> Option<Url> {
    debug!(property = %property, "Extracting URL from meta property");
    Self::extract_meta_property(document, property).and_then(|url| {
      match Url::parse(&url) {
        Ok(parsed_url) => {
          debug!(property = %property, url = %parsed_url, "URL parsed successfully");
          Some(parsed_url)
        }
        Err(e) => {
          debug!(property = %property, url = %url, error = %e, "Failed to parse URL");
          None
        }
      }
    })
  }

  /// Extracts URL from meta name attribute
  #[tracing::instrument(skip(document))]
  fn extract_url_from_meta(document: &scraper::Html, attr: &str, value: &str) -> Option<Url> {
    debug!(attr = %attr, value = %value, "Extracting URL from meta");
    Self::extract_meta_content(document, attr, value).and_then(|url| {
      match Url::parse(&url) {
        Ok(parsed_url) => {
          debug!(attr = %attr, value = %value, url = %parsed_url, "URL parsed successfully");
          Some(parsed_url)
        }
        Err(e) => {
          debug!(attr = %attr, value = %value, url = %url, error = %e, "Failed to parse URL");
          None
        }
      }
    })
  }

  /// Extracts alternate language versions from hreflang link tags
  #[tracing::instrument(skip(document))]
  fn extract_alternate_languages(document: &scraper::Html) -> HashMap<String, Url> {
    debug!("Extracting alternate language versions");
    let mut languages = HashMap::new();
    let selector = match scraper::Selector::parse("link[rel=\"alternate\"][hreflang]") {
      Ok(s) => s,
      Err(e) => {
        debug!(error = %e, "Failed to parse alternate language selector");
        return languages;
      }
    };

    for element in document.select(&selector) {
      if let (Some(lang), Some(href)) = (
        element.value().attr("hreflang"),
        element.value().attr("href"),
      ) {
        match Url::parse(href) {
          Ok(url) => {
            debug!(lang = %lang, url = %url, "Alternate language found");
            languages.insert(lang.to_string(), url);
          }
          Err(e) => {
            debug!(lang = %lang, href = %href, error = %e, "Failed to parse alternate language URL");
          }
        }
      }
    }

    debug!(count = languages.len(), "Alternate languages extraction completed");
    languages
  }

  /// Extracts link href by rel attribute
  #[tracing::instrument(skip(document))]
  fn extract_link_href(document: &scraper::Html, rel: &str) -> Option<Url> {
    debug!(rel = %rel, "Extracting link href");
    let selector = scraper::Selector::parse(&format!("link[rel=\"{}\"]", rel)).ok()?;
    let result = document
      .select(&selector)
      .next()
      .and_then(|el| el.value().attr("href"))
      .and_then(|href| {
        match Url::parse(href) {
          Ok(url) => {
            debug!(rel = %rel, url = %url, "Link URL parsed successfully");
            Some(url)
          }
          Err(e) => {
            debug!(rel = %rel, href = %href, error = %e, "Failed to parse link URL");
            None
          }
        }
      });
    
    result
  }
}