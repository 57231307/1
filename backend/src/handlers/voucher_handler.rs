//! 凭证管理 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, warn};

use crate::middleware::auth_context::AuthContext;
use crate::models::voucher;
use crate::services::voucher_service::{
    CreateVoucherRequest, VoucherItemRequest, VoucherQueryParams, VoucherService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use rust_decimal::Decimal;

/// 查询参数
#[derive(Debug, Deserialize)]
pub struct VoucherQuery {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVoucherRequestDto {
    pub voucher_type: String,
    pub voucher_date: String,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoucherItemDto {
    pub line_no: i32,
    pub subject_code: String,
    pub subject_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub summary: Option<String>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
}

/// 查询凭证列表
pub async fn list_vouchers(
    Query(params): Query<VoucherQuery>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<voucher::Model>>>, (StatusCode, String)> {
    info!("用户 {} 查询凭证列表", auth.username);

    let service = VoucherService::new(db);
    let query_params = VoucherQueryParams {
        voucher_type: params.voucher_type,
        status: params.status,
        start_date: params.start_date.and_then(|d| d.parse().ok()),
        end_date: params.end_date.and_then(|d| d.parse().ok()),
        batch_no: params.batch_no,
        color_no: params.color_no,
        page: params.page,
        page_size: params.page_size,
    };

    let (vouchers, total) = service
        .get_list(query_params)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    info!("用户 {} 查询凭证成功，共 {} 条", auth.username, total);

    Ok(Json(ApiResponse::success(vouchers)))
}

/// 获取凭证详情
pub async fn get_voucher(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, (StatusCode, String)> {
    info!("用户 {} 查询凭证详情 ID: {}", auth.username, id);

    let service = VoucherService::new(db);
    let detail = service
        .get_by_id(id)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    info!(
        "用户 {} 查询凭证成功：{}",
        auth.username, detail.voucher.voucher_no
    );

    Ok(Json(ApiResponse::success(detail.voucher)))
}

/// 创建凭证
#[axum::debug_handler]
pub async fn create_voucher(
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
    Json(req): Json<CreateVoucherRequestDto>,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 创建凭证：{}", auth.username, req.voucher_type);

    let voucher_date = req.voucher_date.parse().map_err(|e| {
        warn!("用户 {} 凭证日期格式错误：{}", auth.username, e);
        AppError::ValidationError(format!("凭证日期格式错误：{}", e))
    })?;

    let items: Vec<VoucherItemRequest> = req
        .items
        .into_iter()
        .map(|item| VoucherItemRequest {
            line_no: item.line_no,
            subject_code: item.subject_code,
            subject_name: item.subject_name,
            debit: item.debit,
            credit: item.credit,
            summary: item.summary,
            assist_customer_id: None,
            assist_supplier_id: None,
            assist_department_id: None,
            assist_employee_id: None,
            assist_project_id: None,
            assist_batch_id: item.assist_batch_id,
            assist_color_no_id: item.assist_color_no_id,
            assist_dye_lot_id: None,
            assist_grade: None,
            assist_workshop_id: None,
            quantity_meters: item.quantity_meters,
            quantity_kg: item.quantity_kg,
            unit_price: None,
        })
        .collect();

    let create_req = CreateVoucherRequest {
        voucher_type: req.voucher_type,
        voucher_date,
        source_type: req.source_type,
        source_module: req.source_module,
        source_bill_id: req.source_bill_id,
        source_bill_no: req.source_bill_no,
        batch_no: req.batch_no,
        color_no: req.color_no,
        items,
    };

    let service = VoucherService::new(db);
    let voucher = service
        .create(create_req, auth.user_id)
        .await?;
    info!(
        "用户 {} 创建凭证成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证创建成功",
    )))
}

/// 提交凭证
pub async fn submit_voucher(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, (StatusCode, String)> {
    info!("用户 {} 提交凭证 ID: {}", auth.username, id);

    let service = VoucherService::new(db);
    let voucher = service
        .submit(id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    info!(
        "用户 {} 提交凭证成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证提交成功",
    )))
}

/// 审核凭证
pub async fn review_voucher(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, (StatusCode, String)> {
    info!("用户 {} 审核凭证 ID: {}", auth.username, id);

    let service = VoucherService::new(db);
    let voucher = service
        .review(id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    info!(
        "用户 {} 审核凭证成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证审核成功",
    )))
}

/// 凭证过账
pub async fn post_voucher(
    Path(id): Path<i32>,
    State(db): State<Arc<sea_orm::DatabaseConnection>>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, (StatusCode, String)> {
    info!("用户 {} 凭证过账 ID: {}", auth.username, id);

    let service = VoucherService::new(db);
    let voucher = service
        .post(id, auth.user_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    info!(
        "用户 {} 凭证过账成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证过账成功",
    )))
}
