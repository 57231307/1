//! 库存盘点 HTTP 端点
//!
//! v11 批次 143 P1-1：真实实现库存盘点功能（v8 占位已恢复）
//!
//! 提供端点：
//! - `POST /api/v1/erp/inventory/counts` — 创建盘点单（自动生成库存快照明细）
//! - `GET /api/v1/erp/inventory/counts` — 查询盘点单列表（分页 + 仓库/状态过滤）
//! - `GET /api/v1/erp/inventory/counts/:id` — 查询盘点单详情（含明细）
//! - `PUT /api/v1/erp/inventory/counts/:id` — 更新盘点单（仅 pending 状态）
//! - `DELETE /api/v1/erp/inventory/counts/:id` — 删除盘点单（仅 pending 状态）
//! - `POST /api/v1/erp/inventory/counts/:id/record` — 录入实盘数量并自动计算差异
//! - `POST /api/v1/erp/inventory/counts/:id/submit` — 提交盘点单进入审批
//! - `POST /api/v1/erp/inventory/counts/:id/approve` — 审批通过并完成盘点（同步更新库存）
//! - `POST /api/v1/erp/inventory/counts/:id/reject` — 驳回审批（退回 pending 状态）

use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_count;
use crate::models::inventory_count_item;
use crate::services::inventory_count_service::{
    CountItemInput, CreateCountRequest, InventoryCountService, UpdateCountRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 创建盘点单请求体
#[derive(Debug, Deserialize)]
pub struct CreateCountPayload {
    pub warehouse_id: i32,
    /// ISO 8601 日期字符串（如 "2026-07-06T00:00:00Z"）
    pub count_date: String,
    pub notes: Option<String>,
    /// 指定库存快照 ID 列表；None 或空数组表示仓库下全部库存
    pub stock_ids: Option<Vec<i32>>,
}

/// 盘点单响应
#[derive(Debug, Serialize)]
pub struct CountResponse {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<CountItemResponse>,
}

impl From<inventory_count::Model> for CountResponse {
    fn from(c: inventory_count::Model) -> Self {
        Self {
            id: c.id,
            count_no: c.count_no,
            warehouse_id: c.warehouse_id,
            count_date: c.count_date,
            status: c.status,
            total_items: c.total_items,
            counted_items: c.counted_items,
            variance_items: c.variance_items,
            notes: c.notes,
            created_by: c.created_by,
            approved_by: c.approved_by,
            approved_at: c.approved_at,
            completed_at: c.completed_at,
            created_at: c.created_at,
            updated_at: c.updated_at,
            items: Vec::new(),
        }
    }
}

/// 盘点明细响应
#[derive(Debug, Serialize)]
pub struct CountItemResponse {
    pub id: i32,
    pub count_id: i32,
    pub stock_id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity_before: Decimal,
    pub quantity_actual: Decimal,
    pub quantity_difference: Decimal,
    pub unit_cost: Decimal,
    pub total_cost: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<inventory_count_item::Model> for CountItemResponse {
    fn from(it: inventory_count_item::Model) -> Self {
        Self {
            id: it.id,
            count_id: it.count_id,
            stock_id: it.stock_id,
            product_id: it.product_id,
            warehouse_id: it.warehouse_id,
            quantity_before: it.quantity_before,
            quantity_actual: it.quantity_actual,
            quantity_difference: it.quantity_difference,
            unit_cost: it.unit_cost,
            total_cost: it.total_cost,
            notes: it.notes,
            created_at: it.created_at,
            updated_at: it.updated_at,
        }
    }
}

/// 盘点单列表响应
#[derive(Debug, Serialize)]
pub struct CountListResponse {
    pub counts: Vec<CountSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize)]
pub struct CountSummary {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub created_at: DateTime<Utc>,
}

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListCountsParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
}

/// 录入实盘数量请求体
#[derive(Debug, Deserialize)]
pub struct RecordItemsPayload {
    pub items: Vec<RecordItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct RecordItemInput {
    pub stock_id: i32,
    /// 数量字符串（避免 JSON 浮点精度丢失）
    pub quantity_actual: String,
    pub notes: Option<String>,
}

/// 更新盘点单请求体
#[derive(Debug, Deserialize)]
pub struct UpdateCountPayload {
    pub count_date: Option<String>,
    pub notes: Option<String>,
}

/// 创建盘点单
pub async fn create_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(payload): Json<CreateCountPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let count_date: DateTime<Utc> = payload
        .count_date
        .parse::<DateTime<Utc>>()
        .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;

    let req = CreateCountRequest {
        warehouse_id: payload.warehouse_id,
        count_date,
        notes: payload.notes,
        created_by: Some(auth.user_id),
        stock_ids: payload.stock_ids,
    };
    let detail = service.create_count(req).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 查询盘点单列表
pub async fn list_counts(
    State(state): State<AppState>,
    Query(params): Query<ListCountsParams>,
) -> Result<Json<ApiResponse<CountListResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    let (counts, total) = service
        .list_counts(page, page_size, params.warehouse_id, params.status)
        .await?;
    let summaries = counts
        .into_iter()
        .map(|c| CountSummary {
            id: c.id,
            count_no: c.count_no,
            warehouse_id: c.warehouse_id,
            count_date: c.count_date,
            status: c.status,
            total_items: c.total_items,
            counted_items: c.counted_items,
            variance_items: c.variance_items,
            created_at: c.created_at,
        })
        .collect();
    Ok(Json(ApiResponse::success(CountListResponse {
        counts: summaries,
        total,
        page,
        page_size,
    })))
}

/// 查询盘点单详情
pub async fn get_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let detail = service.get_count(id).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 更新盘点单
pub async fn update_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateCountPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let count_date = payload
        .count_date
        .map(|s| {
            s.parse::<DateTime<Utc>>()
                .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))
        })
        .transpose()?;
    let req = UpdateCountRequest {
        count_date,
        notes: payload.notes,
    };
    let updated = service.update_count(id, req, Some(auth.user_id)).await?;
    let detail = service.get_count(id).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 删除盘点单
pub async fn delete_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    service.delete_count(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// 录入实盘数量并自动计算差异
pub async fn record_count_items(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<RecordItemsPayload>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let mut inputs = Vec::with_capacity(payload.items.len());
    for it in payload.items {
        let qty = it
            .quantity_actual
            .parse::<Decimal>()
            .map_err(|e| AppError::validation(format!("数量格式错误：{}", e)))?;
        inputs.push(CountItemInput {
            stock_id: it.stock_id,
            quantity_actual: qty,
            notes: it.notes,
        });
    }
    let detail = service.record_count_items(id, inputs).await?;
    let mut resp: CountResponse = detail.count.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 提交盘点单进入审批
pub async fn submit_for_approval(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let updated = service.submit_for_approval(id).await?;
    let detail = service.get_count(id).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 审批通过并完成盘点
pub async fn approve_count(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let updated = service.approve_count(id, auth.user_id).await?;
    let detail = service.get_count(id).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 驳回审批
pub async fn reject_count(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<CountResponse>>, AppError> {
    let service = InventoryCountService::new(state.db.clone());
    let updated = service.reject_count(id).await?;
    let detail = service.get_count(id).await?;
    let mut resp: CountResponse = updated.into();
    resp.items = detail.items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(resp)))
}
