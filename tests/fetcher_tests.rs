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

#[tokio::test]
async fn test_fetch_truncation_at_550kb() {
    let mock_server = MockServer::start().await;
    let line = "User-agent: bot_DISALLOW VERY LONG PATH HERE\nDisallow: /very/long/path/that/should/be/truncated\n";
    let lines_needed = 563_200 / line.len() + 10; // Ensure we exceed 550KB
    let large_content = line.repeat(lines_needed);

    assert!(
        large_content.len() > 550 * 1024,
        "Test content should exceed 550KB"
    );
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

    assert!(result.truncated, "Should be marked as truncated");
    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.access_result, AccessResult::Success);

    let body_bytes = large_content.as_bytes();
    let expected_boundary = 550 * 1024;

    let _ = body_bytes[..expected_boundary]
        .iter()
        .rposition(|&b| b == b'\n')
        .expect("Should have a newline before 550KB");

    assert!(
        result.content_length_bytes > 550 * 1024 as u64,
        "Original content_length should show full size"
    );

    assert!(!result.groups.is_empty(), "Should have parsed some groups");
}

#[tokio::test]
async fn test_fetch_accepts_text_plain() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nDisallow: /")
                .insert_header("content-type", "text/plain"),
        )
        .mount(&mock_server)
        .await;
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());
    let result = fetcher.fetch(&url).await.unwrap();

    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.access_result, AccessResult::Success);
}
#[tokio::test]
async fn test_fetch_accepts_text_plain_with_charset() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nDisallow: /")
                .insert_header("content-type", "text/plain; charset=utf-8"),
        )
        .mount(&mock_server)
        .await;
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());
    let result = fetcher.fetch(&url).await.unwrap();

    assert_eq!(result.http_status_code, 200);
}
#[tokio::test]
async fn test_fetch_case_insensitive() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nDisallow: /")
                .insert_header("content-type", "TEXT/PLAIN"), // uppercase
        )
        .mount(&mock_server)
        .await;
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());
    let result = fetcher.fetch(&url).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_truncation_no_newlines() {
    let mock_server = MockServer::start().await;
    let content = "User-agent: bot".repeat(40_000); // ~680KB, no newlines
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string(content))
        .mount(&mock_server)
        .await;
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", mock_server.address());
    let result = fetcher.fetch(&url).await.unwrap();
    assert!(
        result.groups.len() > 0,
        "Should have parsed at least one group"
    );
    assert!(result.truncated, "Should be marked as truncated");
}

#[tokio::test]
async fn test_fetch_follows_redirect() {
    let redirect_server = MockServer::start().await;
    let target_server = MockServer::start().await;
    // Target server returns actual robots.txt
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /"))
        .mount(&target_server)
        .await;
    // Redirect server returns 301 to target
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(301).insert_header(
            "location",
            format!("http://{}/robots.txt", target_server.address()),
        ))
        .mount(&redirect_server)
        .await;
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", redirect_server.address());
    let result = fetcher.fetch(&url).await.unwrap();
    assert_eq!(result.http_status_code, 200);
    assert_eq!(result.access_result, AccessResult::Success);
}

#[tokio::test]
async fn test_fetch_too_many_redirects() {
    // Create a chain of 7 servers (6 redirects)
    let mut servers = Vec::new();
    for _ in 0..7 {
        servers.push(MockServer::start().await);
    }

    // Last server returns actual content
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /"))
        .mount(&servers[6])
        .await;
    // Each server redirects to the next (6 redirects total)
    for i in 0..6 {
        Mock::given(method("GET"))
            .and(path("/robots.txt"))
            .respond_with(ResponseTemplate::new(301).insert_header(
                "location",
                format!("http://{}/robots.txt", servers[i + 1].address()),
            ))
            .mount(&servers[i])
            .await;
    }
    let fetcher = RobotsFetcher::new();
    let url = format!("http://{}/", servers[0].address());
    let result = fetcher.fetch(&url).await;
    // Should fail after 5 redirects (6th redirect exceeds limit)
    assert!(result.is_err());
}
