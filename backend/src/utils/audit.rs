//! 安全审计模块
//!
//! 提供安全事件记录的统一接口。当前实现仅使用 `tracing` 输出结构化日志，
//! 未直接写入 `log_security_event` 数据库表（该表在当前 migration 中不存在，
//! 避免引入额外的 DB 迁移；后续可按需扩展落库逻辑）。
//!
//! 调用方在关键权限校验与状态变更后调用 `log_security_event`，
//! 确保高风险操作可被事后审计。
//!
//! 安全事件当前覆盖：
//! - `ResetPassword` —— 任意用户重置密码
//! - `AuthorizationDenied` —— 鉴权失败（角色不足 / 资源越权）
//! - `UserDeleted` —— 用户被删除（软删除 + 吊销其所有活跃 JWT）
//! - `TestDatabaseConnection` —— 测试数据库连接（数据库敏感探测行为）

use crate::middleware::audit_context::AuditContext;

/// 安全事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEvent {
    /// 任意用户重置密码
    ResetPassword,
    /// 鉴权失败（角色不足 / 资源越权）
    AuthorizationDenied,
    /// 用户被删除（软删除 + 吊销其所有活跃 JWT）
    UserDeleted,
    /// 测试数据库连接（数据库敏感探测行为）
    TestDatabaseConnection,
}

impl std::fmt::Display for SecurityEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityEvent::ResetPassword => write!(f, "RESET_PASSWORD"),
            SecurityEvent::AuthorizationDenied => write!(f, "AUTHORIZATION_DENIED"),
            SecurityEvent::UserDeleted => write!(f, "USER_DELETED"),
            SecurityEvent::TestDatabaseConnection => write!(f, "TEST_DATABASE_CONNECTION"),
        }
    }
}

/// 记录安全审计事件
///
/// 失败不应阻塞主业务流（采用 best-effort 写入），调用方一般以 `.await.ok()` 形式忽略错误。
/// 一旦后续引入 `log_security_event` 表，可在此函数中追加 DB 写入。
pub async fn log_security_event(
    event: SecurityEvent,
    actor_user_id: i32,
    actor_username: &str,
    actor_role_id: Option<i32>,
    target: Option<&str>,
    extra: Option<&str>,
    audit_ctx: Option<&AuditContext>,
) {
    let ip_address = audit_ctx.map(|c| c.ip_address.as_str()).unwrap_or("unknown");
    let user_agent = audit_ctx.map(|c| c.user_agent.as_str()).unwrap_or("unknown");

    // 当前实现仅输出结构化日志；后续可在此追加 DB 写入
    tracing::warn!(
        target: "security_audit",
        event = %event,
        actor_user_id = actor_user_id,
        actor_username = %actor_username,
        actor_role_id = ?actor_role_id,
        target = target.unwrap_or("-"),
        extra = extra.unwrap_or("-"),
        ip_address = %ip_address,
        user_agent = %user_agent,
        "[SECURITY] {} actor_user_id={} actor_username={} target={:?} extra={:?}",
        event,
        actor_user_id,
        actor_username,
        target,
        extra,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_display() {
        assert_eq!(SecurityEvent::ResetPassword.to_string(), "RESET_PASSWORD");
        assert_eq!(
            SecurityEvent::AuthorizationDenied.to_string(),
            "AUTHORIZATION_DENIED"
        );
        assert_eq!(SecurityEvent::UserDeleted.to_string(), "USER_DELETED");
        assert_eq!(
            SecurityEvent::TestDatabaseConnection.to_string(),
            "TEST_DATABASE_CONNECTION"
        );
    }
}
