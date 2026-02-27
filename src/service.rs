use tonic::{Request, Response, Status, transport::Server};

use robots::{
    AccessResult, GetRobotsRequest, GetRobotsResponse,
    robots_service_server::{RobotsService, RobotsServiceServer},
};

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
    async fn get_robots_txt(
        &self,
        request: Request<GetRobotsRequest>,
    ) -> Result<Response<GetRobotsResponse>, Status> {
        let req = request.into_inner();
        let robots_url = extract_robots_url(&req.url).expect("invalid url");
        println!("Got request: {req:?}");
        match self.cache.get(&robots_url).await {
            Ok(Some(data)) => Ok(Response::new(data.into())),
            Ok(None) => match self.fetcher.fetch(&req.url).await {
                Ok(data) => {
                    self.cache
                        .set(data.robots_txt_url.clone(), data.clone())
                        .await
                        .ok();
                    Ok(Response::new(data.into()))
                }
                Err(FetchError::Unavailable(s)) => {
                    println!("got status code: {s}");
                    Ok(Response::new(
                        RobotsData {
                            target_url: req.url,
                            robots_txt_url: "".to_string(),
                            access_result: AccessResult::Unavailable,
                            ..Default::default()
                        }
                        .into(),
                    ))
                }
                Err(e) => Err(Status::internal(e.to_string())),
            },
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }
}
