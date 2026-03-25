use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::models::dto::{ApiResponse, PageRequest};
use crate::utils::response::PaginatedResponse;
use crate::services::inventory_count_service::{InventoryCountService, CreateInventoryCountRequest, UpdateInventoryCountRequest};

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct InventoryCountQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub warehouse_id: Option<i32>,
    pub count_no: Option<String>,
}

/// 审核盘点请求
#[derive(Debug, Deserialize)]
pub struct ApproveCountRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 获取库存盘点列表
/// GET /api/v1/erp/inventory/counts
pub async fn list_counts(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<InventoryCountQuery>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(10),
    };
    
    match count_service.list_counts(
        page_req,
        query.status,
        query.warehouse_id,
        query.count_no,
    ).await {
        Ok(counts) => {
            let paginated: PaginatedResponse<_> = PaginatedResponse::new(counts.data, counts.total, counts.page, counts.page_size);
            paginated.into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&e.to_string())),
        ).into_response(),
    }
}

/// 获取库存盘点详情
/// GET /api/v1/erp/inventory/counts/:id
pub async fn get_count(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    match count_service.get_count_detail(id).await {
        Ok(count) => ApiResponse::success(count).into_response(),
        Err(e) => {
            if e.to_string().contains("未找到") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            }
        }
    }
}

/// 创建库存盘点
/// POST /api/v1/erp/inventory/counts
pub async fn create_count(
    State(db): State<Arc<DatabaseConnection>>,
    Json(request): Json<CreateInventoryCountRequest>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    match count_service.create_count(request).await {
        Ok(count) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_message(count, "库存盘点单创建成功")),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&e.to_string())),
        ).into_response(),
    }
}

/// 更新库存盘点
/// PUT /api/v1/erp/inventory/counts/:id
pub async fn update_count(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateInventoryCountRequest>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    match count_service.update_count(id, request).await {
        Ok(count) => {
            let response: ApiResponse<_> = ApiResponse::success_with_message(count, "库存盘点单更新成功");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            if e.to_string().contains("未找到") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            }
        }
    }
}

/// 审核库存盘点
/// POST /api/v1/erp/inventory/counts/:id/approve
pub async fn approve_count(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(request): Json<ApproveCountRequest>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    match count_service.approve_count(id, request.approved, request.notes).await {
        Ok(count) => {
            let message = if request.approved {
                "库存盘点单已审核"
            } else {
                "库存盘点单已驳回"
            };
            ApiResponse::success_with_message(count, message).into_response()
        },
        Err(e) => {
            if e.to_string().contains("未找到") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            }
        }
    }
}

/// 完成库存盘点并调整库存
/// POST /api/v1/erp/inventory/counts/:id/complete
pub async fn complete_count(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let count_service = InventoryCountService::new(db.clone());
    
    match count_service.complete_count(id).await {
        Ok(count) => ApiResponse::success_with_message(count, "库存盘点已完成，库存已调整").into_response(),
        Err(e) => {
            if e.to_string().contains("未找到") {
                (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(&e.to_string())),
                ).into_response()
            }
        }
    }
}
