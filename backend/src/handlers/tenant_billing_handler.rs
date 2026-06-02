use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::tenant_billing_service::TenantBillingService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpgradePlanRequest {
    pub plan_id: i32,
    pub billing_cycle: String,
}

#[derive(Debug, Deserialize)]
pub struct RenewRequest {
    pub billing_cycle: Option<String>,
}

pub async fn get_current_plan(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::services::tenant_billing_service::CurrentPlanInfo>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantBillingService::new(state.db);

    let current_plan = service
        .get_current_plan(tenant_id)
        .await?
        .ok_or_else(|| AppError::NotFound("当前无有效套餐".to_string()))?;

    Ok(Json(ApiResponse::success(current_plan)))
}

pub async fn upgrade_plan(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpgradePlanRequest>,
) -> Result<Json<ApiResponse<crate::services::tenant_billing_service::CurrentPlanInfo>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantBillingService::new(state.db);

    let upgrade_req = crate::services::tenant_billing_service::UpgradePlanRequest {
        plan_id: req.plan_id,
        billing_cycle: req.billing_cycle,
    };

    let result = service.upgrade_plan(tenant_id, upgrade_req).await?;

    Ok(Json(ApiResponse::success_with_message(
        result,
        "套餐升级成功",
    )))
}

pub async fn get_usage(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::services::tenant_billing_service::UsageStats>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantBillingService::new(state.db);

    let usage = service
        .get_usage_stats(tenant_id)
        .await?
        .ok_or_else(|| AppError::NotFound("租户不存在".to_string()))?;

    Ok(Json(ApiResponse::success(usage)))
}

pub async fn list_invoices(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<PaginationQuery>,
) -> Result<
    Json<ApiResponse<PaginatedResponse<crate::services::tenant_billing_service::InvoiceItem>>>,
    AppError,
> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let service = TenantBillingService::new(state.db);
    let (items, total) = service.list_invoices(tenant_id, page, page_size).await?;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        items, total, page, page_size,
    ))))
}

pub async fn renew_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RenewRequest>,
) -> Result<Json<ApiResponse<crate::services::tenant_billing_service::CurrentPlanInfo>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantBillingService::new(state.db);

    let renew_req = crate::services::tenant_billing_service::RenewSubscriptionRequest {
        billing_cycle: req.billing_cycle,
    };

    let result = service.renew_subscription(tenant_id, renew_req).await?;

    Ok(Json(ApiResponse::success_with_message(result, "续费成功")))
}
