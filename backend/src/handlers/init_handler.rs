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
use std::collections::HashMap;

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

pub async fn test_database_connection(
    Json(payload): Json<TestDatabaseRequest>,
) -> Result<Json<ApiResponse<TestDatabaseResponse>>, AppError> {
    let db_config = DatabaseConfig {
        host: payload.host,
        port: payload.port,
        name: payload.name,
        username: payload.username,
        password: payload.password,
    };

    match InitService::test_database(&db_config).await {
        Ok(_) => Ok(Json(ApiResponse::success_with_message(
            TestDatabaseResponse {
                success: true,
                message: "数据库连接成功".to_string(),
            },
            "数据库连接测试成功",
        ))),
        Err(e) => Err(AppError::bad_request(format!("数据库连接失败: {}", e))),
    }
}

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

/// 查询初始化任务状态
pub async fn get_task_status(
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
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

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": status_str,
    })))
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
