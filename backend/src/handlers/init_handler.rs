//! 系统初始化处理器

use crate::services::init_service::{DatabaseConfig, InitRequest, InitService, InitStatus};
use axum::{extract::State, http::StatusCode, Json};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

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

pub async fn get_init_status(State(db): State<Arc<DatabaseConnection>>) -> Json<InitStatus> {
    let init_service = InitService::new(db);
    let (initialized, message) = init_service.check_initialized().await;
    Json(InitStatus {
        initialized,
        message,
    })
}

pub async fn test_database_connection(
    Json(payload): Json<TestDatabaseRequest>,
) -> Result<Json<TestDatabaseResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db_config = DatabaseConfig {
        host: payload.host,
        port: payload.port,
        name: payload.name,
        username: payload.username,
        password: payload.password,
    };

    match InitService::test_database(&db_config).await {
        Ok(_) => Ok(Json(TestDatabaseResponse {
            success: true,
            message: "数据库连接成功".to_string(),
        })),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "database_connection_failed".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

pub async fn initialize_system(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<InitRequest>,
) -> Result<
    Json<crate::services::init_service::InitializationResult>,
    (StatusCode, Json<ErrorResponse>),
> {
    let init_service = InitService::new(db);

    match init_service
        .initialize(&payload.admin_username, &payload.admin_password)
        .await
    {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            let error = match e {
                crate::services::init_service::InitError::AlreadyInitialized => "already_initialized",
                crate::services::init_service::InitError::HashError(_) => "hash_error",
                crate::services::init_service::InitError::DatabaseError(_) => "database_error",
                crate::services::init_service::InitError::UserNotFound => "user_not_found",
                crate::services::init_service::InitError::ConfigError(_) => "config_error",
            };

            let message = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已经初始化，不能重复初始化".to_string()
                }
                crate::services::init_service::InitError::HashError(msg) => {
                    format!("密码加密失败: {}", msg)
                }
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
                crate::services::init_service::InitError::ConfigError(msg) => {
                    format!("配置错误: {}", msg)
                }
            };

            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: error.to_string(), message }),
            ))
        }
    }
}

pub async fn initialize_system_with_db(
    Json(payload): Json<InitWithDbRequest>,
) -> Result<
    Json<crate::services::init_service::InitializationResult>,
    (StatusCode, Json<ErrorResponse>),
> {
    match InitService::initialize_with_db(
        &payload.db_config,
        &payload.admin_username,
        &payload.admin_password,
    )
    .await
    {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            let error = match e {
                crate::services::init_service::InitError::AlreadyInitialized => "already_initialized",
                crate::services::init_service::InitError::HashError(_) => "hash_error",
                crate::services::init_service::InitError::DatabaseError(_) => "database_error",
                crate::services::init_service::InitError::UserNotFound => "user_not_found",
                crate::services::init_service::InitError::ConfigError(_) => "config_error",
            };

            let message = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已经初始化，不能重复初始化".to_string()
                }
                crate::services::init_service::InitError::HashError(msg) => {
                    format!("密码加密失败: {}", msg)
                }
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
                crate::services::init_service::InitError::ConfigError(msg) => {
                    format!("配置错误: {}", msg)
                }
            };

            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: error.to_string(), message }),
            ))
        }
    }
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

pub async fn reset_admin_password(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, (StatusCode, Json<ErrorResponse>)> {
    let init_service = InitService::new(db);

    match init_service.reset_password(&payload.username, &payload.new_password).await {
        Ok(_) => Ok(Json(ResetPasswordResponse {
            success: true,
            message: "密码重置成功".to_string(),
        })),
        Err(e) => {
            let message = match e {
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
                crate::services::init_service::InitError::HashError(msg) => {
                    format!("密码加密失败: {}", msg)
                }
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已初始化".to_string()
                }
                crate::services::init_service::InitError::ConfigError(msg) => {
                    format!("配置错误: {}", msg)
                }
            };

            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "reset_password_failed".to_string(),
                    message,
                }),
            ))
        }
    }
}
