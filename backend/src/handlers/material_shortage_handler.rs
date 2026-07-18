//! 缺料预警 Handler
//!
//! 提供缺料预警列表、手动触发检查、缺料汇总等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::material_shortage_service::{MaterialShortageService, ShortageCheckRequest};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 缺料预警状态更新请求
#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

/// 缺料预警数据传输对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialShortageDto {
    pub id: i32,
    pub material_code: String,
    pub material_name: String,
    pub spec: Option<String>,
    pub current_stock: Decimal,
    pub required_quantity: Decimal,
    pub shortage_quantity: Decimal,
    pub unit: Option<String>,
    pub expected_date: Option<String>,
    pub source_type: Option<String>,
    pub source_no: Option<String>,
    pub status: String,
    pub severity: String,
}

/// 缺料预警列表查询参数
#[derive(Debug, Deserialize)]
pub struct ShortageAlertParams {
    pub level: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 缺料汇总查询参数
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
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    tracing::debug!(user_id = auth.user_id, "缺料预警列表查询");
    let service = MaterialShortageService::new(state.db.clone());

    // 批次 95 P3-3~8 修复：max(1) 保证页码 >=1（防止 page=0 被接受），saturating_sub(1) 转 0-based offset
    let page = params.page.unwrap_or(1).clamp(1, 1000).saturating_sub(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (items, total) = service
        .list_alerts(params.level.as_deref(), page, page_size)
        .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page + 1,
        "page_size": page_size,
    }))))
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

/// PUT /api/v1/erp/material-shortage/:id/status - 更新缺料预警状态
// 批次 94 P2-8 修复：_auth → auth，记录鉴权审计日志（避免 unused 警告）
// V15 P0-B15（Batch 484）：状态值与 migration m0068 状态机对齐
//   identified → purchase_request → purchase_order → received → resolved
// 并从持久化 alert 读取完整 DTO（替代原桩实现返回零值字段）
pub async fn update_shortage_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<MaterialShortageDto>>, AppError> {
    tracing::debug!(user_id = auth.user_id, id = id, "更新缺料预警状态");
    // 校验状态值（V15 P0-B15：与 migration m0068 状态机一致）
    let valid = matches!(
        req.status.as_str(),
        "identified" | "purchase_request" | "purchase_order" | "received" | "resolved"
    );
    if !valid {
        return Err(AppError::validation(format!(
            "无效的缺料状态：{}（允许值：identified / purchase_request / purchase_order / received / resolved）",
            req.status
        )));
    }

    let service = MaterialShortageService::new(state.db.clone());
    // V15 P0-B15：service 返回更新后的 alert 快照，handler 据此构建完整 DTO
    let alert = service.update_status(id, &req.status).await?;

    // level（Critical/Severe/Warning/Normal）→ severity（critical/high/medium/low）
    let severity = match alert.level.as_str() {
        "Critical" => "critical",
        "Severe" => "high",
        "Warning" => "medium",
        _ => "low",
    }
    .to_string();

    let dto = MaterialShortageDto {
        id: alert.material_id,
        material_code: alert.material_code.unwrap_or_default(),
        material_name: alert.material_name,
        spec: None,
        current_stock: alert.available_quantity,
        required_quantity: alert.required_quantity,
        shortage_quantity: alert.shortage_quantity,
        unit: alert.unit,
        expected_date: None,
        source_type: None,
        source_no: None,
        status: alert.status,
        severity,
    };

    Ok(Json(ApiResponse::success_with_message(
        dto,
        "缺料状态已更新",
    )))
}
