use crate::services::inventory_adjustment_service::{
    AdjustmentItemRequest, CreateAdjustmentRequest, InventoryAdjustmentService,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 创建调整单请求
#[derive(Debug, Deserialize)]
pub struct CreateAdjustmentRequestPayload {
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<AdjustmentItemPayload>,
}

#[derive(Debug, Deserialize)]
pub struct AdjustmentItemPayload {
    pub stock_id: i32,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
}

/// 调整单响应
#[derive(Debug, Serialize)]
pub struct AdjustmentResponse {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_date: DateTime<Utc>,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub total_quantity: Decimal,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub items: Vec<AdjustmentItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentItemResponse {
    pub id: i32,
    pub stock_id: i32,
    pub quantity: Decimal,
    pub quantity_before: Decimal,
    pub quantity_after: Decimal,
    pub unit_cost: Option<Decimal>,
    pub amount: Option<Decimal>,
    pub notes: Option<String>,
}

/// 调整单列表响应
#[derive(Debug, Serialize)]
pub struct AdjustmentListResponse {
    pub adjustments: Vec<AdjustmentSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct AdjustmentSummary {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_type: String,
    pub reason_type: String,
    pub status: String,
    pub total_quantity: Decimal,
    pub created_at: DateTime<Utc>,
}

/// 创建调整单
pub async fn create_adjustment(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdjustmentRequestPayload>,
) -> Result<Json<AdjustmentResponse>, (StatusCode, String)> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    // 解析日期
    let adjustment_date: DateTime<Utc> = payload
        .adjustment_date
        .parse::<DateTime<Utc>>()
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("日期格式错误：{}", e)))?;

    // 转换请求
    let request = CreateAdjustmentRequest {
        warehouse_id: payload.warehouse_id,
        adjustment_date,
        adjustment_type: payload.adjustment_type,
        reason_type: payload.reason_type,
        reason_description: payload.reason_description,
        notes: payload.notes,
        created_by: Some(1), // TODO: 从 Token 中获取
        items: payload
            .items
            .into_iter()
            .map(|item| AdjustmentItemRequest {
                stock_id: item.stock_id,
                quantity: item
                    .quantity
                    .parse::<Decimal>()
                    .map_err(|e| format!("数量格式错误：{}", e))
                    .unwrap(),
                unit_cost: item.unit_cost.and_then(|s| s.parse::<Decimal>().ok()),
                notes: item.notes,
            })
            .collect(),
    };

    match service.create_adjustment(request).await {
        Ok(detail) => Ok(Json(AdjustmentResponse {
            id: detail.adjustment.id,
            adjustment_no: detail.adjustment.adjustment_no,
            warehouse_id: detail.adjustment.warehouse_id,
            adjustment_date: detail.adjustment.adjustment_date,
            adjustment_type: detail.adjustment.adjustment_type,
            reason_type: detail.adjustment.reason_type,
            reason_description: detail.adjustment.reason_description,
            total_quantity: detail.adjustment.total_quantity,
            notes: detail.adjustment.notes,
            status: detail.adjustment.status,
            created_at: detail.adjustment.created_at,
            items: detail
                .items
                .into_iter()
                .map(|item| AdjustmentItemResponse {
                    id: item.id,
                    stock_id: item.stock_id,
                    quantity: item.quantity,
                    quantity_before: item.quantity_before,
                    quantity_after: item.quantity_after,
                    unit_cost: item.unit_cost,
                    amount: item.amount,
                    notes: item.notes,
                })
                .collect(),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 审核调整单
pub async fn approve_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdjustmentResponse>, (StatusCode, String)> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    match service.approve_adjustment(id, 1).await {
        // TODO: 从 Token 中获取用户 ID
        Ok(_adjustment) => {
            let detail = service
                .get_adjustment(id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok(Json(AdjustmentResponse {
                id: detail.adjustment.id,
                adjustment_no: detail.adjustment.adjustment_no,
                warehouse_id: detail.adjustment.warehouse_id,
                adjustment_date: detail.adjustment.adjustment_date,
                adjustment_type: detail.adjustment.adjustment_type,
                reason_type: detail.adjustment.reason_type,
                reason_description: detail.adjustment.reason_description,
                total_quantity: detail.adjustment.total_quantity,
                notes: detail.adjustment.notes,
                status: detail.adjustment.status,
                created_at: detail.adjustment.created_at,
                items: detail
                    .items
                    .into_iter()
                    .map(|item| AdjustmentItemResponse {
                        id: item.id,
                        stock_id: item.stock_id,
                        quantity: item.quantity,
                        quantity_before: item.quantity_before,
                        quantity_after: item.quantity_after,
                        unit_cost: item.unit_cost,
                        amount: item.amount,
                        notes: item.notes,
                    })
                    .collect(),
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 驳回调整单
pub async fn reject_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdjustmentResponse>, (StatusCode, String)> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    match service.reject_adjustment(id).await {
        Ok(_adjustment) => {
            let detail = service
                .get_adjustment(id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            Ok(Json(AdjustmentResponse {
                id: detail.adjustment.id,
                adjustment_no: detail.adjustment.adjustment_no,
                warehouse_id: detail.adjustment.warehouse_id,
                adjustment_date: detail.adjustment.adjustment_date,
                adjustment_type: detail.adjustment.adjustment_type,
                reason_type: detail.adjustment.reason_type,
                reason_description: detail.adjustment.reason_description,
                total_quantity: detail.adjustment.total_quantity,
                notes: detail.adjustment.notes,
                status: detail.adjustment.status,
                created_at: detail.adjustment.created_at,
                items: detail
                    .items
                    .into_iter()
                    .map(|item| AdjustmentItemResponse {
                        id: item.id,
                        stock_id: item.stock_id,
                        quantity: item.quantity,
                        quantity_before: item.quantity_before,
                        quantity_after: item.quantity_after,
                        unit_cost: item.unit_cost,
                        amount: item.amount,
                        notes: item.notes,
                    })
                    .collect(),
            }))
        }
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

/// 查询调整单列表
pub async fn list_adjustments(
    State(state): State<AppState>,
    Query(params): Query<ListAdjustmentsParams>,
) -> Result<Json<AdjustmentListResponse>, (StatusCode, String)> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    match service
        .list_adjustments(params.page.unwrap_or(0), params.page_size.unwrap_or(20))
        .await
    {
        Ok((adjustments, total)) => Ok(Json(AdjustmentListResponse {
            adjustments: adjustments
                .into_iter()
                .map(|a| AdjustmentSummary {
                    id: a.id,
                    adjustment_no: a.adjustment_no,
                    warehouse_id: a.warehouse_id,
                    adjustment_type: a.adjustment_type,
                    reason_type: a.reason_type,
                    status: a.status,
                    total_quantity: a.total_quantity,
                    created_at: a.created_at,
                })
                .collect(),
            total,
            page: params.page.unwrap_or(0),
            page_size: params.page_size.unwrap_or(20),
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListAdjustmentsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询调整单详情
pub async fn get_adjustment(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AdjustmentResponse>, (StatusCode, String)> {
    let service = InventoryAdjustmentService::new(state.db.clone());

    match service.get_adjustment(id).await {
        Ok(detail) => Ok(Json(AdjustmentResponse {
            id: detail.adjustment.id,
            adjustment_no: detail.adjustment.adjustment_no,
            warehouse_id: detail.adjustment.warehouse_id,
            adjustment_date: detail.adjustment.adjustment_date,
            adjustment_type: detail.adjustment.adjustment_type,
            reason_type: detail.adjustment.reason_type,
            reason_description: detail.adjustment.reason_description,
            total_quantity: detail.adjustment.total_quantity,
            notes: detail.adjustment.notes,
            status: detail.adjustment.status,
            created_at: detail.adjustment.created_at,
            items: detail
                .items
                .into_iter()
                .map(|item| AdjustmentItemResponse {
                    id: item.id,
                    stock_id: item.stock_id,
                    quantity: item.quantity,
                    quantity_before: item.quantity_before,
                    quantity_after: item.quantity_after,
                    unit_cost: item.unit_cost,
                    amount: item.amount,
                    notes: item.notes,
                })
                .collect(),
        })),
        Err(e) => Err((StatusCode::NOT_FOUND, e.to_string())),
    }
}

use axum::extract::Query;
