use crate::service::robots::{
    AccessResult, GetRobotsResponse, Group as ProtoBufGroup, Rule as ProtoBufRule,
};

#[derive(Clone, Debug, Default)]
pub struct RobotsData {
    pub target_url: String,
    pub robots_txt_url: String,
    pub access_result: AccessResult,
    pub http_status_code: i32,
    pub groups: Vec<Group>,
    pub sitemaps: Vec<String>,
    pub content_length_bytes: i64,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub user_agents: Vec<String>,
    pub rules: Vec<Rule>,
    pub crawl_delay_seconds: i32,
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
            crawl_delay_seconds: value.crawl_delay_seconds,
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
