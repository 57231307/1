//! 报表引擎增强 Handler
//!
//! 提供报表模板管理、数据导出、报表订阅等 API 接口

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

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

/// P1-2n 修复（批次 81 v1 复审）：切换报表订阅启用状态请求 DTO
/// 替代 toggle_subscription 中的 Json<serde_json::Value>，提供强类型校验
#[derive(Debug, Deserialize, Validate)]
pub struct ToggleSubscriptionDto {
    /// 是否启用：必填
    pub enabled: bool,
}

/// 报表执行查询参数
#[derive(Debug, Deserialize)]
pub struct ReportExecuteParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// POST /api/v1/erp/reports-enhanced/templates - 创建报表模板
pub async fn create_report_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateReportTemplateRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template = service
        .create(auth.user_id, auth.role_id, req)
        .await?;

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

    let (items, total) = service.list(auth.user_id, query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/reports-enhanced/templates/:id - 获取报表模板详情
pub async fn get_report_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let template = service
        .get_by_id(id, auth.user_id)
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

    let template = service
        .update(
            id,
            auth.user_id,
            auth.role_id,
            req,
        )
        .await?;

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

    service
        .delete(id, auth.user_id)
        .await?;

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

    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (headers, data, total) = service
        .execute_custom_report(
            id,
            auth.user_id,
            auth.role_id,
            page,
            page_size,
        )
        .await?;

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
    let (headers, data, _total) = service
        .execute_custom_report(
            template_id,
            auth.user_id,
            auth.role_id,
            1,
            10000,
        )
        .await?;

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
    let (headers, data, _total) = service
        .execute_custom_report(
            template_id,
            auth.user_id,
            auth.role_id,
            1,
            10000,
        )
        .await?;

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
        "filename": format!("{}.xlsx", title),
        "size": excel_content.len(),
        "content_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "content": encoded,
        "message": "Excel导出成功"
    }))))
}

/// 报表订阅 CRUD Handler（通过宏生成）
///
/// 基础增删改查由 `define_tuple_crud_handlers!` 宏生成，自定义接口
/// （toggle/trigger/send）保留在文件下方以保持可读性。
pub mod subscriptions {
    use super::*;
    use crate::define_tuple_crud_handlers;

    define_tuple_crud_handlers!(
        ReportSubscriptionService,
        CreateSubscriptionRequest,
        UpdateSubscriptionRequest,
        SubscriptionQuery,
        i32,
        "订阅不存在"
    );
}

/// POST /api/v1/erp/reports-enhanced/subscriptions/:id/toggle - 启用/禁用报表订阅
pub async fn toggle_subscription(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ToggleSubscriptionDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2n 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = ReportSubscriptionService::new(state.db.clone());

    let subscription = service.toggle(id, req.enabled).await?;

    let action = if req.enabled { "启用" } else { "禁用" };
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

/// 报表模板导出请求
#[derive(Debug, Deserialize)]
pub struct TemplateExportRequest {
    pub format: Option<String>,
    pub title: Option<String>,
}

/// 报表导出请求（PDF/Excel 共用，v11 批次 154b：已在 export_report/export_template 中使用）
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub template_id: String,
    pub title: Option<String>,
    pub format: Option<String>,
}

/// GET /api/v1/erp/reports-enhanced/fields/:template_type - 获取指定模板类型可用的字段定义
pub async fn get_available_fields(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Path(template_type): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 批次 128 v8 复审 P2 修复：原硬编码 serde_json::json! 字段定义，
    // 现调用 ReportTemplateService::available_fields_for_type 静态方法返回类型化 struct。
    // 字段元数据绑定 DB schema，不宜放数据库动态管理（与 print_handler 批次 126 一致）。
    let fields = ReportTemplateService::available_fields_for_type(&template_type);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "template_type": template_type,
        "fields": fields,
    }))))
}

/// POST /api/v1/erp/reports-enhanced/templates/:id/export - 导出指定模板
pub async fn export_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<TemplateExportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let (headers, data, _total) = service
        .execute_custom_report(
            id,
            auth.user_id,
            auth.role_id,
            1,
            10000,
        )
        .await?;

    let format = req.format.unwrap_or_else(|| "csv".to_string());
    let title = req.title.unwrap_or_else(|| format!("报表模板 {}", id));

    let export_data = crate::services::export_service::ExportData {
        title: title.clone(),
        headers,
        rows: data,
        summary: None,
    };

    // 规则 3：所有非 PDF 导出统一使用 xlsx 格式（含原 csv 请求）
    let (content_type, encoded, ext) = match format.to_lowercase().as_str() {
        "pdf" => {
            let bytes = crate::services::export_service::ExportService::export_pdf(&export_data)?;
            let ct = "application/pdf".to_string();
            (ct, bytes, "pdf")
        }
        _ => {
            let bytes = crate::services::export_service::ExportService::export_excel(&export_data)?;
            let ct =
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string();
            (ct, bytes, "xlsx")
        }
    };

    use base64::Engine;
    let encoded_content = base64::engine::general_purpose::STANDARD.encode(&encoded);

    tracing::info!(
        "用户 {} 导出报表模板: ID={}, 格式={}",
        auth.username,
        id,
        format
    );

    Ok(Json(ApiResponse::success(serde_json::json!({
        "template_id": id,
        "filename": format!("{}.{}", title, ext),
        "size": encoded.len(),
        "content_type": content_type,
        "content": encoded_content,
        "message": "模板导出成功"
    }))))
}

/// GET /api/v1/erp/reports-enhanced/templates/:id/preview - 预览报表模板数据
pub async fn preview_template(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportTemplateService::new(state.db.clone());

    let (columns, data, total) = service
        .execute_custom_report(
            id,
            auth.user_id,
            auth.role_id,
            1,
            50,
        )
        .await?;

    tracing::info!("用户 {} 预览报表模板: ID={}", auth.username, id);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "template_id": id,
        "columns": columns,
        "data": data,
        "total": total,
        "preview_rows": data.len(),
    }))))
}

/// POST /api/v1/erp/reports-enhanced/subscriptions/:id/send - 立即发送报表订阅
pub async fn send_subscription_now(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ReportSubscriptionService::new(state.db.clone());

    // 验证订阅存在
    let subscription = service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("订阅不存在"))?;

    // 立即触发该订阅
    service.trigger(id).await?;

    tracing::info!(
        "用户 {} 立即发送报表订阅: ID={}, 名称={}",
        auth.username,
        id,
        subscription.name
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
            "subscription_id": id,
            "name": subscription.name,
            "recipients": subscription.recipients,
            "export_format": subscription.export_format,
        }),
        "报表订阅已立即发送",
    )))
}
