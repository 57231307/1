//! 缺料预警 Handler
//!
//! 提供缺料预警列表、手动触发检查、缺料汇总等 API 接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::material_shortage as alert_model;
use crate::models::status::purchase_inventory::shortage_alert_status;
use crate::services::material_shortage_service::{
    MaterialShortageService, ShortageAlertView, ShortageCheckRequest, ShortageLevel,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 枚举入参校验：返回 legal 中的规范取值（大小写不敏感匹配），
/// 不在 legal 列表内直接 400 报出合法值——既避免非法筛选值被当成
/// 「查无数据」静默返回空列表（假控件），也避免非规范写法被原样落库
fn validate_enum_param<'a>(
    raw: &str,
    legal: &'a [&'a str],
    field: &str,
) -> Result<&'a str, AppError> {
    legal
        .iter()
        .find(|v| v.eq_ignore_ascii_case(raw))
        .copied()
        .ok_or_else(|| {
            AppError::validation(format!(
                "无效的{}：{}（允许值：{}）",
                field,
                raw,
                legal.join(" / ")
            ))
        })
}

/// 缺料级别合法取值（与 ShortageLevel 变体同源，落库值一致）
fn shortage_level_names() -> Vec<&'static str> {
    ShortageLevel::ALL.iter().map(|l| l.as_str()).collect()
}

/// 缺料预警状态更新请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

/// 缺料预警列表查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ShortageAlertParams {
    pub level: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缺料汇总查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ShortageSummaryParams {
    pub product_ids: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

/// GET /api/v1/erp/material-shortage/alerts - 缺料预警列表
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn list_shortage_alerts(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ShortageAlertParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<ShortageAlertView>>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "缺料预警列表查询");
    let service = MaterialShortageService::new(state.db.clone());

    let level_names = shortage_level_names();
    // 空串是「未选择该筛选项」，与其余列表接口口径一致；非空才校验取值域
    let level = params
        .level
        .as_deref()
        .filter(|raw| !raw.is_empty())
        .map(|raw| validate_enum_param(raw, &level_names, "缺料级别"))
        .transpose()?;
    let status = params
        .status
        .as_deref()
        .filter(|raw| !raw.is_empty())
        .map(|raw| validate_enum_param(raw, shortage_alert_status::ALL, "缺料预警状态"))
        .transpose()?;

    // 页码 1-based 传给 service（service 内换算 offset），clamp 保证 page=0 不会被接受
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service.list_alerts(level, status, page, page_size).await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

/// POST /api/v1/erp/material-shortage/check - 手动触发缺料检查
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn check_material_shortage(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<ShortageCheckRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "手动触发缺料检查");
    let service = MaterialShortageService::new(state.db.clone());
    let summary = service.detect_shortages(payload).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(summary)?)))
}

/// GET /api/v1/erp/material-shortage/summary - 缺料汇总
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_shortage_summary(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ShortageSummaryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "缺料汇总查询");
    let service = MaterialShortageService::new(state.db.clone());

    let product_ids = params
        .product_ids
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse::<i32>().ok())
                .collect::<Vec<i32>>()
        })
        .filter(|v| !v.is_empty());

    let request = ShortageCheckRequest {
        product_ids,
        date_from: params
            .date_from
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        date_to: params
            .date_to
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        threshold: None,
    };

    let summary = service.detect_shortages(request).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(summary)?)))
}

/// 保存预警阈值配置请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct SaveThresholdConfigRequest {
    pub safety_factor: Option<rust_decimal::Decimal>,
    pub critical_threshold: Option<rust_decimal::Decimal>,
    pub severe_threshold: Option<rust_decimal::Decimal>,
}

/// POST /api/v1/erp/material-shortage/threshold - 保存预警阈值配置
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn save_threshold_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<SaveThresholdConfigRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "保存缺料预警阈值配置");
    let service = MaterialShortageService::new(state.db.clone());

    // 加载现有配置
    let mut config = service.load_threshold_config().await?;

    // 更新配置
    if let Some(safety_factor) = payload.safety_factor {
        config.safety_factor = safety_factor;
    }
    if let Some(critical_threshold) = payload.critical_threshold {
        config.critical_threshold = critical_threshold;
    }
    if let Some(severe_threshold) = payload.severe_threshold {
        config.severe_threshold = severe_threshold;
    }

    service.save_threshold_config(&config).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(config)?,
        "预警阈值配置已保存",
    )))
}

/// GET /api/v1/erp/material-shortage/threshold - 获取预警阈值配置
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_threshold_config(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "获取缺料预警阈值配置");
    let service = MaterialShortageService::new(state.db.clone());

    let config = service.load_threshold_config().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(config)?)))
}

/// GET /api/v1/erp/material-shortage/replenishment - 获取补货建议
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn get_replenishment_suggestions(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ShortageSummaryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "获取缺料补货建议");
    let service = MaterialShortageService::new(state.db.clone());

    let product_ids = params
        .product_ids
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse::<i32>().ok())
                .collect::<Vec<i32>>()
        })
        .filter(|v| !v.is_empty());

    let request = ShortageCheckRequest {
        product_ids,
        date_from: params
            .date_from
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        date_to: params
            .date_to
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        threshold: None,
    };

    let summary = service.detect_shortages(request).await?;
    let suggestions = service
        .generate_replenishment_suggestions(&summary.items)
        .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "suggestions": suggestions,
        "total": suggestions.len(),
    }))))
}

/// 缺料月报查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct MonthlyReportParams {
    pub year: Option<i32>,
    pub month: Option<u32>,
}

/// 缺料月报数据
#[derive(Debug, Serialize)]
#[allow(dead_code, reason = "序列化输出字段")]
pub struct MonthlyShortageReport {
    pub year: i32,
    pub month: u32,
    pub total_alerts: i64,
    pub critical_count: i64,
    pub severe_count: i64,
    pub warning_count: i64,
    pub resolved_count: i64,
    pub top_shortage_materials: Vec<TopShortageMaterial>,
    pub status_distribution: Vec<StatusDistribution>,
}

/// 缺料 Top 物料
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct TopShortageMaterial {
    pub material_id: i32,
    pub material_name: String,
    pub material_code: String,
    pub shortage_quantity: Decimal,
    pub alert_count: i64,
}

/// 状态分布
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize)]
pub struct StatusDistribution {
    pub status: String,
    pub count: i64,
}

/// GET /api/v1/erp/material-shortage/report/monthly - 缺料月报
pub async fn get_monthly_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<MonthlyReportParams>,
) -> Result<Json<ApiResponse<MonthlyShortageReport>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "缺料月报查询");
    let service = MaterialShortageService::new(state.db.clone());

    use chrono::Datelike;
    let now = chrono::Utc::now();
    let year = params.year.unwrap_or_else(|| now.year());
    let month = params.month.unwrap_or_else(|| now.month());

    let report = service.get_monthly_report(year, month).await?;

    Ok(Json(ApiResponse::success(report)))
}

/// PUT /api/v1/erp/material-shortage/:id/status - 更新缺料预警状态
///
/// `:id` 语义为 material_id（同物料至多一条未解决预警，见 persist_alerts）。
/// 状态取值只能落在 shortage_alert_status::ALL（identified → purchase_request →
/// purchase_order → received → resolved），出参为该条 alert 的持久化快照。
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
pub async fn update_shortage_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(material_id): Path<i32>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<alert_model::Model>>, AppError> {
    tracing::debug!(
        user_id = auth.user_id,
        material_id = material_id,
        "更新缺料预警状态"
    );
    let status = validate_enum_param(&req.status, shortage_alert_status::ALL, "缺料预警状态")?;

    let service = MaterialShortageService::new(state.db.clone());
    // service 返回更新后的 alert 快照，直接作为出参（字段与列表视图同源，不再派生别名字段）
    let alert = service.update_status(material_id, status).await?;

    Ok(Json(ApiResponse::success_with_message(
        alert,
        "缺料状态已更新",
    )))
}
