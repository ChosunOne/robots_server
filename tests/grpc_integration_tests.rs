use robots_server::cache::MokaCache;
use robots_server::fetcher::RobotsFetcher;
use robots_server::service::robots::{AccessResult, GetRobotsRequest};
use robots_server::service::{RobotsServer, robots::robots_service_server::RobotsServiceServer};
use tonic::transport::Server;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_full_grpc_success() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /admin"))
        .mount(&mock_server)
        .await;

    let addr = "[::1]:50051".parse().unwrap();
    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let (tx, rx) = tokio::sync::oneshot::channel();

    let server = Server::builder()
        .add_service(RobotsServiceServer::new(service))
        .serve_with_shutdown(addr, async {
            rx.await.ok();
        });

    let server_handle = tokio::spawn(server);

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let channel = tonic::transport::Channel::from_static("http://[::1]:50051")
        .connect()
        .await
        .unwrap();

    let mut client =
        robots_server::service::robots::robots_service_client::RobotsServiceClient::new(channel);

    let url = format!("http://{}/", mock_server.address());
    let request = tonic::Request::new(GetRobotsRequest { url });

    let response = client.get_robots_txt(request).await.unwrap();

    assert_eq!(response.get_ref().http_status_code, 200);
    assert_eq!(
        response.get_ref().access_result,
        AccessResult::Success as i32
    );
    assert_eq!(response.get_ref().groups.len(), 1);

    tx.send(()).unwrap();
    server_handle.await.unwrap().unwrap();
}
