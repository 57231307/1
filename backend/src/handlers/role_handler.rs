use crate::services::role_permission_service::RolePermissionService;
use crate::services::role_permission_service::{
    AssignPermissionRequest, CreateRoleRequest, UpdateRoleRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<RoleListResponse>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    match service.list_roles().await {
        Ok(roles) => {
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

            Ok(Json(RoleListResponse {
                roles: role_responses,
                total,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取角色详情
pub async fn get_role(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<RoleDetailResponse>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    match service.get_role_detail(id).await {
        Ok(role) => {
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

            Ok(Json(RoleDetailResponse {
                id: role.id,
                name: role.name,
                code: role.code,
                description: role.description,
                is_system: role.is_system,
                created_at: role.created_at,
                updated_at: role.updated_at,
                permissions,
            }))
        }
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

/// 创建角色
pub async fn create_role(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<RoleDetailResponse>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    let request = CreateRoleRequest {
        name: payload.name,
        code: payload.code,
        description: payload.description,
        is_system: payload.is_system,
    };

    match service.create_role(request).await {
        Ok(role) => {
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

            Ok(Json(RoleDetailResponse {
                id: role.id,
                name: role.name,
                code: role.code,
                description: role.description,
                is_system: role.is_system,
                created_at: role.created_at,
                updated_at: role.updated_at,
                permissions,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 更新角色
pub async fn update_role(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRolePayload>,
) -> Result<Json<RoleDetailResponse>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    let request = UpdateRoleRequest {
        name: payload.name,
        code: payload.code,
        description: payload.description,
        is_system: payload.is_system,
    };

    match service.update_role(id, request).await {
        Ok(role) => {
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

            Ok(Json(RoleDetailResponse {
                id: role.id,
                name: role.name,
                code: role.code,
                description: role.description,
                is_system: role.is_system,
                created_at: role.created_at,
                updated_at: role.updated_at,
                permissions,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 删除角色
pub async fn delete_role(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    match service.delete_role(id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 分配权限
pub async fn assign_permission(
    State(db): State<Arc<DatabaseConnection>>,
    Path(role_id): Path<i32>,
    Json(payload): Json<AssignPermissionPayload>,
) -> Result<Json<PermissionResponse>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    let request = AssignPermissionRequest {
        role_id,
        resource_type: payload.resource_type,
        resource_id: payload.resource_id,
        action: payload.action,
        allowed: payload.allowed,
    };

    match service.assign_permission(request).await {
        Ok(perm) => Ok(Json(PermissionResponse {
            id: perm.id,
            resource_type: perm.resource_type,
            resource_id: perm.resource_id,
            action: perm.action,
            allowed: perm.allowed,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 移除权限
pub async fn remove_permission(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    match service.remove_permission(id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取角色权限列表
pub async fn get_role_permissions(
    State(db): State<Arc<DatabaseConnection>>,
    Path(role_id): Path<i32>,
) -> Result<Json<Vec<PermissionResponse>>, (StatusCode, String)> {
    let service = RolePermissionService::new(db.clone());

    match service.get_role_permissions(role_id).await {
        Ok(permissions) => {
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

            Ok(Json(perm_responses))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
