//! 系统更新处理器

use axum::{
    extract::Multipart,
    http::StatusCode,
    Json,
};
use std::path::PathBuf;
use tokio::fs;
use crate::services::system_update_service::{SystemUpdateService, UpdateError};

#[derive(Debug, serde::Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub release_date: String,
    pub changelog: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub is_updating: bool,
    pub last_update_time: Option<String>,
    pub backup_versions: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub new_version: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub async fn get_version() -> Json<VersionResponse> {
    let service = SystemUpdateService::new();
    let version = service.get_current_version();

    Json(VersionResponse {
        version,
        release_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        changelog: None,
    })
}

pub async fn get_update_status() -> Json<UpdateStatusResponse> {
    let service = SystemUpdateService::new();
    let status = service.get_update_status();

    Json(UpdateStatusResponse {
        current_version: status.current_version,
        is_updating: status.is_updating,
        last_update_time: status.last_update_time,
        backup_versions: status.backup_versions,
    })
}

pub async fn upload_and_update(
    mut multipart: Multipart,
) -> Result<Json<UpdateResult>, (StatusCode, Json<ErrorResponse>)> {
    let mut update_file_path: Option<PathBuf> = None;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field.file_name().unwrap_or("update.zip").to_string();

        if file_name.ends_with(".zip") {
            let temp_dir = std::env::temp_dir();
            let save_path = temp_dir.join(&file_name);

            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: "upload_failed".to_string(),
                        message: format!("文件上传失败: {}", e),
                    }),
                )
            })?;

            fs::write(&save_path, &data).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "save_failed".to_string(),
                        message: format!("文件保存失败: {}", e),
                    }),
                )
            })?;

            update_file_path = Some(save_path);
        }
    }

    let update_file = update_file_path.ok_or((
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: "no_file".to_string(),
            message: "未找到更新包文件".to_string(),
        }),
    ))?;

    let service = SystemUpdateService::new();

    match service.apply_update(&update_file).await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(UpdateResult {
                success: true,
                message,
                new_version: Some(new_version),
            }))
        }
        Err(e) => {
            let error_type = match e {
                UpdateError::IoError(_) => "io_error",
                UpdateError::UnzipError(_) => "unzip_error",
                UpdateError::BackupError(_) => "backup_error",
                UpdateError::ValidationError(_) => "validation_error",
                UpdateError::VersionError(_) => "version_error",
                UpdateError::AlreadyUpdating => "already_updating",
            };

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: error_type.to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RollbackRequest {
    pub version: String,
}

pub async fn rollback_version(
    Json(payload): Json<RollbackRequest>,
) -> Result<Json<UpdateResult>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();

    match service.rollback_to_version(&payload.version) {
        Ok(message) => {
            let current_version = service.get_current_version();
            Ok(Json(UpdateResult {
                success: true,
                message,
                new_version: Some(current_version),
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "rollback_failed".to_string(),
                message: e.to_string(),
            }),
        )),
    }
}

pub async fn get_backup_versions() -> Json<Vec<String>> {
    let service = SystemUpdateService::new();
    Json(service.list_backup_versions())
}
