//! 委外加工物资 Handler
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算 + §5.5 委外织布场景 + §5.7 损耗率标准 + §6.5 委托加工模式
//! 真实业务流程：
//!   委外订单（draft→issued→processing→received→settled→closed）
//!   发料明细（按面料四维标识追溯）
//!   收回入库单（draft→confirmed，含损耗分类与质量等级）
//!   会计分录凭证（issue/fee/receipt/loss 四类凭证）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{outsourcing_order, outsourcing_order_item, outsourcing_receipt, outsourcing_voucher};
use crate::services::outsourcing_service::{
    CreateOutsourcingOrderItemRequest, CreateOutsourcingOrderRequest,
    CreateOutsourcingReceiptRequest, CreateOutsourcingVoucherRequest,
    OutsourcingOrderItemService, OutsourcingOrderQuery, OutsourcingOrderService,
    OutsourcingReceiptQuery, OutsourcingReceiptService, OutsourcingVoucherQuery,
    OutsourcingVoucherService, UpdateOutsourcingOrderItemRequest, UpdateOutsourcingOrderRequest,
    UpdateOutsourcingReceiptRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

// ============================================================================
// 辅助函数
// ============================================================================

fn order_service(state: &AppState) -> OutsourcingOrderService {
    OutsourcingOrderService::new(state.db.clone())
}

fn item_service(state: &AppState) -> OutsourcingOrderItemService {
    OutsourcingOrderItemService::new(state.db.clone())
}

fn receipt_service(state: &AppState) -> OutsourcingReceiptService {
    OutsourcingReceiptService::new(state.db.clone())
}

fn voucher_service(state: &AppState) -> OutsourcingVoucherService {
    OutsourcingVoucherService::new(state.db.clone())
}

// ============================================================================
// 查询参数（HTTP Query 转 Service Query）
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OutsourcingOrderListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub order_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub issue_date_from: Option<chrono::NaiveDate>,
    pub issue_date_to: Option<chrono::NaiveDate>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OutsourcingOrderItemListQuery {
    // 按委外订单查询明细（路径参数 order_id 提供）
}

#[derive(Debug, Deserialize)]
pub struct OutsourcingReceiptListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub outsourcing_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub receipt_date_from: Option<chrono::NaiveDate>,
    pub receipt_date_to: Option<chrono::NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct OutsourcingVoucherListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub outsourcing_order_id: Option<i32>,
    pub voucher_type: Option<String>,
    pub is_posted: Option<bool>,
    pub voucher_date_from: Option<chrono::NaiveDate>,
    pub voucher_date_to: Option<chrono::NaiveDate>,
}

// ============================================================================
// 委外订单 Handler
// ============================================================================

/// GET /api/v1/erp/outsourcing-orders - 分页查询委外订单
pub async fn list_outsourcing_orders(
    State(state): State<AppState>,
    Query(q): Query<OutsourcingOrderListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<outsourcing_order::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = OutsourcingOrderQuery {
        order_type: q.order_type,
        supplier_id: q.supplier_id,
        production_order_id: q.production_order_id,
        dye_batch_id: q.dye_batch_id,
        dye_lot_no: q.dye_lot_no,
        status: q.status,
        issue_date_from: q.issue_date_from,
        issue_date_to: q.issue_date_to,
        keyword: q.keyword,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = order_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/outsourcing-orders - 创建委外订单
pub async fn create_outsourcing_order(
    State(state): State<AppState>,
    Json(req): Json<CreateOutsourcingOrderRequest>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/outsourcing-orders/by-no/:no - 按订单号查询委外订单
pub async fn get_outsourcing_order_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/outsourcing-orders/:id - 查询委外订单详情
pub async fn get_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).get_by_id(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/outsourcing-orders/:id - 更新委外订单
pub async fn update_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateOutsourcingOrderRequest>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/outsourcing-orders/:id - 软删除委外订单（仅 draft 可删）
pub async fn delete_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    order_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /api/v1/erp/outsourcing-orders/:id/issue - 发料（draft → issued）
pub async fn issue_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).issue_order(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-orders/:id/processing - 标记加工中（issued → processing）
pub async fn record_processing(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).record_processing(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-orders/:id/settle - 结算（received → settled）
pub async fn settle_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).settle(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-orders/:id/close - 关闭（settled → closed）
pub async fn close_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).close_order(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-orders/:id/cancel - 取消（非 closed → cancelled）
pub async fn cancel_outsourcing_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_order::Model>>, AppError> {
    let model = order_service(&state).cancel(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

// ============================================================================
// 委外发料明细 Handler
// ============================================================================

/// GET /api/v1/erp/outsourcing-orders/items/by-order/:order_id - 按委外订单查询明细
pub async fn list_outsourcing_items(
    State(state): State<AppState>,
    Path(order_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<outsourcing_order_item::Model>>>, AppError> {
    let items = item_service(&state).list_by_order(order_id).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// POST /api/v1/erp/outsourcing-orders/items - 创建委外发料明细
pub async fn create_outsourcing_item(
    State(state): State<AppState>,
    Json(req): Json<CreateOutsourcingOrderItemRequest>,
) -> Result<Json<ApiResponse<outsourcing_order_item::Model>>, AppError> {
    let model = item_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/outsourcing-orders/items/:id - 更新委外发料明细
pub async fn update_outsourcing_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateOutsourcingOrderItemRequest>,
) -> Result<Json<ApiResponse<outsourcing_order_item::Model>>, AppError> {
    let model = item_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/outsourcing-orders/items/:id - 删除委外发料明细
pub async fn delete_outsourcing_item(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    item_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 委外收回入库单 Handler
// ============================================================================

/// GET /api/v1/erp/outsourcing-receipts - 分页查询委外收回单
pub async fn list_outsourcing_receipts(
    State(state): State<AppState>,
    Query(q): Query<OutsourcingReceiptListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<outsourcing_receipt::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = OutsourcingReceiptQuery {
        outsourcing_order_id: q.outsourcing_order_id,
        product_id: q.product_id,
        dye_lot_no: q.dye_lot_no,
        status: q.status,
        receipt_date_from: q.receipt_date_from,
        receipt_date_to: q.receipt_date_to,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = receipt_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/outsourcing-receipts - 创建委外收回单
pub async fn create_outsourcing_receipt(
    State(state): State<AppState>,
    Json(req): Json<CreateOutsourcingReceiptRequest>,
) -> Result<Json<ApiResponse<outsourcing_receipt::Model>>, AppError> {
    let model = receipt_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/outsourcing-receipts/by-no/:no - 按收回单号查询
pub async fn get_outsourcing_receipt_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<outsourcing_receipt::Model>>, AppError> {
    let model = receipt_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-receipts/:id/confirm - 确认收回单（draft → confirmed）
pub async fn confirm_outsourcing_receipt(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_receipt::Model>>, AppError> {
    let model = receipt_service(&state).confirm(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// PUT /api/v1/erp/outsourcing-receipts/:id - 更新委外收回单
pub async fn update_outsourcing_receipt(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateOutsourcingReceiptRequest>,
) -> Result<Json<ApiResponse<outsourcing_receipt::Model>>, AppError> {
    let model = receipt_service(&state).update(id, req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/outsourcing-receipts/:id - 软删除委外收回单（仅 draft 可删）
pub async fn delete_outsourcing_receipt(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    receipt_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// 委外会计分录凭证 Handler
// ============================================================================

/// GET /api/v1/erp/outsourcing-vouchers - 分页查询委外凭证
pub async fn list_outsourcing_vouchers(
    State(state): State<AppState>,
    Query(q): Query<OutsourcingVoucherListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<outsourcing_voucher::Model>>>, AppError> {
    let page = q.page.unwrap_or(1);
    let page_size = q.page_size.unwrap_or(20);
    let query = OutsourcingVoucherQuery {
        outsourcing_order_id: q.outsourcing_order_id,
        voucher_type: q.voucher_type,
        is_posted: q.is_posted,
        voucher_date_from: q.voucher_date_from,
        voucher_date_to: q.voucher_date_to,
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = voucher_service(&state).list(query).await?;
    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/outsourcing-vouchers - 创建委外凭证
pub async fn create_outsourcing_voucher(
    State(state): State<AppState>,
    Json(req): Json<CreateOutsourcingVoucherRequest>,
) -> Result<Json<ApiResponse<outsourcing_voucher::Model>>, AppError> {
    let model = voucher_service(&state).create(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// GET /api/v1/erp/outsourcing-vouchers/by-no/:no - 按凭证号查询
pub async fn get_outsourcing_voucher_by_no(
    State(state): State<AppState>,
    Path(no): Path<String>,
) -> Result<Json<ApiResponse<outsourcing_voucher::Model>>, AppError> {
    let model = voucher_service(&state).get_by_no(&no).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// POST /api/v1/erp/outsourcing-vouchers/:id/post - 过账凭证
pub async fn post_outsourcing_voucher(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<outsourcing_voucher::Model>>, AppError> {
    let model = voucher_service(&state).post(id).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// DELETE /api/v1/erp/outsourcing-vouchers/:id - 删除委外凭证（未过账可删）
pub async fn delete_outsourcing_voucher(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    voucher_service(&state).delete(id).await?;
    Ok(Json(ApiResponse::success(())))
}
