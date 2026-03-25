use crate::services::init_service::{InitRequest, InitService, InitStatus};
use axum::{extract::State, http::StatusCode, Json};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub async fn get_init_status(State(db): State<Arc<DatabaseConnection>>) -> Json<InitStatus> {
    let init_service = InitService::new(db);
    let initialized = init_service.check_initialized().await.unwrap_or(false);

    let message = if initialized {
        "系统已经初始化".to_string()
    } else {
        "系统未初始化，需要进行初始化".to_string()
    };

    Json(InitStatus {
        initialized,
        message,
    })
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
            let error_type = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "already_initialized"
                }
                crate::services::init_service::InitError::HashError => "hash_error",
                crate::services::init_service::InitError::DatabaseError(_) => "database_error",
                crate::services::init_service::InitError::UserNotFound => "user_not_found",
            };

            let message = match e {
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已经初始化，不能重复初始化".to_string()
                }
                crate::services::init_service::InitError::HashError => "密码加密失败".to_string(),
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
            };

            Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: error_type.to_string(),
                    message,
                }),
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

    match init_service
        .reset_password(&payload.username, &payload.new_password)
        .await
    {
        Ok(()) => Ok(Json(ResetPasswordResponse {
            success: true,
            message: "密码重置成功".to_string(),
        })),
        Err(e) => {
            let message = match e {
                crate::services::init_service::InitError::UserNotFound => "用户不存在".to_string(),
                crate::services::init_service::InitError::HashError => "密码加密失败".to_string(),
                crate::services::init_service::InitError::DatabaseError(msg) => msg,
                crate::services::init_service::InitError::AlreadyInitialized => {
                    "系统已初始化".to_string()
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
