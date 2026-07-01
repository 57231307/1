//! MRP Handler
//!
//! MRP物料需求计划API端点

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::services::mrp_engine_service::{
    MaterialRequirement, MrpCalculationItem, MrpCalculationRequest, MrpEngineService,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// MRP计算请求
#[derive(Debug, Deserialize, Validate)]
pub struct MrpCalculatePayload {
    #[validate(length(min = 1, message = "计算项不能为空"))]
    pub items: Vec<MrpCalculateItemPayload>,
    pub source_type: Option<String>,
    pub source_id: Option<i32>,
    pub consider_safety_stock: Option<bool>,
    pub consider_in_transit: Option<bool>,
}

/// MRP计算项
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct MrpCalculateItemPayload {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
}

/// MRP结果响应
#[derive(Debug, Serialize)]
pub struct MrpResultResponse {
    pub id: i32,
    pub calculation_no: String,
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: Option<NaiveDate>,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub planned_order_quantity: Option<Decimal>,
    pub planned_order_date: Option<NaiveDate>,
    pub status: String,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// MRP计算结果摘要响应
#[derive(Debug, Serialize)]
pub struct MrpCalculationSummaryResponse {
    pub calculation_no: String,
    pub total_items: i32,
    pub items_with_shortage: i32,
    pub results: Vec<MrpResultResponse>,
    pub requirements: Vec<MaterialRequirementResponse>,
}

/// 物料需求响应
#[derive(Debug, Serialize)]
pub struct MaterialRequirementResponse {
    pub product_id: i32,
    pub required_quantity: Decimal,
    pub required_date: NaiveDate,
    pub on_hand_quantity: Decimal,
    pub in_transit_quantity: Decimal,
    pub safety_stock: Decimal,
    pub available_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub source_type: String,
    pub source_id: Option<i32>,
    pub bom_level: i32,
}

/// MRP结果查询参数
#[derive(Debug, Deserialize)]
pub struct MrpResultQuery {
    pub calculation_no: Option<String>,
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 物料需求查询参数
#[derive(Debug, Deserialize)]
pub struct MrpRequirementQuery {
    pub product_id: Option<i32>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub only_shortage: Option<bool>,
}

/// MRP产品查询参数
#[derive(Debug, Deserialize)]
pub struct MrpProductQuery {
    pub keyword: Option<String>,
}

/// 订单转换请求
#[derive(Debug, Deserialize, Validate)]
pub struct ConvertOrderPayload {
    #[validate(length(min = 1, message = "结果ID不能为空"))]
    pub result_ids: Vec<i32>,
    #[validate(length(min = 1, message = "订单类型不能为空"))]
    pub order_type: String,
}

fn to_result_response(model: &crate::models::mrp_result::Model) -> MrpResultResponse {
    MrpResultResponse {
        id: model.id,
        calculation_no: model.calculation_no.clone(),
        product_id: model.product_id,
        required_quantity: model.required_quantity,
        required_date: model.required_date,
        source_type: model.source_type.clone(),
        source_id: model.source_id,
        planned_order_quantity: model.planned_order_quantity,
        planned_order_date: model.planned_order_date,
        status: model.status.clone(),
        remarks: model.remarks.clone(),
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

fn to_requirement_response(req: &MaterialRequirement) -> MaterialRequirementResponse {
    MaterialRequirementResponse {
        product_id: req.product_id,
        required_quantity: req.required_quantity,
        required_date: req.required_date,
        on_hand_quantity: req.on_hand_quantity,
        in_transit_quantity: req.in_transit_quantity,
        safety_stock: req.safety_stock,
        available_quantity: req.available_quantity,
        shortage_quantity: req.shortage_quantity,
        source_type: req.source_type.clone(),
        source_id: req.source_id,
        bom_level: req.bom_level,
    }
}

/// 触发MRP计算
pub async fn calculate_mrp(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<MrpCalculatePayload>,
) -> Result<Json<ApiResponse<MrpCalculationSummaryResponse>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = MrpEngineService::new(state.db.clone());

    let items: Vec<MrpCalculationItem> = payload
        .items
        .into_iter()
        .map(|item| MrpCalculationItem {
            product_id: item.product_id,
            required_quantity: item.required_quantity,
            required_date: item.required_date,
        })
        .collect();

    let request = MrpCalculationRequest {
        items,
        source_type: payload
            .source_type
            .unwrap_or_else(|| "FORECAST".to_string()),
        source_id: payload.source_id,
        consider_safety_stock: payload.consider_safety_stock.unwrap_or(true),
        consider_in_transit: payload.consider_in_transit.unwrap_or(true),
    };

    let summary = service.batch_calculate(request).await?;

    let response = MrpCalculationSummaryResponse {
        calculation_no: summary.calculation_no,
        total_items: summary.total_items,
        items_with_shortage: summary.items_with_shortage,
        results: summary.results.iter().map(to_result_response).collect(),
        requirements: summary
            .requirements
            .iter()
            .map(to_requirement_response)
            .collect(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// 查询MRP计算结果
pub async fn get_mrp_results(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<MrpResultQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<MrpResultResponse>>>, AppError> {
    let service = MrpEngineService::new(state.db.clone());

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let (results, total) = service
        .get_results(
            query.calculation_no,
            query.product_id,
            query.status,
            page,
            page_size,
        )
        .await?;

    let responses: Vec<MrpResultResponse> = results.iter().map(to_result_response).collect();

    Ok(Json(ApiResponse::success_paginated(
        responses, total, page, page_size,
    )))
}

/// 获取物料需求清单
pub async fn get_mrp_requirements(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<MrpRequirementQuery>,
) -> Result<Json<ApiResponse<Vec<MaterialRequirementResponse>>>, AppError> {
    let service = MrpEngineService::new(state.db.clone());

    let requirements = service
        .get_requirements(
            query.product_id,
            query.date_from,
            query.date_to,
            query.only_shortage.unwrap_or(false),
        )
        .await?;

    let responses: Vec<MaterialRequirementResponse> =
        requirements.iter().map(to_requirement_response).collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 将MRP需求转为采购/生产订单
pub async fn convert_to_orders(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<ConvertOrderPayload>,
) -> Result<Json<ApiResponse<Vec<MrpResultResponse>>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    if payload.order_type != "PURCHASE" && payload.order_type != "PRODUCTION" {
        return Err(AppError::validation(
            "订单类型必须是 PURCHASE 或 PRODUCTION".to_string(),
        ));
    }

    let service = MrpEngineService::new(state.db.clone());

    let results = service
        .convert_to_orders(payload.result_ids, payload.order_type)
        .await?;

    let responses: Vec<MrpResultResponse> = results.iter().map(to_result_response).collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 列出可用于 MRP 计算的产品
pub async fn list_products_for_mrp(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<MrpProductQuery>,
) -> Result<Json<ApiResponse<Vec<crate::models::product::Model>>>, AppError> {
    let service = MrpEngineService::new(state.db.clone());
    let products = service.list_products_for_mrp(query.keyword).await?;
    Ok(Json(ApiResponse::success(products)))
}

/// 取消 MRP 计算
pub async fn cancel_calculation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<MrpResultResponse>>, AppError> {
    let service = MrpEngineService::new(state.db.clone());
    let result = service.cancel_calculation(id).await?;
    Ok(Json(ApiResponse::success(to_result_response(&result))))
}

/// 导出 MRP 计算结果为 CSV
pub async fn export_calculation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<axum::response::Response, AppError> {
    let service = MrpEngineService::new(state.db.clone());
    let bytes = service.export_calculation(id).await?;
    axum::response::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .body(axum::body::Body::from(bytes))
        .map_err(|e| AppError::internal(format!("导出响应构建失败: {e}")))
}

/// 获取 MRP 计算中某物料的明细
pub async fn get_material_detail(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path((calculation_id, material_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = MrpEngineService::new(state.db.clone());
    let detail = service
        .get_material_detail(calculation_id, material_id)
        .await?;
    Ok(Json(ApiResponse::success(detail)))
}
