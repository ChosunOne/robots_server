use robots_server::cache::{Cache, MokaCache};

#[tokio::test]
async fn test_cache_get_set() {
    let cache: MokaCache<String, String> = MokaCache::new();

    // Get on empty returns None
    let result = cache.get(&"key".to_string()).await.unwrap();
    assert!(result.is_none());

    // Set value
    cache
        .set("key".to_string(), "value".to_string())
        .await
        .unwrap();

    // Get returns the value
    let result = cache.get(&"key".to_string()).await.unwrap();
    assert_eq!(result, Some("value".to_string()));
}
#[tokio::test]
async fn test_cache_delete() {
    let cache: MokaCache<String, String> = MokaCache::new();

    // Delete non-existent returns false
    let result = cache.delete(&"key".to_string()).await.unwrap();
    assert!(!result);

    // Set and delete
    cache
        .set("key".to_string(), "value".to_string())
        .await
        .unwrap();
    let result = cache.delete(&"key".to_string()).await.unwrap();
    assert!(result);

    // Verify deleted
    let result = cache.get(&"key".to_string()).await.unwrap();
    assert!(result.is_none());

    // Delete again returns false
    let result = cache.delete(&"key".to_string()).await.unwrap();
    assert!(!result);
}
#[tokio::test]
async fn test_cache_multiple_keys() {
    let cache: MokaCache<String, String> = MokaCache::new();

    cache
        .set("key1".to_string(), "value1".to_string())
        .await
        .unwrap();
    cache
        .set("key2".to_string(), "value2".to_string())
        .await
        .unwrap();

    assert_eq!(
        cache.get(&"key1".to_string()).await.unwrap(),
        Some("value1".to_string())
    );
    assert_eq!(
        cache.get(&"key2".to_string()).await.unwrap(),
        Some("value2".to_string())
    );
}
#[tokio::test]
async fn test_cache_complex_value() {
    use robots_server::robots_data::RobotsData;

    let cache: MokaCache<String, RobotsData> = MokaCache::new();

    let data = RobotsData {
        target_url: "https://example.com".to_string(),
        robots_txt_url: "https://example.com/robots.txt".to_string(),
        http_status_code: 200,
        ..Default::default()
    };

    cache.set("key".to_string(), data.clone()).await.unwrap();

    let result = cache.get(&"key".to_string()).await.unwrap();
    assert_eq!(result.unwrap().target_url, "https://example.com");
}
#[tokio::test]
async fn test_cache_clone_behavior() {
    let cache: MokaCache<String, Vec<u8>> = MokaCache::new();

    let data = vec![1, 2, 3];
    cache.set("key".to_string(), data.clone()).await.unwrap();

    let result = cache.get(&"key".to_string()).await.unwrap().unwrap();
    assert_eq!(result, vec![1, 2, 3]);

    assert_eq!(data, vec![1, 2, 3]);
}
