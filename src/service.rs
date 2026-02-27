use tonic::{Request, Response, Status, transport::Server};

use robots::{
    AccessResult, GetRobotsRequest, GetRobotsResponse,
    robots_service_server::{RobotsService, RobotsServiceServer},
};

use crate::{cache::Cache, robots_data::RobotsData};

pub mod robots {
    include!("generated/robots.rs");
}

pub struct RobotsServer<T: Cache<String, RobotsData>> {
    cache: T,
}

impl<T: Cache<String, RobotsData>> RobotsServer<T> {
    pub fn new(cache: T) -> Self {
        Self { cache }
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
        let data = self
            .cache
            .get(&req.url)
            .await
            .map_err(|e| Status::unavailable("Cache unavailable"))?
            .unwrap_or_else(|| RobotsData {
                target_url: req.url.clone(),
                robots_txt_url: "".to_string(),
                access_result: AccessResult::Unavailable,
                ..Default::default()
            });

        Ok(Response::new(data.into()))
    }
}
