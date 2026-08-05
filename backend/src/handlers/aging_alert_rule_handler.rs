//! 账龄预警规则 Handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::aging_alert_rule;
use crate::services::aging_alert_rule_service::{
    AgingAlertRuleService, AlertRuleQueryParams, CreateAlertRuleRequest, UpdateAlertRuleRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::info;

/// 查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct AlertRuleQuery {
    pub aging_bucket: Option<String>,
    pub alert_level: Option<String>,
    pub is_active: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateAlertRuleDto {
    pub rule_name: String,
    pub rule_code: String,
    pub aging_bucket: String,
    pub threshold_days: i32,
    pub threshold_amount: Option<rust_decimal::Decimal>,
    pub alert_level: String,
    pub notify_method: String,
    pub notify_roles: Option<Vec<String>>,
    pub is_active: Option<bool>,
    pub remarks: Option<String>,
}

/// 更新请求 DTO
#[derive(Debug, Deserialize)]
pub struct UpdateAlertRuleDto {
    pub rule_name: Option<String>,
    pub aging_bucket: Option<String>,
    pub threshold_days: Option<i32>,
    pub threshold_amount: Option<Option<rust_decimal::Decimal>>,
    pub alert_level: Option<String>,
    pub notify_method: Option<String>,
    pub notify_roles: Option<Option<Vec<String>>>,
    pub is_active: Option<bool>,
    pub remarks: Option<Option<String>>,
}

/// 创建预警规则
pub async fn create_rule(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateAlertRuleDto>,
) -> Result<Json<ApiResponse<aging_alert_rule::Model>>, AppError> {
    info!(
        "用户 {} 正在创建账龄预警规则：{}",
        auth.user_id, req.rule_name
    );

    let service = AgingAlertRuleService::new(state.db.clone());
    let rule = service
        .create(
            CreateAlertRuleRequest {
                rule_name: req.rule_name,
                rule_code: req.rule_code,
                aging_bucket: req.aging_bucket,
                threshold_days: req.threshold_days,
                threshold_amount: req.threshold_amount,
                alert_level: req.alert_level,
                notify_method: req.notify_method,
                notify_roles: req.notify_roles,
                is_active: req.is_active,
                remarks: req.remarks,
            },
            auth.user_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(rule)))
}

/// 查询预警规则列表
pub async fn list_rules(
    Query(params): Query<AlertRuleQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<PaginatedResponse<aging_alert_rule::Model>>>, AppError> {
    info!("用户 {} 正在查询账龄预警规则列表", auth.user_id);

    let service = AgingAlertRuleService::new(state.db.clone());
    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);
    let (rules, total) = service
        .list(AlertRuleQueryParams {
            aging_bucket: params.aging_bucket,
            alert_level: params.alert_level,
            is_active: params.is_active,
            page,
            page_size,
        })
        .await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        rules,
        total,
        page,
        page_size,
    ))))
}

/// 获取预警规则详情
pub async fn get_rule(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<aging_alert_rule::Model>>, AppError> {
    info!("用户 {} 正在查询账龄预警规则 {}", auth.user_id, id);

    let service = AgingAlertRuleService::new(state.db.clone());
    let rule = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(rule)))
}

/// 更新预警规则
pub async fn update_rule(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateAlertRuleDto>,
) -> Result<Json<ApiResponse<aging_alert_rule::Model>>, AppError> {
    info!("用户 {} 正在更新账龄预警规则 {}", auth.user_id, id);

    let service = AgingAlertRuleService::new(state.db.clone());
    let rule = service
        .update(
            id,
            UpdateAlertRuleRequest {
                rule_name: req.rule_name,
                aging_bucket: req.aging_bucket,
                threshold_days: req.threshold_days,
                threshold_amount: req.threshold_amount,
                alert_level: req.alert_level,
                notify_method: req.notify_method,
                notify_roles: req.notify_roles,
                is_active: req.is_active,
                remarks: req.remarks,
            },
        )
        .await?;

    Ok(Json(ApiResponse::success(rule)))
}

/// 删除预警规则
pub async fn delete_rule(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除账龄预警规则 {}", auth.user_id, id);

    let service = AgingAlertRuleService::new(state.db.clone());
    service.delete(id).await?;

    Ok(Json(ApiResponse::success(format!("规则 {} 已删除", id))))
}
