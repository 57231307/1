//! 系统更新处理器

use crate::middleware::auth_context::AuthContext;
use crate::services::system_update_service::{LocalRelease, SystemUpdateService, UpdateError};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Multipart, State},
    Json,
};
use std::path::PathBuf;
use tokio::fs;

/// P0 7-2 修复：要求调用者具备 admin 角色，否则拒绝并记录审计日志
///
/// 安全原因：原 `download_and_update`、`upload_and_update`、`rollback_version`、
/// `apply_local_update` 四个 handler 完全未注入 `AuthContext`，仅靠全局
/// `permission_middleware` 做 RBAC，缺少 handler 层深度防御。
/// 系统更新属高危操作（涉及二进制替换、版本回滚，可导致 RCE），
/// 必须在 handler 层显式校验 admin 角色，防止权限中间件被绕过或配置错误时
/// 任意登录用户上传/应用恶意更新包。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            "[SECURITY] 非 admin 用户调用系统更新敏感接口被拒绝"
        );
        return Err(AppError::permission_denied(
            "系统更新操作仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

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

pub async fn download_and_update(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // P0 7-2 修复：远程下载并应用更新属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

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
    State(state): State<AppState>,
    auth: AuthContext,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // P0 7-2 修复：上传并应用更新包属高危操作（可导致 RCE），仅 admin 可执行
    require_admin_role(&state, &auth).await?;

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
                // L-5 修复（批次 375 v13 复审）：清理失败不再吞错，记录 warn 日志
                if let Err(rm_err) = std::fs::remove_file(&save_path) {
                    tracing::warn!(error = %rm_err, "清理已写入文件失败（可忽略）");
                }
                AppError::bad_request(format!("无效的文件路径：{}", e))
            })?;

            let canonical_temp_dir = temp_dir.canonicalize().map_err(|e| {
                // L-5 修复（批次 375 v13 复审）：清理失败不再吞错，记录 warn 日志
                if let Err(rm_err) = std::fs::remove_file(&save_path) {
                    tracing::warn!(error = %rm_err, "清理已写入文件失败（可忽略）");
                }
                AppError::internal(format!("临时目录错误：{}", e))
            })?;

            if !canonical_save_path.starts_with(&canonical_temp_dir) {
                // L-5 修复（批次 375 v13 复审）：清理失败不再吞错，记录 warn 日志
                if let Err(rm_err) = std::fs::remove_file(&save_path) {
                    tracing::warn!(error = %rm_err, "清理已写入文件失败（可忽略）");
                }
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
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<RollbackRequest>,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // P0 7-2 修复：版本回滚属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

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
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<ApplyLocalUpdateRequest>,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // P0 7-2 修复：应用本地更新包属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

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

#[cfg(test)]
mod tests {
    //! 系统更新 Handler 单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - verify_zip_magic ZIP 文件头校验纯函数（5 个分支）
    //! - VersionResponse / UpdateResult DTO 构造（1 个测试）

    use super::*;

    /// 测试_verify_zip_magic合法ZIP
    ///
    /// 场景：以 PK\x03\x04 开头的合法 ZIP 数据应返回 true
    #[test]
    fn 测试_verify_zip_magic合法ZIP() {
        let data = [0x50, 0x4B, 0x03, 0x04, 0x00, 0x00];
        assert!(verify_zip_magic(&data), "合法 ZIP 头应返回 true");
    }

    /// 测试_verify_zip_magic空数据
    ///
    /// 场景：空切片应返回 false（无法匹配 4 字节前缀）
    #[test]
    fn 测试_verify_zip_magic空数据() {
        let data: [u8; 0] = [];
        assert!(!verify_zip_magic(&data), "空数据应返回 false");
    }

    /// 测试_verify_zip_magic非ZIP文件
    ///
    /// 场景：JPEG 文件头 [FF D8 FF E0] 应返回 false
    #[test]
    fn 测试_verify_zip_magic非ZIP文件() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x00];
        assert!(!verify_zip_magic(&jpeg_header), "JPEG 头应返回 false");

        // PNG 文件头
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A];
        assert!(!verify_zip_magic(&png_header), "PNG 头应返回 false");
    }

    /// 测试_verify_zip_magic部分匹配
    ///
    /// 场景：仅前 3 字节匹配（缺第 4 字节 0x04）应返回 false
    #[test]
    fn 测试_verify_zip_magic部分匹配() {
        let partial = [0x50, 0x4B, 0x03, 0x00]; // 第 4 字节不匹配
        assert!(!verify_zip_magic(&partial), "部分匹配应返回 false");

        let partial2 = [0x50, 0x4B, 0x03]; // 仅 3 字节
        assert!(!verify_zip_magic(&partial2), "仅 3 字节应返回 false");
    }

    /// 测试_verify_zip_magic恰好4字节
    ///
    /// 场景：恰好 4 字节 [0x50, 0x4B, 0x03, 0x04] 应返回 true
    #[test]
    fn 测试_verify_zip_magic恰好4字节() {
        let exact = [0x50, 0x4B, 0x03, 0x04];
        assert!(verify_zip_magic(&exact), "恰好 4 字节合法 ZIP 头应返回 true");
    }

    /// 测试_VersionResponse和UpdateResult构造
    ///
    /// 验证 VersionResponse 和 UpdateResult 结构体能正确构造并设置字段
    #[test]
    fn 测试_VersionResponse和UpdateResult构造() {
        // VersionResponse 构造
        let version_resp = VersionResponse {
            version: "1.0.0".to_string(),
            release_date: "2026-07-14".to_string(),
            changelog: Some("修复若干问题".to_string()),
        };
        assert_eq!(version_resp.version, "1.0.0");
        assert_eq!(version_resp.release_date, "2026-07-14");
        assert!(version_resp.changelog.is_some());

        // UpdateResult 成功构造
        let update_result = UpdateResult {
            success: true,
            message: "更新成功".to_string(),
            new_version: Some("2.0.0".to_string()),
        };
        assert!(update_result.success);
        assert_eq!(update_result.message, "更新成功");
        assert_eq!(update_result.new_version, Some("2.0.0".to_string()));

        // UpdateResult 失败构造
        let failed_result = UpdateResult {
            success: false,
            message: "更新失败".to_string(),
            new_version: None,
        };
        assert!(!failed_result.success);
        assert!(failed_result.new_version.is_none());
    }
}
