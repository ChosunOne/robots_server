use robots_server::service::{RobotsServer, robots::robots_service_server::RobotsServiceServer};
use tonic::transport::Server;

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
