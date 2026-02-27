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
    Unreachable(String),
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
            Err(e) if e.is_timeout() => return Err(FetchError::Timeout),
            Err(e) => return Err(FetchError::Unreachable(e.to_string())),
        };

        let status = response.status();
        let content_length = response.content_length().unwrap_or(0);

        match status.as_u16() {
            200..=299 => {
                let body = response.text().await.map_err(|e| {
                    FetchError::Unreachable("unsupported robots.txt format".to_string())
                })?;
                let robots = RobotsTxt::parse(&body);
                let mut data: RobotsData = robots.into();
                data.content_length_bytes = content_length;
                data.target_url = target_url.to_string();
                data.http_status_code = status.as_u16() as u32;
                data.access_result = AccessResult::Success;
                Ok(data)
            }
            400..=499 => Err(FetchError::Unavailable(status.as_u16())),
            500..=599 => Err(FetchError::Unreachable(format!("Server error: {status}"))),
            _ => Err(FetchError::Unreachable(format!(
                "Unexpected status: {status}"
            ))),
        }
    }
}

fn extract_robots_url(target_url: &str) -> String {
    todo!()
}
