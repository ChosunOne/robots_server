use crate::robots_data::{Group, RobotsData, Rule};
use crate::service::robots::AccessResult;
use reqwest::Client;
use robotstxt_rs::RobotsTxt;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Too many redirects")]
    TooManyRedirects,
    #[error("Robots.txt unavailable: HTTP {0}")]
    Unavailable(u16),
    #[error("Server unreachable: {0}")]
    Unreachavle(String),
    #[error("Request timeout")]
    Timeout,
    #[error("Failed to parse robots.txt")]
    ParseError(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

pub struct RobotsFetcher {
    client: reqwest::Client,
}

impl RobotsFetcher {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn fetch(&self, target_url: &str) -> Result<RobotsData, FetchError> {
        let robots_url = extract_robots_url(target_url);
        let response = match self.client.get(&robots_url).send().await {
            Ok(r) => r,
            _ => todo!(),
        };
        todo!()
    }
}

fn extract_robots_url(target_url: &str) -> String {
    todo!()
}
