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
    // P1 2-1/2-2 修复（批次 64）：批量删除显式校验 products:delete 权限
    // 原实现路由用 POST /products/batch/delete，permission 中间件按 POST→create 映射，
    // 持 products:create 权限即可批量删除，越权删除漏洞。
    // 修复：handler 内显式校验 products:delete 权限（已有 auth 参数）。
    let role_permission_service =
        crate::services::role_permission_service::RolePermissionService::new(state.db.clone());
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行批量删除操作"))?;
    let has_permission = role_permission_service
        .check_permission(role_id, "products", "delete", None)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    if !has_permission {
        return Err(AppError::permission_denied("没有批量删除产品的权限"));
    }

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
