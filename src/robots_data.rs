use robotstxt_rs::RobotsTxt;

use crate::service::robots::{
    AccessResult, GetRobotsResponse, Group as ProtoBufGroup, Rule as ProtoBufRule, rule::RuleType,
};

#[derive(Clone, Debug, Default)]
pub struct RobotsData {
    pub target_url: String,
    pub robots_txt_url: String,
    pub access_result: AccessResult,
    pub http_status_code: u32,
    pub groups: Vec<Group>,
    pub sitemaps: Vec<String>,
    pub content_length_bytes: u64,
    pub truncated: bool,
}

impl RobotsData {
    pub fn is_allowed(&self, user_agent: &str, path: &str) -> bool {
        // RFC 9309 Section 2.2.1: Case-insensitive matching
        let user_agent_lower = user_agent.to_lowercase();
        // Find all matching groups per RFC 9309
        let matching_groups: Vec<&Group> = self
            .groups
            .iter()
            .filter(|group| {
                group.user_agents.iter().any(|ua| {
                    let ua_lower = ua.to_lowercase();
                    // Exact match or substring match (product token is substring of UA)
                    user_agent_lower == ua_lower || user_agent_lower.contains(&ua_lower)
                })
            })
            .collect();
        // RFC 9309: If no matching group, check for wildcard
        let groups_to_check = if matching_groups.is_empty() {
            self.groups
                .iter()
                .filter(|g| g.user_agents.iter().any(|ua| ua == "*"))
                .collect::<Vec<_>>()
        } else {
            matching_groups
        };
        // If still no groups, no rules apply (allowed)
        if groups_to_check.is_empty() {
            return true;
        }
        // Combine all rules from matching groups per RFC 9309
        let mut all_rules = Vec::new();
        for group in &groups_to_check {
            for rule in &group.rules {
                if let Ok(rule_type) = RuleType::try_from(rule.rule_type) {
                    if rule_type == RuleType::Allow || rule_type == RuleType::Disallow {
                        all_rules.push(rule);
                    }
                }
            }
        }
        // Find matching rules for this path
        let matching_rules: Vec<&Rule> = all_rules
            .iter()
            .filter(|rule| Self::path_matches_rfc9309(path, &rule.path_pattern))
            .copied()
            .collect();
        // RFC 9309 Section 2.2.2: If no match, URI is allowed
        if matching_rules.is_empty() {
            return true;
        }
        // Find the longest match (most octets per RFC 9309)
        let max_len = matching_rules
            .iter()
            .map(|r| r.path_pattern.len())
            .max()
            .unwrap();
        // Get all rules with the longest pattern
        let longest_rules: Vec<_> = matching_rules
            .iter()
            .filter(|r| r.path_pattern.len() == max_len)
            .collect();
        // RFC 9309: If allow and disallow are equivalent, allow wins
        let has_allow = longest_rules
            .iter()
            .any(|r| RuleType::try_from(r.rule_type).ok() == Some(RuleType::Allow));
        let has_disallow = longest_rules
            .iter()
            .any(|r| RuleType::try_from(r.rule_type).ok() == Some(RuleType::Disallow));
        // Allow wins on tie (RFC 9309 Section 2.2.2)
        if has_allow {
            return true;
        }
        // Otherwise follow disallow
        !has_disallow
    }

    /// RFC 9309 Section 2.2.2: Path matching with wildcards and special characters
    fn path_matches_rfc9309(path: &str, pattern: &str) -> bool {
        // Handle end-of-path anchor $ (RFC 9309 Section 2.2.3)
        if pattern.ends_with('$') {
            let prefix = &pattern[..pattern.len() - 1];
            return Self::match_pattern(path, prefix, true);
        }
        // Regular prefix match
        Self::match_pattern(path, pattern, false)
    }
    /// Match pattern against path with wildcard support
    fn match_pattern(path: &str, pattern: &str, exact: bool) -> bool {
        // Handle wildcards (* matches any sequence per RFC 9309 Section 2.2.3)
        if pattern.contains('*') {
            return Self::wildcard_match(path, pattern, exact);
        }
        // RFC 9309: Match MUST start with first octet of path (prefix match)
        if exact {
            path == pattern
        } else {
            path.starts_with(pattern)
        }
    }
    /// RFC 9309 wildcard matching (* matches any characters)
    fn wildcard_match(path: &str, pattern: &str, exact: bool) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }
        if parts.len() == 1 {
            // No wildcards after split (should not happen due to earlier check)
            return if exact {
                path == pattern
            } else {
                path.starts_with(pattern)
            };
        }
        // Multi-part wildcard matching
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i == 0 {
                // First part must be at start
                if !path.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 && exact {
                // Last part with exact match must be at end
                if !path.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts can be anywhere after current position
                if let Some(found) = path[pos..].find(part) {
                    pos += found + part.len();
                } else {
                    return false;
                }
            }
        }
        true
    }
}

impl From<&RobotsData> for String {
    fn from(value: &RobotsData) -> Self {
        let mut lines = Vec::new();

        for group in &value.groups {
            for ua in &group.user_agents {
                lines.push(format!("User-agent: {ua}"));
            }

            for rule in &group.rules {
                let Ok(rule_type) = RuleType::try_from(rule.rule_type) else {
                    continue;
                };
                let directive = match rule_type {
                    RuleType::Allow => "Allow",
                    RuleType::Disallow => "Disallow",
                    _ => continue,
                };
                lines.push(format!("{directive}: {}", rule.path_pattern));
            }

            lines.push(String::new());
        }

        for sitemap in &value.sitemaps {
            lines.push(format!("Sitemap: {sitemap}"));
        }

        lines.join("\n")
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub user_agents: Vec<String>,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub rule_type: i32,
    pub path_pattern: String,
}

impl From<Rule> for ProtoBufRule {
    fn from(value: Rule) -> Self {
        Self {
            rule_type: value.rule_type,
            path_pattern: value.path_pattern,
        }
    }
}

impl From<Group> for ProtoBufGroup {
    fn from(value: Group) -> Self {
        Self {
            user_agents: value.user_agents,
            rules: value.rules.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RobotsData> for GetRobotsResponse {
    fn from(value: RobotsData) -> Self {
        Self {
            target_url: value.target_url,
            robots_txt_url: value.robots_txt_url,
            access_result: value.access_result.into(),
            http_status_code: value.http_status_code,
            groups: value.groups.into_iter().map(Into::into).collect(),
            sitemaps: value.sitemaps,
            content_length_bytes: value.content_length_bytes,
            truncated: value.truncated,
        }
    }
}

impl From<RobotsTxt> for RobotsData {
    fn from(value: RobotsTxt) -> Self {
        let mut groups = Vec::new();
        for (user_agent, rule) in value.get_rules() {
            let mut rules = Vec::new();
            for path in &rule.allowed {
                rules.push(Rule {
                    rule_type: 1,
                    path_pattern: path.clone(),
                });
            }
            for path in &rule.disallowed {
                rules.push(Rule {
                    rule_type: 2,
                    path_pattern: path.clone(),
                });
            }

            groups.push(Group {
                user_agents: vec![user_agent.clone()],
                rules,
            });
        }

        let sitemaps = value
            .get_sitemaps()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        Self {
            target_url: "".to_string(),
            robots_txt_url: "".to_string(),
            access_result: AccessResult::Unspecified,
            http_status_code: 0,
            groups,
            sitemaps,
            content_length_bytes: 0,
            truncated: false,
        }
    }
}
