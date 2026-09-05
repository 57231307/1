//! 面料行业版库存批次 handler
//!
//! 缺陷 3 修复：原实现直接操作 Entity（批次 CRUD/调拨事务内联在 handler），
//! 现已下沉至 `InventoryStockService`（list_batches / create_batch_fabric /
//! update_batch_fields / delete_batch_with_audit / transfer_batch），
//! 本文件仅保留请求 DTO + 参数提取 + service 调用。

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_stock;
use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 批次列表（反序列化输入字段）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct BatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub grade: Option<String>,
    pub warehouse_id: Option<i32>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

/// 创建批次请求（面料行业版）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub batch_no: String,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub color_no: String,
    pub color_name: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub production_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub supplier_id: Option<i32>,
    pub purchase_order_no: Option<String>,
    pub remarks: Option<String>,
}

/// 更新批次请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateBatchRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub remarks: Option<String>,
    pub stock_status: Option<String>,
    pub quality_status: Option<String>,
}

/// 批次转移请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct TransferBatchRequest {
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub remarks: Option<String>,
}

/// 获取批次列表
pub async fn list_batches(
    State(state): State<AppState>,
    Query(query): Query<BatchListQuery>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<inventory_stock::Model>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let (batches, total) = service.list_batches(page, page_size).await?;
    let paginated =
        PaginatedResponse::new(batches, total, page.clamp(1, 1000), page_size.clamp(1, 100));
    Ok(Json(ApiResponse::success(paginated)))
}

/// 获取批次详情
pub async fn get_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<inventory_stock::Model>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    let batch = service.find_by_id(id).await.map_err(|e| match e {
        AppError::NotFound(msg) => AppError::not_found(msg),
        other => other,
    })?;
    Ok(Json(ApiResponse::success(batch)))
}

/// 创建批次（入库）
pub async fn create_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateBatchRequest>,
) -> Result<Json<ApiResponse<inventory_stock::Model>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    let created = service
        .create_batch_fabric(
            crate::services::inventory_stock_service::CreateBatchFabricArgs {
                batch_no: req.batch_no,
                product_id: req.product_id,
                warehouse_id: req.warehouse_id,
                color_no: req.color_no,
                dye_lot_no: req.dye_lot_no,
                grade: req.grade,
                quantity_meters: req.quantity_meters,
                quantity_kg: req.quantity_kg,
                gram_weight: req.gram_weight,
                width: req.width,
                production_date: req.production_date,
                expiry_date: req.expiry_date,
            },
        )
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "批次创建成功",
    )))
}

/// 更新批次
pub async fn update_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateBatchRequest>,
) -> Result<Json<ApiResponse<inventory_stock::Model>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    let updated = service
        .update_batch_fields(
            id,
            req.color_no,
            req.dye_lot_no,
            req.grade,
            req.gram_weight,
            req.width,
            req.expiry_date,
            req.stock_status,
            req.quality_status,
        )
        .await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "批次更新成功",
    )))
}

/// 删除批次
pub async fn delete_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    service.delete_batch_with_audit(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "批次删除成功")))
}

/// 批次转移（调拨）
pub async fn transfer_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<TransferBatchRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());
    service
        .transfer_batch(
            id,
            req.from_warehouse_id,
            req.to_warehouse_id,
            req.quantity_meters,
            req.quantity_kg,
        )
        .await?;
    Ok(Json(ApiResponse::success_with_message((), "批次转移成功")))
}
