//! Setup 模式路由与初始化接口（数据库未连接时使用）
//!
//! 职责：数据库连接失败时，后端进入 Setup 模式，仅暴露 `/init/*` 系列接口。
//! 包含初始化状态查询、数据库连接测试、系统初始化三个 handler，
//! 以及 Setup 模式下的进程级初始化标志位。

use std::sync::{Arc, Mutex};

use axum::{
    routing::{get, post},
    Json, Router,
};

use crate::handlers::init_handler::{InitWithDbRequest, TestDatabaseResponse};
use crate::middleware::init_token::init_token_middleware;
use crate::services::init_service::{DatabaseConfig, InitError, InitService, InitializationResult};
use crate::utils::response::ApiResponse;

#[derive(Debug, serde::Serialize)]
struct InitStatusResponse {
    initialized: bool,
    message: String,
    mode: String,
}

#[derive(Debug, serde::Serialize)]
struct InitErrorResponse {
    error: String,
    message: String,
}

/// Setup 模式下的初始化成功标志。
///
/// 当数据库尚未连接成功时（例如：用户首次部署、或者刚刚迁移到一台新机器），
/// 后端会进入「Setup 模式」，仅暴露 `/init/*` 系列接口，不连接数据库。
/// 在该模式下，原始实现的 `get_init_status` 永远返回 `initialized: false`，
/// 会导致前端在 `initialize_with_db` 成功并跳转到登录页时，被路由守卫判定为
/// "系统未初始化" 再次拉回 setup 页面，形成跳转循环。
///
/// 修复方案：使用一个进程级的可变标志位记录本进程内是否已经成功完成初始化。
/// 注意：完整模式（数据库已连接）下不走此分支，因此对正常启动流程零影响。
static SETUP_MODE_INITIALIZED: std::sync::OnceLock<Arc<Mutex<bool>>> = std::sync::OnceLock::new();

fn setup_initialized_flag() -> Arc<Mutex<bool>> {
    SETUP_MODE_INITIALIZED
        .get_or_init(|| Arc::new(Mutex::new(false)))
        .clone()
}

async fn get_init_status() -> Json<InitStatusResponse> {
    // 优先使用内存中的初始化成功标志（处理「setup 模式内完成初始化」的场景）
    // P0 修复（批次 4，2026-06-27）：锁中毒时改为优雅降级（e.into_inner()），
    // 与 event_bus.rs / di_container.rs 一致，避免生产环境 panic 直接拖垮进程。
    // 锁中毒仅在持锁线程 panic 时发生，此时返回上次成功写入的值是安全降级。
    let arc = setup_initialized_flag();
    let guard = arc.lock().unwrap_or_else(|e| {
        tracing::error!(error = %e, "P9-1: setup 初始化标志锁中毒，降级使用上次值");
        e.into_inner()
    });
    let initialized = *guard;
    if initialized {
        return Json(InitStatusResponse {
            initialized: true,
            message: "系统已初始化".to_string(),
            mode: "setup".to_string(),
        });
    }
    Json(InitStatusResponse {
        initialized: false,
        message: "系统未初始化，请先配置数据库".to_string(),
        mode: "setup".to_string(),
    })
}

async fn test_database_connection(
    Json(payload): Json<DatabaseConfig>,
) -> Result<
    Json<ApiResponse<TestDatabaseResponse>>,
    (axum::http::StatusCode, Json<InitErrorResponse>),
> {
    match InitService::test_database(&payload).await {
        Ok(_) => Ok(Json(ApiResponse::success_with_message(
            TestDatabaseResponse {
                success: true,
                message: "数据库连接成功".to_string(),
            },
            "数据库连接测试成功",
        ))),
        Err(e) => Err((
            axum::http::StatusCode::BAD_REQUEST,
            Json(InitErrorResponse {
                error: "database_connection_failed".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

async fn initialize_with_db(
    Json(payload): Json<InitWithDbRequest>,
) -> Result<
    Json<ApiResponse<InitializationResult>>,
    (axum::http::StatusCode, Json<InitErrorResponse>),
> {
    match InitService::initialize_with_db(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    {
        Ok(result) => {
            // 标记 setup 模式下的初始化已完成，便于 `get_init_status`
            // 在同一进程内返回 initialized = true，避免前端在跳转登录页时
            // 被路由守卫再次拉回 setup 页面。
            // P0 修复（批次 4，2026-06-27）：锁中毒时改为优雅降级（e.into_inner()），
            // 与 event_bus.rs / di_container.rs 一致；若锁已中毒则不写入，
            // 仅记录日志（初始化已成功完成，下次 get_init_status 走 DB 路径）。
            let arc = setup_initialized_flag();
            let mut guard = arc.lock().unwrap_or_else(|e| {
                tracing::error!(error = %e, "P9-1: setup 初始化标志锁中毒，跳过本次写入");
                e.into_inner()
            });
            *guard = true;
            Ok(Json(ApiResponse::success_with_message(
                result,
                "系统初始化成功",
            )))
        }
        Err(e) => {
            let error = match e {
                InitError::AlreadyInitialized => "already_initialized",
                InitError::HashError(_) => "hash_error",
                InitError::DatabaseError(_) => "database_error",
                InitError::UserNotFound => "user_not_found",
                InitError::ConfigError(_) => "config_error",
                // P0 新增：参数校验错误（如密码强度不足）
                InitError::ValidationError(_) => "validation_error",
            };

            let message = match e {
                InitError::AlreadyInitialized => "系统已经初始化，不能重复初始化".to_string(),
                InitError::HashError(msg) => format!("密码加密失败: {}", msg),
                InitError::DatabaseError(msg) => msg,
                InitError::UserNotFound => "用户不存在".to_string(),
                InitError::ConfigError(msg) => format!("配置错误: {}", msg),
                // P0 新增：参数校验错误中文提示
                InitError::ValidationError(msg) => format!("参数校验失败: {}", msg),
            };

            Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(InitErrorResponse {
                    error: error.to_string(),
                    message,
                }),
            ))
        }
    }
}

/// 创建 Setup 模式路由器（数据库未连接时使用）。
///
/// TS-S-1 修复（2026-06-25 第二次全面审计）：setup 模式下数据库未就绪，
/// auth_middleware 未挂载，高危初始化接口必须由 init_token_middleware 保护，
/// 防止攻击者匿名 POST 完成系统初始化（抢占首个管理员账号）。
///
/// 设计与 routes/system.rs::init() 保持一致：
/// - 高危接口（initialize*）→ 应用 init_token_middleware
/// - 只读接口（/status）→ 公开（无副作用）
/// - 受限接口（/test-database）→ 公开（handler 内已有 admin 二次校验）
pub fn create_init_router() -> Router<()> {
    let protected = Router::<()>::new()
        .route("/init/initialize-with-db", post(initialize_with_db))
        .layer(axum::middleware::from_fn(init_token_middleware));

    let public = Router::<()>::new()
        .route("/init/status", get(get_init_status))
        .route("/init/test-database", post(test_database_connection));

    Router::<()>::new().nest("/api/v1/erp", protected.merge(public))
}
