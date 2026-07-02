//! 批量操作 Handler

use crate::middleware::auth_context::AuthContext;
use crate::services::batch_service::{
    BatchCreateProductRequest, BatchService, BatchUpdateProductRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// 批量创建产品请求
#[derive(Debug, Deserialize)]
pub struct BatchCreateProductsPayload {
    pub products: Vec<BatchCreateProductRequest>,
}

/// 批量更新产品请求
#[derive(Debug, Deserialize)]
pub struct BatchUpdateProductsPayload {
    pub products: Vec<BatchUpdateProductRequest>,
}

/// 批量删除产品请求
#[derive(Debug, Deserialize)]
pub struct BatchDeleteProductsPayload {
    pub ids: Vec<i32>,
}

/// 批量操作响应
#[derive(Debug, Serialize)]
pub struct BatchResponse<T> {
    pub success: bool,
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub failed: usize,
    pub data: Option<T>,
    pub errors: Option<Vec<String>>,
}

/// 批量创建产品
pub async fn batch_create_products(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<BatchCreateProductsPayload>,
) -> Result<Json<ApiResponse<BatchResponse<Vec<serde_json::Value>>>>, AppError> {
    let service = BatchService::new(state.db.clone());

    let result = service
        .batch_create_products(auth.user_id, payload.products)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let data: Vec<serde_json::Value> = result
        .data
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "code": p.code,
                "category_id": p.category_id,
                "status": p.status
            })
        })
        .collect();

    let errors: Vec<String> = result
        .errors
        .iter()
        .map(|e| format!("索引 {}: {}", e.index, e.message))
        .collect();

    Ok(Json(ApiResponse::success_with_message(
        BatchResponse {
            success: result.success,
            total: result.total,
            created: result.created,
            updated: result.updated,
            deleted: 0,
            failed: result.failed,
            data: Some(data),
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        },
        "批量创建产品完成",
    )))
}

/// 批量更新产品
pub async fn batch_update_products(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<BatchUpdateProductsPayload>,
) -> Result<Json<ApiResponse<BatchResponse<Vec<serde_json::Value>>>>, AppError> {
    let service = BatchService::new(state.db.clone());

    let result = service
        .batch_update_products(auth.user_id, payload.products)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let data: Vec<serde_json::Value> = result
        .data
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "code": p.code,
                "category_id": p.category_id,
                "status": p.status
            })
        })
        .collect();

    let errors: Vec<String> = result
        .errors
        .iter()
        .map(|e| format!("索引 {}: {}", e.index, e.message))
        .collect();

    Ok(Json(ApiResponse::success_with_message(
        BatchResponse {
            success: result.success,
            total: result.total,
            created: 0,
            updated: result.updated,
            deleted: 0,
            failed: result.failed,
            data: Some(data),
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        },
        "批量更新产品完成",
    )))
}

/// 批量删除产品
pub async fn batch_delete_products(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<BatchDeleteProductsPayload>,
) -> Result<Json<ApiResponse<BatchResponse<()>>>, AppError> {
    let service = BatchService::new(state.db.clone());

    let result = service
        .batch_delete_products(auth.user_id, payload.ids)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let errors: Vec<String> = result
        .errors
        .iter()
        .map(|e| format!("索引 {}: {}", e.index, e.message))
        .collect();

    Ok(Json(ApiResponse::success_with_message(
        BatchResponse {
            success: result.success,
            total: result.total,
            created: 0,
            updated: 0,
            deleted: result.total - result.failed,
            failed: result.failed,
            data: None,
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        },
        "批量删除产品完成",
    )))
}
