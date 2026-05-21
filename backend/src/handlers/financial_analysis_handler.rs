//! 财务分析 Handler
//!
//! 提供财务指标查询、趋势分析、报告管理等功能

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::{Order, QueryOrder};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::middleware::auth_context::AuthContext;
use crate::models::{financial_analysis, financial_analysis_result};
use crate::services::financial_analysis_service::{FinancialAnalysisService, CreateIndicatorRequest, IndicatorQueryParams};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 财务趋势查询参数
#[derive(Debug, Deserialize)]
pub struct TrendQueryParams {
    pub indicator_id: Option<i32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period: Option<String>,
    pub page_size: Option<i64>,
}

/// 创建报告请求
#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub name: String,
    pub report_type: String,
    pub period_start: String,
    pub period_end: String,
    pub indicators: Option<Vec<i32>>,
    pub description: Option<String>,
}

/// GET /api/v1/erp/financial-analysis/indicators - 获取财务指标列表
pub async fn get_indicators(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<IndicatorQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let page = params.page;
    let page_size = params.page_size;

    let query = IndicatorQueryParams {
        indicator_type: params.indicator_type,
        status: params.status,
        page,
        page_size,
    };

    let (items, total) = service.get_indicators_list(query).await?;

    info!("财务指标查询成功，共 {} 条记录", total);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/financial-analysis/indicators - 创建财务指标
pub async fn create_indicator(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let indicator_name = req.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let indicator_code = req.get("code")
        .and_then(|v| v.as_str())
        .unwrap_or(&format!("IND_{}", chrono::Utc::now().timestamp()))
        .to_string();

    let indicator_type = req.get("indicator_type")
        .and_then(|v| v.as_str())
        .unwrap_or("ratio")
        .to_string();

    let formula = req.get("formula")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let unit = req.get("unit")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let remark = req.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let create_req = crate::services::financial_analysis_service::CreateIndicatorRequest {
        indicator_name,
        indicator_code,
        indicator_type,
        formula,
        unit,
        remark,
    };

    let indicator = service.create_indicator(create_req, auth.user_id).await?;

    info!("用户 {} 创建财务指标: {}", auth.username, indicator.indicator_name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(indicator)?,
        "财务指标创建成功",
    )))
}

/// GET /api/v1/erp/financial-analysis/trends - 获取财务趋势
pub async fn get_trends(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<TrendQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let indicator_id = params.indicator_id.unwrap_or(0);
    let limit = params.page_size.unwrap_or(50) as i64;

    let trends = service.get_trends(indicator_id, limit).await?;

    info!("财务趋势查询成功，共 {} 条记录", trends.len());

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": trends,
        "total": trends.len(),
    }))))
}

/// POST /api/v1/erp/financial-analysis/trends - 创建财务趋势数据
pub async fn create_trend(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let analysis_type = req.get("analysis_type")
        .and_then(|v| v.as_str())
        .unwrap_or("trend")
        .to_string();

    let period = req.get("period")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let indicator_id = req.get("indicator_id")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let indicator_value = req.get("value")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    let target_value = req.get("target_value")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default());

    let analysis_req = crate::services::financial_analysis_service::FinancialAnalysisRequest {
        analysis_type,
        period,
        indicator_id,
        indicator_value,
        target_value,
    };

    let result = service.create_analysis_result(analysis_req, auth.user_id).await?;

    info!("用户 {} 创建财务趋势数据: {}", auth.username, result.period);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(result)?,
        "财务趋势数据创建成功",
    )))
}

/// GET /api/v1/erp/financial-analysis/reports - 财务分析报告列表
pub async fn list_reports(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let page = params.get("page")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    let page_size = params.get("page_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(20);

    let query_params = IndicatorQueryParams {
        page: page - 1,
        page_size,
        ..Default::default()
    };

    let (indicators, total) = service.get_indicators_list(query_params).await?;

    let items: Vec<serde_json::Value> = indicators.iter().map(|i| {
        serde_json::json!({
            "id": i.id,
            "name": i.indicator_name,
            "indicator_code": i.indicator_code,
            "indicator_type": i.indicator_type,
            "status": i.status,
            "created_at": i.created_at.to_rfc3339(),
        })
    }).collect();

    info!("查询财务分析报告列表: page={}, page_size={}, total={}", page, page_size, total);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/financial-analysis/reports - 创建财务分析报告
pub async fn create_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateReportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = FinancialAnalysisService::new(state.db.clone());

    let indicator_req = CreateIndicatorRequest {
        indicator_name: req.name.clone(),
        indicator_code: format!("RPT-{}", chrono::Utc::now().timestamp()),
        indicator_type: req.report_type.clone(),
        formula: None,
        unit: None,
        remark: req.description,
    };

    let indicator = service.create_indicator(indicator_req, auth.user_id).await?;

    info!("用户 {} 创建财务分析报告: {}", auth.username, req.name);

    Ok(Json(ApiResponse::success_with_message(serde_json::json!({
        "id": indicator.id,
        "name": indicator.indicator_name,
        "indicator_code": indicator.indicator_code,
        "indicator_type": indicator.indicator_type,
        "status": indicator.status,
        "created_at": indicator.created_at.to_rfc3339(),
    }), "财务分析报告创建成功")))
}

/// GET /api/v1/erp/financial-analysis/reports/:id - 获取财务分析报告详情
pub async fn get_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

    let indicator = financial_analysis::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("财务分析报告不存在".to_string()))?;

    info!("获取财务分析报告详情: ID={}", id);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": indicator.id,
        "name": indicator.indicator_name,
        "indicator_code": indicator.indicator_code,
        "indicator_type": indicator.indicator_type,
        "formula": indicator.formula,
        "unit": indicator.unit,
        "status": indicator.status,
        "remark": indicator.remark,
        "created_at": indicator.created_at.to_rfc3339(),
        "updated_at": indicator.updated_at.to_rfc3339(),
    }))))
}

/// POST /api/v1/erp/financial-analysis/reports/:id/execute - 执行财务分析报告
pub async fn execute_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

    let indicator = financial_analysis::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("财务分析报告不存在".to_string()))?;

    // 查询该指标的最新分析结果
    let latest_result = financial_analysis_result::Entity::find()
        .filter(financial_analysis_result::Column::IndicatorId.eq(id))
        .order_by(financial_analysis_result::Column::CreatedAt, sea_orm::Order::Desc)
        .one(state.db.as_ref())
        .await?;

    info!("用户 {} 执行财务分析报告: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(serde_json::json!({
        "id": id,
        "name": indicator.indicator_name,
        "status": "completed",
        "latest_result": latest_result.map(|r| serde_json::json!({
            "analysis_type": r.analysis_type,
            "period": r.period,
            "indicator_value": r.indicator_value,
            "target_value": r.target_value,
            "variance": r.variance,
            "variance_rate": r.variance_rate,
            "trend": r.trend,
            "analysis_date": r.analysis_date,
        })),
        "executed_at": chrono::Utc::now().to_rfc3339(),
    }), "财务分析报告执行成功")))
}
