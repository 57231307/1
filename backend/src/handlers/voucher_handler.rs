//! 凭证管理 Handler
//!
//! HTTP 接口层

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::middleware::auth_context::AuthContext;
use crate::models::voucher;
use crate::services::voucher_service::{
    CreateVoucherRequest, UpdateVoucherRequest, VoucherItemRequest, VoucherQueryParams,
    VoucherService,
};
use crate::utils::app_state::AppState;
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

pub struct VoucherItemDto {
    pub line_no: Option<i32>,
    pub subject_code: Option<String>,
    pub subject_name: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub summary: Option<String>,
    pub assist_customer_id: Option<i32>,
    pub assist_supplier_id: Option<i32>,
    pub assist_department_id: Option<i32>,
    pub assist_employee_id: Option<i32>,
    pub assist_project_id: Option<i32>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub assist_dye_lot_id: Option<i32>,
    pub assist_grade: Option<String>,
    pub assist_workshop_id: Option<i32>,
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,
}

/// 查询凭证列表
pub async fn list_vouchers(
    Query(params): Query<VoucherQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<voucher::Model>>>, AppError> {
    info!("用户 {} 查询凭证列表", auth.username);

    let service = VoucherService::new(state.db.clone());
    let query_params = VoucherQueryParams {
        voucher_type: params.voucher_type,
        status: params.status,
        start_date: params.start_date.and_then(|d| d.parse().ok()),
        end_date: params.end_date.and_then(|d| d.parse().ok()),
        batch_no: params.batch_no,
        color_no: params.color_no,
        page: params.page,
        // v10 P1-1 修复：page_size clamp(1,100) 防 DoS
        page_size: params.page_size.map(|ps| ps.clamp(1, 100)),
    };

    let (vouchers, total) = service
        .get_list(query_params)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;
    info!("用户 {} 查询凭证成功，共 {} 条", auth.username, total);

    Ok(Json(ApiResponse::success(vouchers)))
}

/// 获取凭证详情
pub async fn get_voucher(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 查询凭证详情 ID: {}", auth.username, id);

    let service = VoucherService::new(state.db.clone());
    let detail = service
        .get_by_id(id)
        .await
        .map_err(|e| AppError::not_found(e.to_string()))?;
    info!(
        "用户 {} 查询凭证成功：{}",
        auth.username, detail.voucher.voucher_no
    );

    Ok(Json(ApiResponse::success(detail.voucher)))
}

/// 创建凭证
#[axum::debug_handler]
pub async fn create_voucher(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateVoucherRequestDto>,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 创建凭证：{}", auth.username, req.voucher_type);

    let voucher_date = req.voucher_date.parse().map_err(|e| {
        warn!("用户 {} 凭证日期格式错误：{}", auth.username, e);
        AppError::validation(format!("凭证日期格式错误：{}", e))
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
            assist_customer_id: item.assist_customer_id,
            assist_supplier_id: item.assist_supplier_id,
            assist_department_id: item.assist_department_id,
            assist_employee_id: item.assist_employee_id,
            assist_project_id: item.assist_project_id,
            assist_batch_id: item.assist_batch_id,
            assist_color_no_id: item.assist_color_no_id,
            assist_dye_lot_id: item.assist_dye_lot_id,
            assist_grade: item.assist_grade,
            assist_workshop_id: item.assist_workshop_id,
            quantity_meters: item.quantity_meters,
            quantity_kg: item.quantity_kg,
            unit_price: item.unit_price,
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

    let service = VoucherService::new(state.db.clone());
    let voucher = service.create(create_req, auth.user_id).await?;
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
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 提交凭证 ID: {}", auth.username, id);

    let service = VoucherService::new(state.db.clone());
    let voucher = service
        .submit(id, auth.user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
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
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 审核凭证 ID: {}", auth.username, id);

    let service = VoucherService::new(state.db.clone());
    let voucher = service
        .review(id, auth.user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
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
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 凭证过账 ID: {}", auth.username, id);

    let service = VoucherService::new(state.db.clone());
    let voucher = service
        .post(id, auth.user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    info!(
        "用户 {} 凭证过账成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证过账成功",
    )))
}

/// 获取凭证类型列表
pub async fn get_voucher_types() -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let types = vec![
        serde_json::json!({"code": "记", "name": "记账凭证"}),
        serde_json::json!({"code": "收", "name": "收款凭证"}),
        serde_json::json!({"code": "付", "name": "付款凭证"}),
        serde_json::json!({"code": "转", "name": "转账凭证"}),
    ];
    Json(ApiResponse::success(types))
}

/// 生成凭证编号
pub async fn generate_voucher_no(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::voucher;
    use crate::utils::number_generator::DocumentNumberGenerator;

    let voucher_no = DocumentNumberGenerator::generate_no(
        &*state.db,
        "JZ",
        voucher::Entity,
        voucher::Column::VoucherNo,
    )
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "voucher_no": voucher_no
    }))))
}

/// 更新凭证请求 DTO
#[derive(Debug, serde::Deserialize)]
pub struct UpdateVoucherRequestDto {
    pub voucher_type: Option<String>,
    pub voucher_date: Option<String>,
    pub items: Option<Vec<VoucherItemDto>>,
}

/// 更新凭证
pub async fn update_voucher(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateVoucherRequestDto>,
) -> Result<Json<ApiResponse<voucher::Model>>, AppError> {
    info!("用户 {} 更新凭证 ID: {}", auth.username, id);

    let voucher_date = match req.voucher_date {
        Some(d) => Some(
            d.parse()
                .map_err(|e| AppError::validation(format!("凭证日期格式错误：{}", e)))?,
        ),
        None => None,
    };

    let items = req.items.map(|v| {
        v.into_iter()
            .map(|item| VoucherItemRequest {
                line_no: item.line_no,
                subject_code: item.subject_code,
                subject_name: item.subject_name,
                debit: item.debit,
                credit: item.credit,
                summary: item.summary,
                assist_customer_id: item.assist_customer_id,
                assist_supplier_id: item.assist_supplier_id,
                assist_department_id: item.assist_department_id,
                assist_employee_id: item.assist_employee_id,
                assist_project_id: item.assist_project_id,
                assist_batch_id: item.assist_batch_id,
                assist_color_no_id: item.assist_color_no_id,
                assist_dye_lot_id: item.assist_dye_lot_id,
                assist_grade: item.assist_grade,
                assist_workshop_id: item.assist_workshop_id,
                quantity_meters: item.quantity_meters,
                quantity_kg: item.quantity_kg,
                unit_price: item.unit_price,
            })
            .collect::<Vec<_>>()
    });

    let update_req = UpdateVoucherRequest {
        voucher_type: req.voucher_type,
        voucher_date,
        items,
    };

    let service = VoucherService::new(state.db.clone());
    let voucher = service.update(id, update_req, auth.user_id).await?;
    info!(
        "用户 {} 更新凭证成功：{}",
        auth.username, voucher.voucher_no
    );

    Ok(Json(ApiResponse::success_with_message(
        voucher,
        "凭证更新成功",
    )))
}

/// 删除凭证
pub async fn delete_voucher(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除凭证 ID: {}", auth.username, id);

    let service = VoucherService::new(state.db.clone());
    service.delete(id, auth.user_id).await?;
    info!("用户 {} 删除凭证成功", auth.username);

    Ok(Json(ApiResponse::success_with_message((), "凭证删除成功")))
}
