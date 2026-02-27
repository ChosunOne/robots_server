use robots_server::{
    cache::MokaCache,
    fetcher::RobotsFetcher,
    service::{RobotsServer, robots::robots_service_server::RobotsServiceServer},
};
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let addr = "[::1]:50051".parse()?;
    info!(%addr, "Starting robots-server");
    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    Server::builder()
        .add_service(RobotsServiceServer::new(service))
        .serve(addr)
        .await?;

    info!("Shutting down");

    Ok(())
}
