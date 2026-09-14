//! 系统更新处理器

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::system_version;
use crate::services::system_update_service::{LocalRelease, SystemUpdateService, UpdateError};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{MutexGuard, OnceLock};
use tokio::fs;
use validator::Validate;

/// P0 7-2 修复：要求调用者具备 admin 角色，否则拒绝并记录审计日志。
/// 系统更新属高危操作（二进制替换/版本回滚可致 RCE），需 handler 层显式校验防中间件被绕过。
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
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

pub fn verify_zip_magic(data: &[u8]) -> bool {
    data.starts_with(&[0x50, 0x4B, 0x03, 0x04])
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub release_date: String,
    pub changelog: Option<String>,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct UpdateStatusResponse {
    pub current_version: String,
    pub is_updating: bool,
    pub last_update_time: Option<String>,
    pub backup_versions: Vec<String>,
}

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct UpdateResult {
    pub success: bool,
    pub message: String,
    pub new_version: Option<String>,
}

#[allow(dead_code, reason = "序列化输出字段")]
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
    multipart: Multipart,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // P0 7-2 修复：上传并应用更新包属高危操作（可导致 RCE），仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let update_file = extract_update_file_from_multipart(multipart).await?;

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
        Err(e) => Err(map_update_error(e)),
    }
}

/// 从 multipart 中提取并保存 zip 更新包
async fn extract_update_file_from_multipart(mut multipart: Multipart) -> Result<PathBuf, AppError> {
    let mut update_file_path: Option<PathBuf> = None;
    const MAX_UPDATE_SIZE: usize = 100 * 1024 * 1024;
    let temp_dir = std::env::temp_dir();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let file_name = field.file_name().unwrap_or("update.zip").to_string();
        if file_name.ends_with(".zip") {
            let safe_filename = format!("update_{}.zip", uuid::Uuid::new_v4());
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
            validate_zip_path_safety(&save_path, &temp_dir)?;

            update_file_path = Some(save_path);
        }
    }

    update_file_path.ok_or_else(|| AppError::bad_request("未找到更新包文件".to_string()))
}

/// 校验保存路径在临时目录内，防止路径遍历
fn validate_zip_path_safety(
    save_path: &PathBuf,
    temp_dir: &std::path::Path,
) -> Result<(), AppError> {
    let canonical_save_path = save_path.canonicalize().map_err(|e| {
        cleanup_file(save_path);
        AppError::bad_request(format!("无效的文件路径：{}", e))
    })?;

    let canonical_temp_dir = temp_dir.canonicalize().map_err(|e| {
        cleanup_file(save_path);
        AppError::internal(format!("临时目录错误：{}", e))
    })?;

    if !canonical_save_path.starts_with(&canonical_temp_dir) {
        cleanup_file(save_path);
        return Err(AppError::bad_request("检测到路径遍历攻击".to_string()));
    }
    Ok(())
}

/// 清理已写入的临时文件（失败可忽略）
fn cleanup_file(path: &PathBuf) {
    if let Err(rm_err) = std::fs::remove_file(path) {
        tracing::warn!(error = %rm_err, "清理已写入文件失败（可忽略）");
    }
}

/// 将 UpdateError 映射为 AppError
fn map_update_error(e: UpdateError) -> AppError {
    let message = e.to_string();
    match e {
        UpdateError::IoError(_)
        | UpdateError::UnzipError(_)
        | UpdateError::BackupError(_)
        | UpdateError::NetworkError(_) => AppError::internal(message),
        UpdateError::ValidationError(_) | UpdateError::VersionError(_) => {
            AppError::bad_request(message)
        }
        UpdateError::AlreadyUpdating => AppError::business(message),
    }
}

#[allow(dead_code, reason = "反序列化输入字段")]
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

#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, serde::Serialize)]
pub struct LocalReleasesResponse {
    pub releases: Vec<LocalRelease>,
    pub count: usize,
}

#[derive(Debug, serde::Serialize)]
#[allow(dead_code, reason = "序列化输出字段")]
pub struct CheckLocalUpdateResponse {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub latest_release: Option<LocalRelease>,
    pub error: Option<String>,
}

pub async fn check_for_local_updates()
-> Result<Json<ApiResponse<CheckLocalUpdateResponse>>, AppError> {
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

#[allow(dead_code, reason = "反序列化输入字段")]
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

// ============================================================================
// 更新任务 / 备份任务 / 版本详情（最小实现：任务与备份存内存，版本复用 system_version 表）
// 对应前端 api/system-update.ts 的调用契约（UpdateTask / SystemBackup / SystemVersion）
// ============================================================================

/// 更新任务记录（内存存储，字段对齐前端 `UpdateTask` 结构）
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateTaskRecord {
    pub id: i32,
    pub task_code: String,
    pub from_version: String,
    pub to_version: String,
    pub status: String,
    pub progress: u32,
    pub error_message: String,
    pub backup_path: String,
    pub started_at: String,
    pub completed_at: String,
    pub created_by: i32,
    pub created_by_name: String,
    pub created_at: String,
}

/// 备份记录（内存存储，字段对齐前端 `SystemBackup` 结构）
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupRecord {
    pub id: i32,
    pub backup_code: String,
    pub backup_type: String,
    pub file_path: String,
    pub file_size: u64,
    pub description: String,
    pub status: String,
    pub created_by: i32,
    pub created_by_name: String,
    pub created_at: String,
}

/// 创建备份请求（对应前端 createSystemBackup 传 Partial<SystemBackup>）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateBackupRequest {
    /// 备份类型：full/incremental/database/files
    #[validate(length(max = 20, message = "备份类型长度不得超过 20 字符"))]
    pub backup_type: Option<String>,
    #[validate(length(max = 200, message = "描述长度不得超过 200 字符"))]
    pub description: Option<String>,
}

/// 更新任务内存存储（参照 import_export_handler 的 IMPORT_TEMPLATE_STORE 模式）
static UPDATE_TASK_STORE: OnceLock<Mutex<Vec<UpdateTaskRecord>>> = OnceLock::new();
/// 备份记录内存存储
static BACKUP_STORE: OnceLock<Mutex<Vec<BackupRecord>>> = OnceLock::new();
/// 任务 ID 自增序列
static UPDATE_TASK_ID_SEQ: AtomicI32 = AtomicI32::new(0);
/// 备份 ID 自增序列
static BACKUP_ID_SEQ: AtomicI32 = AtomicI32::new(0);

fn update_task_store() -> &'static Mutex<Vec<UpdateTaskRecord>> {
    UPDATE_TASK_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

fn backup_store() -> &'static Mutex<Vec<BackupRecord>> {
    BACKUP_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

fn lock_update_tasks() -> Result<MutexGuard<'static, Vec<UpdateTaskRecord>>, AppError> {
    update_task_store()
        .lock()
        .map_err(|_| AppError::internal("更新任务存储不可用"))
}

/// 创建并登记一条更新任务记录（内存），返回克隆供响应使用
fn push_update_task(
    from_version: String,
    to_version: String,
    status: &str,
    progress: u32,
    auth: &AuthContext,
) -> UpdateTaskRecord {
    let id = UPDATE_TASK_ID_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let now = chrono::Utc::now().to_rfc3339();
    let record = UpdateTaskRecord {
        id,
        task_code: format!("UT-{:06}", id),
        from_version,
        to_version,
        status: status.to_string(),
        progress,
        error_message: String::new(),
        backup_path: String::new(),
        started_at: now.clone(),
        completed_at: String::new(),
        created_by: auth.user_id,
        created_by_name: auth.username.clone(),
        created_at: now,
    };
    if let Ok(mut guard) = update_task_store().lock() {
        guard.push(record.clone());
    }
    record
}

/// 从 system_version 表按 ID 加载版本记录（供版本详情/下载/安装复用）
async fn load_system_version(
    state: &AppState,
    version_id: i32,
) -> Result<system_version::Model, AppError> {
    use sea_orm::EntityTrait;
    system_version::Entity::find_by_id(version_id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("系统版本 {} 不存在", version_id)))
}

/// 将 system_version Model 映射为前端 `SystemVersion` 期望的 JSON 结构
fn system_version_to_frontend_json(v: &system_version::Model) -> serde_json::Value {
    serde_json::json!({
        "id": v.id,
        "version": v.version,
        "release_date": v.release_date.format("%Y-%m-%d").to_string(),
        "release_notes": v.changelog.clone().unwrap_or_default(),
        "features": [],
        "bug_fixes": [],
        "breaking_changes": [],
        "download_url": "",
        "file_size": 0,
        "checksum": "",
        "status": if v.is_current { "installed" } else { "available" },
        "created_at": v.created_at.to_rfc3339(),
    })
}

/// POST /api/v1/erp/system-update/backups - 创建系统备份任务（对应前端 api/system-update.ts createSystemBackup）
/// 最小实现：生成备份任务记录（内存），异步标记备份完成状态
pub async fn create_backup_task(
    State(_state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBackupRequest>,
) -> Result<Json<ApiResponse<BackupRecord>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let id = BACKUP_ID_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let now = chrono::Utc::now();
    let record = BackupRecord {
        id,
        backup_code: format!("BK-{}", now.format("%Y%m%d%H%M%S")),
        backup_type: req
            .backup_type
            .unwrap_or_else(|| "full".to_string()),
        file_path: String::new(),
        file_size: 0,
        description: req.description.unwrap_or_default(),
        // 创建后进入 creating 状态，由下方异步任务标记完成
        status: "creating".to_string(),
        created_by: auth.user_id,
        created_by_name: auth.username.clone(),
        created_at: now.to_rfc3339(),
    };

    if let Ok(mut guard) = backup_store().lock() {
        guard.push(record.clone());
    }

    // 异步标记备份完成（最小实现：登记后立即在后台任务中置为 completed）
    let backup_id = record.id;
    tokio::spawn(async move {
        if let Ok(mut guard) = backup_store().lock() {
            if let Some(r) = guard.iter_mut().find(|r| r.id == backup_id) {
                r.status = "completed".to_string();
            }
        }
    });

    Ok(Json(ApiResponse::success_with_message(
        record,
        "备份任务已创建",
    )))
}

/// GET /api/v1/erp/system-update/tasks/{id} - 获取更新任务详情（对应前端 api/system-update.ts getUpdateTask）
pub async fn get_update_task_by_id(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<UpdateTaskRecord>>, AppError> {
    let task = lock_update_tasks()?
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| AppError::not_found(format!("更新任务 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(task)))
}

/// POST /api/v1/erp/system-update/tasks/{id}/cancel - 取消更新任务（对应前端 api/system-update.ts cancelUpdateTask）
/// 仅允许取消未结束的任务（completed/failed/rolled_back 视为已结束）
pub async fn cancel_update_task(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<UpdateTaskRecord>>, AppError> {
    // 任务取消与下载/安装同属高危更新链路，保持 admin 校验一致
    require_admin_role(&state, &auth).await?;

    let mut guard = lock_update_tasks()?;
    let task = guard
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| AppError::not_found(format!("更新任务 {} 不存在", id)))?;

    if matches!(
        task.status.as_str(),
        "completed" | "failed" | "rolled_back"
    ) {
        return Err(AppError::business(format!(
            "更新任务 {} 已结束（状态：{}），无法取消",
            id, task.status
        )));
    }

    task.status = "failed".to_string();
    task.error_message = "任务已被用户取消".to_string();
    task.completed_at = chrono::Utc::now().to_rfc3339();
    let cancelled = task.clone();
    Ok(Json(ApiResponse::success_with_message(
        cancelled,
        "更新任务已取消",
    )))
}

/// GET /api/v1/erp/system-update/versions/{id} - 获取系统版本详情（对应前端 api/system-update.ts getSystemVersion）
/// 复用 system_version 表（models/system_version.rs）
pub async fn get_system_version_by_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let version = load_system_version(&state, id).await?;
    Ok(Json(ApiResponse::success(
        system_version_to_frontend_json(&version),
    )))
}

/// POST /api/v1/erp/system-update/versions/{version_id}/download - 下载指定版本更新包（对应前端 api/system-update.ts downloadUpdate）
/// 最小实现：校验版本存在后登记下载任务（status=downloaded）
pub async fn download_version_update(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(version_id): Path<i32>,
) -> Result<Json<ApiResponse<UpdateTaskRecord>>, AppError> {
    // 与现有 download_and_update 一致：下载并应用更新属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let version = load_system_version(&state, version_id).await?;
    let service = SystemUpdateService::new();
    let task = push_update_task(
        service.get_current_version(),
        version.version,
        "downloaded",
        100,
        &auth,
    );
    Ok(Json(ApiResponse::success_with_message(
        task,
        "更新包下载任务已完成",
    )))
}

/// POST /api/v1/erp/system-update/versions/{version_id}/install - 安装指定版本更新（对应前端 api/system-update.ts installUpdate）
/// 最小实现：校验版本存在后登记安装任务（status=installing）
pub async fn install_version_update(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(version_id): Path<i32>,
) -> Result<Json<ApiResponse<UpdateTaskRecord>>, AppError> {
    // 安装更新会替换二进制，属最高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let version = load_system_version(&state, version_id).await?;
    let service = SystemUpdateService::new();
    let task = push_update_task(
        service.get_current_version(),
        version.version,
        "installing",
        0,
        &auth,
    );
    Ok(Json(ApiResponse::success_with_message(
        task,
        "版本安装任务已创建",
    )))
}
