//! 采购质检 Handler
//! 
//! 采购质检 HTTP 接口层

use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::services::purchase_inspection_service::{
    PurchaseInspectionService, CreatePurchaseInspectionRequest, UpdatePurchaseInspectionRequest,
    CompleteInspectionRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use validator::Validate;
use serde::Deserialize;

/// 查询采购质检单列表
pub async fn list_inspections(
    Query(params): Query<InspectionQueryParams>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(db);
    let (inspections, total) = service.list_inspections(
        params.page.unwrap_or(1),
        params.page_size.unwrap_or(20),
        params.status,
        params.supplier_id,
    ).await?;
    
    let result = serde_json::json!({
        "items": inspections,
        "total": total,
        "page": params.page.unwrap_or(1),
        "page_size": params.page_size.unwrap_or(20),
    });
    
    Ok(Json(ApiResponse::success(result)))
}

/// 获取采购质检单详情
pub async fn get_inspection(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(db);
    let inspection = service.get_inspection(id).await?;
    
    Ok(Json(ApiResponse::success(serde_json::to_value(inspection)?)))
}

/// 创建采购质检单
#[axum::debug_handler]
pub async fn create_inspection(
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreatePurchaseInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(|e| {
        AppError::ValidationError(e.to_string())
    })?;
    
    let service = PurchaseInspectionService::new(db);
    let user_id = 1;
    
    let inspection = service.create_inspection(req, user_id).await?;
    
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单创建成功",
    )))
}

/// 更新采购质检单
#[axum::debug_handler]
pub async fn update_inspection(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<UpdatePurchaseInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PurchaseInspectionService::new(db);
    let user_id = 1;
    
    let inspection = service.update_inspection(id, req, user_id).await?;
    
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单更新成功",
    )))
}

/// 完成采购质检单
#[axum::debug_handler]
pub async fn complete_inspection(
    Path(id): Path<i32>,
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CompleteInspectionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(|e| {
        AppError::ValidationError(e.to_string())
    })?;
    
    let service = PurchaseInspectionService::new(db);
    let user_id = 1;
    
    let inspection = service.complete_inspection(id, req, user_id).await?;
    
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(inspection)?,
        "采购质检单已完成",
    )))
}


// =====================================================
// 请求 DTO
// =====================================================

/// 采购质检单查询参数
#[derive(Debug, Deserialize)]
pub struct InspectionQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}
