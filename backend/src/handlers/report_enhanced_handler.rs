//! 报表引擎增强 Handler
//!
//! 提供报表模板管理、数据导出、报表订阅等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::report_subscription_service::{
    CreateSubscriptionRequest, ReportSubscriptionService, SubscriptionQuery,
    UpdateSubscriptionRequest,
};
use crate::services::report_template_service::{
    CreateReportTemplateRequest, ReportTemplateQuery, ReportTemplateService,
    UpdateReportTemplateRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 报表执行查询参数
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ReportExecuteParams {
    pub template_id: String,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 导出请求
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub template_id: String,
    pub format: String,
    pub title: Option<String>,
}

/// POST /api/v1/erp/reports-enhanced/templates - 创建报表模板
pub async fn create_report_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateReportTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());
    let tenant_id = auth.tenant_id.unwrap_or(0);

    let template = service.create(tenant_id, auth.user_id, req).await?;

    tracing::info!("用户 {} 创建报表模板: {}", auth.username, template.name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(template)?,
        "报表模板创建成功",
    )))
}

/// GET /api/v1/erp/reports-enhanced/templates - 获取报表模板列表
pub async fn list_report_templates(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ReportTemplateQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());
    let tenant_id = auth.tenant_id.unwrap_or(0);

    let (items, total) = service.list(tenant_id, auth.user_id, query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/reports-enhanced/templates/:id - 获取报表模板详情
pub async fn get_report_template(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template = service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("报表模板不存在"))?;

    Ok(Json(ApiResponse::success(serde_json::to_value(template)?)))
}

/// PUT /api/v1/erp/reports-enhanced/templates/:id - 更新报表模板
pub async fn update_report_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateReportTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template = service.update(id, req).await?;

    tracing::info!("用户 {} 更新报表模板: {}", auth.username, template.name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(template)?,
        "报表模板更新成功",
    )))
}

/// DELETE /api/v1/erp/reports-enhanced/templates/:id - 删除报表模板
pub async fn delete_report_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    service.delete(id).await?;

    tracing::info!("用户 {} 删除报表模板: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "报表模板已删除",
    )))
}

/// POST /api/v1/erp/reports-enhanced/templates/:id/execute - 执行自定义报表
pub async fn execute_custom_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Query(params): Query<ReportExecuteParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);

    let (headers, data, total) = service.execute_custom_report(id, page, page_size).await?;

    tracing::info!("用户 {} 执行自定义报表: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "columns": headers,
        "data": data,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/reports-enhanced/export/pdf - 导出PDF
pub async fn export_pdf(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template_id: i32 = req
        .template_id
        .parse()
        .map_err(|_| AppError::validation("无效的模板ID"))?;

    // 执行报表获取数据
    let (headers, data, _total) = service.execute_custom_report(template_id, 1, 10000).await?;

    let title = req
        .title
        .unwrap_or_else(|| format!("报表 {}", req.template_id));

    let export_data = crate::services::export_service::ExportData {
        title: title.clone(),
        headers,
        rows: data,
        summary: None,
    };

    let pdf_content = crate::services::export_service::ExportService::export_pdf(&export_data)?;

    tracing::info!("用户 {} 导出PDF报表: {}", auth.username, req.template_id);

    // 返回base64编码的内容
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&pdf_content);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.pdf", title),
        "size": pdf_content.len(),
        "content_type": "application/pdf",
        "content": encoded,
        "message": "PDF导出成功"
    }))))
}

/// POST /api/v1/erp/reports-enhanced/export/excel - 导出Excel
pub async fn export_excel(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ExportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template_id: i32 = req
        .template_id
        .parse()
        .map_err(|_| AppError::validation("无效的模板ID"))?;

    // 执行报表获取数据
    let (headers, data, _total) = service.execute_custom_report(template_id, 1, 10000).await?;

    let title = req
        .title
        .unwrap_or_else(|| format!("报表 {}", req.template_id));

    let export_data = crate::services::export_service::ExportData {
        title: title.clone(),
        headers,
        rows: data,
        summary: None,
    };

    let excel_content = crate::services::export_service::ExportService::export_excel(&export_data)?;

    tracing::info!("用户 {} 导出Excel报表: {}", auth.username, req.template_id);

    // 返回base64编码的内容
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&excel_content);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "filename": format!("{}.csv", title),
        "size": excel_content.len(),
        "content_type": "text/csv",
        "content": encoded,
        "message": "Excel导出成功"
    }))))
}

/// POST /api/v1/erp/reports-enhanced/subscriptions - 创建报表订阅
pub async fn create_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSubscriptionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());
    let tenant_id = auth.tenant_id.unwrap_or(0);

    let subscription = service.create(tenant_id, auth.user_id, req).await?;

    tracing::info!("用户 {} 创建报表订阅: {}", auth.username, subscription.name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(subscription)?,
        "报表订阅创建成功",
    )))
}

/// GET /api/v1/erp/reports-enhanced/subscriptions - 获取报表订阅列表
pub async fn list_subscriptions(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<SubscriptionQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());
    let tenant_id = auth.tenant_id.unwrap_or(0);

    let (items, total) = service.list(tenant_id, query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/reports-enhanced/subscriptions/:id - 获取报表订阅详情
pub async fn get_subscription(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    let subscription = service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("订阅不存在"))?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        subscription,
    )?)))
}

/// PUT /api/v1/erp/reports-enhanced/subscriptions/:id - 更新报表订阅
pub async fn update_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateSubscriptionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    let subscription = service.update(id, req).await?;

    tracing::info!("用户 {} 更新报表订阅: {}", auth.username, subscription.name);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(subscription)?,
        "报表订阅更新成功",
    )))
}

/// DELETE /api/v1/erp/reports-enhanced/subscriptions/:id - 删除报表订阅
pub async fn delete_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    service.delete(id).await?;

    tracing::info!("用户 {} 删除报表订阅: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "报表订阅已删除",
    )))
}

/// POST /api/v1/erp/reports-enhanced/subscriptions/:id/toggle - 启用/禁用报表订阅
pub async fn toggle_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    let enabled = req.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);

    let subscription = service.toggle(id, enabled).await?;

    let action = if enabled { "启用" } else { "禁用" };
    tracing::info!(
        "用户 {} {}报表订阅: {}",
        auth.username,
        action,
        subscription.name
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(subscription)?,
        &format!("报表订阅已{}", action),
    )))
}

/// POST /api/v1/erp/reports-enhanced/subscriptions/:id/trigger - 手动触发报表订阅
pub async fn trigger_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    service.trigger(id).await?;

    tracing::info!("用户 {} 手动触发报表订阅: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "报表订阅已触发",
    )))
}
