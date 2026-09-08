//! RLS（行级安全）上下文中间件 + PostgreSQL 连接池钩子（dept 语义版）
//!
//! A.21.1 + dept 语义扩展（2026-09-08）：为 PostgreSQL RLS 策略提供 `app.user_id`
//! 与 `app.dept_ids` 双 GUC 会话上下文，使部门经理（data_scope=dept）能按
//! 「本部门数据」语义查看 5 张 RLS 表的行，而非被 self 级策略锁出团队数据。
//!
//! 工作原理（连接池钩子方案，dept 扩展）：
//! - 本中间件把已认证非 admin 用户构造 RlsGuc{user_id, dept_ids} 写入
//!   tokio task-local，包裹整个请求处理链
//! - 生产 PG 连接池钩子（before_acquire / after_connect）在发起查询的请求 task
//!   内执行，直接在即将使用的这条连接上单条 SQL 双 set_config：
//!     set_config('app.user_id', '<id>', false), set_config('app.dept_ids', '<csv>', false)
//! - 因此双 GUC 与随后该连接上的业务查询天然同连接、同会话，PG RLS 策略
//!   （`department_id = ANY(app_dept_ids())`）按 dept 语义激活
//! - admin/data_scope=all 用户不进作用域 → 双 GUC 均 NULL → 策略 fail-open 放行
//! - self 用户：dept_ids=None → 钩子只设 app.user_id → 策略 dept 分支
//!   string_to_array(NULL) 返回空数组 → 退化为 self 匹配 + 公海
//! - 无上下文借出（公开路径/admin/未认证/spawn 旁路）：钩子 RESET 双 GUC 清理残留
//!
//! 安全语义：
//! - 策略 fail-open：双 GUC 未设置时放行所有访问，应用层 apply_data_scope 兜底
//! - 连接被超时中间件强制丢弃时，残留 GUC 由下次借出钩子 RESET 清理，无跨请求泄漏
//! - sqlite（测试）连接构建不经过本函数，task-local 无副作用
//!
//! 详见 .monkeycode/specs/dept-data-scope-semantics/（需求 + 设计）。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::utils::data_scope::DataScope;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use sea_orm::ConnectOptions;
use std::sync::Arc;

/// 请求级 RLS 上下文：写入 task-local，由连接池钩子读取。
///
/// - `user_id`：非 admin 用户必填，self/dept 共用（策略 self 分支 `owner_id = user_id`）
/// - `dept_ids`：仅 dept 用户填充（逗号分隔部门 ID 串）；None = self 语义或无部门
#[derive(Clone)]
pub struct RlsGuc {
    pub user_id: i32,
    pub dept_ids: Option<Arc<String>>,
}

tokio::task_local! {
    pub static RLS_GUC: Option<RlsGuc>;
}

/// 在当前 future 上安装 RLS 上下文并执行（测试与内部工具用）。
pub async fn with_rls_context<F, R>(guc: Option<RlsGuc>, fut: F) -> R
where
    F: std::future::Future<Output = R>,
{
    RLS_GUC.scope(guc, fut).await
}

/// 读取当前 task 的 RLS 上下文（None = 无上下文，钩子走 RESET 清理分支）。
pub fn current_rls_guc() -> Option<RlsGuc> {
    RLS_GUC.try_with(|v| v.clone()).ok().flatten()
}

/// RLS 上下文中间件：把非 admin 用户构造 RlsGuc 绑定到请求 task。
///
/// 在 auth 中间件之后调用（需要 AuthContext 已注入到 request extensions）。
/// admin/data_scope=all 用户不设置上下文（钩子 RESET → GUC NULL → 策略放行）；
/// self 用户：dept_ids=None；dept 用户：dept_ids=逗号分隔串（由 auth 预加载）。
pub async fn rls_context_middleware(
    State(_state): State<AppState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let guc = request.extensions().get::<AuthContext>().and_then(|auth| {
        // admin/data_scope=all 跳过 RLS（全权限，无需行级隔离）
        let is_admin_scope = auth
            .data_scope
            .as_deref()
            .map(|s| DataScope::parse_scope(s) == DataScope::All)
            .unwrap_or(false);

        if is_admin_scope {
            return None;
        }

        // dept_ids 串：dept 用户由 auth 预加载（Arc<String>），self 用户为 None
        let dept_ids_csv = auth.dept_ids.clone().filter(|csv| !csv.is_empty());

        Some(RlsGuc {
            user_id: auth.user_id,
            dept_ids: dept_ids_csv,
        })
    });

    // task-local 作用域覆盖 handler 及其全部查询（同一 tokio task，连接池钩子
    // 在查询借出连接时即可读到该值）
    RLS_GUC.scope(guc, next.run(request)).await
}

/// 为 PostgreSQL 连接池安装 RLS 会话上下文钩子（A.21.1 + dept 扩展）。
///
/// 机制：sqlx before_acquire（idle 连接借出前）与 after_connect（新建连接后）
/// 回调都在发起查询的请求 task 内执行，读取 task-local 的 RlsGuc，直接在即将
/// 使用的这条连接上单条 SQL 双 set_config——因此双 GUC 与随后该连接上的业务
/// 查询天然同连接、同会话，PG RLS 策略按 dept 语义激活。
///
/// - 有上下文：`SELECT set_config('app.user_id', $uid, false),
///   set_config('app.dept_ids', $csv, false)`（dept_ids 为空时只设 user_id）
/// - 无上下文：`RESET app.user_id; RESET app.dept_ids`（清理上一任残留）
///
/// 设置失败仅记录日志并继续借出（返回 Ok(true)）：GUC 为 NULL 策略 fail-open，
/// 应用层 apply_data_scope 兜底，不阻断业务。仅对 PostgreSQL 生效。
pub fn install_rls_pool_hooks(db_opts: &mut ConnectOptions) {
    db_opts.map_sqlx_postgres_before_acquire(|conn, _meta| {
        Box::pin(async move {
            match current_rls_guc() {
                Some(guc) => set_rls_guc(conn, &guc).await,
                None => reset_rls_guc(conn).await,
            }
            Ok(true)
        })
    });

    db_opts.map_sqlx_postgres_pool_opts(|pool_opts| {
        pool_opts.after_connect(|conn, _meta| {
            Box::pin(async move {
                if let Some(guc) = current_rls_guc() {
                    set_rls_guc(conn, &guc).await;
                }
                // 新建连接本无残留，无上下文时无需 RESET
                Ok(())
            })
        })
    });
}

/// 在连接上设置会话级 app.user_id（与 app.dept_ids，若 guc.dept_ids 有值）。
/// 失败仅 warn：GUC 保持/变为 NULL，策略 fail-open，应用层兜底。
async fn set_rls_guc(conn: &mut sqlx::postgres::PgConnection, guc: &RlsGuc) {
    // user_id 为 i32，无注入面；dept_ids 由 auth 层构造的逗号分隔整数串，无注入面
    // （部门 ID 来自 DB 主键）。set_config(text, text, bool) 第三参 false = 会话级。
    let uid_str = guc.user_id.to_string();
    let sql = match &guc.dept_ids {
        Some(csv) if !csv.is_empty() => format!(
            "SELECT set_config('app.user_id', '{uid}', false), \
             set_config('app.dept_ids', '{csv}', false)",
            uid = uid_str,
            csv = csv.replace('\'', "''"),
        ),
        _ => format!("SELECT set_config('app.user_id', '{uid}', false)", uid = uid_str),
    };
    if let Err(e) = sqlx::query(sqlx::AssertSqlSafe(sql)).execute(conn).await {
        tracing::warn!(
            error = %e,
            user_id = guc.user_id,
            "RLS set_config 失败（本连接以无 RLS 上下文借出，策略 fail-open，应用层 apply_data_scope 兜底）"
        );
    }
}

/// 在连接上重置 app.user_id 与 app.dept_ids，清理上一任请求残留。
/// 失败仅 debug：RESET 失败的连接若复用，下次借出钩子会再次尝试清理。
/// 注意：PG prepared statement 不允许多命令（"cannot insert multiple commands
/// into a prepared statement"），两条 RESET 必须独立执行。
async fn reset_rls_guc(conn: &mut sqlx::postgres::PgConnection) {
    if let Err(e) =
        sqlx::query(sqlx::AssertSqlSafe("RESET app.user_id".to_owned()))
            .execute(&mut *conn)
            .await
    {
        tracing::debug!(error = %e, "RESET app.user_id 失败（可忽略，下次借出会重试清理）");
    }
    if let Err(e) =
        sqlx::query(sqlx::AssertSqlSafe("RESET app.dept_ids".to_owned()))
            .execute(&mut *conn)
            .await
    {
        tracing::debug!(error = %e, "RESET app.dept_ids 失败（可忽略，下次借出会重试清理）");
    }
}
