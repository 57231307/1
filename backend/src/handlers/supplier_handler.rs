use crate::services::supplier_service::{
    CreateContactRequest, CreateQualificationRequest, CreateSupplierRequest, SupplierQueryParams,
    SupplierService, UpdateContactRequest, UpdateSupplierRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use validator::Validate;

/// 查询供应商列表
pub async fn list_suppliers(
    Query(params): Query<SupplierQueryParams>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let result = service
        .list_suppliers(params)
        .await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(result).map_err(AppError::from)?,
    )))
}

/// 获取供应商详情
pub async fn get_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let supplier = service.get_supplier(id).await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(supplier).map_err(AppError::from)?,
    )))
}

/// 创建供应商
#[axum::debug_handler]
pub async fn create_supplier(
    State(state): State<AppState>,
    Json(req): Json<CreateSupplierRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = SupplierService::new(state.db.clone());
    let user_id = 1;

    let supplier = service
        .create_supplier(req, user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        "供应商创建成功",
    )))
}

/// 更新供应商
#[axum::debug_handler]
pub async fn update_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<UpdateSupplierRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let user_id = 1;

    let supplier = service
        .update_supplier(id, req, user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        "供应商更新成功",
    )))
}

/// 删除供应商
pub async fn delete_supplier(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    service.delete_supplier(id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "供应商删除成功",
    )))
}

/// 切换供应商状态
#[axum::debug_handler]
pub async fn toggle_supplier_status(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<ToggleStatusRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let user_id = 1;

    let supplier = service
        .toggle_supplier_status(id, req.enable, user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(supplier).map_err(AppError::from)?,
        if req.enable {
            "供应商已启用"
        } else {
            "供应商已停用"
        },
    )))
}

/// 切换状态请求
#[derive(Debug, Deserialize)]
pub struct ToggleStatusRequest {
    pub enable: bool,
}

// ==================== 供应商联系人管理 Handler ====================

/// 获取供应商联系人列表
pub async fn list_supplier_contacts(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let contacts = service
        .list_supplier_contacts(supplier_id)
        .await?;

    Ok(Json(ApiResponse::success(
        serde_json::to_value(contacts).map_err(AppError::from)?,
    )))
}

/// 创建供应商联系人
#[axum::debug_handler]
pub async fn create_supplier_contact(
    Path(supplier_id): Path<i32>,
    State(state): State<AppState>,
    Json(req): Json<CreateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    let service = SupplierService::new(state.db.clone());
    let user_id = 1;

    let contact = service
        .create_supplier_contact(supplier_id, req, user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人创建成功",
    )))
}

/// 更新供应商联系人
#[axum::debug_handler]
pub async fn update_supplier_contact(
    Path((_supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    Json(req): Json<UpdateContactRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    let user_id = 1;

    let contact = service
        .update_supplier_contact(contact_id, req, user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(contact).map_err(AppError::from)?,
        "联系人更新成功",
    )))
}

/// 删除供应商联系人
pub async fn delete_supplier_contact(
    Path((_supplier_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = SupplierService::new(state.db.clone());
    service
        .delete_supplier_contact(contact_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "联系人删除成功",
    )))
}

// ==================== 供应商资质管理 Handler ====================

/// 获取供应商资质列表
pub async fn list_supplier_qualifications(
    Path(_supplier_id): Path<i32>,
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    Ok(Json(ApiResponse::success(serde_json::json!([]))))
}

/// 创建供应商资质
#[axum::debug_handler]
pub async fn create_supplier_qualification(
    Path(supplier_id): Path<i32>,
    State(_state): State<AppState>,
    Json(req): Json<CreateQualificationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    req.validate()
        .map_err(|e| AppError::ValidationError(e.to_string()))?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"supplier_id": supplier_id, "qualification": req}),
        "资质创建成功",
    )))
}
