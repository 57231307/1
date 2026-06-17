//! 邮件 Handler
//!
//! 提供邮件发送、模板管理和发送记录查询功能

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;

use crate::services::email_log_service::{CreateEmailLogRequest, EmailLogQuery, EmailLogService};
use crate::services::email_template_service::{
    CreateEmailTemplateRequest, EmailTemplateQuery, EmailTemplateService,
    UpdateEmailTemplateRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 发送邮件请求
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

// 邮件模板 CRUD Handler（带租户隔离）
crate::define_tenant_crud_handlers!(
    EmailTemplateService,
    CreateEmailTemplateRequest,
    UpdateEmailTemplateRequest,
    EmailTemplateQuery,
    i32,
    "邮件模板不存在"
);

/// POST /api/v1/erp/email/send - 发送邮件
pub async fn send_email(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SendEmailRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let email_service = state
        .email_service
        .as_ref()
        .ok_or_else(|| AppError::business("邮件服务未配置"))?;

    let log_service = EmailLogService::new(state.db.clone());
    let tenant_id = extract_tenant_id(&auth)?;

    // 创建邮件发送记录
    let log = log_service
        .create(
            tenant_id,
            CreateEmailLogRequest {
                user_id: Some(auth.user_id),
                recipients: req.to.clone(),
                cc: req.cc.clone(),
                bcc: req.bcc.clone(),
                subject: req.subject.clone(),
                body: req.html_content.clone().or(req.text_content.clone()),
                template_id: req.template_id,
            },
        )
        .await?;

    // 构建邮件消息
    let message = crate::services::email_service::EmailMessage {
        to: req.to.clone(),
        cc: req.cc,
        bcc: req.bcc,
        subject: req.subject.clone(),
        html_content: req.html_content,
        text_content: req.text_content,
        attachments: None,
    };

    // 发送邮件
    match email_service.send_email(message).await {
        Ok(_) => {
            // 更新发送状态为成功
            log_service
                .update_status(log.id, "SENT", None, Some(uuid::Uuid::new_v4().to_string()))
                .await?;

            tracing::info!("用户 {} 发送邮件成功，收件人: {:?}", auth.username, req.to);

            Ok(Json(ApiResponse::success_with_message(
                serde_json::json!({
                    "message_id": log.id,
                    "status": "SENT",
                    "sent_at": chrono::Utc::now().to_rfc3339(),
                }),
                "邮件发送成功",
            )))
        }
        Err(e) => {
            // 更新发送状态为失败
            log_service
                .update_status(log.id, "FAILED", Some(e.to_string()), None)
                .await?;

            Err(e)
        }
    }
}

/// GET /api/v1/erp/email/records - 获取邮件发送记录
pub async fn get_email_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<EmailLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EmailLogService::new(state.db.clone());
    let tenant_id = extract_tenant_id(&auth)?;

    let (items, total) = service.list(tenant_id, query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/email/statistics - 获取邮件发送统计
pub async fn get_email_statistics(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EmailLogService::new(state.db.clone());
    let tenant_id = extract_tenant_id(&auth)?;

    let statistics = service.get_statistics(tenant_id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        statistics,
    )?)))
}
