//! 系统更新处理器
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use crate::services::system_update_service::{LocalRelease, SystemUpdateService, UpdateError};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{extract::Multipart, Json};
use std::path::PathBuf;
use tokio::fs;

fn verify_zip_magic(data: &[u8]) -> bool {
    data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
}

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
pub struct CheckUpdateResponse {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
    pub file_size: Option<u64>,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

pub async fn check_for_updates() -> Result<Json<ApiResponse<CheckUpdateResponse>>, AppError> {
    let service = SystemUpdateService::new();
    let result = service.check_for_updates().await;

    if let Some(err) = result.error {
        return Err(AppError::internal(err));
    }

    let response = if result.has_update {
        let release = result.release_info.as_ref();
        let asset = release.and_then(|r| {
            r.assets
                .iter()
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
            current_version: result.current_version,
            latest_version: result.latest_version,
            download_url: None,
            file_size: None,
            release_notes: None,
            published_at: None,
        }
    };

    Ok(Json(ApiResponse::success_with_message(
        response,
        "检查更新成功",
    )))
}

pub async fn download_and_update() -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    let service = SystemUpdateService::new();

    match service.download_and_update().await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(new_version),
                },
                "更新下载完成",
            )))
        }
        Err(e) => {
            let message = e.to_string();
            Err(match e {
                UpdateError::NetworkError(_) => AppError::internal(message),
                UpdateError::VersionError(_) => AppError::bad_request(message),
                UpdateError::AlreadyUpdating => AppError::business(message),
                _ => AppError::internal(message),
            })
        }
    }
}

pub async fn get_version() -> Json<ApiResponse<VersionResponse>> {
    let service = SystemUpdateService::new();
    let version = service.get_current_version();

    Json(ApiResponse::success(VersionResponse {
        version,
        release_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
        changelog: None,
    }))
}

pub async fn get_update_status() -> Json<ApiResponse<UpdateStatusResponse>> {
    let service = SystemUpdateService::new();
    let status = service.get_status();

    Json(ApiResponse::success(UpdateStatusResponse {
        current_version: status.current_version,
        is_updating: status.is_updating,
        last_update_time: status.last_update_time,
        backup_versions: status.backup_versions,
    }))
}

pub async fn upload_and_update(
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    let mut update_file_path: Option<PathBuf> = None;

    const MAX_UPDATE_SIZE: usize = 100 * 1024 * 1024;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field.file_name().unwrap_or("update.zip").to_string();

        if file_name.ends_with(".zip") {
            let safe_filename = format!("update_{}.zip", uuid::Uuid::new_v4());
            let temp_dir = std::env::temp_dir();
            let save_path = temp_dir.join(&safe_filename);

            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("文件上传失败：{}", e)))?;

            if data.len() > MAX_UPDATE_SIZE {
                return Err(AppError::bad_request(format!(
                    "文件大小超过限制 ({}MB)",
                    MAX_UPDATE_SIZE / 1024 / 1024
                )));
            }

            if !verify_zip_magic(&data) {
                return Err(AppError::bad_request(
                    "上传的文件不是有效的 ZIP 格式".to_string(),
                ));
            }

            fs::write(&save_path, &data)
                .await
                .map_err(|e| AppError::internal(format!("文件保存失败：{}", e)))?;

            // 路径遍历防护：验证保存路径在预期目录内
            let canonical_save_path = save_path.canonicalize().map_err(|e| {
                // 清理已写入的文件
                let _ = std::fs::remove_file(&save_path);
                AppError::bad_request(format!("无效的文件路径：{}", e))
            })?;

            let canonical_temp_dir = temp_dir.canonicalize().map_err(|e| {
                // 清理已写入的文件
                let _ = std::fs::remove_file(&save_path);
                AppError::internal(format!("临时目录错误：{}", e))
            })?;

            if !canonical_save_path.starts_with(&canonical_temp_dir) {
                // 清理已写入的文件
                let _ = std::fs::remove_file(&save_path);
                return Err(AppError::bad_request("检测到路径遍历攻击".to_string()));
            }

            update_file_path = Some(save_path);
        }
    }

    let update_file =
        update_file_path.ok_or_else(|| AppError::bad_request("未找到更新包文件".to_string()))?;

    let service = SystemUpdateService::new();

    match service.apply_update(&update_file).await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(new_version),
                },
                "更新应用成功",
            )))
        }
        Err(e) => {
            let message = e.to_string();
            Err(match e {
                UpdateError::IoError(_)
                | UpdateError::UnzipError(_)
                | UpdateError::BackupError(_)
                | UpdateError::NetworkError(_) => AppError::internal(message),
                UpdateError::ValidationError(_) | UpdateError::VersionError(_) => {
                    AppError::bad_request(message)
                }
                UpdateError::AlreadyUpdating => AppError::business(message),
            })
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct RollbackRequest {
    pub version: String,
}

pub async fn rollback_version(
    Json(payload): Json<RollbackRequest>,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    let service = SystemUpdateService::new();

    match service.rollback_to_version(&payload.version) {
        Ok(message) => {
            let current_version = service.get_current_version();
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(current_version),
                },
                "版本回滚成功",
            )))
        }
        Err(e) => Err(AppError::internal(e.to_string())),
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

pub async fn check_for_local_updates(
) -> Result<Json<ApiResponse<CheckLocalUpdateResponse>>, AppError> {
    let service = SystemUpdateService::new();
    let result = service.check_local_updates();

    Ok(Json(ApiResponse::success(CheckLocalUpdateResponse {
        has_update: result.has_update,
        current_version: result.current_version,
        latest_version: result.latest_version,
        latest_release: result.local_release,
        error: result.error,
    })))
}

pub async fn list_local_releases() -> Result<Json<ApiResponse<LocalReleasesResponse>>, AppError> {
    let service = SystemUpdateService::new();

    match service.list_local_releases() {
        Ok(releases) => Ok(Json(ApiResponse::success(LocalReleasesResponse {
            count: releases.len(),
            releases,
        }))),
        Err(e) => Err(AppError::internal(e.to_string())),
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct ApplyLocalUpdateRequest {
    pub version: String,
}

pub async fn apply_local_update(
    Json(payload): Json<ApplyLocalUpdateRequest>,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    let service = SystemUpdateService::new();

    let releases = service
        .list_local_releases()
        .map_err(|e| AppError::internal(e.to_string()))?;

    let release = releases
        .into_iter()
        .find(|r| r.version == payload.version)
        .ok_or_else(|| AppError::not_found(format!("找不到版本 {} 的发布包", payload.version)))?;

    match service.apply_local_update(&release).await {
        Ok(message) => {
            let new_version = service.get_current_version();
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(new_version),
                },
                "本地更新应用成功",
            )))
        }
        Err(e) => {
            let message = e.to_string();
            Err(match e {
                UpdateError::IoError(_)
                | UpdateError::UnzipError(_)
                | UpdateError::BackupError(_)
                | UpdateError::NetworkError(_) => AppError::internal(message),
                UpdateError::ValidationError(_) | UpdateError::VersionError(_) => {
                    AppError::bad_request(message)
                }
                UpdateError::AlreadyUpdating => AppError::business(message),
            })
        }
    }
}

pub async fn get_backup_versions() -> Json<ApiResponse<Vec<String>>> {
    let service = SystemUpdateService::new();
    Json(ApiResponse::success(service.list_backup_versions()))
}
