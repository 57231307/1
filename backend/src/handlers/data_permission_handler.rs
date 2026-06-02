//! 数据权限 Handler
//!
//! 数据权限 HTTP 接口层，提供数据权限管理功能

use crate::services::data_permission_service::DataPermissionService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 设置数据权限请求
#[derive(Debug, Deserialize)]
pub struct SetDataPermissionRequest {
    pub role_id: i32,
    pub resource_type: String,
    pub scope_type: String,
    pub custom_condition: Option<Value>,
    pub allowed_fields: Option<Value>,
    pub hidden_fields: Option<Value>,
}

/// 数据权限响应
#[derive(Debug, Serialize)]
pub struct DataPermissionResponse {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub scope_type: String,
    pub custom_condition: Option<Value>,
    pub allowed_fields: Option<Value>,
    pub hidden_fields: Option<Value>,
    pub is_enabled: bool,
}

impl From<crate::models::data_permission::Model> for DataPermissionResponse {
    fn from(model: crate::models::data_permission::Model) -> Self {
        Self {
            id: model.id,
            role_id: model.role_id,
            resource_type: model.resource_type,
            scope_type: model.scope_type,
            custom_condition: model.custom_condition,
            allowed_fields: model.allowed_fields,
            hidden_fields: model.hidden_fields,
            is_enabled: model.is_enabled,
        }
    }
}

/// 设置数据权限
pub async fn set_data_permission(
    State(state): State<AppState>,
    Json(req): Json<SetDataPermissionRequest>,
) -> Result<Json<ApiResponse<DataPermissionResponse>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());

    let valid_scope_types = ["ALL", "DEPT", "DEPT_AND_BELOW", "SELF", "CUSTOM"];
    if !valid_scope_types.contains(&req.scope_type.as_str()) {
        return Err(AppError::ValidationError("无效的数据范围类型".to_string()));
    }

    let permission = service
        .set_data_permission(
            req.role_id,
            req.resource_type,
            req.scope_type,
            req.custom_condition,
            req.allowed_fields,
            req.hidden_fields,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        DataPermissionResponse::from(permission),
        "数据权限设置成功",
    )))
}

/// 获取角色的数据权限列表
pub async fn list_role_data_permissions(
    Path(role_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DataPermissionResponse>>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());
    let permissions = service.list_role_data_permissions(role_id).await?;

    let responses: Vec<DataPermissionResponse> = permissions
        .into_iter()
        .map(DataPermissionResponse::from)
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 删除数据权限
pub async fn delete_data_permission(
    Path((role_id, resource_type)): Path<(i32, String)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());
    service
        .delete_data_permission(role_id, &resource_type)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "数据权限删除成功",
    )))
}

/// 获取数据权限详情
pub async fn get_data_permission(
    Path((role_id, resource_type)): Path<(i32, String)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Option<DataPermissionResponse>>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());
    let permission = service
        .get_role_data_permission(role_id, &resource_type)
        .await?;

    Ok(Json(ApiResponse::success(permission.map(|p| {
        DataPermissionResponse {
            id: 0,
            role_id,
            resource_type: resource_type.clone(),
            scope_type: p.scope_type,
            custom_condition: p.custom_condition,
            allowed_fields: p.allowed_fields.map(|fields| {
                serde_json::Value::Array(
                    fields.into_iter().map(serde_json::Value::String).collect(),
                )
            }),
            hidden_fields: p.hidden_fields.map(|fields| {
                serde_json::Value::Array(
                    fields.into_iter().map(serde_json::Value::String).collect(),
                )
            }),
            is_enabled: true,
        }
    }))))
}

/// 数据范围类型列表
pub async fn list_scope_types(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let types = vec![
        serde_json::json!({
            "value": "ALL",
            "label": "全部数据",
            "description": "可以查看所有数据"
        }),
        serde_json::json!({
            "value": "DEPT",
            "label": "本部门数据",
            "description": "只能查看本部门的数据"
        }),
        serde_json::json!({
            "value": "DEPT_AND_BELOW",
            "label": "本部门及以下数据",
            "description": "可以查看本部门及下级部门的数据"
        }),
        serde_json::json!({
            "value": "SELF",
            "label": "仅本人数据",
            "description": "只能查看自己创建的数据"
        }),
        serde_json::json!({
            "value": "CUSTOM",
            "label": "自定义数据范围",
            "description": "通过自定义条件过滤数据"
        }),
    ];

    Ok(Json(ApiResponse::success(types)))
}

/// 获取所有数据权限列表
pub async fn list_data_permissions(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DataPermissionResponse>>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());
    let permissions = service.list_all_data_permissions().await?;

    let responses: Vec<DataPermissionResponse> = permissions
        .into_iter()
        .map(DataPermissionResponse::from)
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}
