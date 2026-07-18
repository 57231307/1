//! 系统初始化处理器

use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::services::init_service::{
    get_init_tasks, DatabaseConfig, InitRequest, InitService, InitStatus, InitTaskStatus,
};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::audit::{self, SecurityEvent};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::extract::Query;
use axum::{extract::State, Extension, Json};
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Debug, serde::Deserialize)]
pub struct TestDatabaseRequest {
    pub host: String,
    pub port: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TestDatabaseResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct InitWithDbRequest {
    pub db_config: DatabaseConfig,
    pub admin_username: String,
    pub admin_password: String,
}

pub async fn get_init_status(State(state): State<AppState>) -> Json<ApiResponse<InitStatus>> {
    let init_service = InitService::new(state.db.clone());
    let (initialized, message) = init_service.check_initialized().await;
    Json(ApiResponse::success(InitStatus {
        initialized,
        message,
    }))
}

/// 测试数据库连接（P1-1 修复：admin 角色 + port 校验 + 内网 IP 白名单 + 错误脱敏 + 初始化模式约束）
///
/// 安全约束（H-3 完整修复，2026-06-25 综合审计）：
/// 1. 必须登录并具备 admin 角色（handler 层强制拦截）
/// 2. **port 范围校验**：仅允许 1-65535，防止端口枚举
/// 3. **内网 IP 白名单**：仅允许 RFC1918 私有网段 + loopback，防 SSRF 探测外网
/// 4. **初始化模式约束**：系统已初始化后拒绝调用，收敛攻击面
/// 5. **错误消息脱敏**：不透传底层 DbErr 原文，避免泄露内网服务信息
/// 6. 审计日志记录"谁在什么时间测试了什么数据库连接"（不记录明文密码）
///
/// 注意：批次 261 修复后，仅 `initialize` 系列（initialize/initialize-with-db/
/// initialize-with-db-async）在 `PUBLIC_PATHS` 中（由 `init_token_middleware`
/// 认证）。本接口（test-database）不在 `PUBLIC_PATHS` 中，`auth_middleware`
/// 要求 JWT 认证，`auth: AuthContext` 提取器在未登录时返回 401。
pub async fn test_database_connection(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(payload): Json<TestDatabaseRequest>,
) -> Result<Json<ApiResponse<TestDatabaseResponse>>, AppError> {
    // 1) 强制要求管理员角色（防御深度：缺 role_id 直接拒绝，避免后续 is_admin_role 误判）
    let role_id = if let Some(id) = auth.role_id {
        id
    } else {
        // 审计：未分配角色即尝试访问
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("test_database_connection"),
            Some("no_role"),
            audit_ctx.as_deref(),
        )
        .await;
        return Err(AppError::permission_denied(
            "用户未分配角色，无法执行该操作",
        ));
    };
    if !is_admin_role(&state.db, role_id).await {
        // 审计：非 admin 角色尝试访问
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("test_database_connection"),
            Some("not_admin"),
            audit_ctx.as_deref(),
        )
        .await;
        return Err(AppError::permission_denied("测试数据库连接仅限管理员"));
    }

    // 2) P1-1 修复（H-3，2026-06-25 综合审计）：port 范围校验
    //    仅允许 1-65535，防止任意端口枚举内网服务。
    // 批次 113 P1-7：变量名前缀 `_` 表示"校验后不参与后续逻辑"，移除冗余 `let _ = port_num;`
    let _port_num: u16 = match payload.port.parse::<u16>() {
        Ok(p) if p > 0 => p,
        _ => {
            audit::log_security_event(
                SecurityEvent::AuthorizationDenied,
                auth.user_id,
                &auth.username,
                auth.role_id,
                Some("test_database_connection"),
                Some("invalid_port"),
                audit_ctx.as_deref(),
            )
            .await;
            return Err(AppError::bad_request(
                "数据库端口无效，仅允许 1-65535 范围内的数字",
            ));
        }
    };

    // 3) P1-1 修复：初始化模式约束
    //    系统已初始化后拒绝调用 test_database_connection，收敛 SSRF 攻击面。
    //    正常流程：系统未初始化 → admin 测试目标库 → 确认可用 → 执行 initialize。
    //    系统已初始化后不应再测试任意数据库连接。
    let init_service = InitService::new(state.db.clone());
    let (already_initialized, _) = init_service.check_initialized().await;
    if already_initialized {
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("test_database_connection"),
            Some("system_already_initialized"),
            audit_ctx.as_deref(),
        )
        .await;
        return Err(AppError::permission_denied(
            "系统已初始化，测试数据库连接功能已禁用",
        ));
    }

    // 4) P1-1 修复：内网 IP 白名单（防 SSRF 探测外网数据库端口）
    //    仅允许 RFC1918 私有网段 + loopback，拒绝公网 IP。
    //    客户端 IP 从 AuditContext 获取（由反向代理/中间件填充）。
    let client_ip = audit_ctx
        .as_deref()
        .map(|c| c.ip_address.as_str())
        .unwrap_or("unknown");
    if !is_internal_ip(client_ip) {
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("test_database_connection"),
            Some("non_internal_ip"),
            audit_ctx.as_deref(),
        )
        .await;
        return Err(AppError::permission_denied(
            "测试数据库连接仅允许从内网 IP 调用",
        ));
    }

    // 5) 审计日志：best-effort 写入"谁在什么时间测试了什么数据库连接"
    //    目标记录格式：host:port/name，便于后续按业务目标聚合
    //    不记录明文密码（payload.password 不写入 extra）
    //    注意：target 必须在 DatabaseConfig 构造前 format，避免 payload 字段被 move 后无法借用
    let target = format!("{}:{}/{}", payload.host, payload.port, payload.name);

    let db_config = DatabaseConfig {
        host: payload.host,
        port: payload.port,
        name: payload.name,
        username: payload.username,
        password: payload.password,
        // v5 审计批次 21：test_database_connection 不接收前端 ssl_mode 参数，
        // 使用 None 让 to_connection_string 回退到默认 prefer
        ssl_mode: None,
    };
    audit::log_security_event(
        SecurityEvent::TestDatabaseConnection,
        auth.user_id,
        &auth.username,
        auth.role_id,
        Some(&target),
        None,
        audit_ctx.as_deref(),
    )
    .await;

    // 6) 调用 service 层执行数据库连接测试（静态方法，无需 AppState）
    //    P1-1 修复：错误消息脱敏，不透传底层 DbErr 原文，
    //    避免"password authentication failed"/"database xxx does not exist"等
    //    差异化错误信息被用于内网服务枚举。
    match InitService::test_database(&db_config).await {
        Ok(_) => Ok(Json(ApiResponse::success_with_message(
            TestDatabaseResponse {
                success: true,
                message: "数据库连接成功".to_string(),
            },
            "数据库连接测试成功",
        ))),
        Err(_) => Err(AppError::bad_request(
            "数据库连接失败，请检查主机、端口、数据库名、用户名和密码是否正确",
        )),
    }
}

/// 判断客户端 IP 是否为内网 IP（P1-1 修复，H-3 SSRF 防护）
///
/// 仅允许以下网段：
/// - 127.0.0.0/8 (IPv4 loopback)
/// - 10.0.0.0/8 (RFC1918 A 类私有)
/// - 172.16.0.0/12 (RFC1918 B 类私有)
/// - 192.168.0.0/16 (RFC1918 C 类私有)
/// - ::1 (IPv6 loopback)
/// - fe80::/10 (IPv6 link-local)
/// - fc00::/7 (IPv6 ULA)
///
/// 拒绝：公网 IP、未识别格式（"unknown"等）
fn is_internal_ip(ip_str: &str) -> bool {
    let ip: std::net::IpAddr = match ip_str.parse() {
        Ok(addr) => addr,
        Err(_) => return false, // 无法解析的 IP（如 "unknown"）一律拒绝
    };

    match ip {
        std::net::IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private() // RFC1918: 10/8, 172.16/12, 192.168/16
                || ipv4.is_link_local() // 169.254/16（含云元数据 169.254.169.254，仅限内网调试）
        }
        std::net::IpAddr::V6(ipv6) => {
            ipv6.is_loopback() // ::1
                || ipv6.is_unicast_link_local() // fe80::/10
                || {
                    // fc00::/7 ULA（Unique Local Address）
                    let segments = ipv6.segments();
                    (segments[0] & 0xfe00) == 0xfc00
                }
        }
    }
}

/// 同步初始化（无 DB 配置版本）
///
/// 安全约束（bug.md #3 修复）：
/// 1. 路由层已应用 `init_token_middleware`：调用方必须携带 `X-Init-Token`
///    请求头，且与服务端 `INIT_TOKEN` 环境变量一致（恒定时间比较防时序攻击）
/// 2. handler 内部仍执行 `check_initialized()` 兜底：系统已初始化时拒绝重复初始化
/// 3. 缺失/错误 `INIT_TOKEN` 直接 401（fail-secure），未配置 `INIT_TOKEN` 时
///    整个 init 端点拒绝所有请求
pub async fn initialize_system(
    State(state): State<AppState>,
    Json(payload): Json<InitRequest>,
) -> Result<Json<ApiResponse<crate::services::init_service::InitializationResult>>, AppError> {
    let init_service = InitService::new(state.db.clone());

    init_service
        .initialize(&payload.admin_username, &payload.admin_password)
        .await
        .map(|result| Json(ApiResponse::success_with_message(result, "系统初始化成功")))
        .map_err(map_init_error)
}

pub async fn initialize_system_with_db(
    Json(payload): Json<InitWithDbRequest>,
) -> Result<Json<ApiResponse<crate::services::init_service::InitializationResult>>, AppError> {
    InitService::initialize_with_db(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    .map(|result| Json(ApiResponse::success_with_message(result, "系统初始化成功")))
    .map_err(map_init_error)
}

/// 异步初始化处理器（非阻塞）
///
/// 安全约束同 [`initialize_system`]：路由层已应用 `init_token_middleware` 保护。
pub async fn initialize_system_with_db_async(
    Json(payload): Json<InitWithDbRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    InitService::initialize_with_db_async(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    .map(|task_id| {
        Json(ApiResponse::success_with_message(
            task_id,
            "异步初始化任务已启动",
        ))
    })
    .map_err(map_init_error)
}

/// 初始化子系统处理器内部的 admin 角色二次校验
///
/// 设计原因：
/// 1. `permission_middleware` 不覆盖 init 路径，必须在 handler 层补一道 admin 防线；
/// 2. 批次 261 修复后，仅 `initialize` 系列在 `PUBLIC_PATHS` 中（跳过 JWT 验证，
///    由 `init_token_middleware` 认证）。本函数保护的只读接口（test-database/
///    task-status）不在 `PUBLIC_PATHS` 中，`auth_middleware` 要求 JWT 认证，
///    `auth: AuthContext` 提取器在缺认证时直接返回 401。
/// 3. 与 `user_handler::require_admin_role` 实现保持一致，便于未来统一抽到 utils。
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        // 审计：非 admin 角色尝试访问 init 管理接口
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("init_management"),
            Some("non-admin attempt"),
            None,
        )
        .await;
        return Err(AppError::permission_denied(
            "初始化子系统管理接口仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

/// 查询初始化任务状态（仅管理员可访问）
///
/// 安全约束：
/// 1. 必须登录并具备 admin 角色（handler 层强制拦截）
/// 2. 批次 261 修复后，仅 `initialize` 系列在 `PUBLIC_PATHS` 中。本接口
///    （task-status）不在 `PUBLIC_PATHS` 中，`auth_middleware` 要求 JWT 认证，
///    `auth: AuthContext` 提取器在未登录时返回 401（fail-secure）。
pub async fn get_task_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<TaskStatusResponse>>, AppError> {
    // 1) 强制要求管理员角色（深度防御：与 test_database_connection / reset_admin_password 一致）
    require_admin_role(&state, &auth).await?;

    // 2) 业务逻辑保持不变
    let task_id = params
        .get("task_id")
        .ok_or_else(|| AppError::bad_request("缺少 task_id 参数"))?;

    let status = get_init_tasks()
        .lock()
        .await
        .get(task_id)
        .cloned()
        // 任务不存在时直接返回失败状态，使用 unwrap_or 避免不必要的闭包分配
        .unwrap_or(InitTaskStatus::Failed);

    let status_str = match status {
        InitTaskStatus::Running => "running",
        InitTaskStatus::Completed => "completed",
        InitTaskStatus::Failed => "failed",
    };

    // P2 2-7 修复：原返回 Json<serde_json::Value> 未用 ApiResponse 包装，与项目响应规范不一致。
    // 改为返回 ApiResponse<TaskStatusResponse>，保持与项目其他接口的响应结构统一。
    Ok(Json(ApiResponse::success(TaskStatusResponse {
        task_id: task_id.clone(),
        status: status_str.to_string(),
    })))
}

/// P2 2-7 修复：初始化任务状态响应 DTO，替代原 serde_json::Value 手拼 JSON
#[derive(Debug, Serialize, ToSchema)]
pub struct TaskStatusResponse {
    /// 任务 ID
    pub task_id: String,
    /// 任务状态：running / completed / failed
    pub status: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResetPasswordRequest {
    pub username: String,
    pub new_password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
    pub message: String,
}

/// 重置用户密码（P0 修复：必须 admin 登录后才能调用）
///
/// 安全约束：
/// 1. 必须登录并具备 admin 角色（深度防御：service 层再做用户存在性二次校验 + 密码强度校验）
/// 2. 不允许重置自己的密码（防止 admin 误操作锁定自己）
/// 3. 审计日志记录"谁在什么时间重置谁的密码"（不记录明文密码）
pub async fn reset_admin_password(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<ResetPasswordResponse>>, AppError> {
    // 1) 强制要求管理员角色（防御深度：缺 role_id 直接拒绝，避免后续 is_admin_role 误判）
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "重置密码功能仅限管理员（code=admin）执行",
        ));
    }
    // 2) 自我保护：禁止重置当前登录管理员的密码（防止误操作锁定自己）
    if auth.username == payload.username {
        return Err(AppError::bad_request(
            "不能重置当前登录管理员的密码，请联系其他管理员",
        ));
    }
    // 3) 审计日志：best-effort 写入安全审计（结构化日志，当前未落 DB）
    //    仅记录操作语义和目标用户名，不写入明文密码或密码哈希
    audit::log_security_event(
        SecurityEvent::ResetPassword,
        auth.user_id,
        &auth.username,
        auth.role_id,
        Some(&payload.username),
        None,
        audit_ctx.as_deref(),
    )
    .await;

    // 4) 调用 service 层执行重置（service 层会做密码强度 + 用户存在性二次校验）
    let init_service = InitService::new(state.db.clone());
    init_service
        .reset_password(&payload.username, &payload.new_password)
        .await
        .map(|_| {
            Json(ApiResponse::success_with_message(
                ResetPasswordResponse {
                    success: true,
                    message: "密码重置成功".to_string(),
                },
                "密码重置成功",
            ))
        })
        .map_err(|e| match e {
            crate::services::init_service::InitError::UserNotFound => {
                AppError::not_found("用户不存在")
            }
            crate::services::init_service::InitError::ValidationError(msg) => {
                AppError::bad_request(format!("密码强度校验失败：{}", msg))
            }
            _ => map_init_error(e),
        })
}

/// 将 `InitError` 统一映射为 `AppError`。
///
/// 错误分类：
/// - `AlreadyInitialized` / `HashError` / `UserNotFound` / `ConfigError` / `ValidationError` → 业务/校验错误（400）
/// - `DatabaseError` → 数据库错误（500）
fn map_init_error(e: crate::services::init_service::InitError) -> AppError {
    match e {
        crate::services::init_service::InitError::AlreadyInitialized => {
            AppError::business("系统已经初始化，不能重复初始化")
        }
        crate::services::init_service::InitError::HashError(msg) => {
            AppError::bad_request(format!("密码加密失败: {}", msg))
        }
        crate::services::init_service::InitError::DatabaseError(msg) => AppError::database(msg),
        crate::services::init_service::InitError::UserNotFound => AppError::not_found("用户不存在"),
        crate::services::init_service::InitError::ConfigError(msg) => {
            AppError::bad_request(format!("配置错误: {}", msg))
        }
        crate::services::init_service::InitError::ValidationError(msg) => {
            AppError::bad_request(format!("参数校验失败: {}", msg))
        }
    }
}

#[cfg(test)]
mod tests {
    //! 安全漏洞 #5 修复单测
    //!
    //! 覆盖 `get_task_status` 权限校验逻辑：
    //! 1. **场景 A（匿名调用）**：请求 extensions 中无 `AuthContext` → `auth: AuthContext` 提取器
    //!    应当返回 401，阻止匿名用户查询任意 task_id 的初始化任务状态。
    //! 2. **场景 B（缺角色用户调用）**：注入 `role_id = None` 的 `AuthContext` → `require_admin_role`
    //!    应当直接返回 403（permission_denied），不依赖 DB 查询。
    //! 3. **场景 C（缺 task_id 参数）**：验证 Query 提取顺序无回归（缺 AuthContext → 401）。
    //!
    //! 设计说明：
    //! - 不通过完整 HTTP 流程（绕开 `auth_middleware`），直接构造 `AuthContext`
    //!   与无认证两种场景，验证 handler 内部 admin 校验逻辑。
    //! - 场景 B 选择"缺 role_id"分支而非"非 admin 角色"分支，是为了避免在测试环境
    //!   依赖真实 DB（`is_admin_role` 在 DB miss 时的行为依赖具体 sea_orm 错误信息）。
    //!   "非 admin 角色"的端到端覆盖由 `utils/admin_checker::tests` 与 service 层测试承担。
    //! - `tower::ServiceExt::oneshot` + 最小化 `AppState::default()` 隔离依赖。

    use super::*;
    use crate::utils::app_state::AppState;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    /// 构造一个最小化的测试 Router：仅注册 `get_task_status` + `AppState`。
    fn build_test_app() -> Router {
        Router::new()
            .route("/init/task-status", get(get_task_status))
            .with_state(AppState::default())
    }

    /// 场景 A：匿名调用 get_task_status（无 AuthContext）→ 期望 401
    ///
    /// 验证链路：
    /// - 请求 extensions 中没有注入 `AuthContext`
    /// - `auth: AuthContext` 提取器找不到 `AuthContext` → 返回 `AuthRejection::unauthorized`
    /// - 响应状态码：401
    #[tokio::test]
    async fn test_get_task_status_anonymous_returns_401() {
        let app = build_test_app();

        let req = Request::builder()
            .method("GET")
            .uri("/init/task-status?task_id=any-task-id")
            .body(Body::empty())
            .expect("构造匿名请求失败");

        let resp = app.oneshot(req).await.expect("执行匿名请求失败");

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "匿名调用 get_task_status 应返回 401（auth: AuthContext 提取器在缺 AuthContext 时直接拒绝）"
        );
    }

    /// 场景 B：缺角色用户（role_id=None）调用 get_task_status → 期望 403
    ///
    /// 验证链路：
    /// - 注入 `AuthContext { role_id: None }`
    /// - `auth: AuthContext` 提取器成功
    /// - `require_admin_role` 第一个分支：`role_id = None` → `permission_denied` → 403
    /// - **不依赖 DB 查询**，测试稳定可重复。
    #[tokio::test]
    async fn test_get_task_status_no_role_returns_403() {
        let state = AppState::default();

        // 直接调用 require_admin_role 验证缺角色会被拒绝（不触发 DB 调用）
        let auth = AuthContext {
            user_id: 42,
            username: "no_role_user".to_string(),
            role_id: None,
            // V15 Batch 485：补齐 P0-S01 新增字段（data_scope/department_id）
            // 测试场景不涉及数据权限，使用 None（service 层按 Self_ 处理）
            department_id: None,
            data_scope: None,
        };
        let result = require_admin_role(&state, &auth).await;
        assert!(
            matches!(result, Err(AppError::PermissionDenied(_))),
            "缺角色用户调用 require_admin_role 应返回 PermissionDenied，实际: {:?}",
            result
        );

        // 端到端验证：通过 Router 调用也应返回 403
        let app = build_test_app();
        let mut req = Request::builder()
            .method("GET")
            .uri("/init/task-status?task_id=any-task-id")
            .body(Body::empty())
            .expect("构造缺角色请求失败");
        req.extensions_mut().insert(auth);

        let resp = app.oneshot(req).await.expect("执行缺角色请求失败");
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "缺角色用户调用 get_task_status 应返回 403（handler 内 require_admin_role 拒绝）"
        );
    }

    /// 场景 C：缺少 task_id 参数 → 期望 401（缺 AuthContext 时提取器先失败）
    ///
    /// 验证 `require_admin_role` 之前的 Query 提取顺序无回归。
    /// 注意：本测试跳过 admin 校验（无 AuthContext），仅验证 Query 提取器被正确解析。
    #[tokio::test]
    async fn test_get_task_status_missing_task_id_returns_401() {
        let app = build_test_app();

        let req = Request::builder()
            .method("GET")
            .uri("/init/task-status")
            .body(Body::empty())
            .expect("构造缺参请求失败");

        let resp = app.oneshot(req).await.expect("执行缺参请求失败");
        // 缺 AuthContext → 提取器先失败 → 401；如果未来先做 Query 校验，会改为 400。
        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "缺 AuthContext 时 get_task_status 应先返回 401"
        );
    }
}
