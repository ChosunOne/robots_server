use robotstxt_rs::RobotsTxt;

use crate::service::robots::{
    AccessResult, GetRobotsResponse, Group as ProtoBufGroup, Rule as ProtoBufRule,
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
