use bingxi_backend::services::cache_backend::*;


#[tokio::test]
async fn test_memory_backend_set_get() {
    let backend = MemoryCacheBackend::new();
    backend.set("k1".to_string(), b"v1".to_vec()).await;
    assert_eq!(backend.get("k1").await, Some(b"v1".to_vec()));
    assert_eq!(backend.get("missing").await, None);
}

#[tokio::test]
async fn test_memory_backend_invalidate() {
    let backend = MemoryCacheBackend::new();
    backend.set("k1".to_string(), b"v1".to_vec()).await;
    backend.invalidate("k1").await;
    assert_eq!(backend.get("k1").await, None);
}

#[tokio::test]
async fn test_memory_backend_invalidate_prefix() {
    let backend = MemoryCacheBackend::new();
    backend.set("inventory:1".to_string(), b"v1".to_vec()).await;
    backend.set("inventory:2".to_string(), b"v2".to_vec()).await;
    backend.set("sales:1".to_string(), b"v3".to_vec()).await;

    backend.invalidate_prefix("inventory:").await;
    assert_eq!(backend.get("inventory:1").await, None);
    assert_eq!(backend.get("inventory:2").await, None);
    assert_eq!(backend.get("sales:1").await, Some(b"v3".to_vec()));
}

#[tokio::test]
async fn test_mock_backend_yszbzjjs() {
    let mock = MockCacheBackend::new();
    mock.set_value("k1".to_string(), b"v1".to_vec());
    assert_eq!(mock.set_call_count(), 0, "set_value 不应增加 set 计数");
    assert_eq!(mock.get("k1").await, Some(b"v1".to_vec()));
    assert_eq!(mock.get_call_count(), 1, "get 应增加 get 计数");
}

#[tokio::test]
async fn test_mock_backend_set_zjjs() {
    let mock = MockCacheBackend::new();
    mock.set("k1".to_string(), b"v1".to_vec()).await;
    mock.set("k2".to_string(), b"v2".to_vec()).await;
    assert_eq!(mock.set_call_count(), 2, "set 应被计数");
    assert_eq!(mock.len(), 2);
}

#[tokio::test]
async fn test_mock_backend_invalidate_js() {
    let mock = MockCacheBackend::new();
    mock.set_value("k1".to_string(), b"v1".to_vec());
    mock.invalidate("k1").await;
    assert_eq!(mock.invalidate_call_count(), 1);
    assert_eq!(mock.get("k1").await, None);
}

#[tokio::test]
async fn test_mock_backend_invalidate_prefix_js() {
    let mock = MockCacheBackend::new();
    mock.set_value("inv:1".to_string(), b"v1".to_vec());
    mock.set_value("inv:2".to_string(), b"v2".to_vec());
    mock.set_value("sales:1".to_string(), b"v3".to_vec());

    mock.invalidate_prefix("inv:").await;
    assert_eq!(mock.invalidate_prefix_call_count(), 1);
    assert_eq!(mock.get("inv:1").await, None);
    assert_eq!(mock.get("inv:2").await, None);
    assert_eq!(mock.get("sales:1").await, Some(b"v3".to_vec()));
}

#[tokio::test]
async fn test_mock_backend_dtzr_trait() {
    // V15 批次 07 P1-7 修复核心目标：通过 trait 对象动态注入，单测不依赖真实 moka
    async fn exercise_backend(backend: &dyn CacheBackend) {
        backend.set("k1".to_string(), b"v1".to_vec()).await;
        let _ = backend.get("k1").await;
        backend.invalidate("k1").await;
    }

    let mock: Box<dyn CacheBackend> = Box::new(MockCacheBackend::new());
    exercise_backend(mock.as_ref()).await;
    // 不 panic 即通过
}