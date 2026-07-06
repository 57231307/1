//! 邮件 Handler
//!
//! 提供邮件发送、模板管理和发送记录查询功能

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::middleware::auth_context::AuthContext;
use crate::utils::admin_checker::is_admin_role;

use crate::services::email_log_service::{CreateEmailLogRequest, EmailLogQuery, EmailLogService};
use crate::services::email_template_service::{
    CreateEmailTemplateRequest, EmailTemplateQuery, EmailTemplateService,
    UpdateEmailTemplateRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// M-1 修复：每用户每小时邮件发送配额
///
/// 安全原因：避免邮件炸弹/DoS 滥用组织 SMTP 配额。
/// 设计：使用进程内 DashMap 存储 `{user_id, hour_bucket} -> count`，
/// 每次 send_email 前检查并自增。
const EMAIL_PER_USER_PER_HOUR: u32 = 50;

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
    // v11 批次 151 P2-A：接入模板参数渲染
    // 当 template_id 和 template_params 同时提供时，加载模板并用 params 替换 {{key}} 占位符
    // template_params 应为 JSON 对象（如 {"name": "张三", "order_no": "ORD001"}）
    pub template_params: Option<serde_json::Value>,
}

// 邮件模板 CRUD Handler（通过宏生成）
crate::define_tuple_crud_handlers!(
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
    // M-1 修复：仅 admin 角色可调用 send_email
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "邮件发送仅限管理员（code=admin）执行",
        ));
    }

    // M-1 修复：每用户每小时发送配额检查
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::internal(e.to_string()))?
        .as_secs();
    let hour_bucket = now / 3600;
    let key = (auth.user_id, hour_bucket);
    let counter = state
        .email_send_counters
        .entry(key)
        .or_insert(Arc::new(AtomicU32::new(0)));
    let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
    if current > EMAIL_PER_USER_PER_HOUR {
        // 回滚
        counter.fetch_sub(1, Ordering::SeqCst);
        return Err(AppError::validation(format!(
            "邮件发送配额已用完：本小时已发送 {} 封（上限 {} 封）",
            current - 1,
            EMAIL_PER_USER_PER_HOUR
        )));
    }

    let email_service = state
        .email_service
        .as_ref()
        .ok_or_else(|| AppError::business("邮件服务未配置"))?;

    // v11 批次 151 P2-A：接入 template_params 模板参数渲染
    // 当 template_id 存在时，加载模板并用 template_params 替换占位符 {{key}}
    let mut req = req;
    if let Some(template_id) = req.template_id {
        let template_service = EmailTemplateService::new(state.db.clone());
        let template = template_service
            .get_by_id(template_id)
            .await?
            .ok_or_else(|| AppError::not_found("邮件模板不存在"))?;

        if !template.is_active {
            return Err(AppError::business("邮件模板已停用，无法使用"));
        }

        // template_params 必须配合 template_id 使用，且必须为 JSON 对象
        if let Some(params) = req.template_params.clone() {
            if !params.is_object() {
                return Err(AppError::validation(
                    "template_params 必须为 JSON 对象（如 {\"name\": \"张三\"}）",
                ));
            }
            // 用 params 渲染 subject_template 和 body_template
            req.subject = render_template(&template.subject_template, &params);
            req.html_content = Some(render_template(&template.body_template, &params));
        } else {
            // 仅提供 template_id 无 params：直接使用模板原始内容（不替换占位符）
            req.subject = template.subject_template;
            req.html_content = Some(template.body_template);
        }
    } else if req.template_params.is_some() {
        return Err(AppError::validation(
            "template_params 必须配合 template_id 使用",
        ));
    }

    let log_service = EmailLogService::new(state.db.clone());

    // 创建邮件发送记录
    let log = log_service
        .create(
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
    _auth: AuthContext,
    Query(query): Query<EmailLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EmailLogService::new(state.db.clone());

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": items,
        "total": total,
    }))))
}

/// GET /api/v1/erp/email/statistics - 获取邮件发送统计
pub async fn get_email_statistics(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = EmailLogService::new(state.db.clone());

    let statistics = service.get_statistics().await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(
        statistics,
    )?)))
}

/// 渲染邮件模板：将 {{key}} / {{ key }} 占位符替换为 params 中对应的值
///
/// v11 批次 151 P2-A：接入 SendEmailRequest.template_params
/// - params 必须为 JSON 对象，key 为占位符名称，value 为替换值
/// - value 为字符串时直接替换；为其他类型时使用 JSON 字符串表示
/// - 模板中未匹配到 params 的占位符保持原样（不删除，便于排查模板问题）
fn render_template(template: &str, params: &serde_json::Value) -> String {
    let mut result = template.to_string();
    if let Some(obj) = params.as_object() {
        for (key, value) in obj {
            // 转换 value 为字符串：字符串类型去掉引号，其他类型使用 JSON 表示
            let replacement = if let Some(s) = value.as_str() {
                s.to_string()
            } else {
                value.to_string()
            };
            // 替换 {{ key }} 和 {{key}} 两种格式
            let pattern_with_spaces = format!("{{{{ {} }}}}", key);
            let pattern_without_spaces = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern_with_spaces, &replacement);
            result = result.replace(&pattern_without_spaces, &replacement);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 测试渲染模板_基本替换() {
        let template = "你好 {{name}}，订单号 {{order_no}} 已确认";
        let params = serde_json::json!({"name": "张三", "order_no": "ORD001"});
        let result = render_template(template, &params);
        assert_eq!(result, "你好 张三，订单号 ORD001 已确认");
    }

    #[test]
    fn 测试渲染模板_带空格占位符() {
        let template = "你好 {{ name }}，订单号 {{ order_no }} 已确认";
        let params = serde_json::json!({"name": "李四", "order_no": "ORD002"});
        let result = render_template(template, &params);
        assert_eq!(result, "你好 李四，订单号 ORD002 已确认");
    }

    #[test]
    fn 测试渲染模板_未匹配占位符保持原样() {
        let template = "你好 {{name}}，{{unknown_key}}";
        let params = serde_json::json!({"name": "王五"});
        let result = render_template(template, &params);
        assert_eq!(result, "你好 王五，{{unknown_key}}");
    }

    #[test]
    fn 测试渲染模板_非字符串值使用_json表示() {
        let template = "数量：{{count}}，金额：{{amount}}";
        let params = serde_json::json!({"count": 100, "amount": 99.5});
        let result = render_template(template, &params);
        assert_eq!(result, "数量：100，金额：99.5");
    }
}
