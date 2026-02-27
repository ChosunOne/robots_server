use tonic::{Request, Response, Status, transport::Server};

use robots::{
    AccessResult, GetRobotsRequest, GetRobotsResponse,
    robots_service_server::{RobotsService, RobotsServiceServer},
};

use crate::{
    cache::Cache,
    fetcher::{FetchError, RobotsFetcher},
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
    async fn get_robots_txt(
        &self,
        request: Request<GetRobotsRequest>,
    ) -> Result<Response<GetRobotsResponse>, Status> {
        let req = request.into_inner();
        println!("Got request: {req:?}");
        match self.cache.get(&req.url).await {
            Ok(Some(data)) => Ok(Response::new(data.into())),
            Ok(None) => match self.fetcher.fetch(&req.url).await {
                Ok(data) => {
                    self.cache.set(req.url.clone(), data.clone()).await.ok();
                    Ok(Response::new(data.into()))
                }
                Err(FetchError::Unavailable(_)) => Ok(Response::new(
                    RobotsData {
                        target_url: req.url,
                        robots_txt_url: "".to_string(),
                        access_result: AccessResult::Unavailable,
                        ..Default::default()
                    }
                    .into(),
                )),
                Err(e) => Err(Status::internal(e.to_string())),
            },
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
