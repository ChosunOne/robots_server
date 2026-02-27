use crate::robots_data::RobotsData;
use crate::service::robots::AccessResult;
use reqwest::Client;
use robotstxt_rs::RobotsTxt;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, info, instrument};
use url::Url;

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Too many redirects")]
    TooManyRedirects,
    #[error("Robots.txt unavailable: HTTP {0}")]
    Unavailable(u16),
    #[error("Server unreachable: {}", 0.0)]
    Unreachable((String, Option<u16>)),
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
        info!("Creating fetcher with 30s timeout");
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    #[instrument(skip(self), fields(target_url = %target_url))]
    pub async fn fetch(&self, target_url: &str) -> Result<RobotsData, FetchError> {
        let robots_url = extract_robots_url(target_url)?;
        debug!(%robots_url, "Extracted robots.txt url");
        let response = match self.client.get(&robots_url).send().await {
            Ok(r) => {
                debug!(status = %r.status(), "Received HTTP response");
                r
            }
            Err(e) if e.is_timeout() => {
                debug!("Request timed out");
                return Err(FetchError::Timeout);
            }
            Err(e) => {
                debug!(error = %e, "robots.txt unreachable");
                return Err(FetchError::Unreachable((e.to_string(), None)));
            }
        };

        let status = response.status();
        let content_length = response.content_length().unwrap_or(0);
        debug!(%status, content_length, "Response details");

        match status.as_u16() {
            200..=299 => {
                let body = response.text().await.map_err(|e| {
                    info!(error = %e, "Failed to read request body");
                    FetchError::Unreachable((
                        "unsupported robots.txt format".to_string(),
                        Some(status.as_u16()),
                    ))
                })?;

                debug!(body_len = body.len(), "Parsing robots.txt content");

                let robots = RobotsTxt::parse(&body);
                let mut data: RobotsData = robots.into();
                data.content_length_bytes = content_length;
                data.robots_txt_url = robots_url.clone();
                data.target_url = target_url.to_string();
                data.http_status_code = status.as_u16() as u32;
                data.access_result = AccessResult::Success;

                info!(
                    groups_count = data.groups.len(),
                    sitemaps_count = data.sitemaps.len(),
                    "Parsed robots.txt"
                );

                Ok(data)
            }
            400..=499 => {
                debug!(status_code = status.as_u16(), "Client error response");
                Err(FetchError::Unavailable(status.as_u16()))
            }
            500..=599 => {
                debug!(status_code = status.as_u16(), "Server error response");
                Err(FetchError::Unreachable((
                    format!("Server error: {status}"),
                    Some(status.as_u16()),
                )))
            }
            _ => {
                debug!(status_code = status.as_u16(), "Unexpected status code");
                Err(FetchError::Unreachable((
                    format!("Unexpected status: {status}"),
                    None,
                )))
            }
        }
    }
}

#[instrument]
pub fn extract_robots_url(target_url: &str) -> Result<String, FetchError> {
    debug!("Parsing target url");
    let parsed = Url::parse(target_url).map_err(|e| {
        debug!(error = %e, "Invalid url");
        FetchError::InvalidUrl(format!("Failed to parse URL: {e}"))
    })?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        debug!(scheme = %scheme, "Unsupported scheme");
        return Err(FetchError::InvalidUrl(format!(
            "Unsupported scheme: {scheme}"
        )));
    }
    let host = parsed.host_str().ok_or_else(|| {
        debug!("URL has no nost component");
        FetchError::InvalidUrl("URL has no host".to_string())
    })?;
    let port = parsed.port();
    let robots_url = match port {
        Some(p) if (scheme == "http" && p != 80) || (scheme == "https" && p != 443) => {
            debug!(port = p, "Using non-standard port");
            format!("{scheme}://{host}:{p}/robots.txt")
        }
        _ => {
            debug!("Using standard port");
            format!("{scheme}://{host}/robots.txt")
        }
    };
    debug!(%robots_url, "Constructed robots.txt URL");
    Ok(robots_url)
}
