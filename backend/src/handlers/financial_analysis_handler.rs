//! 财务分析 Handler
//!
//! 提供财务指标查询、趋势分析、报告管理等功能

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::QueryOrder;
use serde::Deserialize;
use tracing::info;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::{financial_analysis, financial_analysis_result};
use crate::services::financial_analysis_service::{
    CreateIndicatorRequest, FinancialAnalysisService, IndicatorQueryParams,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// P1-2c 修复（批次 81 v1 复审）：创建财务指标请求 DTO
/// 替代 create_indicator 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct CreateIndicatorDto {
    /// 指标名称：必填，长度至少 1
    #[validate(length(min = 1, max = 100, message = "指标名称长度必须在1到100字符之间"))]
    pub name: String,
    /// 指标编码：可选，缺失时由 service 层自动生成
    #[validate(length(max = 50, message = "指标编码长度不能超过50字符"))]
    pub code: Option<String>,
    /// 指标类型：可选，缺失时默认 "ratio"
    #[validate(length(max = 30, message = "指标类型长度不能超过30字符"))]
    pub indicator_type: Option<String>,
    /// 公式：可选
    pub formula: Option<String>,
    /// 单位：可选
    pub unit: Option<String>,
    /// 描述/备注：可选
    pub description: Option<String>,
}

/// P1-2c 修复（批次 81 v1 复审）：创建财务趋势数据请求 DTO
/// 替代 create_trend 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTrendDto {
    /// 分析类型：可选，缺失时默认 "trend"
    #[validate(length(max = 30, message = "分析类型长度不能超过30字符"))]
    pub analysis_type: Option<String>,
    /// 周期：必填，长度至少 1（如 "2026-07"）
    #[validate(length(min = 1, max = 20, message = "周期长度必须在1到20字符之间"))]
    pub period: String,
    /// 指标 ID：必填
    pub indicator_id: i32,
    /// 指标值：必填
    pub value: rust_decimal::Decimal,
    /// 目标值：可选
    pub target_value: Option<rust_decimal::Decimal>,
}

/// 财务趋势查询参数
#[derive(Debug, Deserialize)]
pub struct TrendQueryParams {
    pub indicator_id: Option<i32>,
    #[allow(dead_code)] // TODO(tech-debt): 财务分析模块接入业务后移除
    pub start_date: Option<String>,
    #[allow(dead_code)] // TODO(tech-debt): 财务分析模块接入业务后移除
    pub end_date: Option<String>,
    #[allow(dead_code)] // TODO(tech-debt): 财务分析模块接入业务后移除
    pub period: Option<String>,
    pub page_size: Option<i64>,
}

/// 创建报告请求
#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub name: String,
    pub report_type: String,
    #[allow(dead_code)] // TODO(tech-debt): 报表创建接口接入业务后移除
    pub period_start: String,
    #[allow(dead_code)] // TODO(tech-debt): 报表创建接口接入业务后移除
    pub period_end: String,
    #[allow(dead_code)] // TODO(tech-debt): 报表创建接口接入业务后移除
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
    // v11 批次 36 修复：page_size clamp 防止 DoS（i64 无 unwrap_or，直接 clamp；负值经 as u64 会放大为 u64::MAX）
    let page_size = params.page_size.clamp(1, 100);

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
    Json(req): Json<CreateIndicatorDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2c 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = FinancialAnalysisService::new(state.db.clone());

    // code 缺失时由 service 层自动生成（保持原逻辑）
    let indicator_code = req.code.unwrap_or_else(|| {
        format!("IND_{}", chrono::Utc::now().timestamp())
    });
    let indicator_type = req.indicator_type.unwrap_or_else(|| "ratio".to_string());

    let create_req = CreateIndicatorRequest {
        indicator_name: req.name,
        indicator_code,
        indicator_type,
        formula: req.formula,
        unit: req.unit,
        remark: req.description,
    };

    let indicator = service.create_indicator(create_req, auth.user_id).await?;

    info!(
        "用户 {} 创建财务指标: {}",
        auth.username, indicator.indicator_name
    );

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

    // 指标 ID 缺失时返回 4xx 错误，避免脏 indicator_id=0 污染
    let indicator_id: i32 = params
        .indicator_id
        .ok_or_else(|| AppError::validation("财务分析请求缺少指标ID"))?;
    let limit = params.page_size.unwrap_or(50).clamp(1, 100);

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
    Json(req): Json<CreateTrendDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2c 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = FinancialAnalysisService::new(state.db.clone());

    // analysis_type 缺失时默认 "trend"（保持原逻辑）
    let analysis_type = req.analysis_type.unwrap_or_else(|| "trend".to_string());

    let analysis_req = crate::services::financial_analysis_service::FinancialAnalysisRequest {
        analysis_type,
        period: req.period,
        indicator_id: req.indicator_id,
        indicator_value: req.value,
        target_value: req.target_value,
    };

    let result = service
        .create_analysis_result(analysis_req, auth.user_id)
        .await?;

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

    let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS

    let page_size = params
        .get("page_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(20)
        .clamp(1, 100); // v11 批次 36 修复：防止 DoS

    let query_params = IndicatorQueryParams {
        page: page.saturating_sub(1),
        page_size,
        ..Default::default()
    };

    let (indicators, total) = service.get_indicators_list(query_params).await?;

    let items: Vec<serde_json::Value> = indicators
        .iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "name": i.indicator_name,
                "indicator_code": i.indicator_code,
                "indicator_type": i.indicator_type,
                "status": i.status,
                "created_at": i.created_at.to_rfc3339(),
            })
        })
        .collect();

    info!(
        "查询财务分析报告列表: page={}, page_size={}, total={}",
        page, page_size, total
    );

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

    let indicator = service
        .create_indicator(indicator_req, auth.user_id)
        .await?;

    info!("用户 {} 创建财务分析报告: {}", auth.username, req.name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
            "id": indicator.id,
            "name": indicator.indicator_name,
            "indicator_code": indicator.indicator_code,
            "indicator_type": indicator.indicator_type,
            "status": indicator.status,
            "created_at": indicator.created_at.to_rfc3339(),
        }),
        "财务分析报告创建成功",
    )))
}

/// GET /api/v1/erp/financial-analysis/reports/:id - 获取财务分析报告详情
pub async fn get_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::EntityTrait;

    let indicator = financial_analysis::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found("财务分析报告不存在"))?;

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
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let indicator = financial_analysis::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found("财务分析报告不存在"))?;

    // 查询该指标的最新分析结果
    let latest_result = financial_analysis_result::Entity::find()
        .filter(financial_analysis_result::Column::IndicatorId.eq(id))
        .order_by(
            financial_analysis_result::Column::CreatedAt,
            sea_orm::Order::Desc,
        )
        .one(state.db.as_ref())
        .await?;

    info!("用户 {} 执行财务分析报告: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
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
        }),
        "财务分析报告执行成功",
    )))
}
