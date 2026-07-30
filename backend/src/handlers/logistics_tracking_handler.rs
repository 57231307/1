//! 物流跟踪 handler（基于 LogisticsService）

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::logistics_service::{LogisticsService, TrackingEvent};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

/// 关联采购订单请求
#[derive(Debug, Deserialize)]
pub struct LinkPurchaseOrderRequest {
    pub po_id: i32,
}

/// 关联采购订单到运单
pub async fn link_purchase_order(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(waybill_id): Path<i32>,
    Json(req): Json<LinkPurchaseOrderRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LogisticsService::new(state.db.clone());
    let model = service.link_purchase_order(waybill_id, req.po_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 记录物流跟踪事件
pub async fn record_tracking_event(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(waybill_id): Path<i32>,
    Json(event): Json<TrackingEvent>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LogisticsService::new(state.db.clone());
    let model = service.record_tracking_event(waybill_id, event).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 查询运单跟踪事件历史
pub async fn list_tracking_events(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(waybill_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LogisticsService::new(state.db.clone());
    let list = service.list_tracking_events(waybill_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 计算运费
pub async fn calculate_freight(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(waybill_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = LogisticsService::new(state.db.clone());
    let freight = service.calculate_freight(waybill_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(freight)?)))
}
