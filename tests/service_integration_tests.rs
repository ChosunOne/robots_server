use robots_server::cache::MokaCache;
use robots_server::fetcher::RobotsFetcher;
use robots_server::service::robots::robots_service_server::RobotsService;
use robots_server::service::robots::{AccessResult, IsAllowedRequest};
use robots_server::service::{RobotsServer, robots::GetRobotsRequest};
use tonic::Request;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_service_cache_miss_then_hit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /private"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/", mock_server.address());

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(response.get_ref().http_status_code, 200);

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(response.get_ref().http_status_code, 200);
}
#[tokio::test]
async fn test_service_404_is_cached() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/", mock_server.address());

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(
        response.get_ref().access_result,
        AccessResult::Unavailable as i32
    );

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(
        response.get_ref().access_result,
        AccessResult::Unavailable as i32
    );
}
#[tokio::test]
async fn test_service_500_is_cached() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/", mock_server.address());

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(
        response.get_ref().access_result,
        AccessResult::Unreachable as i32
    );

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let response = service.get_robots_txt(request).await.unwrap();
    assert_eq!(
        response.get_ref().access_result,
        AccessResult::Unreachable as i32
    );
}
#[tokio::test]
async fn test_service_invalid_url() {
    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let request = Request::new(GetRobotsRequest {
        url: "not-a-valid-url".to_string(),
    });

    let result = service.get_robots_txt(request).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
}
#[tokio::test]
async fn test_service_different_urls_different_cache() {
    let mock_server_1 = MockServer::start().await;
    let mock_server_2 = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nAllow: /"))
        .expect(1)
        .mount(&mock_server_1)
        .await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nAllow: /"))
        .expect(1)
        .mount(&mock_server_2)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url1 = format!("http://{}/", mock_server_1.address());
    let url2 = format!("http://{}/", mock_server_2.address());

    let request = Request::new(GetRobotsRequest { url: url1 });
    service.get_robots_txt(request).await.unwrap();

    let request = Request::new(GetRobotsRequest { url: url2 });
    service.get_robots_txt(request).await.unwrap();
}

#[tokio::test]
async fn test_service_timeout_is_cached() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(31)) // Exceeds 30s timeout
                .set_body_string("User-agent: *\nDisallow: /"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;
    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);
    let url = format!("http://{}/", mock_server.address());
    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let _ = service.get_robots_txt(request).await;

    let request = Request::new(GetRobotsRequest { url: url.clone() });
    let _ = service.get_robots_txt(request).await;
}

#[tokio::test]
async fn test_is_allowed_simple_allow() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nAllow: /"))
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/page.html", mock_server.address());
    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "MyBot".to_string(),
    });

    let response = service.is_allowed(request).await.unwrap();
    assert!(response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_simple_disallow() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /admin/"),
        )
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/admin/secret.html", mock_server.address());
    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "MyBot".to_string(),
    });

    let response = service.is_allowed(request).await.unwrap();
    assert!(!response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_specific_user_agent() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: MyBot\nDisallow: /\n\nUser-agent: *\nAllow: /"),
        )
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let base_url = format!("http://{}", mock_server.address());

    let request = Request::new(IsAllowedRequest {
        target_url: format!("{}/page.html", base_url),
        user_agent: "MyBot".to_string(),
    });
    let response = service.is_allowed(request).await.unwrap();
    assert!(!response.get_ref().allowed);

    let request = Request::new(IsAllowedRequest {
        target_url: format!("{}/page.html", base_url),
        user_agent: "OtherBot".to_string(),
    });
    let response = service.is_allowed(request).await.unwrap();
    assert!(response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_unavailable_robots_txt() {
    let mock_server = MockServer::start().await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/page.html", mock_server.address());
    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "MyBot".to_string(),
    });

    let response = service.is_allowed(request).await.unwrap();
    assert!(response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_with_query_string() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string("User-agent: *\nDisallow: /search?"),
        )
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/search?q=test", mock_server.address());
    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "MyBot".to_string(),
    });

    let response = service.is_allowed(request).await.unwrap();
    assert!(!response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_empty_path() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(ResponseTemplate::new(200).set_body_string("User-agent: *\nAllow: /"))
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/", mock_server.address());
    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "MyBot".to_string(),
    });

    let response = service.is_allowed(request).await.unwrap();
    assert!(response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_wildcard_matching() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: *\nDisallow: /*.pdf$\nAllow: /"),
        )
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let base_url = format!("http://{}", mock_server.address());

    let request = Request::new(IsAllowedRequest {
        target_url: format!("{}/file.pdf", base_url),
        user_agent: "MyBot".to_string(),
    });
    let response = service.is_allowed(request).await.unwrap();
    assert!(!response.get_ref().allowed);

    let request = Request::new(IsAllowedRequest {
        target_url: format!("{}/page.html", base_url),
        user_agent: "MyBot".to_string(),
    });
    let response = service.is_allowed(request).await.unwrap();
    assert!(response.get_ref().allowed);
}
#[tokio::test]
async fn test_is_allowed_case_insensitive_user_agent() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/robots.txt"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("User-agent: Googlebot\nDisallow: /\n\nUser-agent: *\nAllow: /"),
        )
        .mount(&mock_server)
        .await;

    let cache = MokaCache::new();
    let fetcher = RobotsFetcher::new();
    let service = RobotsServer::new(cache, fetcher);

    let url = format!("http://{}/page.html", mock_server.address());

    let request = Request::new(IsAllowedRequest {
        target_url: url,
        user_agent: "googlebot/1.0".to_string(),
    });
    let response = service.is_allowed(request).await.unwrap();
    assert!(!response.get_ref().allowed);
}
