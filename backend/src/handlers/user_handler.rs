use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::models::user;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::auth_service::{self, AuthService};
use crate::services::role_permission_service::RolePermissionService;
use crate::services::user_service::UserService;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::audit::{self, SecurityEvent};
use crate::utils::error::AppError;
use crate::utils::password_validator::{get_password_feedback, validate_password};
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::{Validate, ValidationError};

/// H-1 修复：用户管理 admin 校验 + 限制非 admin 修改 role_id
///
/// 安全原因：低权限用户调用 create_user 时可指定 role_id=admin_role_id
/// 提权；update_user 时可改写他人 role_id 字段。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
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
    let result = validate_password(password);
    if result.is_valid {
        Ok(())
    } else {
        let msg = get_password_feedback(&result);
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
        return Err(AppError::permission_denied(
            "仅管理员可查询其他用户信息",
        ));
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

    let password_hash = AuthService::hash_password(&payload.password)
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
            params.page.unwrap_or_default(),
            params.page_size.unwrap_or(20).clamp(1, 100),
        )
        .await?;

    let user_responses: Vec<UserResponse> = users.into_iter().map(|user| user.into()).collect();

    Ok(Json(ApiResponse::success(UserListResponse {
        users: user_responses,
        total,
        page: params.page.unwrap_or_default(),
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

    // H-1 修复：禁止通过 update_user 提权到 admin 角色
    // 即使调用者是 admin（仅 admin 可调用此处理器），仍禁止把用户改成 admin
    // 除非调用者本身就是 admin。is_admin_role 已通过 require_admin_role 验证。
    // 进一步防御：如果 req.role_id 是 admin 角色 ID 且调用者非 admin，禁止。
    if let Some(new_role_id) = req.role_id {
        if is_admin_role(&state.db, new_role_id).await && !is_admin_role(&state.db, auth.role_id.unwrap_or(-1)).await {
            return Err(AppError::permission_denied(
                "禁止将用户角色改为 admin 角色",
            ));
        }
    }

    let user_service = UserService::new(state.db.clone());

    // P1 8-1 修复：更新前查询旧用户信息作为 before_snapshot
    // 修复背景：原 update_user 完全无审计日志，role_id 变更（权限提升/降级）、
    // status 变更（启用/禁用）未审计，无法追溯用户权限变更历史。
    let old_user = user_service.find_by_id(id).await?;
    let before_snapshot = serde_json::json!({
        "user_id": old_user.id,
        "username": old_user.username,
        "email": old_user.email,
        "phone": old_user.phone,
        "role_id": old_user.role_id,
        "department_id": old_user.department_id,
        "is_active": old_user.is_active,
    });

    let user = user_service
        .update_user(
            id,
            req.email.clone(),
            req.phone.clone(),
            req.role_id,
            req.department_id,
            req.status,
        )
        .await?;

    // P1 8-1 修复：update_user 补审计日志（operation=Update，before/after_snapshot）
    let event = AuditEvent {
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
        before_snapshot: Some(before_snapshot),
        after_snapshot: Some(serde_json::json!({
            "user_id": user.id,
            "username": user.username,
            "email": user.email,
            "phone": user.phone,
            "role_id": user.role_id,
            "department_id": user.department_id,
            "is_active": user.is_active,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(user.into())))
}

/// 删除用户（软删除）
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<DeleteUserResponse>>, AppError> {
    // 检查是否是删除自己的账户
    if auth.user_id != id {
        // 非自己账户需要权限检查
        let role_permission_service = RolePermissionService::new(state.db.clone());

        // 缺角色时直接拒绝（避免 role_id=0 误匹配"超级管理员"角色）
        let role_id = auth
            .role_id
            .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行删除操作"))?;

        // 检查是否有权限删除用户
        let has_permission = role_permission_service
            .check_permission(role_id, "user", "delete", Some(id))
            .await
            .map_err(|e| AppError::internal(e.to_string()))?;

        if !has_permission {
            return Err(AppError::permission_denied("没有删除用户的权限"));
        }
    }

    let user_service = UserService::new(state.db.clone());

    // 检查用户是否存在
    let existing_user = user_service.find_by_id(id).await?;

    // 这里可以添加更多禁止删除的逻辑，例如：
    // 1. 系统管理员不允许删除
    // 2. 有特殊权限的用户不允许删除
    // 3. 正在使用中的用户不允许删除

    // P1 8-7 修复：删除前捕获用户完整信息作为 before_snapshot
    // 修复背景：原 delete_user 仅调 log_security_event（只 tracing 不落库），
    // 软删除无 before_snapshot，无法追溯被删除用户的完整信息。
    let before_snapshot = serde_json::json!({
        "user_id": existing_user.id,
        "username": existing_user.username,
        "email": existing_user.email,
        "phone": existing_user.phone,
        "role_id": existing_user.role_id,
        "department_id": existing_user.department_id,
        "is_active": existing_user.is_active,
        "is_totp_enabled": existing_user.is_totp_enabled,
    });

    // 软删除：将 is_active 标记为 false
    // P0 7-3 修复：JWT 吊销逻辑已下沉到 UserService::delete_user 内部，
    //    作为单一真相源保证任何调用方都自动获得吊销保护。
    user_service.delete_user(id).await?;

    // P1 8-7 修复：改用 AuditLogService::record_async 落库审计日志
    // 修复背景：原 log_security_event 仅 tracing 输出不写 DB，可被篡改/丢失。
    let event = AuditEvent {
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
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(DeleteUserResponse {
        success: true,
    })))
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

    // 获取当前用户信息
    let user = user_service.find_by_id(auth.user_id).await?;

    // P1 8-15 修复：捕获旧密码哈希的 SHA-256 摘要前 8 位作为审计指纹
    // 修复背景：原 before/after 仅占位 {"action":"change_password"}，无真实哈希指纹，
    // 无法检测密码是否被篡改或绕过审计修改。
    let old_hash_fingerprint = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(user.password_hash.as_bytes());
        format!("{:x}", hasher.finalize())[..8].to_string()
    };

    // 验证原密码
    let is_valid = AuthService::verify_password(&req.old_password, &user.password_hash)
        .map_err(|e| AppError::internal(e.to_string()))?;

    if !is_valid {
        // 记录审计：原密码错误
        let event = AuditEvent {
            user_id: Some(auth.user_id),
            username: Some(auth.username.clone()),
            operation_type: OperationType::Update,
            severity: Severity::Warn,
            resource_type: Some("user".to_string()),
            resource_id: Some(auth.user_id.to_string()),
            resource_name: None,
            description: Some("修改密码失败：原密码不正确".to_string()),
            request_method: Some("PUT".to_string()),
            request_path: Some("/api/v1/erp/users/change-password".to_string()),
            before_snapshot: None,
            after_snapshot: None,
        };
        let svc = Arc::new(AuditLogService::new(state.db.clone()));
        svc.record_async(event, audit_ctx.map(|e| e.0));
        return Err(AppError::unauthorized("原密码不正确"));
    }

    // 检查新密码不能与原密码相同
    let is_same = AuthService::verify_password(&req.new_password, &user.password_hash)
        .map_err(|e| AppError::internal(e.to_string()))?;

    if is_same {
        return Err(AppError::bad_request("新密码不能与原密码相同"));
    }

    // 哈希新密码
    let new_password_hash = AuthService::hash_password(&req.new_password)
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-15 修复：计算新密码哈希的 SHA-256 摘要前 8 位
    let new_hash_fingerprint = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(new_password_hash.as_bytes());
        format!("{:x}", hasher.finalize())[..8].to_string()
    };

    // 更新密码
    use sea_orm::ActiveModelTrait;
    let mut user_model: crate::models::user::ActiveModel = user.into();
    user_model.password_hash = sea_orm::Set(new_password_hash);
    user_model.updated_at = sea_orm::Set(chrono::Utc::now());

    user_model.update(state.db.as_ref()).await?;

    // P0 7-4 修复：密码修改成功后吊销该用户的所有活跃 JWT
    //    防止攻击者持有的旧 Token 在剩余有效期（最长 2 小时）内继续访问。
    //    auth_middleware 会拒绝该用户 iat < revoked_at 的 Token，
    //    迫使用户使用新密码重新登录获取新 Token。
    //    吊销属 best-effort，失败仅 warn，不阻塞密码修改主流程。
    if let Err(e) = auth_service::revoke_user_jtis(auth.user_id, "PASSWORD_CHANGED").await {
        tracing::warn!(
            target: "security_audit",
            event = "TOKEN_REVOKE_FAILED",
            user_id = auth.user_id,
            username = %auth.username,
            error = %e,
            "[SECURITY] 修改密码后吊销用户 {} 的活跃 JWT 失败",
            auth.user_id
        );
    }

    // 记录审计：密码修改成功
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
        // P1 8-15 修复：before/after 记录 password_hash 的 SHA-256 摘要前 8 位
        // 不记录完整哈希避免泄露，仅记录指纹用于检测密码是否被篡改
        before_snapshot: Some(serde_json::json!({
            "action": "change_password",
            "hash_fingerprint": old_hash_fingerprint,
        })),
        after_snapshot: Some(serde_json::json!({
            "action": "change_password",
            "status": "success",
            "hash_fingerprint": new_hash_fingerprint,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success_with_message(
        ChangePasswordResponse {
            success: true,
            message: "密码修改成功".to_string(),
        },
        "密码修改成功，请使用新密码重新登录",
    )))
}
