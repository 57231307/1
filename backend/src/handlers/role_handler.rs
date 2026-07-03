use crate::middleware::audit_context::AuditContext;
use crate::middleware::auth_context::AuthContext;
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::role_permission_service::RolePermissionService;
use crate::services::role_permission_service::{
    AssignPermissionRequest, CreateRoleRequest, UpdateRoleRequest,
};
use crate::utils::admin_checker::{is_admin_role, ADMIN_ROLE_CODE};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// C-1 修复：处理器内部的 admin 角色二次校验
///
/// 设计原因：`permission_middleware` 仅做资源类型级粗粒度权限（roles:create 等），
/// 拥有 `roles:read` 权限的低权限用户也能通过粗粒度校验进入处理器，造成权限提升。
/// 修复方案：所有写处理器顶部调用 `require_admin_role`，强制要求 `role.code == ADMIN_ROLE_CODE`。
/// 防御深度：与全局 `permission_middleware` 形成"粗粒度 + 细粒度 admin 校验"双重防线。
/// 批次 24 v6 P1-1 修复：错误提示与注释改用 ADMIN_ROLE_CODE 常量，避免硬编码 "admin"。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(format!(
            "该操作仅限管理员（code={}）执行",
            ADMIN_ROLE_CODE
        )));
    }
    Ok(())
}

/// 角色响应
#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 角色详情响应（包含权限列表）
#[derive(Debug, Serialize)]
pub struct RoleDetailResponse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub permissions: Option<Vec<PermissionResponse>>,
}

/// 权限响应
#[derive(Debug, Serialize)]
pub struct PermissionResponse {
    pub id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 角色列表响应
#[derive(Debug, Serialize)]
pub struct RoleListResponse {
    pub roles: Vec<RoleResponse>,
    pub total: u64,
}

/// 创建角色请求
#[derive(Debug, Deserialize)]
pub struct CreateRolePayload {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 更新角色请求
#[derive(Debug, Deserialize)]
pub struct UpdateRolePayload {
    pub name: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 分配权限请求
#[derive(Debug, Deserialize)]
pub struct AssignPermissionPayload {
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 获取角色列表
pub async fn list_roles(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<RoleListResponse>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    let roles = service
        .list_roles()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let role_responses: Vec<RoleResponse> = roles
        .into_iter()
        .map(|role| RoleResponse {
            id: role.id,
            name: role.name,
            code: role.code,
            description: role.description,
            is_system: role.is_system,
            created_at: role.created_at,
            updated_at: role.updated_at,
        })
        .collect();

    // v9 P1-C 修复：当前接口全量返回无分页，total=len 正确。
    // 注意：若后续添加分页，需改为 COUNT(*) 查询，否则 total 退化为当前页条数。
    let total = role_responses.len() as u64;

    Ok(Json(ApiResponse::success(RoleListResponse {
        roles: role_responses,
        total,
    })))
}

/// 获取角色详情
pub async fn get_role(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    let role = service
        .get_role_detail(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;

    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    Ok(Json(ApiResponse::success(RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    })))
}

/// 创建角色
pub async fn create_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = CreateRoleRequest {
        name: payload.name.clone(),
        code: payload.code.clone(),
        description: payload.description.clone(),
        is_system: payload.is_system,
    };

    let role = service
        .create_role(request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-3 修复：create_role 补审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Create,
        severity: Severity::Info,
        resource_type: Some("role".to_string()),
        resource_id: Some(role.id.to_string()),
        resource_name: Some(role.name.clone()),
        description: Some(format!(
            "管理员 {} 创建角色 {}（code={}）",
            auth.username, role.name, role.code
        )),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/roles".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "role_id": role.id,
            "name": role.name,
            "code": role.code,
            "description": role.description,
            "is_system": role.is_system,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    Ok(Json(ApiResponse::success(RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    })))
}

/// 更新角色
pub async fn update_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    // P1 8-3 修复：更新前查询旧角色信息作为 before_snapshot
    let old_role = service
        .get_role_detail(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    let before_snapshot = serde_json::json!({
        "role_id": old_role.id,
        "name": old_role.name,
        "code": old_role.code,
        "description": old_role.description,
        "is_system": old_role.is_system,
    });

    let request = UpdateRoleRequest {
        name: payload.name.clone(),
        code: payload.code.clone(),
        description: payload.description.clone(),
        is_system: payload.is_system,
    };

    let role = service
        .update_role(id, request, auth.user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-3 修复：update_role 补审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Info,
        resource_type: Some("role".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(role.name.clone()),
        description: Some(format!(
            "管理员 {} 更新角色 {}（id={}）",
            auth.username, role.name, id
        )),
        request_method: Some("PUT".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}", id)),
        before_snapshot: Some(before_snapshot),
        after_snapshot: Some(serde_json::json!({
            "role_id": role.id,
            "name": role.name,
            "code": role.code,
            "description": role.description,
            "is_system": role.is_system,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    let permissions = role.permission_list.map(|perms| {
        perms
            .into_iter()
            .map(|perm| PermissionResponse {
                id: perm.id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
            })
            .collect()
    });

    Ok(Json(ApiResponse::success(RoleDetailResponse {
        id: role.id,
        name: role.name,
        code: role.code,
        description: role.description,
        is_system: role.is_system,
        created_at: role.created_at,
        updated_at: role.updated_at,
        permissions,
    })))
}

/// 删除角色
pub async fn delete_role(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    // P1 8-3 修复：删除前查询旧角色信息作为 before_snapshot
    let old_role = service
        .get_role_detail(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 2-3 修复（批次 64）：系统内置角色禁止删除
    // 原实现仅 require_admin_role，未检查 is_system 字段，
    // admin 角色被删除后系统永久锁定（无管理员可操作）。
    if old_role.is_system {
        return Err(AppError::bad_request(
            "系统内置角色不可删除",
        ));
    }

    let before_snapshot = serde_json::json!({
        "role_id": old_role.id,
        "name": old_role.name,
        "code": old_role.code,
        "description": old_role.description,
        "is_system": old_role.is_system,
    });

    service
        .delete_role(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-3 修复：delete_role 补审计日志
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Delete,
        severity: Severity::Warn,
        resource_type: Some("role".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: Some(old_role.name.clone()),
        description: Some(format!(
            "管理员 {} 删除角色 {}（id={}）",
            auth.username, old_role.name, id
        )),
        request_method: Some("DELETE".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}", id)),
        before_snapshot: Some(before_snapshot),
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(())))
}

/// 分配权限
pub async fn assign_permission(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(role_id): Path<i32>,
    Json(payload): Json<AssignPermissionPayload>,
) -> Result<Json<ApiResponse<PermissionResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = AssignPermissionRequest {
        role_id,
        resource_type: payload.resource_type.clone(),
        resource_id: payload.resource_id,
        action: payload.action.clone(),
        allowed: payload.allowed,
    };

    let perm = service
        .assign_permission(request, auth.user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-3 修复：assign_permission 改用 record_async 落库审计日志（原仅 tracing::warn）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Update,
        severity: Severity::Warn,
        resource_type: Some("role_permission".to_string()),
        resource_id: Some(perm.id.to_string()),
        resource_name: Some(format!(
            "role_id={}/resource_type={}/action={}",
            role_id, payload.resource_type, payload.action
        )),
        description: Some(format!(
            "管理员 {} 为角色 id={} 分配权限：resource_type={} action={} allowed={}",
            auth.username, role_id, payload.resource_type, payload.action, payload.allowed
        )),
        request_method: Some("POST".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/{}/permissions", role_id)),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "permission_id": perm.id,
            "role_id": role_id,
            "resource_type": payload.resource_type,
            "resource_id": payload.resource_id,
            "action": payload.action,
            "allowed": payload.allowed,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(PermissionResponse {
        id: perm.id,
        resource_type: perm.resource_type,
        resource_id: perm.resource_id,
        action: perm.action,
        allowed: perm.allowed,
    })))
}

/// 移除权限
pub async fn remove_permission(
    State(state): State<AppState>,
    auth: AuthContext,
    audit_ctx: Option<Extension<AuditContext>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    service
        .remove_permission(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // P1 8-3 修复：remove_permission 改用 record_async 落库审计日志（原仅 tracing::warn）
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Delete,
        severity: Severity::Warn,
        resource_type: Some("role_permission".to_string()),
        resource_id: Some(id.to_string()),
        resource_name: None,
        description: Some(format!(
            "管理员 {} 移除权限 id={}",
            auth.username, id
        )),
        request_method: Some("DELETE".to_string()),
        request_path: Some(format!("/api/v1/erp/roles/permissions/{}", id)),
        before_snapshot: Some(serde_json::json!({
            "permission_id": id,
            "action": "remove",
        })),
        after_snapshot: None,
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, audit_ctx.map(|e| e.0));

    Ok(Json(ApiResponse::success(())))
}

/// 获取角色权限列表
pub async fn get_role_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(role_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    let service = RolePermissionService::new(state.db.clone());

    let permissions = service
        .get_role_permissions(role_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    let perm_responses: Vec<PermissionResponse> = permissions
        .into_iter()
        .map(|perm| PermissionResponse {
            id: perm.id,
            resource_type: perm.resource_type,
            resource_id: perm.resource_id,
            action: perm.action,
            allowed: perm.allowed,
        })
        .collect();

    Ok(Json(ApiResponse::success(perm_responses)))
}

/// 获取所有权限列表（用于前端权限选择器）
///
/// P2 2-8 修复：从 role_permission 表 distinct 查询权限列表，
/// 替代原 24 项硬编码权限，确保与数据库实际配置、middleware 动态提取保持一致。
pub async fn list_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let all_perms = crate::models::role_permission::Entity::find()
        .filter(crate::models::role_permission::Column::Allowed.eq(true))
        .all(state.db.as_ref())
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 按 (resource_type, action) 去重，resource_id 取首条记录值
    let mut seen = std::collections::HashSet::new();
    let mut permissions = Vec::new();
    let mut id_counter = 1i32;
    for perm in all_perms {
        let key = (perm.resource_type.clone(), perm.action.clone());
        if seen.insert(key) {
            permissions.push(PermissionResponse {
                id: id_counter,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: true,
            });
            id_counter += 1;
        }
    }

    Ok(Json(ApiResponse::success(permissions)))
}
