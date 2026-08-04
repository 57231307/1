//! 存货跌价准备 HTTP 端点（V15 P2 B08-16）
//!
//! 提供端点：
//! - `GET /api/v1/erp/inventory/write-downs` — 查询跌价准备列表（分页 + 产品/类型过滤）
//! - `GET /api/v1/erp/inventory/write-downs/:id` — 查询跌价准备详情
//! - `POST /api/v1/erp/inventory/write-downs` — 创建跌价准备记录
//! - `POST /api/v1/erp/inventory/write-downs/:id/confirm` — 确认跌价准备

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_write_down;
use crate::services::inventory_write_down_service::{
    CreateWriteDownReq, InventoryWriteDownService, ListParams,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct WriteDownQuery {
    pub product_id: Option<i32>,
    pub write_down_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWriteDownPayload {
    pub product_id: i32,
    pub write_down_type: String,
    pub original_cost: Decimal,
    pub net_realizable_value: Decimal,
    pub reason: Option<String>,
    pub period: chrono::NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct WriteDownResponse {
    pub id: i32,
    pub product_id: i32,
    pub write_down_type: String,
    pub original_cost: Decimal,
    pub net_realizable_value: Decimal,
    pub write_down_amount: Decimal,
    pub reason: Option<String>,
    pub period: chrono::NaiveDate,
    pub status: String,
    pub created_by: i32,
    pub confirmed_by: Option<i32>,
    pub confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<inventory_write_down::Model> for WriteDownResponse {
    fn from(m: inventory_write_down::Model) -> Self {
        Self {
            id: m.id,
            product_id: m.product_id,
            write_down_type: m.write_down_type,
            original_cost: m.original_cost,
            net_realizable_value: m.net_realizable_value,
            write_down_amount: m.write_down_amount,
            reason: m.reason,
            period: m.period,
            status: m.status,
            created_by: m.created_by,
            confirmed_by: m.confirmed_by,
            confirmed_at: m.confirmed_at,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// GET /api/v1/erp/inventory/write-downs - 查询跌价准备列表
pub async fn list_write_downs(
    Query(params): Query<WriteDownQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<WriteDownResponse>>>, AppError> {
    tracing::info!("用户 {} 正在查询存货跌价准备列表", auth.user_id);

    let service = InventoryWriteDownService::new(state.db.clone());
    let list_params = ListParams {
        product_id: params.product_id,
        write_down_type: params.write_down_type,
        page: params.page,
        page_size: params.page_size,
    };

    let (items, _total) = service.list(list_params).await?;
    let response: Vec<WriteDownResponse> = items.into_iter().map(WriteDownResponse::from).collect();

    Ok(Json(ApiResponse::success(response)))
}

/// GET /api/v1/erp/inventory/write-downs/:id - 查询跌价准备详情
pub async fn get_write_down(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WriteDownResponse>>, AppError> {
    tracing::info!("用户 {} 正在查询存货跌价准备详情：{}", auth.user_id, id);

    let service = InventoryWriteDownService::new(state.db.clone());
    let model = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(WriteDownResponse::from(model))))
}

/// POST /api/v1/erp/inventory/write-downs - 创建跌价准备记录
pub async fn create_write_down(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateWriteDownPayload>,
) -> Result<Json<ApiResponse<WriteDownResponse>>, AppError> {
    tracing::info!(
        "用户 {} 正在创建存货跌价准备：产品ID={}",
        auth.user_id,
        req.product_id
    );

    let service = InventoryWriteDownService::new(state.db.clone());
    let create_req = CreateWriteDownReq {
        product_id: req.product_id,
        write_down_type: req.write_down_type,
        original_cost: req.original_cost,
        net_realizable_value: req.net_realizable_value,
        reason: req.reason,
        period: req.period,
        created_by: auth.user_id,
    };

    let model = service.create(create_req).await?;
    tracing::info!("存货跌价准备创建成功：ID={}", model.id);

    Ok(Json(ApiResponse::success(WriteDownResponse::from(model))))
}

/// POST /api/v1/erp/inventory/write-downs/:id/confirm - 确认跌价准备
pub async fn confirm_write_down(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<WriteDownResponse>>, AppError> {
    tracing::info!("用户 {} 正在确认存货跌价准备：{}", auth.user_id, id);

    let service = InventoryWriteDownService::new(state.db.clone());
    let model = service.confirm(id, auth.user_id).await?;

    tracing::info!("存货跌价准备确认成功：ID={}", id);
    Ok(Json(ApiResponse::success(WriteDownResponse::from(model))))
}
