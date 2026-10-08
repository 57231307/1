use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_reservation;
use crate::utils::data_scope::check_resource_owner_by_member_scope;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 预留查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ReservationQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
}

/// 创建预留请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub order_id: i32,
    pub notes: Option<String>,
}

/// 预留响应
#[allow(dead_code, reason = "序列化输出字段")]
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
    // 提取行级数据权限上下文，供归属门判定
    let data_scope_ctx = auth.to_data_scope_context();

    // 铁律：service 返回的已是 AppError，直接 `?` 透传其真实 HTTP 状态与错误码，
    // 禁止再 map_err 强转 internal 把业务拒绝/404 压成 500
    let (reservations, total) = service
        .list_reservations(
            page,
            page_size,
            query.product_id,
            query.warehouse_id,
            query.status,
            Some(&data_scope_ctx),
        )
        .await?;

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

    // service 已返回 AppError，`?` 透传（含未来补业务校验时的 400/404 不被压成 500）
    let reservation = service
        .create_reservation(
            payload.order_id,
            payload.product_id,
            payload.warehouse_id,
            payload.quantity,
            Some(auth.user_id),
            payload.notes,
        )
        .await?;

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

    // IDOR 防护：删除前先经 get_reservation 的行级归属门
    // 归属校验失败的 403/404 必须原样透传，不得强转 500 掩盖拒绝原因
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_reservation(id, Some(&data_scope_ctx)).await?;

    // 状态门拒绝（如"释放的预留不可删除"）为 400 BUSINESS_ERROR，`?` 透传
    service.delete_reservation(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "预留已删除"
    }))))
}

/// 锁定预留（从 pending 到 locked）
/// POST /api/v1/erp/inventory/reservations/:id/lock
///
/// 单行写动作归属门（写前）：`inventory_reservation` 有 `created_by` 归属列
/// （models/inventory_reservation.rs::created_by）而无 `department_id` 列 ⇒ 按 `id`
/// 取本行、过 `check_resource_owner_by_member_scope`（判据与 `list_reservations` 列表侧
/// Dept 分支同源）。取行不存在走 404（文案不含记录 ID），归属门在 service 状态翻转
/// 落库点之前，越权零写入。
pub async fn lock_reservation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReservationResponse>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let row = inventory_reservation::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库存预留不存在"))?;
    if !check_resource_owner_by_member_scope(&ctx, row.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该库存预留（数据范围限制）",
        ));
    }

    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    // 状态门拒绝（400 BUSINESS_ERROR）与不存在（404）由 `?` 原样透传
    let reservation = service.lock_reservation(id).await?;

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
///
/// 单行写动作归属门（写前）：与 `lock_reservation` 同款——`inventory_reservation` 有
/// `created_by` 归属列无部门列，按 `id` 取本行、过 `check_resource_owner_by_member_scope`
/// （判据与 `list_reservations` 列表侧 Dept 分支同源）。取行不存在走 404（文案不含记录
/// ID），归属门在 service 状态翻转落库点之前，越权零写入。
pub async fn release_reservation(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReservationResponse>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let row = inventory_reservation::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("库存预留不存在"))?;
    if !check_resource_owner_by_member_scope(&ctx, row.created_by) {
        return Err(AppError::permission_denied(
            "无权操作该库存预留（数据范围限制）",
        ));
    }

    let service = crate::services::inventory_reservation_service::InventoryReservationService::new(
        state.db.clone(),
    );

    // 状态门拒绝（400 BUSINESS_ERROR）与不存在（404）由 `?` 原样透传
    let reservation = service.release_reservation(id).await?;

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
