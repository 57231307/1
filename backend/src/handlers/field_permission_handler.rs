use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::{field_permission, role};
use crate::services::field_permission_service::{
    CreateFieldPermissionRequest, FieldPermissionService, UpdateFieldPermissionRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, JoinType, QueryFilter, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};

/// 字段权限响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct FieldPermissionResponse {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: String,
    pub is_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 角色名称，来自 roles.name（LEFT JOIN role_id）
    pub role_name: Option<String>,
}

/// 列表查询内部视图行（仅用于 LEFT JOIN 富化，不对外暴露）
#[derive(Debug, FromQueryResult)]
struct FieldPermissionListRow {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: String,
    pub is_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub role_name: Option<String>,
}

/// 字段权限列表查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct FieldPermissionQuery {
    pub resource_type: Option<String>,
    pub role_id: Option<i32>,
}

/// 创建字段权限请求体
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateFieldPermissionPayload {
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: Option<String>,
}

/// 更新字段权限请求体
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateFieldPermissionPayload {
    pub can_read: Option<bool>,
    pub can_write: Option<bool>,
    pub mask_strategy: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 获取字段权限列表
pub async fn list_field_permissions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<FieldPermissionQuery>,
) -> Result<Json<ApiResponse<Vec<FieldPermissionResponse>>>, AppError> {
    let mut db_query = field_permission::Entity::find()
        .column_as(role::Column::Name, "role_name")
        .join(JoinType::LeftJoin, field_permission::Relation::Role.def());

    if let Some(rt) = query.resource_type.as_deref() {
        db_query = db_query.filter(field_permission::Column::ResourceType.eq(rt));
    }
    if let Some(rid) = query.role_id {
        db_query = db_query.filter(field_permission::Column::RoleId.eq(rid));
    }

    let rows: Vec<FieldPermissionListRow> = db_query
        .into_model::<FieldPermissionListRow>()
        .all(&*state.db)
        .await?;

    let responses: Vec<FieldPermissionResponse> = rows
        .into_iter()
        .map(|row| FieldPermissionResponse {
            id: row.id,
            role_id: row.role_id,
            resource_type: row.resource_type,
            field_name: row.field_name,
            can_read: row.can_read,
            can_write: row.can_write,
            mask_strategy: row.mask_strategy,
            is_enabled: row.is_enabled,
            created_at: row.created_at,
            updated_at: row.updated_at,
            role_name: row.role_name,
        })
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 获取字段权限详情
pub async fn get_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let perm = service.get_field_permission(id).await?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
        role_name: None,
    })))
}

/// 创建字段权限
pub async fn create_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateFieldPermissionPayload>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let request = CreateFieldPermissionRequest {
        role_id: payload.role_id,
        resource_type: payload.resource_type,
        field_name: payload.field_name,
        can_read: payload.can_read,
        can_write: payload.can_write,
        mask_strategy: payload.mask_strategy,
    };

    let perm = service.create_field_permission(request).await?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
        role_name: None,
    })))
}

/// 更新字段权限
pub async fn update_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateFieldPermissionPayload>,
) -> Result<Json<ApiResponse<FieldPermissionResponse>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    let request = UpdateFieldPermissionRequest {
        can_read: payload.can_read,
        can_write: payload.can_write,
        mask_strategy: payload.mask_strategy,
        is_enabled: payload.is_enabled,
    };

    let perm = service.update_field_permission(id, request).await?;

    Ok(Json(ApiResponse::success(FieldPermissionResponse {
        id: perm.id,
        role_id: perm.role_id,
        resource_type: perm.resource_type,
        field_name: perm.field_name,
        can_read: perm.can_read,
        can_write: perm.can_write,
        mask_strategy: perm.mask_strategy,
        is_enabled: perm.is_enabled,
        created_at: perm.created_at,
        updated_at: perm.updated_at,
        role_name: None,
    })))
}

/// 删除字段权限
pub async fn delete_field_permission(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = FieldPermissionService::new(state.db.clone());

    service.delete_field_permission(id).await?;

    Ok(Json(ApiResponse::success(())))
}
