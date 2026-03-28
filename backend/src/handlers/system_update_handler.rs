//! 系统更新处理器

use axum::{
    extract::Multipart,
    http::StatusCode,
    Json,
};
use std::path::PathBuf;
use tokio::fs;
use crate::services::system_update_service::{SystemUpdateService, UpdateError, LocalRelease};

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

#[derive(Debug, serde::Serialize)]
pub struct CheckUpdateResponse {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub file_size: Option<u64>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

pub async fn check_for_updates() -> Result<Json<CheckUpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();
    
    let result = service.check_for_updates().await;
    
    if result.error.is_some() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "check_failed".to_string(),
                message: result.error.unwrap_or_default(),
            }),
        ));
    }
    
    let response = if result.has_update {
        let release = result.release_info.as_ref();
        let asset = release.and_then(|r| {
            r.assets.iter()
                .find(|a| a.name.ends_with(".zip") || a.name.ends_with(".tar.gz"))
        });
        
        CheckUpdateResponse {
            has_update: true,
            current_version: result.current_version,
            latest_version: result.latest_version,
            download_url: asset.map(|a| a.browser_download_url.clone()),
            file_size: asset.map(|a| a.size),
            release_notes: release.and_then(|r| r.body.clone()),
            published_at: release.map(|r| r.published_at.clone()),
        }
    } else {
        CheckUpdateResponse {
            has_update: false,
            current_version: result.current_version.clone(),
            latest_version: result.latest_version,
            download_url: None,
            file_size: None,
            release_notes: None,
            published_at: None,
        }
    };
    
    Ok(Json(response))
}

pub async fn download_and_update() -> Result<Json<UpdateResult>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();
    
    match service.download_and_update().await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(UpdateResult {
                success: true,
                message,
                new_version: Some(new_version),
            }))
        }
        Err(e) => {
            let error_type = match &e {
                UpdateError::NetworkError(_) => "network_error",
                UpdateError::VersionError(_) => "version_error",
                UpdateError::AlreadyUpdating => "already_updating",
                _ => "update_failed",
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
    let status = service.get_status();

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
                UpdateError::NetworkError(_) => "network_error",
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

#[derive(Debug, serde::Serialize)]
pub struct LocalReleasesResponse {
    pub releases: Vec<LocalRelease>,
    pub count: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct CheckLocalUpdateResponse {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub latest_release: Option<LocalRelease>,
    pub error: Option<String>,
}

pub async fn check_for_local_updates() -> Result<Json<CheckLocalUpdateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();
    
    let result = service.check_local_updates();
    
    Ok(Json(CheckLocalUpdateResponse {
        has_update: result.has_update,
        current_version: result.current_version,
        latest_version: result.latest_version,
        latest_release: result.local_release,
        error: result.error,
    }))
}

pub async fn list_local_releases() -> Result<Json<LocalReleasesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();
    
    match service.list_local_releases() {
        Ok(releases) => {
            Ok(Json(LocalReleasesResponse {
                count: releases.len(),
                releases,
            }))
        }
        Err(e) => {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "list_failed".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplyLocalUpdateRequest {
    pub version: String,
}

pub async fn apply_local_update(
    Json(payload): Json<ApplyLocalUpdateRequest>,
) -> Result<Json<UpdateResult>, (StatusCode, Json<ErrorResponse>)> {
    let service = SystemUpdateService::new();
    
    let releases = service.list_local_releases().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "list_failed".to_string(),
                message: e.to_string(),
            }),
        )
    })?;
    
    let release = releases.into_iter()
        .find(|r| r.version == payload.version)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "release_not_found".to_string(),
                    message: format!("找不到版本 {} 的发布包", payload.version),
                }),
            )
        })?;
    
    match service.apply_local_update(&release).await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(UpdateResult {
                success: true,
                message,
                new_version: Some(new_version),
            }))
        }
        Err(e) => {
            let error_type = match &e {
                UpdateError::IoError(_) => "io_error",
                UpdateError::UnzipError(_) => "unzip_error",
                UpdateError::BackupError(_) => "backup_error",
                UpdateError::ValidationError(_) => "validation_error",
                UpdateError::VersionError(_) => "version_error",
                UpdateError::AlreadyUpdating => "already_updating",
                _ => "update_failed",
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

pub async fn get_backup_versions() -> Json<Vec<String>> {
    let service = SystemUpdateService::new();
    Json(service.list_backup_versions())
}
