//! RLS（行级安全）上下文中间件
//!
//! A.21.1：实现 SET LOCAL app.user_id 机制，激活 PostgreSQL RLS 策略。
//!
//! 工作原理：
//! - 每个已认证请求，在进入 handler 前执行 `SET LOCAL app.user_id = <id>`
//! - 请求结束后（连接归还连接池）SET LOCAL 自动失效（LOCAL 仅当前事务/会话有效）
//! - 当 app.user_id 设置后，rls.sql 中的策略自动激活行级数据隔离
//! - admin/gm 等全权限角色跳过 RLS（在应用层 data_scope=all 时不设置 app.user_id）
//!
//! 安全降级（已由 rls.sql 保证）：
//! - app.user_id 未设置时 current_setting 返回 NULL，策略放行所有访问
//! - CI 环境 SUPERUSER 绕过 RLS，仅生产环境生效

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::utils::data_scope::DataScope;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

/// RLS 上下文中间件：为已认证请求设置 PostgreSQL 会话变量 app.user_id
///
/// 在 auth 中间件之后调用（需要 AuthContext 已注入到 request extensions）。
/// 仅对非 admin（data_scope != all）的用户设置 RLS 上下文。
pub async fn rls_context_middleware(
    State(state): State<AppState>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // 从 extensions 获取已认证的 AuthContext（auth 中间件注入）
    if let Some(auth) = request.extensions().get::<AuthContext>() {
        // admin/data_scope=all 的用户跳过 RLS（全权限，无需行级隔离）
        let is_admin_scope = auth
            .data_scope
            .as_deref()
            .map(|s| DataScope::parse_scope(&s.to_string()) == DataScope::All)
            .unwrap_or(false);

        if !is_admin_scope {
            // A.21.1：设置 PostgreSQL 会话变量，激活 RLS 策略
            // SET LOCAL 仅在当前连接/事务有效，连接归还连接池后自动失效
            let sql = format!("SET LOCAL app.user_id = '{}'", auth.user_id);
            // 用 sea_orm 的 execute_unprepared 执行裸 SQL，避免直接依赖 sqlx（E0433 修复）
            if let Err(e) = state.db.execute_unprepared(&sql).await {
                tracing::warn!(
                    error = %e,
                    user_id = auth.user_id,
                    "SET LOCAL app.user_id 失败（RLS 未激活，回退应用层隔离）"
                );
                // 安全降级：RLS 设置失败时，应用层 apply_data_scope 仍生效
            }
        }
    }

    let response = next.run(request).await;

    // 请求结束后重置 app.user_id（防止连接复用时 RLS 残留）
    // SET LOCAL 在事务结束后自动重置，但显式 RESET 更安全
    if let Err(e) = state.db.execute_unprepared("RESET app.user_id").await {
        tracing::debug!(error = %e, "RESET app.user_id（可忽略，SET LOCAL 已自动失效）");
    }

    response
}
