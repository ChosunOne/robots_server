use robots_server::fetcher::{FetchError, RobotsFetcher};
use robots_server::service::robots::AccessResult;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_fetch_success_200() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "User-agent: *\nDisallow: /private\n\nSitemap: https://example.com/sitemap.xml",
        ))
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await.unwrap();

    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.access_result, AccessResult::Success);
    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.sitemaps.len(), 1);
    assert_eq!(result.sitemaps[0], "https://example.com/sitemap.xml");
}
#[tokio::test]
async fn test_fetch_404() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await;

    assert!(matches!(result, Err(FetchError::Unavailable(404))));
}
#[tokio::test]
async fn test_fetch_403() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await;

    assert!(matches!(result, Err(FetchError::Unavailable(403))));
}
#[tokio::test]
async fn test_fetch_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await;

    assert!(matches!(
        result,
        Err(FetchError::Unreachable((_, Some(500))))
    ));
}
#[tokio::test]
async fn test_fetch_empty_robots_txt() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await.unwrap();

    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.access_result, AccessResult::Success);
    assert!(result.groups.is_empty());
}
#[tokio::test]
async fn test_fetch_large_content() {
    let mock_server = MockServer::start().await;

    let large_content = "User-agent: *\nDisallow: /private\n".repeat(100);

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&large_content)
                .insert_header("content-length", large_content.len().to_string()),
        )
        .mount(&mock_server)
        .await;

    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());

    let result = fetcher.fetch(&url).await.unwrap();

    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.content_length_bytes, large_content.len() as u64);
}
