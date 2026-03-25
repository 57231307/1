//! 批量操作 Handler

use crate::services::batch_service::{
    BatchCreateProductRequest, BatchService, BatchUpdateProductRequest,
};
use axum::{extract::State, http::StatusCode, Json};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<BatchCreateProductsPayload>,
) -> Result<Json<BatchResponse<Vec<serde_json::Value>>>, (StatusCode, String)> {
    let service = BatchService::new(db.clone());

    match service.batch_create_products(payload.products).await {
        Ok(result) => {
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

            Ok(Json(BatchResponse {
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
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 批量更新产品
pub async fn batch_update_products(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<BatchUpdateProductsPayload>,
) -> Result<Json<BatchResponse<Vec<serde_json::Value>>>, (StatusCode, String)> {
    let service = BatchService::new(db.clone());

    match service.batch_update_products(payload.products).await {
        Ok(result) => {
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

            Ok(Json(BatchResponse {
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
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 批量删除产品
pub async fn batch_delete_products(
    State(db): State<Arc<DatabaseConnection>>,
    Json(payload): Json<BatchDeleteProductsPayload>,
) -> Result<Json<BatchResponse<()>>, (StatusCode, String)> {
    let service = BatchService::new(db.clone());

    match service.batch_delete_products(payload.ids).await {
        Ok(result) => {
            let errors: Vec<String> = result
                .errors
                .iter()
                .map(|e| format!("索引 {}: {}", e.index, e.message))
                .collect();

            Ok(Json(BatchResponse {
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
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}
