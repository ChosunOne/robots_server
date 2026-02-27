use tonic::{Request, Response, Status};

use robots::{
    AccessResult, GetRobotsRequest, GetRobotsResponse, robots_service_server::RobotsService,
};
use tracing::{Span, debug, info, instrument, warn};

use crate::{
    cache::Cache,
    fetcher::{FetchError, RobotsFetcher, extract_robots_url},
    robots_data::RobotsData,
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

        Span::current().record("robots_url", &robots_url);
        info!("Processing robots.txt request");

        match self.cache.get(&robots_url).await {
            Ok(Some(data)) => {
                debug!("Cache hit for request");
                Ok(Response::new(data.into()))
            }
            Ok(None) => {
                debug!("Cache miss for request, fetching from origin");
                match self.fetcher.fetch(&req.url).await {
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
                        Ok(Response::new(data.into()))
                    }
                    Err(FetchError::Unavailable(s)) => {
                        info!(status_code = s, "robots.txt unavailable");
                        let data = RobotsData {
                            target_url: req.url,
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
                        Ok(Response::new(data.into()))
                    }
                    Err(FetchError::Unreachable(e)) => {
                        info!(error = %e.0, status = e.1, "robots.txt unreachable");
                        let s = e.1.unwrap_or(0);
                        let data = RobotsData {
                            target_url: req.url,
                            robots_txt_url: robots_url,
                            access_result: AccessResult::Unreachable,
                            http_status_code: s as u32,
                            ..Default::default()
                        };
                        Ok(Response::new(data.into()))
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
