use robots_server::fetcher::extract_robots_url;

#[test]
fn test_extract_standard_https() {
    assert_eq!(
        extract_robots_url("https://example.com"),
        Ok("https://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_standard_http() {
    assert_eq!(
        extract_robots_url("http://example.com"),
        Ok("http://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_custom_port_https() {
    assert_eq!(
        extract_robots_url("https://example.com:8443"),
        Ok("https://example.com:8443/robots.txt".to_string())
    );
}
#[test]
fn test_extract_custom_port_http() {
    assert_eq!(
        extract_robots_url("http://example.com:8080"),
        Ok("http://example.com:8080/robots.txt".to_string())
    );
}
#[test]
fn test_extract_standard_port_omitted_https() {
    assert_eq!(
        extract_robots_url("https://example.com:443"),
        Ok("https://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_standard_port_omitted_http() {
    assert_eq!(
        extract_robots_url("http://example.com:80"),
        Ok("http://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_with_path() {
    assert_eq!(
        extract_robots_url("https://example.com/path/to/page"),
        Ok("https://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_with_query_params() {
    assert_eq!(
        extract_robots_url("https://example.com?foo=bar"),
        Ok("https://example.com/robots.txt".to_string())
    );
}
#[test]
fn test_extract_invalid_url() {
    let result = extract_robots_url("not-a-valid-url");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid URL"));
}
#[test]
fn test_extract_unsupported_scheme() {
    let result = extract_robots_url("ftp://example.com");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported scheme")
    );
}
#[test]
fn test_extract_no_host() {
    let result = extract_robots_url("https://");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty host"));
}
