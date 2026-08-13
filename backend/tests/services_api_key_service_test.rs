use std::time::Duration;
use bingxi_backend::services::api_key_service::{ApiKeyService, API_KEY_BLACKLIST_PREFIX};
use bingxi_backend::utils::cache::{AppCache, Cache};
use chrono::Duration;

/// 漏洞 #5 修复单元测试：未撤销的 key 不在黑名单（验证：[`ApiKeyService::is_api_key_revoked`] 对全新 key 返回 false）
#[test]
fn test_is_api_key_revoked_returns_false_for_fresh_key() {
    let cache = AppCache::new();
    let plain_key = ApiKeyService::generate_api_key();
    assert!(
        !ApiKeyService::is_api_key_revoked(&cache, &plain_key),
        "未撤销的 API key 不应在黑名单中"
    );
}

/// 漏洞 #5 修复单元测试：is_api_key_revoked 检测已撤销的 key（验证：手动写入黑名单后，is_api_key_revoked 返回 true）
#[test]
fn test_is_api_key_revoked_detects_blacklisted_key() {
    let cache = AppCache::new();
    let plain_key = ApiKeyService::generate_api_key();
    let key_hash = ApiKeyService::hash_api_key(&plain_key);
    let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key_hash);

    // 模拟撤销：手动写入黑名单
    cache
        .get_token_blacklist()
        .set(blacklist_key, true, Some(Duration::from_secs(60)));

    // 黑名单应能检测到
    assert!(
        ApiKeyService::is_api_key_revoked(&cache, &plain_key),
        "已撤销的 API key 应被黑名单识别"
    );

    // 不同的 key 不会被误判
    let other_key = ApiKeyService::generate_api_key();
    assert!(
        !ApiKeyService::is_api_key_revoked(&cache, &other_key),
        "其他未撤销的 API key 不应被黑名单误判"
    );
}

/// 漏洞 #5 修复单元测试：黑名单与 DB 状态独立（验证：黑名单仅依赖 key_hash 缓存值，不依赖 DB 中 is_active 状态；（即原"DB 标记 + 黑名单"双轨机制，黑名单可独立强制吊销））
#[test]
fn test_blacklist_independent_from_db_state() {
    let cache = AppCache::new();
    let plain_key = ApiKeyService::generate_api_key();
    let key_hash = ApiKeyService::hash_api_key(&plain_key);
    let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key_hash);

    // 仅写入黑名单（不更新 DB）
    cache
        .get_token_blacklist()
        .set(blacklist_key, true, Some(Duration::from_secs(60)));

    // 即便 DB 中此 key 仍 is_active=true，黑名单也应能强制吊销
    assert!(
        ApiKeyService::is_api_key_revoked(&cache, &plain_key),
        "黑名单应独立于 DB 状态强制吊销"
    );
}

/// 漏洞 #5 修复单元测试：黑名单键格式包含 hash 防冲突（验证：不同 plain_key 的 hash 不会碰撞到同一黑名单条目）
#[test]
fn test_blacklist_keys_dont_collide() {
    let cache = AppCache::new();
    let key1 = ApiKeyService::generate_api_key();
    let key2 = ApiKeyService::generate_api_key();
    assert_ne!(key1, key2, "两次生成应得到不同 key");
    assert_ne!(
        ApiKeyService::hash_api_key(&key1),
        ApiKeyService::hash_api_key(&key2),
        "不同 key 应有不同 hash"
    );

    // 撤销 key1
    let blacklist_key1 = format!(
        "{}{}",
        API_KEY_BLACKLIST_PREFIX,
        ApiKeyService::hash_api_key(&key1)
    );
    cache
        .get_token_blacklist()
        .set(blacklist_key1, true, Some(Duration::from_secs(60)));

    // key1 被吊销，key2 不受影响
    assert!(ApiKeyService::is_api_key_revoked(&cache, &key1));
    assert!(!ApiKeyService::is_api_key_revoked(&cache, &key2));
}