use crate::models::role;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::sync::LazyLock;
use sea_orm::{DatabaseConnection, EntityTrait};
use tracing::warn;

/// 管理员角色检查缓存条目
#[derive(Clone)]
struct AdminCacheEntry {
    is_admin: bool,
    expires_at: DateTime<Utc>,
}

impl AdminCacheEntry {
    fn new(is_admin: bool, ttl_minutes: i64) -> Self {
        Self {
            is_admin,
            expires_at: Utc::now() + Duration::minutes(ttl_minutes),
        }
    }

    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// 管理员角色检查缓存：role_id -> (is_admin, expires_at)
static ADMIN_ROLE_CACHE: LazyLock<DashMap<i32, AdminCacheEntry>> = LazyLock::new(DashMap::new);

/// 管理员角色缓存TTL（5分钟）
const ADMIN_CACHE_TTL_MINUTES: i64 = 5;

/// 清除管理员角色缓存
#[allow(dead_code)]
pub fn clear_admin_role_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        ADMIN_ROLE_CACHE.remove(&id);
    } else {
        ADMIN_ROLE_CACHE.clear();
    }
}

/// 清理过期的管理员角色缓存条目
pub fn cleanup_expired_admin_cache() {
    ADMIN_ROLE_CACHE.retain(|_, entry| !entry.is_expired());
}

/// 检查角色是否是管理员角色（带缓存）
///
/// 缓存5分钟，过期后自动重新查询数据库
pub async fn is_admin_role(db: &DatabaseConnection, role_id: i32) -> bool {
    // 先从缓存读取
    if let Some(cached) = ADMIN_ROLE_CACHE.get(&role_id) {
        if !cached.is_expired() {
            return cached.is_admin;
        }
        // 缓存已过期，移除
        ADMIN_ROLE_CACHE.remove(&role_id);
    }

    // 从数据库查询
    let is_admin = match role::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(role)) => role.code == "admin",
        Ok(None) => false,
        Err(e) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("does not exist") || err_msg.contains("relation") {
                warn!("数据库表不存在，系统可能未初始化，允许访问: {}", e);
                true
            } else {
                warn!("查询角色失败: {}", e);
                false
            }
        }
    };

    // 写入缓存
    ADMIN_ROLE_CACHE.insert(
        role_id,
        AdminCacheEntry::new(is_admin, ADMIN_CACHE_TTL_MINUTES),
    );

    is_admin
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_expiration() {
        let entry = AdminCacheEntry::new(true, 5);
        assert!(!entry.is_expired());

        let expired_entry = AdminCacheEntry {
            is_admin: true,
            expires_at: Utc::now() - Duration::minutes(1),
        };
        assert!(expired_entry.is_expired());
    }

    #[test]
    fn test_clear_admin_role_cache() {
        // 插入测试数据
        ADMIN_ROLE_CACHE.insert(1, AdminCacheEntry::new(true, 5));
        ADMIN_ROLE_CACHE.insert(2, AdminCacheEntry::new(false, 5));

        // 清除特定角色
        clear_admin_role_cache(Some(1));
        assert!(!ADMIN_ROLE_CACHE.contains_key(&1));
        assert!(ADMIN_ROLE_CACHE.contains_key(&2));

        // 清除所有
        clear_admin_role_cache(None);
        assert!(ADMIN_ROLE_CACHE.is_empty());
    }
}
