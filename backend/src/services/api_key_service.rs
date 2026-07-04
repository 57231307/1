
use crate::models::api_key::{self, ActiveModel as ApiKeyActiveModel, Entity as ApiKey};
use crate::utils::cache::{AppCache, Cache};
use crate::utils::error::AppError;
use crate::utils::random;
use chrono::Utc;
use sea_orm::*;
use std::time::Duration;

crate::define_service!(ApiKeyService);

/// API Key 黑名单缓存 TTL（秒）
///
/// 漏洞 #5 修复：撤销后 key_hash 写入 AppCache.token_blacklist，
/// TTL 与 key 有效期对齐（最长 7 天）。TTL 过后认为黑名单自动失效。
/// 典型业务场景：用户撤销 → 立即生效 → TTL 内强制吊销。
const API_KEY_BLACKLIST_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// 黑名单缓存键前缀
const API_KEY_BLACKLIST_PREFIX: &str = "apikey:revoked:";

impl ApiKeyService {
    /// 生成新的 API 密钥
    pub fn generate_api_key() -> String {
        // 32 位字母数字随机串，统一由 `utils::random` 提供
        let key = random::random_alphanumeric(32);
        format!("bx_{}", key)
    }

    /// 哈希 API 密钥
    pub fn hash_api_key(key: &str) -> String {
        crate::utils::hash::sha256_hex(key.as_bytes())
    }

    /// 创建 API 密钥
    pub async fn create_api_key(
        &self,
        name: &str,
        permissions: Option<&str>,
        rate_limit: i32,
        expires_days: Option<i64>,
    ) -> Result<(api_key::Model, String), AppError> {
        let plain_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&plain_key);
        let key_prefix = plain_key[..8].to_string();

        let expires_at = expires_days.map(|days| Utc::now() + chrono::Duration::days(days));
        let now = Utc::now();

        let active_model = ApiKeyActiveModel {
            name: Set(name.to_string()),
            key_hash: Set(key_hash),
            key_prefix: Set(key_prefix),
            permissions: Set(permissions.map(|s| s.to_string())),
            rate_limit_per_minute: Set(rate_limit),
            last_used_at: Set(None),
            expires_at: Set(expires_at),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        let model = active_model.insert(self.db.as_ref()).await?;
        Ok((model, plain_key))
    }

    /// 撤销 API 密钥
    ///
    /// 漏洞 #5 修复：撤销时同时将 `key_hash` 加入 `AppCache.token_blacklist` 缓存。
    /// 历史问题：原实现仅设置 `is_active = false`，旧的明文 API Key
    /// （若被攻击者截获）在撤销后仍能继续使用。
    /// 修复策略：撤销时通过 `key_hash` 写入黑名单，未来 API Key 认证中间件
    /// 可通过 [`Self::is_api_key_revoked`] 检查是否已撤销。
    /// TTL 7 天后自动失效（与典型 API Key 生命周期对齐）。
    pub async fn revoke_api_key(
        &self,
        id: i32,
        cache: Option<&AppCache>,
    ) -> Result<(), AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        let mut active_model: ApiKeyActiveModel = key.clone().into();
        active_model.is_active = Set(false);
        active_model.updated_at = Set(Utc::now());
        active_model.update(self.db.as_ref()).await?;

        // 漏洞 #5 修复：将 key_hash 加入黑名单缓存，TTL 7 天
        if let Some(cache) = cache {
            let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key.key_hash);
            cache
                .get_token_blacklist()
                .set(blacklist_key, true, Some(Duration::from_secs(API_KEY_BLACKLIST_TTL_SECS)));
            tracing::info!(
                "API 密钥已撤销并加入黑名单：id={}, key_prefix={}",
                id,
                key.key_prefix
            );
        } else {
            // 调用方未传入 cache：仅 DB 标记 is_active=false，黑名单失效
            // 保留 warn 日志便于运维发现未接管的调用方
            tracing::warn!(
                "API 密钥撤销时未传入 AppCache，黑名单失效：id={}",
                id
            );
        }

        Ok(())
    }

    /// 按 ID 获取 API 密钥（批次 91 P0-1）
    pub async fn get_api_key_by_id(&self, id: i32) -> Result<Option<api_key::Model>, AppError> {
        ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 更新 API 密钥（批次 91 P0-1）
    ///
    /// 仅更新传入的字段，未传入的字段保持不变。
    pub async fn update_api_key(
        &self,
        id: i32,
        name: Option<String>,
        permissions: Option<String>,
        rate_limit_per_minute: Option<i32>,
        expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
        is_active: Option<bool>,
    ) -> Result<api_key::Model, AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        let mut active_model: ApiKeyActiveModel = key.into();
        if let Some(name) = name {
            active_model.name = Set(name);
        }
        if let Some(permissions) = permissions {
            active_model.permissions = Set(Some(permissions));
        }
        if let Some(rate_limit) = rate_limit_per_minute {
            active_model.rate_limit_per_minute = Set(rate_limit);
        }
        if let Some(expires_at) = expires_at {
            active_model.expires_at = Set(expires_at);
        }
        if let Some(is_active) = is_active {
            active_model.is_active = Set(is_active);
        }
        active_model.updated_at = Set(Utc::now());

        active_model.update(self.db.as_ref()).await.map_err(AppError::from)
    }

    /// 重新生成 API 密钥（批次 91 P0-1）
    ///
    /// 生成新的明文密钥 + 哈希，旧 key_hash 加入黑名单。
    /// 返回 (更新后的 model, 新明文密钥)。
    pub async fn regenerate_api_key(
        &self,
        id: i32,
        cache: Option<&AppCache>,
    ) -> Result<(api_key::Model, String), AppError> {
        let key = ApiKey::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::business("API 密钥不存在"))?;

        // 旧 key_hash 加入黑名单
        if let Some(cache) = cache {
            let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key.key_hash);
            cache.get_token_blacklist().set(
                blacklist_key,
                true,
                Some(Duration::from_secs(API_KEY_BLACKLIST_TTL_SECS)),
            );
        }

        // 生成新密钥
        let plain_key = Self::generate_api_key();
        let key_hash = Self::hash_api_key(&plain_key);
        let key_prefix = plain_key[..8].to_string();

        let mut active_model: ApiKeyActiveModel = key.into();
        active_model.key_hash = Set(key_hash);
        active_model.key_prefix = Set(key_prefix);
        active_model.is_active = Set(true);
        active_model.updated_at = Set(Utc::now());

        let model = active_model.update(self.db.as_ref()).await?;
        Ok((model, plain_key))
    }

    /// 检查 API Key 是否已被撤销（漏洞 #5 修复）
    ///
    /// 流程：
    /// 1. 计算明文 key 的 SHA-256 哈希
    /// 2. 检查 `AppCache.token_blacklist` 中是否存在 `<prefix><key_hash>` 条目
    ///
    /// 未来 API Key 认证中间件应在每次校验 key 前调用此方法。
    ///
    /// # 参数
    /// - `cache`: 应用全局缓存
    /// - `plain_key`: 明文 API Key（含 `bx_` 前缀）
    ///
    /// # 返回
    /// - `true`: 已撤销（拒绝使用）
    /// - `false`: 未撤销或黑名单已过期
    pub fn is_api_key_revoked(cache: &AppCache, plain_key: &str) -> bool {
        let key_hash = Self::hash_api_key(plain_key);
        let blacklist_key = format!("{}{}", API_KEY_BLACKLIST_PREFIX, key_hash);
        // Cache::get 返回 Option<V>（已 Clone），无需 .copied()
        cache
            .get_token_blacklist()
            .get(&blacklist_key)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::cache::{AppCache, Cache};

    /// 漏洞 #5 修复单元测试：未撤销的 key 不在黑名单
    ///
    /// 验证：[`ApiKeyService::is_api_key_revoked`] 对全新 key 返回 false
    #[test]
    fn test_is_api_key_revoked_returns_false_for_fresh_key() {
        let cache = AppCache::new();
        let plain_key = ApiKeyService::generate_api_key();
        assert!(
            !ApiKeyService::is_api_key_revoked(&cache, &plain_key),
            "未撤销的 API key 不应在黑名单中"
        );
    }

    /// 漏洞 #5 修复单元测试：is_api_key_revoked 检测已撤销的 key
    ///
    /// 验证：手动写入黑名单后，is_api_key_revoked 返回 true
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

    /// 漏洞 #5 修复单元测试：黑名单与 DB 状态独立
    ///
    /// 验证：黑名单仅依赖 key_hash 缓存值，不依赖 DB 中 is_active 状态
    /// （即原"DB 标记 + 黑名单"双轨机制，黑名单可独立强制吊销）
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

    /// 漏洞 #5 修复单元测试：黑名单键格式包含 hash 防冲突
    ///
    /// 验证：不同 plain_key 的 hash 不会碰撞到同一黑名单条目
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
}
