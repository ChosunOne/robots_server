use tonic::{Request, Response, Status, transport::Server};

use robots::{
    GetRobotsRequest, GetRobotsResponse,
    robots_service_server::{RobotsService, RobotsServiceServer},
};

pub mod robots {
    tonic::include_proto!("robots");
}

pub struct RobotsServer;

#[tonic::async_trait]
impl RobotsService for RobotsServer {
    async fn get_robots_txt(
        &self,
        request: Request<GetRobotsRequest>,
    ) -> Result<Response<GetRobotsResponse>, Status> {
        let req = request.into_inner();
        println!("Got request: {req:?}");
        let response = GetRobotsResponse {
            target_url: req.url,
            robots_txt_url: "https://example.com/robots.txt".to_string(),
            access_result: 1,
            http_status_code: 200,
            groups: vec![],
            sitemaps: vec![],
            content_length_bytes: 0,
            truncated: false,
        };

        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = RobotsServer;

    Server::builder()
        .add_service(RobotsServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
