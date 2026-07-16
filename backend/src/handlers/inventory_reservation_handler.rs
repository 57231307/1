use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 预留查询参数
#[derive(Debug, Deserialize)]
pub struct ReservationQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
}

/// 创建预留请求
#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub order_id: i32,
    pub notes: Option<String>,
}

/// 预留响应
#[derive(Debug, Serialize)]
pub struct ReservationResponse {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 获取预留列表
/// GET /api/v1/erp/inventory/reservations
pub async fn list_reservations(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ReservationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let (reservations, total) = service
        .list_reservations(
            page,
            page_size,
            query.product_id,
            query.warehouse_id,
            query.status,
            Some(&data_scope_ctx),
        )
        .await
        .map_err(|e| AppError::internal(format!("获取预留列表失败: {}", e)))?;

    let result = serde_json::json!({
        "list": reservations,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 创建预留
/// POST /api/v1/erp/inventory/reservations
pub async fn create_reservation(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateReservationRequest>,
) -> Result<Json<ApiResponse<ReservationResponse>>, AppError> {
    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    let reservation = service
        .create_reservation(
            payload.order_id,
            payload.product_id,
            payload.warehouse_id,
            payload.quantity,
            Some(auth.user_id),
            payload.notes,
        )
        .await
        .map_err(|e| AppError::internal(format!("创建预留失败: {}", e)))?;

    Ok(Json(ApiResponse::success(ReservationResponse {
        id: reservation.id,
        order_id: reservation.order_id,
        product_id: reservation.product_id,
        warehouse_id: reservation.warehouse_id,
        quantity: reservation.quantity,
        status: reservation.status,
        notes: reservation.notes,
        created_at: reservation.created_at,
        updated_at: reservation.updated_at,
    })))
}

/// 删除预留
/// DELETE /api/v1/erp/inventory/reservations/:id
pub async fn delete_reservation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    service
        .delete_reservation(id, auth.user_id)
        .await
        .map_err(|e| AppError::internal(format!("删除预留失败: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "预留已删除"
    }))))
}

/// 锁定预留（从 pending 到 locked）
/// POST /api/v1/erp/inventory/reservations/:id/lock
pub async fn lock_reservation(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReservationResponse>>, AppError> {
    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    let reservation = service
        .lock_reservation(id)
        .await
        .map_err(|e| AppError::internal(format!("锁定预留失败: {}", e)))?;

    Ok(Json(ApiResponse::success(ReservationResponse {
        id: reservation.id,
        order_id: reservation.order_id,
        product_id: reservation.product_id,
        warehouse_id: reservation.warehouse_id,
        quantity: reservation.quantity,
        status: reservation.status,
        notes: reservation.notes,
        created_at: reservation.created_at,
        updated_at: reservation.updated_at,
    })))
}

/// 释放预留（从 locked/pending 到 released）
/// POST /api/v1/erp/inventory/reservations/:id/release
pub async fn release_reservation(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReservationResponse>>, AppError> {
    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    let reservation = service
        .release_reservation(id)
        .await
        .map_err(|e| AppError::internal(format!("释放预留失败: {}", e)))?;

    Ok(Json(ApiResponse::success(ReservationResponse {
        id: reservation.id,
        order_id: reservation.order_id,
        product_id: reservation.product_id,
        warehouse_id: reservation.warehouse_id,
        quantity: reservation.quantity,
        status: reservation.status,
        notes: reservation.notes,
        created_at: reservation.created_at,
        updated_at: reservation.updated_at,
    })))
}
