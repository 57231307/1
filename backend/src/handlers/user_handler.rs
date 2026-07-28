use crate::container::AppState;
use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::user;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::auth_service::{self, AuthService};
use crate::services::role_permission_service::RolePermissionService;
use crate::services::user_service::UserService;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::audit::{self, SecurityEvent};
use crate::utils::error::AppError;
use crate::utils::password_validator::validate_password;
// 批次 103 P0-3 修复：接入 PasswordPolicyService 的 is_common_password / contains_username_fragment / strength_feedback_zh
// 批次 158 v11 真实接入：build_password_blacklist 用于批量黑名单校验
use crate::services::auth::password_policy_service::{
    build_password_blacklist, contains_username_fragment, is_common_password, strength_feedback_zh,
};
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::{Validate, ValidationError};

/// H-1 修复：用户管理 admin 校验 + 限制非 admin 修改 role_id
/// 安全原因：防止低权限用户通过 create_user/update_user 指定 role_id 提权
async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "用户管理仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let mut result = validate_password(password);
    // 批次 103 P0-3 修复：接入 PasswordPolicyService 的常见密码黑名单检查
    if is_common_password(password) {
        result.is_valid = false;
        result
            .errors
            .push("密码不能是常见弱密码（如 password/123456/qwerty 等）".to_string());
    }
    // 批次 158 v11 真实接入：build_password_blacklist 批量黑名单校验
    // 与 is_common_password 互补：is_common_password 用子串匹配（宽松），
    // build_password_blacklist 用精确匹配（严格），双重防护
    let blacklist = build_password_blacklist();
    if blacklist.contains(password) {
        result.is_valid = false;
        result
            .errors
            .push("密码在系统黑名单中，请更换为更复杂的密码".to_string());
    }
    if result.is_valid {
        Ok(())
    } else {
        // 批次 103 P0-3 修复：使用 strength_feedback_zh 生成中文反馈
        let msg = strength_feedback_zh(&result);
        let mut err = ValidationError::new("password_strength");
        err.message = Some(std::borrow::Cow::Owned(msg));
        Err(err)
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 50, message = "用户名长度必须在3-50之间"))]
    pub username: String,
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 1, message = "电话号码不能为空"))]
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    #[validate(length(min = 1, message = "电话号码不能为空"))]
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role_id: Option<i32>,
    pub department_id: Option<i32>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<user::Model> for UserResponse {
    fn from(user: user::Model) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            phone: user.phone,
            role_id: user.role_id,
            department_id: user.department_id,
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct DeleteUserResponse {
    pub success: bool,
}

pub async fn get_user(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    // 安全漏洞 #3 修复：非 admin 角色只能查自己
    // 缺角色时直接拒绝（避免 role_id=0 误匹配"超级管理员"角色）
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await && auth.user_id != id {
        // 记录鉴权失败审计日志（best-effort，无 audit_ctx 时传 None）
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some(&format!("target_user_id={}", id)),
            Some("非 admin 越权查询其他用户信息"),
            None,
        )
        .await;
        return Err(AppError::permission_denied("仅管理员可查询其他用户信息"));
    }

    let user_service = UserService::new(state.db.clone());

    let user = user_service.find_by_id(id).await?;
    Ok(Json(ApiResponse::success(user.into())))
}

/// 获取当前登录用户个人信息
pub async fn get_current_user_profile(
    auth: AuthContext,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let user_service = UserService::new(state.db.clone());
    let user = user_service.find_by_id(auth.user_id).await?;
    Ok(Json(ApiResponse::success(user.into())))
}

pub async fn create_user(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    payload.validate()?;

    let user_service = UserService::new(state.db.clone());

    // v14 P0-1 修复：使用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker
    let password_hash = AuthService::hash_password_async(payload.password.clone())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let user = user_service
        .create_user(
            payload.username.clone(),
            password_hash,
            payload.email.clone(),
            payload.phone.clone(),
            payload.role_id,
            payload.department_id,
        )
        .await?;

    // P1 8-2 修复：create_user 补审计日志（operation=Create，after_snapshot）
    // 修复背景：原 create_user 完全无审计日志，无法追溯谁创建了用户。
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Create,
        severity: Severity::Info,
        resource_type: Some("user".to_string()),
        resource_id: Some(user.id.to_string()),
        resource_name: Some(payload.username.clone()),
        description: Some(format!(
            "管理员 {} 创建用户 {}（user_id={}）",
            auth.username, payload.username, user.id
        )),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/users".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "user_id": user.id,
            "username": payload.username,
            "email": payload.email,
            "phone": payload.phone,
            "role_id": payload.role_id,
            "department_id": payload.department_id,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(user.into())))
}

pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListUsersParams>,
) -> Result<Json<ApiResponse<UserListResponse>>, AppError> {
    // 安全漏洞 #3 修复：仅 admin 角色可列出所有用户（防止用户枚举攻击）
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        // 记录鉴权失败审计日志（best-effort，无 audit_ctx 时传 None）
        audit::log_security_event(
            SecurityEvent::AuthorizationDenied,
            auth.user_id,
            &auth.username,
            auth.role_id,
            Some("list_users"),
            Some("非 admin 越权调用用户列表"),
            None,
        )
        .await;
        return Err(AppError::permission_denied("列出用户列表仅限管理员"));
    }

    let user_service = UserService::new(state.db.clone());

    let (users, total) = user_service
        .list_users(
            params.page.unwrap_or(1).clamp(1, 1000),
            params.page_size.unwrap_or(20).clamp(1, 100),
        )
        .await?;

    let user_responses: Vec<UserResponse> = users.into_iter().map(|user| user.into()).collect();

    Ok(Json(ApiResponse::success(UserListResponse {
        users: user_responses,
        total,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(20).clamp(1, 100),
    })))
}

#[derive(Debug, Deserialize)]
pub struct ListUsersParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

use axum::extract::Query;

/// 更新用户信息
pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    req.validate()?;

    // H-1 修复：禁止通过 update_user 提权到 admin 角色（调用者非 admin 时禁止改 admin）
    if let Some(new_role_id) = req.role_id {
        if is_admin_role(&state.db, new_role_id).await
            && !is_admin_role(&state.db, auth.role_id.unwrap_or(-1)).await
        {
            return Err(AppError::permission_denied("禁止将用户角色改为 admin 角色"));
        }
    }

    let user_service = UserService::new(state.db.clone());

    // P1 8-1 修复：更新前查询旧用户信息作为 before_snapshot
    let old_user = user_service.find_by_id(id).await?;
    let before_snapshot = build_user_before_snapshot(&old_user);

    // V15 P1 12.8：会话固定攻击防护 — 检测 role_id 变更
    // 安全原因：用户角色变更后，旧 session 仍可能携带 5 分钟权限缓存使用旧权限。
    // 修复：role_id 变更后立即吊销该用户所有 JWT + 清除 CSRF Token，强制重新登录。
    let role_changed = req.role_id.is_some() && req.role_id != old_user.role_id;

    let user = user_service
        .update_user(
            id,
            req.email.clone(),
            req.phone.clone(),
            req.role_id,
            req.department_id,
            req.status,
            auth.user_id,
        )
        .await?;

    // V15 P1 12.8：role_id 变更后吊销目标用户所有旧 session（best-effort，失败仅 warn）
    if role_changed {
        // 1. 吊销该用户所有历史 JWT（任何 iat < 当前时间戳的 Token 将被拒绝）
        if let Err(e) = auth_service::revoke_user_jtis(id, "USER_ROLE_CHANGED").await {
            tracing::warn!(
                user_id = id,
                error = %e,
                "会话固定防护：吊销用户 JWT 失败（best-effort，不阻塞业务）"
            );
        } else {
            tracing::info!(
                user_id = id,
                operator = auth.user_id,
                "会话固定防护：用户角色变更，已吊销所有旧 JWT（强制重新登录）"
            );
        }
        // 2. 清除旧 CSRF Token（强制下次请求重新生成）
        let rotated = state.cache.clear_old_csrf_token_for_user(id);
        if rotated {
            tracing::info!(
                user_id = id,
                "会话固定防护：已清除用户旧 CSRF Token（角色变更后强制轮换）"
            );
        }
        // 3. 失效该用户的角色权限缓存（避免 5 分钟 TTL 内使用旧权限放行）
        crate::middleware::permission::invalidate_permission_cache(old_user.role_id.unwrap_or(0));
        if let Some(new_role_id) = req.role_id {
            crate::middleware::permission::invalidate_permission_cache(new_role_id);
        }
    }

    // P1 8-1 修复：update_user 补审计日志（operation=Update，before/after_snapshot）
    let event = build_update_audit_event(&auth, id, &before_snapshot, &user);
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(user.into())))
}

/// 构造用户更新前的快照
fn build_user_before_snapshot(old_user: &user::Model) -> serde_json::Value {
    serde_json::json!({
        "user_id": old_user.id,
        "username": old_user.username,
        "email": old_user.email,
        "phone": old_user.phone,
        "role_id": old_user.role_id,
        "department_id": old_user.department_id,
        "is_active": old_user.is_active,
    })
}

/// 构造用户更新审计事件
fn build_update_audit_event(
    auth: &AuthContext,
    id: i32,
    before_snapshot: &serde_json::Value,
    user: &user::Model,
) -> AuditEvent {
    AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("user".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(user.username.clone()),
        description: Some(format!(
            "管理员 {} 更新用户 {}（user_id={}）信息",
            auth.username, user.username, id
        )),
        request_method: Some("PUT".to_string()),
        request_path: Some(format!("/api/v1/erp/users/{}", id)),
        before_snapshot: Some(before_snapshot.clone()),
        after_snapshot: Some(serde_json::json!({
            "user_id": user.id,
            "username": user.username,
            "email": user.email,
            "phone": user.phone,
            "role_id": user.role_id,
            "department_id": user.department_id,
            "is_active": user.is_active,
        })),
    }
}

/// 删除用户（软删除）
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<DeleteUserResponse>>, AppError> {
    // 权限检查：非自己账户需要 user:delete 权限
    check_delete_user_permission(&state.db, auth.user_id, auth.role_id, id).await?;
    let user_service = UserService::new(state.db.clone());
    // 检查用户是否存在
    let existing_user = user_service.find_by_id(id).await?;
    // P1 2-4 修复：保护最后一个 admin 禁止删除
    if let Some(user_role_id) = existing_user.role_id {
        protect_last_admin(&state.db, user_role_id).await?;
    }
    // P1 8-7 修复：删除前捕获用户完整信息作为 before_snapshot
    let before_snapshot = build_before_snapshot(&existing_user);
    // 软删除：将 is_active 标记为 false
    // P0 7-3 修复：JWT 吊销逻辑已下沉到 UserService::delete_user 内部（单一真相源）
    user_service.delete_user(id).await?;
    // P1 8-7 修复：改用 AuditLogService::record_async 落库审计日志
    let event = build_delete_audit_event(&auth, &existing_user, id, before_snapshot);
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));
    // v11 批次 156 P2-D：双写 log_security_event（结构化告警，与 AuditLogService 落库互补）
    log_user_deleted_security_event(&auth, &existing_user, id).await;
    Ok(Json(ApiResponse::success(DeleteUserResponse {
        success: true,
    })))
}

/// 检查删除用户权限（非自己账户需要 role_id + user:delete 权限）。
async fn check_delete_user_permission(
    db: &Arc<sea_orm::DatabaseConnection>,
    auth_user_id: i32,
    auth_role_id: Option<i32>,
    target_id: i32,
) -> Result<(), AppError> {
    // 自己账户允许删除（自服务场景）
    if auth_user_id == target_id {
        return Ok(());
    }
    let role_permission_service = RolePermissionService::new(db.clone());
    // 缺角色时直接拒绝（避免 role_id=0 误匹配"超级管理员"角色）
    let role_id = auth_role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行删除操作"))?;
    let has_permission = role_permission_service
        .check_permission(role_id, "user", "delete", Some(target_id))
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    if !has_permission {
        return Err(AppError::permission_denied("没有删除用户的权限"));
    }
    Ok(())
}

/// P1 2-4 修复：保护最后一个 admin 禁止删除（删除后系统将永久锁定）。
async fn protect_last_admin(
    db: &Arc<sea_orm::DatabaseConnection>,
    user_role_id: i32,
) -> Result<(), AppError> {
    if !is_admin_role(db, user_role_id).await {
        return Ok(());
    }
    use crate::models::user;
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
    let active_admin_count = user::Entity::find()
        .filter(user::Column::RoleId.eq(user_role_id))
        .filter(user::Column::IsActive.eq(true))
        .count(db.as_ref())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    if active_admin_count <= 1 {
        return Err(AppError::bad_request(
            "系统仅剩最后一个管理员，禁止删除（删除后系统将永久锁定）",
        ));
    }
    Ok(())
}

/// P1 8-7 修复：删除前捕获用户完整信息作为 before_snapshot。
fn build_before_snapshot(user: &user::Model) -> serde_json::Value {
    serde_json::json!({
        "user_id": user.id,
        "username": user.username,
        "email": user.email,
        "phone": user.phone,
        "role_id": user.role_id,
        "department_id": user.department_id,
        "is_active": user.is_active,
        "is_totp_enabled": user.is_totp_enabled,
    })
}

/// 构建删除用户审计事件（P1 8-7 修复：落库审计日志，可追溯）。
fn build_delete_audit_event(
    auth: &AuthContext,
    existing_user: &user::Model,
    id: i32,
    before_snapshot: serde_json::Value,
) -> AuditEvent {
    AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Delete,
        severity: Severity::Warn,
        resource_type: Some("user".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(existing_user.username.clone()),
        description: Some(format!(
            "管理员 {} 软删除用户 {}（user_id={}）",
            auth.username, existing_user.username, id
        )),
        request_method: Some("DELETE".to_string()),
        request_path: Some(format!("/api/v1/erp/users/{}", id)),
        before_snapshot: Some(before_snapshot),
        after_snapshot: Some(serde_json::json!({
            "user_id": id,
            "is_active": false,
            "action": "soft_delete",
        })),
    }
}

/// v11 批次 156 P2-D：双写 log_security_event（结构化告警，与 AuditLogService 落库互补）。
async fn log_user_deleted_security_event(auth: &AuthContext, existing_user: &user::Model, id: i32) {
    audit::log_security_event(
        audit::SecurityEvent::UserDeleted,
        auth.user_id,
        &auth.username,
        auth.role_id,
        Some(&existing_user.username),
        Some(&format!("user_id={}", id)),
        None,
    )
    .await;
}

/// 修改密码请求
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "原密码不能为空"))]
    pub old_password: String,
    #[validate(custom(function = "validate_password_strength"))]
    pub new_password: String,
}

/// 修改密码响应
#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub success: bool,
    pub message: String,
}

/// 修改当前用户密码
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<ChangePasswordResponse>>, AppError> {
    req.validate()?;

    let user_service = UserService::new(state.db.clone());
    let user = user_service.find_by_id(auth.user_id).await?;

    let old_hash_fingerprint = compute_hash_fingerprint(&user.password_hash);

    let is_valid =
        AuthService::verify_password_async(req.old_password.clone(), user.password_hash.clone())
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

    if !is_valid {
        record_audit_failure(&state.db, &auth, &audit_ctx, "原密码不正确");
        return Err(AppError::unauthorized("原密码不正确"));
    }

    let is_same =
        AuthService::verify_password_async(req.new_password.clone(), user.password_hash.clone())
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

    if is_same {
        return Err(AppError::bad_request("新密码不能与原密码相同"));
    }

    if contains_username_fragment(&req.new_password, &user.username) {
        return Err(AppError::bad_request("密码不能包含用户名片段，请更换密码"));
    }

    let new_password_hash = AuthService::hash_password_async(req.new_password.clone())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let policy_svc = crate::services::auth::password_policy_service::PasswordPolicyService::new();
    validate_password_history(
        &policy_svc,
        state.db.as_ref(),
        auth.user_id,
        &req.new_password,
        &new_password_hash,
    )
    .await?;

    let new_hash_fingerprint = compute_hash_fingerprint(&new_password_hash);

    update_password_and_revoke(state.db.as_ref(), &user, &new_password_hash, auth.user_id).await?;
    save_password_history(
        &policy_svc,
        state.db.as_ref(),
        auth.user_id,
        &user.password_hash,
        auth.user_id,
    )
    .await;

    record_audit_success(
        &state.db,
        &auth,
        &audit_ctx,
        &old_hash_fingerprint,
        &new_hash_fingerprint,
    );

    Ok(Json(ApiResponse::success_with_message(
        ChangePasswordResponse {
            success: true,
            message: "密码修改成功".to_string(),
        },
        "密码修改成功，请使用新密码重新登录",
    )))
}

fn compute_hash_fingerprint(password_hash: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(password_hash.as_bytes());
    format!("{:x}", hasher.finalize())[..8].to_string()
}

fn record_audit_failure(
    db: &Arc<DatabaseConnection>,
    auth: &AuthContext,
    audit_ctx: &Option<Extension<AuditContext>>,
    reason: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Warn,
        resource_type: Some("user".to_string()),
        resource_id: Some(auth.user_id.to_string()),
        resource_name: None,
        description: Some(format!("修改密码失败：{}", reason)),
        request_method: Some("PUT".to_string()),
        request_path: Some("/api/v1/erp/users/change-password".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(db.clone()));
    svc.record_async(event, audit_ctx.as_ref().map(|e| e.0.clone()));
}

async fn validate_password_history(
    policy_svc: &crate::services::auth::password_policy_service::PasswordPolicyService,
    db: &DatabaseConnection,
    user_id: i32,
    new_password: &str,
    new_password_hash: &str,
) -> Result<(), AppError> {
    let history = policy_svc
        .load_history_from_db(db, user_id)
        .await
        .map_err(|e| AppError::internal(format!("加载密码历史失败: {}", e)))?;
    let history_result = policy_svc
        .validate_with_history(new_password, new_password_hash, &history)
        .await;
    if !history_result.is_valid {
        if let Some(history_err) = history_result.errors.iter().find(|e| e.contains("历史")) {
            return Err(AppError::bad_request(history_err.clone()));
        }
    }
    Ok(())
}

async fn update_password_and_revoke(
    db: &DatabaseConnection,
    user: &user::Model,
    new_password_hash: &str,
    auth_user_id: i32,
) -> Result<(), AppError> {
    use sea_orm::ActiveModelTrait;
    let mut user_model: crate::models::user::ActiveModel = user.clone().into();
    user_model.password_hash = sea_orm::Set(new_password_hash.to_string());
    user_model.updated_at = sea_orm::Set(chrono::Utc::now());
    user_model.password_changed_at = sea_orm::Set(Some(chrono::Utc::now()));
    user_model.update(db).await?;

    if let Err(e) = auth_service::revoke_user_jtis(auth_user_id, "PASSWORD_CHANGED").await {
        tracing::warn!(target: "security_audit", event = "TOKEN_REVOKE_FAILED", user_id = auth_user_id, error = %e, "[SECURITY] 修改密码后吊销用户 {} 的活跃 JWT 失败", auth_user_id);
    }
    Ok(())
}

async fn save_password_history(
    policy_svc: &crate::services::auth::password_policy_service::PasswordPolicyService,
    db: &DatabaseConnection,
    user_id: i32,
    old_password_hash: &str,
    auth_user_id: i32,
) {
    if let Err(e) = policy_svc
        .save_to_db(db, user_id, old_password_hash.to_string())
        .await
    {
        tracing::warn!(user_id = auth_user_id, error = %e, "[SECURITY] 密码历史持久化失败（不影响密码修改主流程）");
    }
}

fn record_audit_success(
    db: &Arc<DatabaseConnection>,
    auth: &AuthContext,
    audit_ctx: &Option<Extension<AuditContext>>,
    old_fingerprint: &str,
    new_fingerprint: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("user".to_string()),
        resource_id: Some(auth.user_id.to_string()),
        resource_name: None,
        description: Some("用户修改密码成功".to_string()),
        request_method: Some("PUT".to_string()),
        request_path: Some("/api/v1/erp/users/change-password".to_string()),
        before_snapshot: Some(
            serde_json::json!({ "action": "change_password", "hash_fingerprint": old_fingerprint }),
        ),
        after_snapshot: Some(
            serde_json::json!({ "action": "change_password", "status": "success", "hash_fingerprint": new_fingerprint }),
        ),
    };
    let svc = Arc::new(AuditLogService::new(db.clone()));
    svc.record_async(event, audit_ctx.as_ref().map(|e| e.0.clone()));
}
