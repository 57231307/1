//! RLS（行级安全）上下文中间件 + PostgreSQL 连接池钩子
//!
//! A.21.1：为 PostgreSQL RLS 策略提供 `app.user_id` 会话上下文。
//!
//! 工作原理（连接池钩子方案，2026-09-08 重新设计）：
//! - 本中间件将非 admin 用户的 user_id 写入 tokio task-local（`RLS_USER_ID`），
//!   包裹整个请求处理链（handler 及其全部 DB 查询都在同一 tokio task 内执行）
//! - 生产 PG 连接池（connect_database → install_rls_pool_hooks）安装了 sqlx
//!   `before_acquire`（空闲连接借出前）与 `after_connect`（新建连接后）钩子：
//!   两类回调都在**发起查询的请求 task** 内执行，读取 task-local 后直接在
//!   **即将被使用的这条连接**上执行 `SELECT set_config('app.user_id', '<id>', false)`
//!   ——因此 GUC 与该连接上的业务查询天然同连接、同会话，finance 域迁移中的
//!   RLS 策略（`owner_id = current_setting('app.user_id', true)::int`）真正激活
//! - admin/data_scope=all 用户与未认证请求无 task-local 上下文，借出钩子走
//!   RESET 分支清理上一任残留，`current_setting` 返回 NULL，策略放行全量数据
//!
//! 安全语义：
//! - 策略为 fail-open：`app.user_id` 未设置时放行所有访问，应用层
//!   `apply_data_scope`（utils/data_scope.rs）始终作为第一道防线兜底
//! - 连接被超时中间件强制丢弃时，残留 GUC 由下一次借出钩子的 RESET 分支清理，
//!   不存在跨请求/跨用户泄漏
//! - 测试环境的 sqlite 池不安装钩子（钩子仅在 PG 连接构建路径注册），task-local 无副作用
//!
//! 旧实现（已废弃）的问题：在中间件内用 `execute_unprepared("SET LOCAL ...")`
//! 每请求执行两次——SET LOCAL 在事务块外无效（PG 文档：事务外执行被丢弃），
//! 且独立连接与 handler 业务查询（全局池随机借出）不保证同一连接，
//! RLS 实际从未激活，只产生每请求两次无效池往返。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::utils::data_scope::DataScope;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use sea_orm::ConnectOptions;

/// 请求级 RLS 上下文：当前请求应设置到连接上的 user_id。
///
/// 由 rls_context_middleware 写入、连接池钩子（before_acquire / after_connect）
/// 读取。`None` 表示当前 task 无 RLS 上下文（公开路径 / admin / 未认证），
/// 借出钩子对连接执行 RESET（清理上一任残留）。
tokio::task_local! {
    pub static RLS_USER_ID: Option<i32>;
}

/// 在当前 future 上安装 RLS 上下文并执行。
///
/// 供测试与连接池钩子验证使用；生产路径由 rls_context_middleware 调用。
pub async fn with_rls_context<F, R>(user_id: Option<i32>, fut: F) -> R
where
    F: std::future::Future<Output = R>,
{
    RLS_USER_ID.scope(user_id, fut).await
}

/// 读取当前 task 的 RLS user_id（None = 无上下文）。
///
/// 未进入 rls_context_middleware 作用域的 task（如 spawn 出的旁路任务）
/// 返回 None，连接池钩子对这类查询借出的连接执行 RESET（无 RLS 上下文，
/// 策略 fail-open 放行，应用层 apply_data_scope 兜底）。
pub fn current_rls_user_id() -> Option<i32> {
    RLS_USER_ID.try_with(|v| *v).unwrap_or(None)
}

/// RLS 上下文中间件：把非 admin 用户的 user_id 绑定到请求 task。
///
/// 在 auth 中间件之后调用（需要 AuthContext 已注入到 request extensions）。
/// 仅对非 admin（data_scope != all）的用户设置上下文；实际连接级 GUC 由
/// 连接池借出钩子在业务查询的同一连接上执行。
pub async fn rls_context_middleware(
    State(_state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // 从 extensions 获取已认证的 AuthContext（auth 中间件注入）
    let user_id = request.extensions().get::<AuthContext>().and_then(|auth| {
        // admin/data_scope=all 的用户跳过 RLS（全权限，无需行级隔离）
        let is_admin_scope = auth
            .data_scope
            .as_deref()
            .map(|s| DataScope::parse_scope(s) == DataScope::All)
            .unwrap_or(false);

        if is_admin_scope {
            None
        } else {
            Some(auth.user_id)
        }
    });

    // task-local 作用域覆盖 handler 及其全部查询（同一 tokio task，
    // 连接池钩子在查询借出连接时即可读到该值）
    RLS_USER_ID.scope(user_id, next.run(request)).await
}

/// 为 PostgreSQL 连接池安装 RLS 会话上下文钩子（A.21.1，2026-09-08 重新设计）。
///
/// 机制：sqlx 的 `before_acquire`（idle 连接借出前）与 `after_connect`（新建连接后）
/// 回调都在**发起查询的请求 task** 内执行，可读取 task-local 中的 user_id，
/// 并直接在**即将被使用的这条连接**上执行 GUC 设置——因此 `app.user_id` 与随后
/// 该连接上的业务查询天然同连接、同会话，PG RLS 策略真正激活。
///
/// - 有上下文（非 admin 已认证请求）：`SELECT set_config('app.user_id', '<id>', false)`
///   （会话级；autocommit 下生效且跨语句可见）
/// - 无上下文（公开路径 / admin / 未认证 / spawn 出的旁路任务）：`RESET app.user_id`
///   清理该连接上一任请求的残留，保证无跨请求/跨用户泄漏
///
/// 设置失败仅记录日志并继续借出（返回 Ok(true)）：此时 `current_setting` 为 NULL，
/// RLS 策略 fail-open 放行，应用层 apply_data_scope 兜底，不阻断业务。
///
/// 仅对 PostgreSQL 生效；sqlite（测试）连接构建不经过本函数。
pub fn install_rls_pool_hooks(db_opts: &mut ConnectOptions) {
    // 空闲连接借出前：按当前 task 上下文设置或清理 GUC
    db_opts.map_sqlx_postgres_before_acquire(|conn, _meta| {
        Box::pin(async move {
            match current_rls_user_id() {
                Some(user_id) => set_rls_guc(conn, user_id).await,
                None => reset_rls_guc(conn).await,
            }
            Ok(true)
        })
    });

    // 新建连接（首借出，不经过 before_acquire）：有上下文时设置；
    // 新建连接本无残留，无上下文时无需 RESET
    db_opts.map_sqlx_postgres_pool_opts(|pool_opts| {
        pool_opts.after_connect(|conn, _meta| {
            Box::pin(async move {
                if let Some(user_id) = current_rls_user_id() {
                    set_rls_guc(conn, user_id).await;
                }
                Ok(())
            })
        })
    });
}

/// 在连接上设置会话级 app.user_id。
/// 失败仅 warn（借出继续）：current_setting 保持/变为 NULL，策略 fail-open，应用层兜底。
async fn set_rls_guc(conn: &mut sqlx::postgres::PgConnection, user_id: i32) {
    // user_id 为 i32，无注入面；set_config(text, text, bool) 第三参 false = 会话级
    let sql = format!("SELECT set_config('app.user_id', '{user_id}', false)");
    if let Err(e) = sqlx::query(&sql).execute(conn).await {
        tracing::warn!(
            error = %e,
            user_id,
            "RLS set_config 失败（本连接以无 RLS 上下文借出，策略 fail-open，应用层 apply_data_scope 兜底）"
        );
    }
}

/// 在连接上重置 app.user_id，清理上一任请求残留。
/// 失败仅 debug：RESET 失败的连接若复用，下次借出钩子会再次尝试清理。
async fn reset_rls_guc(conn: &mut sqlx::postgres::PgConnection) {
    if let Err(e) = sqlx::query("RESET app.user_id").execute(conn).await {
        tracing::debug!(error = %e, "RESET app.user_id 失败（可忽略，下次借出会重试清理）");
    }
}
