//! Incoterms 贸易术语 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::incoterms_service::IncotermsService;
use crate::utils::error::AppError;
use crate::utils::incoterms::Incoterms2020;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::Deserialize;

/// 查询参数：术语使用月报年月
#[derive(Debug, Deserialize)]
pub struct UsageReportQuery {
    pub year: i32,
    pub month: u32,
}

/// 按 Incoterm 计算价格构成请求
#[derive(Debug, Deserialize)]
pub struct CalculateCostsRequest {
    pub incoterm: Incoterms2020,
    pub product_cost: Decimal,
    pub freight_cost: Option<Decimal>,
    pub insurance_cost: Option<Decimal>,
    pub duty_cost: Option<Decimal>,
}

/// 获取报价单价格构成（按 Incoterm 解析）
pub async fn get_price_composition(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(quotation_id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = IncotermsService::from_state(&state);
    let result = service.get_price_composition(quotation_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// 按 Incoterm 计算价格构成各成本项（纯函数静态端点，无需数据库）
pub async fn calculate_costs(
    _auth: AuthContext,
    Json(req): Json<CalculateCostsRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let (product_cost, freight_cost, insurance_cost, duty_cost) =
        IncotermsService::calculate_costs_by_incoterm(
            req.incoterm,
            req.product_cost,
            req.freight_cost,
            req.insurance_cost,
            req.duty_cost,
        );
    Ok(Json(ApiResponse::success(serde_json::json!({
        "product_cost": product_cost,
        "freight_cost": freight_cost,
        "insurance_cost": insurance_cost,
        "duty_cost": duty_cost,
    }))))
}

/// 术语使用月报
pub async fn monthly_usage_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<UsageReportQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = IncotermsService::from_state(&state);
    let report = service
        .monthly_usage_report(params.year, params.month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(report)?)))
}
