//! 出口退税（免抵退）handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::export_customs_declaration::Model as CustomsModel;
use crate::models::export_refund_declaration::Model as RefundModel;
use crate::services::export_refund_service::{
    CreateCustomsDeclarationRequest, ExportRefundService, RefundCalculationInput,
    RefundCalculationResult,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 出参定型（原先一律 `serde_json::Value` 动态透出）：
/// 响应形状必须可被静态契约门禁比对，且 `to_value` 失败会变成无意义的 500 来源。
#[derive(Debug, Serialize)]
pub struct DocumentCompletenessResult {
    pub documents_complete: bool,
    pub sales_order_id: i32,
}

/// 查询参数：退税申报期间（可选）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RefundPeriodQuery {
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

/// 生成出口退税申报表请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct GenerateRefundDeclarationRequest {
    pub period_year: i32,
    pub period_month: i32,
    pub refund_rate: Decimal,
    pub input_vat_amount: Decimal,
    pub carryforward_from_prev: Decimal,
}

/// 创建出口报关单
pub async fn create_customs_declaration(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateCustomsDeclarationRequest>,
) -> Result<Json<ApiResponse<CustomsModel>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = ExportRefundService::new(state.db.clone());
    let model = service.create_customs_declaration(req).await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 校验单证齐全（报关单 + 核销单）
pub async fn verify_documents_completeness(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(sales_order_id): Path<i32>,
) -> Result<Json<ApiResponse<DocumentCompletenessResult>>, AppError> {
    let service = ExportRefundService::new(state.db.clone());
    let complete = service
        .verify_documents_completeness(sales_order_id)
        .await?;
    Ok(Json(ApiResponse::success(DocumentCompletenessResult {
        documents_complete: complete,
        sales_order_id,
    })))
}

/// 计算免抵退税额（纯函数静态端点，无需数据库）
pub async fn calculate_refund(
    _auth: AuthContext,
    Json(input): Json<RefundCalculationInput>,
) -> Result<Json<ApiResponse<RefundCalculationResult>>, AppError> {
    let result = ExportRefundService::calculate_exempt_credit_refund(&input);
    Ok(Json(ApiResponse::success(result)))
}

/// 生成出口退税申报表
pub async fn generate_refund_declaration(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<GenerateRefundDeclarationRequest>,
) -> Result<Json<ApiResponse<RefundModel>>, AppError> {
    let service = ExportRefundService::new(state.db.clone());
    let model = service
        .generate_refund_declaration(
            req.period_year,
            req.period_month,
            req.refund_rate,
            req.input_vat_amount,
            req.carryforward_from_prev,
            Some(auth.user_id),
        )
        .await?;
    Ok(Json(ApiResponse::success(model)))
}

/// 查询出口退税申报表
pub async fn list_refund_declarations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<RefundPeriodQuery>,
) -> Result<Json<ApiResponse<Vec<RefundModel>>>, AppError> {
    let service = ExportRefundService::new(state.db.clone());
    let list = service
        .list_refund_declarations(params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(list)))
}
