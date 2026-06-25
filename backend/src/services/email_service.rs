//! 邮件服务
//!
//! 提供邮件发送功能，支持通过 HTTP API 调用第三方邮件服务
//! 可配置为使用 SendGrid、阿里云邮件推送、腾讯云邮件等服务

use crate::utils::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTML 特殊字符转义，防止 XSS 攻击
fn escape_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#x27;"),
            '/' => result.push_str("&#x2F;"),
            _ => result.push(ch),
        }
    }
    result
}

/// SendGrid 官方 API URL（硬编码，防止环境变量注入导致 API Key 泄露）
const SENDGRID_API_URL: &str = "https://api.sendgrid.com/v3/mail/send";

/// 邮件配置
#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    /// 邮件服务提供商：sendgrid, aliyun, tencent, smtp
    pub provider: String,
    /// API 密钥
    pub api_key: String,
    /// 发件人邮箱
    pub from_email: String,
    /// 发件人名称
    pub from_name: String,
    /// API 基础 URL（可选，用于自定义端点；生产环境不建议使用）
    /// ⚠️ 安全约束：自定义 URL 必须经过 SSRF 校验，
    /// 默认使用各服务商官方 URL，禁止通过环境变量直接覆盖。
    pub api_url: Option<String>,
}

/// 邮件内容
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// 收件人邮箱列表
    pub to: Vec<String>,
    /// 抄送邮箱列表
    pub cc: Option<Vec<String>>,
    /// 密送邮箱列表
    pub bcc: Option<Vec<String>>,
    /// 邮件主题
    pub subject: String,
    /// 邮件内容（HTML 格式）
    pub html_content: Option<String>,
    /// 邮件内容（纯文本格式）
    pub text_content: Option<String>,
    /// 附件（文件名 -> 内容）
    pub attachments: Option<HashMap<String, Vec<u8>>>,
}

/// 邮件服务
pub struct EmailService {
    config: EmailConfig,
    http_client: reqwest::Client,
}

impl EmailService {
    /// 创建邮件服务实例
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// 从环境变量创建邮件服务
    ///
    /// 安全约束（H-2 修复）：
    /// - 禁止从 `EMAIL_API_URL` 环境变量读取 API URL，防止环境变量注入导致 API Key 被
    ///   发送到攻击者控制的服务器。
    /// - 各服务商 API URL 使用硬编码的官方地址。
    /// - 如需自定义 URL，必须通过代码显式设置 EmailConfig.api_url，并经过 SSRF 校验。
    pub fn from_env() -> Option<Self> {
        let provider = std::env::var("EMAIL_PROVIDER").ok()?;
        let api_key = std::env::var("EMAIL_API_KEY").ok()?;
        let from_email = std::env::var("EMAIL_FROM").ok()?;
        let from_name = std::env::var("EMAIL_FROM_NAME").unwrap_or_else(|_| "系统通知".to_string());

        Some(Self::new(EmailConfig {
            provider,
            api_key,
            from_email,
            from_name,
            api_url: None,
        }))
    }

    /// 发送邮件
    pub async fn send_email(&self, message: EmailMessage) -> Result<(), AppError> {
        match self.config.provider.as_str() {
            "sendgrid" => self.send_via_sendgrid(message).await,
            "aliyun" => self.send_via_aliyun(message).await,
            "tencent" => self.send_via_tencent(message).await,
            _ => Err(AppError::business(format!(
                "不支持的邮件服务提供商: {}",
                self.config.provider
            ))),
        }
    }

    /// 发送简单文本邮件
    pub async fn send_text_email(
        &self,
        to: Vec<String>,
        subject: String,
        content: String,
    ) -> Result<(), AppError> {
        self.send_email(EmailMessage {
            to,
            cc: None,
            bcc: None,
            subject,
            html_content: None,
            text_content: Some(content),
            attachments: None,
        })
        .await
    }

    /// 发送 HTML 邮件
    ///
    /// ⚠️ 安全警告（M-5 修复）：
    /// 调用方必须确保 `html_content` 中所有用户输入都经过 HTML 转义。
    /// 推荐使用 `EmailTemplate` 系列方法（`notification_template` / `order_notification` 等），
    /// 这些模板内部已自动对所有用户输入做 escape。
    /// 直接拼接用户输入到 HTML 中会导致邮件 XSS。
    pub async fn send_html_email(
        &self,
        to: Vec<String>,
        subject: String,
        html_content: String,
    ) -> Result<(), AppError> {
        // M-5 防御性检查：检测常见邮件 XSS 危险模式
        // 注意：这不是完整的 XSS 过滤，仅作为防御纵深的最后一道防线
        let dangerous_patterns = [
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
            "onclick=",
            "onmouseover=",
            "eval(",
            "expression(",
        ];
        let lower_content = html_content.to_lowercase();
        for pattern in &dangerous_patterns {
            if lower_content.contains(pattern) {
                tracing::warn!(
                    "邮件 HTML 内容包含危险模式: pattern={}, subject_len={}",
                    pattern,
                    subject.len()
                );
            }
        }

        self.send_email(EmailMessage {
            to,
            cc: None,
            bcc: None,
            subject,
            html_content: Some(html_content),
            text_content: None,
            attachments: None,
        })
        .await
    }

    /// 发送通知邮件（安全版本，自动 HTML 转义）
    ///
    /// 接受纯文本标题和内容，内部使用 `EmailTemplate::notification_template`
    /// 对所有用户输入进行 HTML 转义，防止邮件 XSS。
    pub async fn send_notification_email(
        &self,
        to: Vec<String>,
        title: &str,
        content: &str,
        action_url: Option<&str>,
    ) -> Result<(), AppError> {
        let html = EmailTemplate::notification_template(title, content, action_url);
        self.send_email(EmailMessage {
            to,
            cc: None,
            bcc: None,
            subject: title.to_string(),
            html_content: Some(html),
            text_content: Some(content.to_string()),
            attachments: None,
        })
        .await
    }

    /// 通过 SendGrid 发送邮件
    async fn send_via_sendgrid(&self, message: EmailMessage) -> Result<(), AppError> {
        #[derive(Serialize)]
        struct SendGridEmail {
            email: String,
        }

        #[derive(Serialize)]
        struct SendGridContent {
            #[serde(rename = "type")]
            content_type: String,
            value: String,
        }

        #[derive(Serialize)]
        struct SendGridMessage {
            personalizations: Vec<SendGridPersonalization>,
            from: SendGridEmail,
            subject: String,
            content: Vec<SendGridContent>,
        }

        #[derive(Serialize)]
        struct SendGridPersonalization {
            to: Vec<SendGridEmail>,
        }

        let personalizations = vec![SendGridPersonalization {
            to: message
                .to
                .into_iter()
                .map(|email| SendGridEmail { email })
                .collect(),
        }];

        let mut content = Vec::new();
        if let Some(html) = message.html_content {
            content.push(SendGridContent {
                content_type: "text/html".to_string(),
                value: html,
            });
        }
        if let Some(text) = message.text_content {
            content.push(SendGridContent {
                content_type: "text/plain".to_string(),
                value: text,
            });
        }

        let sendgrid_message = SendGridMessage {
            personalizations,
            from: SendGridEmail {
                email: self.config.from_email.clone(),
            },
            subject: message.subject,
            content,
        };

        // H-2 修复：使用硬编码的 SendGrid 官方 API URL，
        // 禁止从环境变量或配置中读取自定义 URL，防止 API Key 泄露到攻击者服务器。
        let api_url = SENDGRID_API_URL;

        let response = self
            .http_client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&sendgrid_message)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("邮件发送请求失败: {}", e)))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            Err(AppError::internal(format!(
                "SendGrid 邮件发送失败: HTTP {} - {}",
                status, body
            )))
        }
    }

    /// 通过阿里云邮件推送发送邮件
    async fn send_via_aliyun(&self, _message: EmailMessage) -> Result<(), AppError> {
        // 阿里云邮件推送实现
        // 参考文档：https://help.aliyun.com/document_detail/29434.html
        // 由于需要签名算法，这里提供框架，具体实现根据实际需求补充
        tracing::info!("阿里云邮件推送功能待实现，请先配置 SendGrid 或其他邮件服务");
        Err(AppError::business("阿里云邮件推送功能待实现".to_string()))
    }

    /// 通过腾讯云邮件发送
    async fn send_via_tencent(&self, _message: EmailMessage) -> Result<(), AppError> {
        // 腾讯云邮件服务实现
        tracing::info!("腾讯云邮件功能待实现，请先配置 SendGrid 或其他邮件服务");
        Err(AppError::business("腾讯云邮件功能待实现"))
    }

    /// 检查邮件服务是否可用
    pub async fn health_check(&self) -> Result<bool, AppError> {
        match self.config.provider.as_str() {
            "sendgrid" => {
                let response = self
                    .http_client
                    .get("https://api.sendgrid.com/v3/user/profile")
                    .header("Authorization", format!("Bearer {}", self.config.api_key))
                    .send()
                    .await
                    .map_err(|e| AppError::internal(format!("健康检查请求失败: {}", e)))?;
                Ok(response.status().is_success())
            }
            _ => Ok(true),
        }
    }
}

/// 邮件模板
pub struct EmailTemplate;

impl EmailTemplate {
    /// 生成通知邮件 HTML 模板
    /// 所有用户输入参数都会自动进行 HTML 转义，防止 XSS 攻击
    pub fn notification_template(title: &str, content: &str, action_url: Option<&str>) -> String {
        let safe_title = escape_html(title);
        let safe_content = escape_html(content);
        let action_button = action_url.map(|url| {
            let safe_url = escape_html(url);
            format!(
                r#"<div style="margin-top: 20px;">
                    <a href="{}" style="background-color: #1890ff; color: white; padding: 10px 20px; text-decoration: none; border-radius: 4px; display: inline-block;">查看详情</a>
                </div>"#,
                safe_url
            )
        }).unwrap_or_default();

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
</head>
<body style="font-family: Arial, sans-serif; background-color: #f5f5f5; margin: 0; padding: 20px;">
    <div style="max-width: 600px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); overflow: hidden;">
        <div style="background-color: #1890ff; color: white; padding: 20px; text-align: center;">
            <h1 style="margin: 0; font-size: 24px;">{}</h1>
        </div>
        <div style="padding: 30px; color: #333; line-height: 1.6;">
            {}
            {}
        </div>
        <div style="background-color: #f8f9fa; padding: 15px; text-align: center; color: #666; font-size: 12px;">
            <p style="margin: 0;">此邮件由系统自动发送，请勿回复</p>
            <p style="margin: 5px 0 0 0;">面料管理系统</p>
        </div>
    </div>
</body>
</html>"#,
            safe_title, safe_title, safe_content, action_button
        )
    }

    /// 生成订单通知邮件
    pub fn order_notification(order_no: &str, status: &str, detail_url: &str) -> String {
        let safe_order_no = escape_html(order_no);
        let safe_status = escape_html(status);
        let content = format!(
            r#"<p>您好，</p>
            <p>您的订单 <strong>{}</strong> 状态已更新为 <strong>{}</strong>。</p>
            <p>请登录系统查看详细信息。</p>"#,
            safe_order_no, safe_status
        );
        Self::notification_template("订单状态更新", &content, Some(detail_url))
    }

    /// 生成审批通知邮件
    pub fn approval_notification(task_title: &str, applicant: &str, approval_url: &str) -> String {
        let safe_task_title = escape_html(task_title);
        let safe_applicant = escape_html(applicant);
        let content = format!(
            r#"<p>您好，</p>
            <p><strong>{}</strong> 提交了一个审批任务需要您处理。</p>
            <p>任务标题：{}</p>"#,
            safe_applicant, safe_task_title
        );
        Self::notification_template("待审批任务提醒", &content, Some(approval_url))
    }

    /// 生成库存预警邮件
    pub fn inventory_alert(product_name: &str, current_stock: &str, threshold: &str) -> String {
        let safe_product_name = escape_html(product_name);
        let safe_current_stock = escape_html(current_stock);
        let safe_threshold = escape_html(threshold);
        let content = format!(
            r#"<p>您好，</p>
            <p>产品 <strong>{}</strong> 的库存已达到预警线。</p>
            <p>当前库存：{}</p>
            <p>预警阈值：{}</p>
            <p style="color: #ff4d4f;">请及时补货！</p>"#,
            safe_product_name, safe_current_stock, safe_threshold
        );
        Self::notification_template("库存预警通知", &content, None)
    }

    /// 保存邮件发送记录
    #[allow(dead_code)] // TODO(tech-debt): 邮件发送完成时统一回写日志后移除
    #[allow(clippy::too_many_arguments)]
    pub async fn save_email_log(
        &self,
        db: &sea_orm::DatabaseConnection,
        tenant_id: i32,
        user_id: Option<i32>,
        to: &[String],
        subject: &str,
        status: &str,
        error_message: Option<String>,
        message_id: Option<String>,
    ) -> Result<(), AppError> {
        use crate::models::email_log;
        use chrono::Utc;
        use sea_orm::{ActiveModelTrait, Set};

        let active_model = email_log::ActiveModel {
            tenant_id: Set(tenant_id),
            user_id: Set(user_id),
            recipients: Set(to.join(",")),
            subject: Set(subject.to_string()),
            status: Set(status.to_string()),
            error_message: Set(error_message),
            external_message_id: Set(message_id),
            sent_at: Set(Some(Utc::now())),
            ..Default::default()
        };

        active_model.insert(db).await?;

        // M-4 修复：日志脱敏，不记录完整收件人邮箱和主题
        // 仅记录：收件人数量、域名列表、主题长度
        let to_count = to.len();
        let domains: Vec<String> = to
            .iter()
            .filter_map(|email| {
                email
                    .split('@')
                    .nth(1)
                    .map(|d| d.to_lowercase())
            })
            .collect();
        let subject_len = subject.len();

        tracing::info!(
            "邮件发送记录已保存: tenant_id={}, user_id={:?}, to_count={}, to_domains={:?}, subject_len={}, status={}",
            tenant_id,
            user_id,
            to_count,
            domains,
            subject_len,
            status
        );

        Ok(())
    }
}
