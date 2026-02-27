use tonic::{Request, Response, Status};

use robots::{
    AccessResult, GetRobotsRequest, GetRobotsResponse, robots_service_server::RobotsService,
};
use tracing::{Span, debug, info, instrument, warn};
use url::Url;

use crate::{
    cache::Cache,
    fetcher::{FetchError, RobotsFetcher, extract_robots_url},
    robots_data::RobotsData,
    service::robots::{IsAllowedRequest, IsAllowedResponse},
};

pub mod robots {
    include!("generated/robots.rs");
}

pub struct RobotsServer<T: Cache<String, RobotsData>> {
    cache: T,
    fetcher: RobotsFetcher,
}

impl<T: Cache<String, RobotsData>> RobotsServer<T> {
    pub fn new(cache: T, fetcher: RobotsFetcher) -> Self {
        Self { cache, fetcher }
    }

    async fn get_robots_data(
        &self,
        robots_url: String,
        target_url: String,
    ) -> Result<RobotsData, Status> {
        match self.cache.get(&robots_url).await {
            Ok(Some(data)) => {
                debug!("Cache hit for request");
                Ok(data)
            }
            Ok(None) => {
                debug!("Cache miss for request, fetching from origin");
                match self.fetcher.fetch(&target_url).await {
                    Ok(data) => {
                        info!(
                            status_code = data.http_status_code,
                            content_length = data.content_length_bytes,
                            "Successfully fetched robots.txt"
                        );
                        if let Err(e) = self
                            .cache
                            .set(data.robots_txt_url.clone(), data.clone())
                            .await
                        {
                            warn!(error = %e, "Failed to cache robots.txt data");
                        }
                        Ok(data)
                    }
                    Err(FetchError::Unavailable(s)) => {
                        info!(status_code = s, "robots.txt unavailable");
                        let data = RobotsData {
                            target_url,
                            robots_txt_url: robots_url,
                            access_result: AccessResult::Unavailable,
                            http_status_code: s as u32,
                            ..Default::default()
                        };

                        if let Err(e) = self
                            .cache
                            .set(data.robots_txt_url.clone(), data.clone())
                            .await
                        {
                            warn!(error = %e, "Failed to cache robots.txt data");
                        }
                        Ok(data)
                    }
                    Err(FetchError::Unreachable(e)) => {
                        info!(error = %e.0, status = e.1, "robots.txt unreachable");
                        let s = e.1.unwrap_or(0);
                        let data = RobotsData {
                            target_url,
                            robots_txt_url: robots_url,
                            access_result: AccessResult::Unreachable,
                            http_status_code: s as u32,
                            ..Default::default()
                        };
                        if let Err(e) = self
                            .cache
                            .set(data.robots_txt_url.clone(), data.clone())
                            .await
                        {
                            warn!(error = %e, "Failed to cache robots.txt data");
                        }
                        Ok(data)
                    }
                    Err(FetchError::Timeout) => {
                        info!("Request timeout");
                        let data = RobotsData {
                            target_url,
                            robots_txt_url: robots_url,
                            access_result: AccessResult::Unreachable,
                            ..Default::default()
                        };
                        if let Err(e) = self
                            .cache
                            .set(data.robots_txt_url.clone(), data.clone())
                            .await
                        {
                            warn!(error = %e, "Failed to cache robots.txt data");
                        }
                        Ok(data)
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to fetch robots.txt");
                        Err(Status::internal(e.to_string()))
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Cache error");
                Err(Status::internal(e.to_string()))
            }
        }
    }
}

#[tonic::async_trait]
impl<T: Cache<String, RobotsData>> RobotsService for RobotsServer<T> {
    #[instrument(skip(self, request), fields(url = %request.get_ref().url, robots_url = tracing::field::Empty))]
    async fn get_robots_txt(
        &self,
        request: Request<GetRobotsRequest>,
    ) -> Result<Response<GetRobotsResponse>, Status> {
        let req = request.into_inner();
        let robots_url =
            extract_robots_url(&req.url).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let target_url = req.url;

        Span::current().record("robots_url", &robots_url);
        info!("Processing robots.txt request");
        let data = self.get_robots_data(robots_url, target_url).await?;
        Ok(Response::new(data.into()))
    }

    #[instrument(
        skip(self, request), 
        fields(
            target_url = %request.get_ref().target_url, 
            user_agent = %request.get_ref().user_agent, 
            robots_url = tracing::field::Empty, 
            allowed = tracing::field::Empty))
    ]
    async fn is_allowed(
        &self,
        request: Request<IsAllowedRequest>,
    ) -> Result<Response<IsAllowedResponse>, Status> {
        let req = request.into_inner();

        let target_url = req.target_url;
        let user_agent = &req.user_agent;
        let robots_url =
            extract_robots_url(&target_url).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let data = self.get_robots_data(robots_url, target_url.clone()).await?;
        match data.access_result {
            AccessResult::Unreachable => {
                return Ok(Response::new(IsAllowedResponse { allowed: false }));
            }
            _ => {}
        }
        let path = extract_path_from_url(&target_url)?;

        let allowed = data.is_allowed(&user_agent, &path);

        Ok(Response::new(IsAllowedResponse { allowed }))
    }
}

fn extract_path_from_url(url: &str) -> Result<String, Status> {
    let parsed = Url::parse(url).map_err(|e| Status::invalid_argument(e.to_string()))?;
    let mut path = parsed.path().to_string();
    if let Some(query) = parsed.query() {
        path.push('?');
        path.push_str(query);
    }

    Ok(path)
}
