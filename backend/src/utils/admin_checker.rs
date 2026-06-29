use crate::models::role;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use std::sync::LazyLock;
use tracing::warn;

/// 管理员角色编码常量（批次 23 v5 P0-3：消除硬编码字符串）
/// 作为角色编码的单一真相源，避免多处硬编码 "admin" 导致不一致。
pub const ADMIN_ROLE_CODE: &str = "admin";

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
pub fn clear_admin_role_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        ADMIN_ROLE_CACHE.remove(&id);
    } else {
        ADMIN_ROLE_CACHE.clear();
    }
}

/// 清理过期的管理员角色缓存条目
#[allow(dead_code)] // TODO(tech-debt): 缓存清理定时任务接入后移除
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
    // 批次 23（2026-06-29 v5 P0-3）：使用 ADMIN_ROLE_CODE 常量替代硬编码 "admin"
    // 批次 23（2026-06-29 v5 P0-3）：修复 fail-open 安全漏洞
    //   原实现：数据库表不存在时返回 true（允许访问），系统未初始化时任何 role_id 都被视为管理员，
    //   存在权限绕过风险。改为 fail-closed（拒绝访问），确保系统未初始化时不放行。
    let is_admin = match role::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(role)) => role.code == ADMIN_ROLE_CODE,
        Ok(None) => false,
        Err(e) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("does not exist") || err_msg.contains("relation") {
                warn!("数据库表不存在，系统可能未初始化，拒绝访问（fail-closed）: {}", e);
                false
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
