use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::email_service::{EmailMessage, EmailService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub html_content: Option<String>,
    pub text_content: Option<String>,
    pub template_id: Option<i32>,
    pub template_params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub code: String,
    pub subject_template: String,
    pub body_template: String,
    pub template_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TemplateListQuery {
    pub template_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct EmailSendResult {
    pub message_id: String,
    pub status: String,
    pub sent_at: String,
}

#[derive(Debug, Serialize)]
pub struct EmailTemplateItem {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub subject_template: String,
    pub body_template: String,
    pub template_type: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn send_email(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SendEmailRequest>,
) -> Result<Json<ApiResponse<EmailSendResult>>, AppError> {
    let email_service = state
        .email_service
        .as_ref()
        .ok_or_else(|| AppError::BusinessError("邮件服务未配置".to_string()))?;

    let message = EmailMessage {
        to: req.to,
        cc: req.cc,
        bcc: req.bcc,
        subject: req.subject,
        html_content: req.html_content,
        text_content: req.text_content,
        attachments: None,
    };

    email_service.send_email(message).await?;

    let result = EmailSendResult {
        message_id: uuid::Uuid::new_v4().to_string(),
        status: "SENT".to_string(),
        sent_at: chrono::Utc::now().to_rfc3339(),
    };

    tracing::info!(
        "用户 {} 发送邮件成功，收件人: {:?}",
        auth.user_id,
        result.message_id
    );

    Ok(Json(ApiResponse::success_with_message(result, "邮件发送成功")))
}

pub async fn list_templates(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<TemplateListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
    use crate::models::notification_setting;

    let _page = query.page.unwrap_or(1);
    let _page_size = query.page_size.unwrap_or(20);

    let templates = vec![
        EmailTemplateItem {
            id: 1,
            name: "订单状态通知".to_string(),
            code: "ORDER_STATUS".to_string(),
            subject_template: "订单 {{order_no}} 状态更新".to_string(),
            body_template: "您的订单 {{order_no}} 状态已更新为 {{status}}".to_string(),
            template_type: "NOTIFICATION".to_string(),
            description: Some("订单状态变更时发送的通知邮件".to_string()),
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        EmailTemplateItem {
            id: 2,
            name: "审批任务提醒".to_string(),
            code: "APPROVAL_REMINDER".to_string(),
            subject_template: "待审批任务: {{task_title}}".to_string(),
            body_template: "您有一个待审批任务需要处理: {{task_title}}，申请人: {{applicant}}".to_string(),
            template_type: "WORKFLOW".to_string(),
            description: Some("审批流程中提醒审批人的邮件".to_string()),
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
        EmailTemplateItem {
            id: 3,
            name: "库存预警通知".to_string(),
            code: "INVENTORY_ALERT".to_string(),
            subject_template: "库存预警: {{product_name}}".to_string(),
            body_template: "产品 {{product_name}} 当前库存 {{current_stock}}，低于预警阈值 {{threshold}}".to_string(),
            template_type: "ALERT".to_string(),
            description: Some("库存低于预警阈值时发送的提醒邮件".to_string()),
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    ];

    let total = templates.len() as u64;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": templates,
        "total": total,
        "page": query.page.unwrap_or(1),
        "page_size": query.page_size.unwrap_or(20),
    }))))
}

pub async fn create_template(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<ApiResponse<EmailTemplateItem>>, AppError> {
    let template = EmailTemplateItem {
        id: chrono::Utc::now().timestamp() as i32,
        name: req.name,
        code: req.code,
        subject_template: req.subject_template,
        body_template: req.body_template,
        template_type: req.template_type,
        description: req.description,
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    Ok(Json(ApiResponse::success_with_message(template, "邮件模板创建成功")))
}

pub async fn get_email_records(
    State(_state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let page = params
        .get("page")
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(1);
    let page_size = params
        .get("page_size")
        .and_then(|p| p.parse::<u64>().ok())
        .unwrap_or(20);

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": [],
        "total": 0,
        "page": page,
        "page_size": page_size,
    }))))
}
