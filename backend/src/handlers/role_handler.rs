use crate::middleware::auth_context::AuthContext;
use crate::services::role_permission_service::RolePermissionService;
use crate::services::role_permission_service::{
    AssignPermissionRequest, CreateRoleRequest, UpdateRoleRequest,
};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

/// C-1 修复：处理器内部的 admin 角色二次校验
///
/// 设计原因：`permission_middleware` 仅做资源类型级粗粒度权限（roles:create 等），
/// 拥有 `roles:read` 权限的低权限用户也能通过粗粒度校验进入处理器，造成权限提升。
/// 修复方案：所有写处理器顶部调用 `require_admin_role`，强制要求 `role.code == "admin"`。
/// 防御深度：与全局 `permission_middleware` 形成"粗粒度 + 细粒度 admin 校验"双重防线。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "该操作仅限管理员（code=admin）执行",
        ));
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
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = CreateRoleRequest {
        name: payload.name,
        code: payload.code,
        description: payload.description,
        is_system: payload.is_system,
    };

    let role = service
        .create_role(request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

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
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<ApiResponse<RoleDetailResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    let request = UpdateRoleRequest {
        name: payload.name,
        code: payload.code,
        description: payload.description,
        is_system: payload.is_system,
    };

    let role = service
        .update_role(id, request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

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
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    service
        .delete_role(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

/// 分配权限
pub async fn assign_permission(
    State(state): State<AppState>,
    auth: AuthContext,
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
        .assign_permission(request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 记录权限变更日志
    tracing::warn!(
        target: "permission_audit",
        "[权限分配] 操作人: {}({}) | 角色ID: {} | 资源类型: {} | 操作: {} | 允许: {}",
        auth.username,
        auth.user_id,
        role_id,
        payload.resource_type,
        payload.action,
        payload.allowed
    );

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
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = RolePermissionService::new(state.db.clone());

    service
        .remove_permission(id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 记录权限移除日志
    tracing::warn!(
        target: "permission_audit",
        "[权限移除] 操作人: {}({}) | 权限ID: {}",
        auth.username,
        auth.user_id,
        id
    );

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
pub async fn list_permissions(
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<PermissionResponse>>>, AppError> {
    // 返回预定义的权限列表
    let permissions = vec![
        PermissionResponse {
            id: 1,
            resource_type: "sales_order".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 2,
            resource_type: "sales_order".to_string(),
            resource_id: None,
            action: "create".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 3,
            resource_type: "sales_order".to_string(),
            resource_id: None,
            action: "edit".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 4,
            resource_type: "sales_order".to_string(),
            resource_id: None,
            action: "delete".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 5,
            resource_type: "sales_order".to_string(),
            resource_id: None,
            action: "approve".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 6,
            resource_type: "purchase_order".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 7,
            resource_type: "purchase_order".to_string(),
            resource_id: None,
            action: "create".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 8,
            resource_type: "purchase_order".to_string(),
            resource_id: None,
            action: "edit".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 9,
            resource_type: "purchase_order".to_string(),
            resource_id: None,
            action: "delete".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 10,
            resource_type: "purchase_order".to_string(),
            resource_id: None,
            action: "approve".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 11,
            resource_type: "inventory".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 12,
            resource_type: "inventory".to_string(),
            resource_id: None,
            action: "adjust".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 13,
            resource_type: "inventory".to_string(),
            resource_id: None,
            action: "transfer".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 14,
            resource_type: "customer".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 15,
            resource_type: "customer".to_string(),
            resource_id: None,
            action: "create".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 16,
            resource_type: "customer".to_string(),
            resource_id: None,
            action: "edit".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 17,
            resource_type: "supplier".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 18,
            resource_type: "supplier".to_string(),
            resource_id: None,
            action: "create".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 19,
            resource_type: "supplier".to_string(),
            resource_id: None,
            action: "edit".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 20,
            resource_type: "finance".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 21,
            resource_type: "finance".to_string(),
            resource_id: None,
            action: "create".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 22,
            resource_type: "report".to_string(),
            resource_id: None,
            action: "view".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 23,
            resource_type: "system".to_string(),
            resource_id: None,
            action: "settings".to_string(),
            allowed: true,
        },
        PermissionResponse {
            id: 24,
            resource_type: "user".to_string(),
            resource_id: None,
            action: "manage".to_string(),
            allowed: true,
        },
    ];

    Ok(Json(ApiResponse::success(permissions)))
}
