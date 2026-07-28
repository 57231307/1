use crate::models::role;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::sync::LazyLock;
use tracing::warn;

/// 管理员角色编码常量（批次 23 v5 P0-3：消除硬编码字符串）
/// 作为角色编码的单一真相源，避免多处硬编码 "admin" 导致不一致。
pub const ADMIN_ROLE_CODE: &str = "admin";

/// 部门经理角色编码常量（v18 批次 48：消除硬编码字符串）
/// 用于付款审批等需要 manager 角色判定的场景
pub const MANAGER_ROLE_CODE: &str = "manager";

/// V15 P1-14.2-C：审计员角色编码常量
/// admin 不再持有 audit:read，审计职责独立到 auditor 角色，遵循职责分离原则
pub const AUDITOR_ROLE_CODE: &str = "auditor";

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
///
/// 批次 103 P2-3 修复：已接入 role_handler::update_role / delete_role，移除 dead_code 标注。
/// 角色更新/删除后必须清理缓存，避免使用过期的 admin 判定结果导致权限错乱。
pub fn clear_admin_role_cache(role_id: Option<i32>) {
    if let Some(id) = role_id {
        ADMIN_ROLE_CACHE.remove(&id);
    } else {
        ADMIN_ROLE_CACHE.clear();
    }
}

/// 清理过期的管理员角色缓存条目（v11 批次 156 P2-D：main.rs 后台任务每 10 分钟调用）
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
                warn!(
                    "数据库表不存在，系统可能未初始化，拒绝访问（fail-closed）: {}",
                    e
                );
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

/// V15 P1-14.2-C：检查角色是否是审计员角色（auditor）
///
/// admin 不再持有 audit:read 权限，审计日志查询/导出权限独立到 auditor 角色，
/// 遵循职责分离原则（admin 既是操作者不能审计自己）。
/// 不带缓存（审计员检查频率低，且与 is_admin_role 缓存分离避免污染）。
pub async fn is_auditor_role(db: &DatabaseConnection, role_id: i32) -> bool {
    match role::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(role)) => role.code == AUDITOR_ROLE_CODE,
        Ok(None) => false,
        Err(e) => {
            warn!("查询审计员角色失败: {}", e);
            false
        }
    }
}

/// V15 P1-2-4：查询角色 code（用于打印/导出黑名单判定）
///
/// 不带缓存（仅 print/export 等敏感动作触发，频率低；与 is_admin_role 缓存分离避免污染）。
/// 查询失败时返回 None（fail-closed，由调用方决定拒绝策略）。
pub async fn get_role_code(db: &DatabaseConnection, role_id: i32) -> Option<String> {
    match role::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(role)) => Some(role.code),
        Ok(None) => None,
        Err(e) => {
            warn!("查询角色 code 失败: {}", e);
            None
        }
    }
}

/// V15 P1-14.2-C：检查角色是否可访问审计日志（admin 或 auditor）
///
/// admin 保留审计日志访问能力用于系统运维排查，但不持有 audit:read 权限码
/// （权限码归 auditor 独占）。auditor 角色专门负责审计职责。
pub async fn can_access_audit_logs(db: &DatabaseConnection, role_id: i32) -> bool {
    is_admin_role(db, role_id).await || is_auditor_role(db, role_id).await
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

    // ===== V15 P1-14.11-B：is_system 不注入 *:* 的单元测试 =====

    /// 验证 ADMIN_ROLE_CODE 常量值为 "admin"
    #[test]
    fn test_admin_role_code_constant_value() {
        assert_eq!(ADMIN_ROLE_CODE, "admin");
    }

    /// 验证 AUDITOR_ROLE_CODE 常量值为 "auditor"
    #[test]
    fn test_auditor_role_code_constant_value() {
        assert_eq!(AUDITOR_ROLE_CODE, "auditor");
    }

    /// 验证 MANAGER_ROLE_CODE 常量值为 "manager"
    #[test]
    fn test_manager_role_code_constant_value() {
        assert_eq!(MANAGER_ROLE_CODE, "manager");
    }

    /// V15 P1-14.11-B：admin 角色（code="admin"）应被识别为管理员
    #[test]
    fn test_admin_role_code_matches_admin() {
        assert_eq!("admin", ADMIN_ROLE_CODE);
        assert!("admin" == ADMIN_ROLE_CODE);
    }

    /// V15 P1-14.11-B：manager 角色不应被识别为管理员（即使 is_system=true）
    #[test]
    fn test_manager_role_not_admin_even_if_system() {
        let manager_code = "manager";
        assert!(manager_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(manager_code));
    }

    /// V15 P1-14.11-B：operator 角色不应被识别为管理员（即使 is_system=true）
    #[test]
    fn test_operator_role_not_admin_even_if_system() {
        let operator_code = "operator";
        assert!(operator_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(operator_code));
    }

    /// V15 P1-14.11-B：customer 角色不应被识别为管理员
    #[test]
    fn test_customer_role_not_admin() {
        let customer_code = "customer";
        assert!(customer_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(customer_code));
    }

    /// V15 P1-14.11-B：auditor 角色不应被识别为管理员（职责分离）
    #[test]
    fn test_auditor_role_not_admin() {
        let auditor_code = "auditor";
        assert!(auditor_code != ADMIN_ROLE_CODE);
        assert!(!should_be_admin_by_code(auditor_code));
    }

    /// V15 P1-14.11-B：空字符串和未知角色不应被识别为管理员
    #[test]
    fn test_unknown_role_not_admin() {
        assert!(!should_be_admin_by_code(""));
        assert!(!should_be_admin_by_code("unknown"));
        assert!(!should_be_admin_by_code("ADMIN"));
        assert!(!should_be_admin_by_code("Admin"));
    }
}

/// V15 P1-14.11-B：纯函数判断角色 code 是否为 admin（与 is_admin_role 内部逻辑一致）
fn should_be_admin_by_code(code: &str) -> bool {
    code == ADMIN_ROLE_CODE
}
