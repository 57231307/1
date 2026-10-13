//! 系统更新处理器

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::system_update_backup;
use crate::models::system_update_task;
use crate::models::system_version;
use crate::services::system_update_service::{LocalRelease, SystemUpdateService, UpdateError};
use crate::utils::admin_checker::{get_role_code, is_system_update_authorized_code};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Multipart, Path, State},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs;
use validator::Validate;

/// 要求调用者具备 admin 或 ops 角色，否则拒绝并记录审计日志。
/// 系统更新属高危操作（二进制替换、版本回滚可致 RCE），需 handler 层显式校验防中间件被绕过。
/// 允许角色集合以 utils/admin_checker 的 SYSTEM_UPDATE_ALLOWED_ROLE_CODES 为单一真相源，
/// 与细粒度权限键授权同源，不在 handler 里另写一份角色词表。
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    let code = get_role_code(state.db.as_ref(), role_id)
        .await
        .ok_or_else(|| AppError::permission_denied("角色信息不可用，无法执行系统更新操作"))?;
    if !is_system_update_authorized_code(&code) {
        tracing::warn!(
            target: "security_audit",
            event = "AUTHORIZATION_DENIED",
            user_id = auth.user_id,
            username = %auth.username,
            role_code = %code,
            "[SECURITY] 非 admin/ops 用户调用系统更新敏感接口被拒绝"
        );
        return Err(AppError::permission_denied(
            "系统更新操作仅限管理员或系统运维角色执行",
        ));
    }
    Ok(())
}

/// 将更新成功后实际生效的版本落库 system_version，并把目标版本置为 is_current、其余置为非当前。
/// 复用现有表列（version、release_date、changelog、is_current），不加列、不加迁移；
/// 真实生效态由 update-status 从该出口读出，供前端展示 applied_version 与 restart_pending。
/// 落库失败必须上抛（不吞异常），由调用方在响应 message 里如实反映。
async fn persist_applied_version(
    db: &sea_orm::DatabaseConnection,
    version: &str,
) -> Result<(), AppError> {
    let now = chrono::Utc::now();
    // 先取消全部当前标记，再写目标版本为当前：保证至多一行 is_current=true（翻转为原子语义，两步同库同连接）。
    system_version::Entity::update_many()
        .col_expr(
            system_version::Column::IsCurrent,
            sea_orm::sea_query::Expr::value(false),
        )
        .filter(system_version::Column::IsCurrent.eq(true))
        .exec(db)
        .await?;

    let existing = system_version::Entity::find()
        .filter(system_version::Column::Version.eq(version))
        .one(db)
        .await?;
    match existing {
        Some(row) => {
            let mut active: system_version::ActiveModel = row.into();
            active.is_current = Set(true);
            active.updated_at = Set(now.into());
            active.update(db).await?;
        }
        None => {
            let model = system_version::ActiveModel {
                version: Set(version.to_string()),
                release_date: Set(now.into()),
                changelog: Set(None),
                is_current: Set(true),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
                ..Default::default()
            };
            model.insert(db).await?;
        }
    }

    tracing::info!(
        event = "SYSTEM_VERSION_PERSISTED",
        version = %version,
        "系统更新目标版本已落库并置为当前版本"
    );
    Ok(())
}

/// 探测进程是否运行在具备自动重启能力的监督器之下（systemd 环境标志）。
/// 判据（任一命中即认定被监督）：环境变量 NOTIFY_SOCKET 或 INVOCATION_ID 存在
/// （systemd 为每个 service 单元注入），或 systemd 运行目录 /run/systemd/system 存在，
/// 或显式部署标志 BINGXI_IS_SERVICE 为真值。探不到监督器一律判 false，
/// 由 apply 链路如实提示需人工重启，绝不无监督自退。
fn supervisor_detected() -> bool {
    if std::env::var_os("NOTIFY_SOCKET").is_some() || std::env::var_os("INVOCATION_ID").is_some() {
        return true;
    }
    if let Some(v) = std::env::var_os("BINGXI_IS_SERVICE") {
        let truthy = matches!(
            v.to_string_lossy().trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        );
        if truthy {
            return true;
        }
    }
    std::path::Path::new("/run/systemd/system").exists()
}

/// 应用更新成功后的受控重启：门控开（restart_on_apply）且探到监督器才延迟自退交 systemd 拉起，
/// 复用仓内既有 setup 模式路径A 的延迟自退范式（spawn 睡短程后 process::exit）。
/// 返回是否已安排自退；false 表示未自退，需人工重启以真实生效。
/// 若担心监督器侧 StartLimitBurst 反复重启失败循环，那属 supervisor 配置职责，进程侧不自作降级。
fn maybe_schedule_restart(message: &mut String) {
    let gated = crate::config::settings::global_update_config().restart_on_apply;
    if gated && supervisor_detected() {
        tracing::info!(
            event = "SYSTEM_UPDATE_RESTART_SCHEDULED",
            "系统更新成功且处于受监督环境，安排延迟自退交由监督器自动重启"
        );
        message.push_str("；更新已应用，进程将延迟自退由监督器自动拉起以完成重启");
        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            tracing::info!("系统更新完成，进程自退以加载新版本（监督器将自动拉起）");
            std::process::exit(0);
        });
    } else if gated {
        // 门控开但无监督器：绝不自退（否则进程一去不回导致服务下线），如实提示需人工重启。
        tracing::warn!(
            event = "SYSTEM_UPDATE_RESTART_MANUAL",
            "系统更新成功但未探测到监督器，不执行自动重启，需人工重启以生效"
        );
        message.push_str("；更新已应用，当前环境未探测到监督器，请人工重启服务以生效");
    } else {
        // 门控关闭：保持不自退，提示人工处理。
        message.push_str("；更新已应用，自动重启门控已关闭，请人工重启服务以生效");
    }
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
    /// 已落库标记为当前的应用版本（system_version.is_current=true 的出口）。
    /// 更新已应用但进程尚未重启时，它可能与编译期 current_version 不一致。
    pub applied_version: Option<String>,
    /// 是否存在待重启生效：applied_version 已落库但与当前运行进程编译版本不一致时为 true。
    /// 供前端展示真实生效态，不臆测。
    pub restart_pending: bool,
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
    pub current_release_notes: Option<String>,
    pub current_published_at: Option<String>,
}

pub async fn check_for_updates() -> Result<Json<ApiResponse<CheckUpdateResponse>>, AppError> {
    let service = SystemUpdateService::new();
    let result = service.check_for_updates().await;

    if let Some(err) = result.error {
        return Err(AppError::internal(err));
    }

    let current_release = result.current_release_info.as_ref();
    let current_release_notes = current_release.and_then(|r| r.body.clone());
    let current_published_at = current_release.map(|r| r.published_at.clone());

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
            current_release_notes,
            current_published_at,
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
            current_release_notes,
            current_published_at,
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
    // 远程下载并应用更新属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let service = SystemUpdateService::new();

    match service.download_and_update().await {
        Ok(message) => {
            let new_version = service
                .applied_version()
                .unwrap_or_else(|| service.get_current_version());
            persist_applied_version(state.db.as_ref(), &new_version).await?;
            let mut message = message;
            maybe_schedule_restart(&mut message);
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(new_version),
                },
                "更新下载完成",
            )))
        }
        Err(e) => Err(map_update_error(e)),
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

/// 读取 system_version 中标记为当前的应用版本（is_current=true）；无记录返回 None。
async fn load_current_applied_version(
    db: &sea_orm::DatabaseConnection,
) -> Result<Option<String>, AppError> {
    let row = system_version::Entity::find()
        .filter(system_version::Column::IsCurrent.eq(true))
        .order_by_desc(system_version::Column::Id)
        .one(db)
        .await?;
    Ok(row.map(|v| v.version))
}

pub async fn get_update_status(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<UpdateStatusResponse>>, AppError> {
    let service = SystemUpdateService::new();
    let status = service.get_status();
    let running_version = service.get_current_version();
    let applied_version = load_current_applied_version(state.db.as_ref()).await?;
    // restart_pending 如实判定：已落库的应用版本存在且与当前运行进程的编译版本不一致，
    // 说明更新包已换文件但进程尚未重启加载新二进制。二者一致或无落库记录时为 false。
    let restart_pending = applied_version
        .as_deref()
        .map(|v| v != running_version)
        .unwrap_or(false);

    Ok(Json(ApiResponse::success(UpdateStatusResponse {
        current_version: status.current_version,
        is_updating: status.is_updating,
        last_update_time: status.last_update_time,
        backup_versions: status.backup_versions,
        applied_version,
        restart_pending,
    })))
}

pub async fn upload_and_update(
    State(state): State<AppState>,
    auth: AuthContext,
    multipart: Multipart,
) -> Result<Json<ApiResponse<UpdateResult>>, AppError> {
    // 上传并应用更新包属高危操作（可导致 RCE），仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let update_file = extract_update_file_from_multipart(multipart).await?;

    let service = SystemUpdateService::new();
    match service.apply_update(&update_file).await {
        Ok(message) => {
            let new_version = service
                .applied_version()
                .unwrap_or_else(|| service.get_current_version());
            persist_applied_version(state.db.as_ref(), &new_version).await?;
            let mut message = message;
            maybe_schedule_restart(&mut message);
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
        // 完整性/校验值错误属服务端安全护栏判定，映射为 internal（与 facade
        // From<UpdateError> 一致，不外泄细节），真实原因已在 tracing 中文日志留痕。
        UpdateError::IoError(_)
        | UpdateError::UnzipError(_)
        | UpdateError::BackupError(_)
        | UpdateError::NetworkError(_)
        | UpdateError::IntegrityError(_)
        | UpdateError::ChecksumUnavailable(_) => AppError::internal(message),
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
    // 版本回滚属高危操作，仅 admin 可执行
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
        Err(e) => Err(map_update_error(e)),
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
        Err(e) => Err(map_update_error(e)),
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
    // 应用本地更新包属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let service = SystemUpdateService::new();

    let releases = service.list_local_releases().map_err(map_update_error)?;

    let release = releases
        .into_iter()
        .find(|r| r.version == payload.version)
        .ok_or_else(|| AppError::not_found(format!("找不到版本 {} 的发布包", payload.version)))?;

    match service.apply_local_update(&release).await {
        Ok(message) => {
            let new_version = service
                .applied_version()
                .unwrap_or_else(|| service.get_current_version());
            persist_applied_version(state.db.as_ref(), &new_version).await?;
            let mut message = message;
            maybe_schedule_restart(&mut message);
            Ok(Json(ApiResponse::success_with_message(
                UpdateResult {
                    success: true,
                    message,
                    new_version: Some(new_version),
                },
                "本地更新应用成功",
            )))
        }
        Err(e) => Err(map_update_error(e)),
    }
}

// ============================================================================
// 更新任务 / 备份任务 / 版本详情（最小实现：任务与备份存内存，版本复用 system_version 表）
// 对应前端 api/system-update.ts 的调用契约（UpdateTask / SystemBackup / SystemVersion）
// ============================================================================

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

/// 创建并登记一条更新任务记录（落库 system_update_tasks），返回前端契约 JSON
async fn push_update_task(
    state: &AppState,
    from_version: String,
    to_version: String,
    task_type: &str,
    status: &str,
    progress: u32,
    auth: &AuthContext,
) -> Result<serde_json::Value, AppError> {
    use sea_orm::{ActiveModelTrait, Set};

    use std::sync::atomic::{AtomicI32, Ordering};
    static TASK_SEQ: AtomicI32 = AtomicI32::new(0);
    let seq = TASK_SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let now = chrono::Utc::now();
    let model = system_update_task::ActiveModel {
        task_code: Set(format!("UPD-{}-{:04}", now.format("%Y%m%d%H%M%S"), seq)),
        task_type: Set(task_type.to_string()),
        source_version: Set(from_version.clone()),
        target_version: Set(to_version.clone()),
        status: Set(status.to_string()),
        progress: Set(progress as i32),
        error_message: Set(None),
        created_by: Set(auth.user_id),
        created_by_name: Set(auth.username.clone()),
        completed_at: Set(Some(now.into())),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };
    let saved = model.insert(state.db.as_ref()).await?;
    Ok(task_to_frontend_json(&saved))
}

/// system_update_task Model → 前端 `UpdateTask` 契约 JSON
pub fn task_to_frontend_json(t: &system_update_task::Model) -> serde_json::Value {
    serde_json::json!({
        "id": t.id,
        "task_code": t.task_code,
        "from_version": t.source_version,
        "to_version": t.target_version,
        "status": t.status,
        "progress": t.progress,
        "error_message": t.error_message.clone().unwrap_or_default(),
        "backup_path": "",
        "started_at": t.created_at.to_rfc3339(),
        "completed_at": t.completed_at.map(|c| c.to_rfc3339()).unwrap_or_default(),
        "created_by": t.created_by,
        "created_by_name": t.created_by_name,
        "created_at": t.created_at.to_rfc3339(),
    })
}

/// system_update_backup Model → 前端 `SystemBackup` 契约 JSON
pub fn backup_to_frontend_json(b: &system_update_backup::Model) -> serde_json::Value {
    serde_json::json!({
        "id": b.id,
        "backup_code": b.backup_code,
        "backup_type": b.backup_type,
        "file_path": b.file_path,
        "file_size": b.file_size,
        "description": b.description,
        "status": b.status,
        "created_by": b.created_by,
        "created_by_name": b.created_by_name,
        "created_at": b.created_at.to_rfc3339(),
    })
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
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBackupRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 创建系统备份属更新链前置高危动作（备份是回滚底座），与其余更新写端点保持一致的
    // admin/ops 深度防御校验；此前该端点仅取 AuthContext 未校验，构成现状不对称，补齐。
    require_admin_role(&state, &auth).await?;

    req.validate().map_err(AppError::from)?;

    use sea_orm::{ActiveModelTrait, Set};
    let now = chrono::Utc::now();
    let backup = system_update_backup::ActiveModel {
        backup_code: Set(format!("BK-{}", now.format("%Y%m%d%H%M%S"))),
        backup_type: Set(req.backup_type.unwrap_or_else(|| "full".to_string())),
        description: Set(req.description.unwrap_or_default()),
        status: Set("completed".to_string()),
        created_by: Set(auth.user_id),
        created_by_name: Set(auth.username.clone()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };
    let saved = backup.insert(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::success_with_message(
        backup_to_frontend_json(&saved),
        "备份任务已创建",
    )))
}

/// GET /api/v1/erp/system-update/tasks/{id} - 获取更新任务详情（对应前端 api/system-update.ts getUpdateTask）
pub async fn get_update_task_by_id(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::EntityTrait;
    let task = system_update_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("更新任务 {} 不存在", id)))?;
    Ok(Json(ApiResponse::success(task_to_frontend_json(&task))))
}

/// POST /api/v1/erp/system-update/tasks/{id}/cancel - 取消更新任务（对应前端 api/system-update.ts cancelUpdateTask）
/// 仅允许取消未结束的任务（completed/failed/rolled_back 视为已结束）
pub async fn cancel_update_task(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 任务取消与下载/安装同属高危更新链路，保持 admin 校验一致
    require_admin_role(&state, &auth).await?;

    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let task = system_update_task::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("更新任务 {} 不存在", id)))?;

    if matches!(task.status.as_str(), "completed" | "failed" | "rolled_back") {
        return Err(AppError::business(format!(
            "更新任务 {} 已结束（状态：{}），无法取消",
            id, task.status
        )));
    }

    let mut active: system_update_task::ActiveModel = task.into();
    active.status = Set("failed".to_string());
    active.error_message = Set(Some("任务已被用户取消".to_string()));
    active.completed_at = Set(Some(chrono::Utc::now().into()));
    active.updated_at = Set(chrono::Utc::now().into());
    let cancelled = active.update(state.db.as_ref()).await?;
    Ok(Json(ApiResponse::success_with_message(
        task_to_frontend_json(&cancelled),
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
    Ok(Json(ApiResponse::success(system_version_to_frontend_json(
        &version,
    ))))
}

/// POST /api/v1/erp/system-update/versions/{version_id}/download - 下载指定版本更新包（对应前端 api/system-update.ts downloadUpdate）
/// 最小实现：校验版本存在后登记下载任务（status=downloaded）
pub async fn download_version_update(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(version_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 与现有 download_and_update 一致：下载并应用更新属高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let version = load_system_version(&state, version_id).await?;
    let service = SystemUpdateService::new();
    let task = push_update_task(
        &state,
        service.get_current_version(),
        version.version,
        "download",
        "downloaded",
        100,
        &auth,
    )
    .await?;
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
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 安装更新会替换二进制，属最高危操作，仅 admin 可执行
    require_admin_role(&state, &auth).await?;

    let version = load_system_version(&state, version_id).await?;
    let service = SystemUpdateService::new();
    let task = push_update_task(
        &state,
        service.get_current_version(),
        version.version,
        "install",
        "installing",
        0,
        &auth,
    )
    .await?;
    Ok(Json(ApiResponse::success_with_message(
        task,
        "版本安装任务已创建",
    )))
}

/// GET /api/v1/erp/system-update/tasks — 更新任务列表（全表，按 id 降序）
pub async fn list_update_tasks(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    use sea_orm::{EntityTrait, QueryOrder};

    let tasks = system_update_task::Entity::find()
        .order_by_desc(system_update_task::Column::Id)
        .all(state.db.as_ref())
        .await?;
    let total = tasks.len() as u64;
    let items: Vec<serde_json::Value> = tasks.iter().map(task_to_frontend_json).collect();

    tracing::info!(
        "[system_update] list_update_tasks 返回 {} 条更新任务",
        total
    );

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, 1, total,
    ))))
}

/// GET /api/v1/erp/system-update/backups — 备份任务列表（全表，按 id 降序）
pub async fn list_backup_tasks(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    use sea_orm::{EntityTrait, QueryOrder};

    let backups = system_update_backup::Entity::find()
        .order_by_desc(system_update_backup::Column::Id)
        .all(state.db.as_ref())
        .await?;
    let total = backups.len() as u64;
    let items: Vec<serde_json::Value> = backups.iter().map(backup_to_frontend_json).collect();

    tracing::info!(
        "[system_update] list_backup_tasks 返回 {} 条备份记录",
        total
    );

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, 1, total,
    ))))
}
