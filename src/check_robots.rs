use url::Url;
use tracing::{debug, warn};

const MAX_ROBOTS_TXT_SIZE: usize = 500 * 1024; // 500 KiB

/// Represents the result of a robots.txt fetch
/// Used for distinguishing between different HTTP response codes
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RobotsFetchResult {
    /// Successfully fetched and parsed robots.txt
    Success(Robot),
    /// 404 Not Found - treat as allowing all paths
    NotFound,
    /// 403 Forbidden - treat as disallowing all paths (conservative)
    Forbidden,
}

/// Represents a single allow or disallow rule
#[derive(Debug, Clone)]
pub struct Rule {
    pub pattern: String,
    pub allow: bool,
}

/// Represents a user-agent group with its rules and directives
#[derive(Debug, Clone)]
pub struct Group {
    pub user_agents: Vec<String>,
    pub rules: Vec<Rule>,
    pub crawl_delay: Option<f64>,
    pub request_rate: Option<f64>,
}

/// Represents the parsed robots.txt file
#[derive(Debug,Clone)]
pub struct Robot {
    groups: Vec<Group>,
    sitemaps: Vec<String>,
}

impl Robot {
    /// Creates a new Robot by parsing a robots.txt file content
    /// This parser is lenient and will skip unparseable lines
    pub fn new(text_file: String) -> Self {
        debug!("Parsing robots.txt (size: {} bytes)", text_file.len());
        
        // Check size limit (500 KiB)
        if text_file.len() > MAX_ROBOTS_TXT_SIZE {
            warn!("robots.txt exceeds 500 KiB limit, ignoring");
            return Robot {
                groups: vec![],
                sitemaps: vec![],
            };
        }

        let mut groups = Vec::new();
        let mut sitemaps = Vec::new();
        let mut current_group: Option<Group> = None;

        for line in text_file.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let (key, value) = match Self::parse_line(trimmed) {
                Some((k, v)) => (k, v),
                None => continue, // Skip unparseable lines
            };

            match key.to_lowercase().as_str() {
                "user-agent" => {
                    // Finalize previous group if exists
                    if let Some(group) = current_group.take() {
                        if !group.user_agents.is_empty() {
                            debug!("Finalized user-agent group: {:?} with {} rules", group.user_agents, group.rules.len());
                            groups.push(group);
                        }
                    }

                    debug!("Starting new user-agent group: {}", value);
                    // Start new group
                    current_group = Some(Group {
                        user_agents: vec![value.to_string()],
                        rules: Vec::new(),
                        crawl_delay: None,
                        request_rate: None,
                    });
                }
                "allow" => {
                    if let Some(ref mut group) = current_group {
                        debug!("Adding allow rule: '{}'", value);
                        group.rules.push(Rule {
                            pattern: value.to_string(),
                            allow: true,
                        });
                    } else {
                        debug!("Ignoring allow rule before first user-agent directive");
                    }
                }
                "disallow" => {
                    if let Some(ref mut group) = current_group {
                        debug!("Adding disallow rule: '{}'", value);
                        group.rules.push(Rule {
                            pattern: value.to_string(),
                            allow: false,
                        });
                    } else {
                        debug!("Ignoring disallow rule before first user-agent directive");
                    }
                }
                "crawl-delay" => {
                    if let Ok(delay) = value.parse::<f64>() {
                        if let Some(ref mut group) = current_group {
                            debug!("Setting crawl-delay to {} seconds", delay);
                            group.crawl_delay = Some(delay);
                        }
                    } else {
                        warn!("Invalid crawl-delay value: '{}'", value);
                    }
                }
                "request-rate" => {
                    if let Ok(rate) = value.parse::<f64>() {
                        if let Some(ref mut group) = current_group {
                            debug!("Setting request-rate to {} requests/second", rate);
                            group.request_rate = Some(rate);
                        }
                    } else {
                        warn!("Invalid request-rate value: '{}'", value);
                    }
                }
                "sitemap" => {
                    debug!("Found sitemap: {}", value);
                    sitemaps.push(value.to_string());
                }
                _ => {
                    // Unknown directives are ignored
                }
            }
        }

        // Finalize last group
        if let Some(group) = current_group {
            if !group.user_agents.is_empty() {
                debug!("Finalized last user-agent group: {:?} with {} rules", group.user_agents, group.rules.len());
                groups.push(group);
            }
        }

        debug!("Parsed robots.txt: {} user-agent groups, {} sitemaps", groups.len(), sitemaps.len());
        Robot { groups, sitemaps }
    }

    /// Parses a single line into (key, value) tuple
    /// Returns None if line is malformed
    fn parse_line(line: &str) -> Option<(&str, &str)> {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim();
            let value = line[colon_pos + 1..].trim();
            if !key.is_empty() {
                return Some((key, value));
            }
        }
        None
    }

    /// Checks if a URL is allowed for a given user-agent
    /// This method will be used in future subcommands (e.g., when crawling with robots.txt validation)
    /// For now, it's preserved for future use.
    #[allow(dead_code)]
    pub fn allow(&self, url: &str, user_agent: &str) -> bool {
        let parsed_url = match Url::parse(url) {
            Ok(u) => u,
            Err(_) => return true, // If URL is invalid, allow by default
        };

        let path = parsed_url.path();
        let normalized_path = Self::normalize_path(path);

        // Find matching group for user-agent
        if let Some(group) = self.find_group(user_agent) {
            // Find longest matching rule
            if let Some(matching_rule) = self.find_longest_matching_rule(&group.rules, &normalized_path) {
                return matching_rule.0.allow;
            }
        }

        // No matching rule found means allowed
        true
    }

    /// Finds the matching group for a given user-agent
    /// Performs matching with specificity: exact > prefix > wildcard
    /// Per RFC 9309: "If more than one group applies to a user-agent, the most specific match should be used"
    pub fn find_group(&self, user_agent: &str) -> Option<&Group> {
        let user_agent_lower = user_agent.to_lowercase();
        
        debug!("Finding matching group for user-agent: '{}'", user_agent);

        // First, try exact match (case-insensitive)
        for group in &self.groups {
            for agent in &group.user_agents {
                if agent.to_lowercase() == user_agent_lower {
                    debug!("Matched exact user-agent: '{}'", agent);
                    return Some(group);
                }
            }
        }

        // Second, try prefix match (case-insensitive)
        // Find the longest prefix match for better specificity
        let mut longest_prefix_match: Option<&Group> = None;
        let mut longest_prefix_len = 0;
        
        for group in &self.groups {
            for agent in &group.user_agents {
                let agent_lower = agent.to_lowercase();
                if user_agent_lower.starts_with(&agent_lower) && agent_lower != "*" {
                    if agent_lower.len() > longest_prefix_len {
                        longest_prefix_match = Some(group);
                        longest_prefix_len = agent_lower.len();
                        debug!("Found prefix match: '{}' (length: {})", agent, agent_lower.len());
                    }
                }
            }
        }
        
        if let Some(group) = longest_prefix_match {
            debug!("Matched prefix user-agent with {} char(s)", longest_prefix_len);
            return Some(group);
        }

        // Third, try wildcard "*" as fallback
        for group in &self.groups {
            for agent in &group.user_agents {
                if agent == "*" {
                    debug!("Matched wildcard user-agent: '*'");
                    return Some(group);
                }
            }
        }

        debug!("No matching group found for user-agent: '{}'", user_agent);
        None
    }

    /// Finds the longest matching rule in a group
    /// Per RFC 9309, the most specific (longest) match should be used
    /// Returns the matching rule and the match reason for human-readable output
    /// This method will be used when implementing path allowance checking in future subcommands
    #[allow(dead_code)]
    pub fn find_longest_matching_rule<'a>(
        &self,
        rules: &'a [Rule],
        path: &str,
    ) -> Option<(&'a Rule, String)> {
        let mut longest_match: Option<&Rule> = None;
        let mut longest_pattern_len = 0;
        let mut match_reason = String::new();

        for rule in rules {
            if Self::matches_pattern(&rule.pattern, path) {
                if rule.pattern.len() > longest_pattern_len {
                    longest_match = Some(rule);
                    longest_pattern_len = rule.pattern.len();
                    match_reason = format!("pattern '{}'", rule.pattern);
                    debug!("Found matching rule: pattern='{}' (len: {}) allow={}", rule.pattern, rule.pattern.len(), rule.allow);
                }
            }
        }

        longest_match.map(|rule| (rule, match_reason))
    }

    /// Matches a pattern against a path
    /// Supports RFC 9309 special characters: * (0+ chars) and $ (end of pattern)
    /// Will be used when implementing path allowance checking in future subcommands
    #[allow(dead_code)]
    fn matches_pattern(pattern: &str, path: &str) -> bool {
        // If pattern ends with $, it's an exact match (end anchor)
        let (pattern, exact_end) = if pattern.ends_with('$') {
            (&pattern[..pattern.len() - 1], true)
        } else {
            (pattern, false)
        };

        // Simple glob-like matching with * support
        let pattern_parts: Vec<&str> = pattern.split('*').collect();

        // If no * in pattern, do simple string matching
        if pattern_parts.len() == 1 {
            if exact_end {
                return path == pattern;
            } else {
                return path.starts_with(pattern);
            }
        }

        // Pattern has one or more *
        let mut path_pos = 0;

        for (i, part) in pattern_parts.iter().enumerate() {
            if i == 0 {
                // First part must match at start
                if !path.starts_with(part) {
                    return false;
                }
                path_pos += part.len();
            } else if i == pattern_parts.len() - 1 {
                // Last part
                if exact_end {
                    // Must match exactly at the end
                    return path.ends_with(part) && path_pos <= path.len() - part.len();
                } else {
                    // Must appear after current position
                    return path[path_pos..].contains(part);
                }
            } else {
                // Middle parts must be found in sequence
                if let Some(pos) = path[path_pos..].find(part) {
                    path_pos += pos + part.len();
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Normalizes a URL path per RFC 3986
    /// Handles percent-encoding: decodes unreserved chars, keeps reserved/non-ASCII encoded
    /// Will be used when implementing path matching in future subcommands
    #[allow(dead_code)]
    fn normalize_path(path: &str) -> String {
        // For now, return path as-is
        // Full implementation would decode percent-encoding appropriately
        path.to_string()
    }

    /// Returns the crawl-delay for a given user-agent
    pub fn crawl_delay(&self, user_agent: &str) -> Option<f64> {
        self.find_group(user_agent).and_then(|g| g.crawl_delay)
    }

    /// Returns the request-rate for a given user-agent
    pub fn request_rate(&self, user_agent: &str) -> Option<f64> {
        self.find_group(user_agent).and_then(|g| g.request_rate)
    }

    /// Returns all sitemaps found in the robots.txt
    pub fn sitemaps(&self) -> Vec<String> {
        self.sitemaps.clone()
    }
    
    /// Returns detailed information about matched rules for a user-agent
    /// Useful for human-readable output showing which rules apply
    pub fn get_group_info(&self, user_agent: &str) -> Option<GroupInfo> {
        self.find_group(user_agent).map(|group| GroupInfo {
            user_agents: group.user_agents.clone(),
            rule_count: group.rules.len(),
            allow_count: group.rules.iter().filter(|r| r.allow).count(),
            disallow_count: group.rules.iter().filter(|r| !r.allow).count(),
            crawl_delay: group.crawl_delay,
            request_rate: group.request_rate,
        })
    }
}

/// Human-readable group information for output
#[derive(Debug, Clone)]
pub struct GroupInfo {
    pub user_agents: Vec<String>,
    pub rule_count: usize,
    pub allow_count: usize,
    pub disallow_count: usize,
    pub crawl_delay: Option<f64>,
    pub request_rate: Option<f64>,
}
