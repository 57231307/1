//! 邮件服务
//!
//! 提供邮件发送功能，支持通过 HTTP API 调用第三方邮件服务
//! 可配置为使用 SendGrid、阿里云邮件推送、腾讯云邮件等服务

use crate::utils::error::AppError;
use crate::utils::html_escape::{ensure_markup_safe, escape_html, scan_dangerous_markup};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

/// 通用发信出口的点名：所有直接调用与后台邮件队列 worker 都收敛到这一条腿，
/// 检测拒绝的日志与 `email_logs.error_message` 里用它定位是哪条链路被拦。
const GENERAL_SEND_SOURCE: &str = "通用发信出口（含邮件队列 worker）";

/// HTML 直发出口的点名（事件通知与报表订阅两条腿共用）
const HTML_SEND_SOURCE: &str = "HTML 直发出口";

/// 纯文本直发出口的点名
const TEXT_SEND_SOURCE: &str = "纯文本直发出口";

/// 通知邮件出口的点名（正文为纯文本，模板内部完成转义）
const NOTIFICATION_SEND_SOURCE: &str = "通知邮件出口";

/// 缺陷 6.3 修复：单附件大小上限（25 MB）
pub const MAX_SINGLE_ATTACHMENT_BYTES: usize = 25 * 1024 * 1024;

/// 缺陷 6.3 修复：单封邮件所有附件总大小上限（50 MB）
pub const MAX_TOTAL_ATTACHMENT_BYTES: usize = 50 * 1024 * 1024;

/// 缺陷 6.3 修复：根据文件名扩展名推断 Content-Type
fn guess_content_type(filename: &str) -> String {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf".to_string()
    } else if lower.ends_with(".doc") {
        "application/msword".to_string()
    } else if lower.ends_with(".docx") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string()
    } else if lower.ends_with(".xls") {
        "application/vnd.ms-excel".to_string()
    } else if lower.ends_with(".xlsx") {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string()
    } else if lower.ends_with(".zip") {
        "application/zip".to_string()
    } else if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".txt") {
        "text/plain".to_string()
    } else if lower.ends_with(".csv") {
        "text/csv".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// 缺陷 6.3 修复：高危附件扩展名黑名单（即使通过病毒扫描也禁止上传）
const DANGEROUS_EXTENSIONS: &[&str] = &[
    ".exe", ".bat", ".cmd", ".com", ".scr", ".vbs", ".js", ".jar", ".ps1", ".sh", ".msi", ".dll",
];

/// 缺陷 6.3 修复：检查文件名扩展名是否为高危类型
fn is_dangerous_extension(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    DANGEROUS_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// 缺陷 6.3 修复：校验附件大小与扩展名安全性
/// 单附件 ≤ 25MB，总附件 ≤ 50MB；拒绝高危扩展名（.exe/.bat/.cmd/.com/.scr/.vbs/.js/.jar/.ps1/.sh/.msi/.dll）；集成点：CLAMAV_URL 配置后调用 ClamAV 病毒扫描
pub async fn validate_attachments(attachments: &HashMap<String, Vec<u8>>) -> Result<(), AppError> {
    let mut total_size: usize = 0;
    for (filename, content) in attachments {
        // 高危扩展名检查
        if is_dangerous_extension(filename) {
            return Err(AppError::validation_displayable(format!(
                "附件 '{}' 扩展名属于高危类型（.exe/.bat/.cmd 等可执行脚本），禁止上传",
                filename
            )));
        }

        // 单附件大小检查
        if content.len() > MAX_SINGLE_ATTACHMENT_BYTES {
            return Err(AppError::validation_displayable(format!(
                "附件 '{}' 大小 {} 字节超过单附件上限 {} 字节（25 MB）",
                filename,
                content.len(),
                MAX_SINGLE_ATTACHMENT_BYTES
            )));
        }

        // 空附件检查
        if content.is_empty() {
            return Err(AppError::validation_displayable(format!(
                "附件 '{}' 内容为空，禁止上传空附件",
                filename
            )));
        }

        total_size = total_size.saturating_add(content.len());
    }

    // 总附件大小检查
    if total_size > MAX_TOTAL_ATTACHMENT_BYTES {
        return Err(AppError::validation_displayable(format!(
            "附件总大小 {} 字节超过上限 {} 字节（50 MB）",
            total_size, MAX_TOTAL_ATTACHMENT_BYTES
        )));
    }

    // 集成点：若 CLAMAV_URL 环境变量配置，则对每个附件调用 ClamAV 病毒扫描
    // 当前为防御性占位：未配置 CLAMAV_URL 时跳过病毒扫描，仅依赖大小 + 扩展名校验
    if let Ok(clamav_url) = std::env::var("CLAMAV_URL") {
        if !clamav_url.is_empty() {
            for (filename, content) in attachments {
                if let Err(e) = scan_with_clamav(&clamav_url, filename, content).await {
                    return Err(AppError::validation(format!(
                        "附件 '{}' 病毒扫描失败：{}",
                        filename, e
                    )));
                }
            }
        }
    }

    Ok(())
}

/// 缺陷 6.3 修复：调用 ClamAV REST API 进行病毒扫描
/// 集成点：CLAMAV_URL 配置后，附件发送前会调用本函数进行病毒扫描
async fn scan_with_clamav(clamav_url: &str, filename: &str, content: &[u8]) -> Result<(), String> {
    let client = reqwest::Client::new();
    let scan_url = format!("{}/scan", clamav_url.trim_end_matches('/'));
    // V15 P1 20.1-A：注入 traceparent 到 ClamAV 出站请求
    let traceparent = crate::observability::trace_context::traceparent_from_current_span();
    let response = client
        .post(&scan_url)
        .header("Content-Type", "application/octet-stream")
        .header(
            crate::observability::trace_context::TRACEPARENT_HEADER,
            traceparent,
        )
        .body(content.to_vec())
        .send()
        .await
        .map_err(|e| format!("ClamAV 请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("ClamAV 返回非 2xx 状态: {}", response.status()));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("读取 ClamAV 响应失败: {}", e))?;

    // ClamAV REST 返回 "stream: OK" 表示无病毒
    if body.contains("OK") {
        Ok(())
    } else {
        Err(format!("附件 '{}' 被识别为病毒: {}", filename, body))
    }
}

/// SendGrid 官方 API URL（硬编码，防止环境变量注入导致 API Key 泄露）
const SENDGRID_API_URL: &str = "https://api.sendgrid.com/v3/mail/send";

/// 阿里云邮件推送 DirectMail API 端点（硬编码，防止环境变量注入）
const ALIYUN_DM_API_URL: &str = "https://dm.aliyuncs.com/";

/// 腾讯云邮件服务 SES API 端点（硬编码，防止环境变量注入）
const TENCENT_SES_API_URL: &str = "https://ses.tencentcloudapi.com/";

/// 阿里云 RPC V1 签名需要二次编码的字符集
/// 规则：RFC 3986 保留字符 + 部分阿里云特殊要求
const ALIYUN_SIGNATURE_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

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
    // P1-3 修复（2026-06-25 综合审计）：
    // 删除 api_url: Option<String> 字段（H-2 残留死字段）。
    // 历史缺陷：该字段曾可由 EMAIL_API_URL 环境变量覆盖，导致 API Key 可被发送到
    // 攻击者控制的服务器。H-2 修复后 from_env 不再读取环境变量，
    // send_via_sendgrid 改用硬编码 SENDGRID_API_URL 常量，字段成为死字段。
    // 删除后避免未来被误用复活环境变量注入路径。
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

/// 腾讯云 V3 签名参数对象
/// 批次 413 技术债务清理：引入参数对象消除 tencent_sign 的 too_many_arguments 警告。；腾讯云 V3 签名为固定参数集（7 个参数），聚合为单一 struct 便于维护。；使用生命周期借用字符串参数，避免不必要的 to_string()。
#[derive(Debug, Clone, Copy)]
pub struct TencentSignParams<'a> {
    /// API 操作名（如 "SendMail"）
    pub action: &'a str,
    /// 服务名（如 "ses"）
    pub service: &'a str,
    /// API 版本（如 "2020-10-02"）
    pub version: &'a str,
    /// 请求时间戳（Unix 秒）
    pub timestamp: i64,
    /// 请求体 JSON
    pub payload: &'a str,
    /// SecretId
    pub secret_id: &'a str,
    /// SecretKey
    pub secret_key: &'a str,
}

/// SendGrid 收件人/发件人邮箱
#[derive(Serialize)]
struct SendGridEmail {
    email: String,
}

/// SendGrid 邮件内容块
#[derive(Serialize)]
struct SendGridContent {
    #[serde(rename = "type")]
    content_type: String,
    value: String,
}

/// SendGrid personalization（收件人列表）
#[derive(Serialize)]
struct SendGridPersonalization {
    to: Vec<SendGridEmail>,
}

/// 缺陷 6.3 修复：SendGrid 附件结构（content 为 base64 编码）
#[derive(Serialize)]
struct SendGridAttachment {
    content: String,
    filename: String,
    #[serde(rename = "type")]
    content_type: String,
    disposition: String,
}

/// SendGrid 发送请求体
#[derive(Serialize)]
struct SendGridMessage {
    personalizations: Vec<SendGridPersonalization>,
    from: SendGridEmail,
    subject: String,
    content: Vec<SendGridContent>,
    /// 缺陷 6.3 修复：附件列表（None 时序列化为空数组，SendGrid 接受空数组）
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<SendGridAttachment>>,
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
    /// 安全约束（H-2 修复 + P1-3 死字段清理，2026-06-25 综合审计）：禁止从 `EMAIL_API_URL` 环境变量读取 API URL，防止环境变量注入导致 API Key 被；发送到攻击者控制的服务器。；各服务商 API URL 使用硬编码的官方地址（如 SENDGRID_API_URL 常量）。；EmailConfig.api_url 字段已删除，不再保留可被误用的自定义 URL 入口。
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
        }))
    }

    /// 发送邮件：所有邮件正文的唯一出口，HTML 正文在此完成危险标记检测。
    /// 调用方：`EmailQueueWorker::process_one_email`（邮件队列腿）与任何直接发信方。
    pub async fn send_email(&self, message: EmailMessage) -> Result<(), AppError> {
        self.send_email_from(message, GENERAL_SEND_SOURCE).await
    }

    /// 带出口点名的发送：HTML 正文命中危险标记就拒绝发送并点名出口与主题，不静默放行。
    /// 入参：`source_label` 为本文件顶部的出口点名常量，随拒绝错误进日志。
    /// 传给谁：`Self::dispatch_to_provider`；拒绝时错误由调用方处置，队列腿会把它写进
    /// `email_logs.error_message`。
    async fn send_email_from(
        &self,
        message: EmailMessage,
        source_label: &str,
    ) -> Result<(), AppError> {
        if let Some(html_content) = message.html_content.as_deref() {
            ensure_markup_safe(html_content, source_label, &message.subject)?;
        }
        self.dispatch_to_provider(message).await
    }

    /// 服务商路由，不含检测；只允许由已完成检测的 `Self::send_email_from` 与
    /// `Self::send_html_email_declared` 调用，避免绕开防线。
    async fn dispatch_to_provider(&self, message: EmailMessage) -> Result<(), AppError> {
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
        self.send_email_from(
            EmailMessage {
                to,
                cc: None,
                bcc: None,
                subject,
                html_content: None,
                text_content: Some(content),
                attachments: None,
            },
            TEXT_SEND_SOURCE,
        )
        .await
    }

    /// 发送 HTML 邮件：正文必须已由构造点完成 HTML 文本上下文转义。
    /// 调用方：`EventNotificationService::send_email_notification`（事件通知腿）、
    /// `ReportSubscriptionScheduler::execute_subscription`（报表订阅腿）。
    /// 入参：`html_content` 已成形的正文，其中插值的业务字段值须经 `crate::utils::html_escape::escape_html`
    /// 转义；模板自身的排版标记保持原样。
    /// 传给谁：`Self::send_email_from`，出口点名后由通用发信出口做标记检测。
    /// 行为：检出 `<script>`、标签上的 `on*` 事件属性、`javascript:` 一类伪协议链接或样式表达式时
    /// 拒绝发送，错误文本点名发信出口、主题与命中类别；不做静默放行。
    /// 需要携带这类内容只有两条正路：把值按文本上下文转义，或走 `Self::send_html_email_declared`
    /// 做代码级显式声明。
    pub async fn send_html_email(
        &self,
        to: Vec<String>,
        subject: String,
        html_content: String,
    ) -> Result<(), AppError> {
        self.send_email_from(
            EmailMessage {
                to,
                cc: None,
                bcc: None,
                subject,
                html_content: Some(html_content),
                text_content: None,
                attachments: None,
            },
            HTML_SEND_SOURCE,
        )
        .await
    }

    /// 管理员编制正文的显式声明出口：检测照做，但命中项以 warn 日志留痕后仍然送出，不静默。
    /// 调用方：确需携带被默认检测拒绝的内容（例如对外部系统回显的原始片段）时使用。
    /// 入参：`declaration` 必须是调用点上的静态字面量（写清是谁授权的），不接受运行时拼接，
    /// 目的是让每一次绕过都必须在代码里留名，可 grep 可复审。
    /// 传给谁：`Self::dispatch_to_provider`。
    /// 边界：默认出口 `Self::send_html_email` 的拒绝语义不受本方法影响；正文里来自业务字段的值
    /// 仍然必须走转义，本方法只豁免"结构可信"的排版标记争议。
    pub async fn send_html_email_declared(
        &self,
        to: Vec<String>,
        subject: String,
        html_content: String,
        declaration: &'static str,
    ) -> Result<(), AppError> {
        for violation in scan_dangerous_markup(&html_content) {
            tracing::warn!(
                declaration = declaration,
                rule = violation.rule,
                detail = %violation.detail,
                subject_len = subject.len(),
                "已声明的 HTML 发信出口检出危险标记，按声明放行并逐条留痕"
            );
        }
        self.dispatch_to_provider(EmailMessage {
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

    /// 发送通知邮件：接受纯文本标题与正文，正文与本模块所有模板一样按 HTML 文本上下文转义。
    /// 入参：`title` 标题（同时作邮件主题）、`content` 纯文本正文、`action_url` 可选跳转链接。
    /// 传给谁：`EmailTemplate::notification_template`（纯文本正文途）构造正文，再交通用发信出口。
    pub async fn send_notification_email(
        &self,
        to: Vec<String>,
        title: &str,
        content: &str,
        action_url: Option<&str>,
    ) -> Result<(), AppError> {
        let html = EmailTemplate::notification_template(title, content, action_url);
        self.send_email_from(
            EmailMessage {
                to,
                cc: None,
                bcc: None,
                subject: title.to_string(),
                html_content: Some(html),
                text_content: Some(content.to_string()),
                attachments: None,
            },
            NOTIFICATION_SEND_SOURCE,
        )
        .await
    }

    /// 通过 SendGrid 发送邮件
    async fn send_via_sendgrid(&self, message: EmailMessage) -> Result<(), AppError> {
        let sendgrid_message = Self::build_sendgrid_message(&self.config.from_email, message);
        self.execute_sendgrid_request(sendgrid_message).await
    }

    /// 构造 SendGrid 请求体（personalizations + content + from + subject + attachments）
    /// 缺陷 6.3 修复：附件以 base64 编码方式提交给 SendGrid API
    fn build_sendgrid_message(from_email: &str, message: EmailMessage) -> SendGridMessage {
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

        // 缺陷 6.3 修复：附件转 base64 后构造 SendGrid 附件结构
        let attachments = message.attachments.as_ref().map(|files| {
            files
                .iter()
                .map(|(filename, file_content)| SendGridAttachment {
                    content: BASE64_STANDARD.encode(file_content),
                    filename: filename.clone(),
                    content_type: guess_content_type(filename),
                    disposition: "attachment".to_string(),
                })
                .collect::<Vec<_>>()
        });

        SendGridMessage {
            personalizations,
            from: SendGridEmail {
                email: from_email.to_string(),
            },
            subject: message.subject,
            content,
            attachments,
        }
    }

    /// 执行 SendGrid POST 请求并处理响应
    async fn execute_sendgrid_request(
        &self,
        sendgrid_message: SendGridMessage,
    ) -> Result<(), AppError> {
        // H-2 修复：使用硬编码的 SendGrid 官方 API URL，
        // 禁止从环境变量或配置中读取自定义 URL，防止 API Key 泄露到攻击者服务器。
        // SENDGRID_API_URL 是 &'static str，IntoUrl 已为 &str 实现，直接传值。
        let api_url: &str = SENDGRID_API_URL;

        // V15 P1 20.1-A：注入 traceparent 到 SendGrid 出站请求
        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = self
            .http_client
            .post(api_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
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
    /// 真实接入阿里云 DirectMail SingleSendMail API：API 端点：https://dm.aliyuncs.com/；签名算法：RPC V1（HMAC-SHA1 + Base64）；文档：https://help.aliyun.com/document_detail/29434.html；api_key 格式：`<AccessKeyId>:<AccessKeySecret>`，冒号分隔两部分。
    async fn send_via_aliyun(&self, message: EmailMessage) -> Result<(), AppError> {
        // 解析 api_key：格式为 "<AccessKeyId>:<AccessKeySecret>"
        let (_, access_key_secret) = self.split_aliyun_credentials().ok_or_else(|| {
            AppError::business(
                "阿里云邮件配置 api_key 格式错误，应为 <AccessKeyId>:<AccessKeySecret>",
            )
        })?;

        // 构造业务参数
        let to_address = message.to.join(",");
        let mut params: Vec<(&str, String)> = vec![
            ("Action", "SingleSendMail".to_string()),
            ("AccountName", self.config.from_email.clone()),
            ("AddressType", "1".to_string()),
            ("ReplyToAddress", "false".to_string()),
            ("ToAddress", to_address),
            ("Subject", message.subject.clone()),
            ("FromAlias", self.config.from_name.clone()),
        ];
        if let Some(html) = message.html_content {
            params.push(("HtmlBody", html));
        }
        if let Some(text) = message.text_content {
            params.push(("TextBody", text));
        }

        // 计算签名并构造完整请求 URL
        let signature = self.aliyun_sign(&params, &access_key_secret)?;
        let mut query = String::new();
        for (k, v) in &params {
            if !query.is_empty() {
                query.push('&');
            }
            query.push_str(k);
            query.push('=');
            query.push_str(&utf8_percent_encode(v, ALIYUN_SIGNATURE_ENCODE_SET).to_string());
        }
        query.push_str(&format!(
            "&Signature={}",
            utf8_percent_encode(&signature, ALIYUN_SIGNATURE_ENCODE_SET)
        ));

        let url = format!("{}?{}", ALIYUN_DM_API_URL, query);

        // V15 P1 20.1-A：注入 traceparent 到阿里云 DM 出站请求
        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = self
            .http_client
            .get(&url)
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
            .send()
            .await
            .map_err(|e| AppError::internal(format!("阿里云邮件发送请求失败: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());

        if status.is_success() {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "阿里云邮件发送失败: HTTP {} - {}",
                status, body
            )))
        }
    }

    /// 通过腾讯云邮件发送
    /// 真实接入腾讯云邮件服务 SES SendMail API：API 端点：https://ses.tencentcloudapi.com/；签名算法：TC3-HMAC-SHA256（V3 签名）；文档：https://cloud.tencent.com/document/product/1288/51034；api_key 格式：`<SecretId>:<SecretKey>`，冒号分隔两部分。
    async fn send_via_tencent(&self, message: EmailMessage) -> Result<(), AppError> {
        let (secret_id, secret_key) = self.split_tencent_credentials().ok_or_else(|| {
            AppError::business("腾讯云邮件配置 api_key 格式错误，应为 <SecretId>:<SecretKey>")
        })?;
        let payload = self.build_tencent_payload(message)?;
        let timestamp = Utc::now().timestamp();
        let authorization = self.tencent_sign(TencentSignParams {
            action: "SendMail",
            service: "ses",
            version: "2020-10-02",
            timestamp,
            payload: payload.as_str(),
            secret_id: secret_id.as_str(),
            secret_key: secret_key.as_str(),
        })?;
        let (status, body) = self
            .send_tencent_http_request(&payload, timestamp, &authorization)
            .await?;
        Self::handle_tencent_response(status, body)
    }

    /// 构建腾讯云邮件发送请求体并序列化为 JSON
    fn build_tencent_payload(&self, message: EmailMessage) -> Result<String, AppError> {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct TencentSimple {
            #[serde(rename = "Html")]
            html: Option<String>,
            #[serde(rename = "Text")]
            text: Option<String>,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct TencentSendMailRequest {
            from_email_address: String,
            destination: Vec<String>,
            subject: String,
            simple: TencentSimple,
        }

        let request_body = TencentSendMailRequest {
            from_email_address: format!("{} <{}>", self.config.from_name, self.config.from_email),
            destination: message.to,
            subject: message.subject,
            simple: TencentSimple {
                html: message.html_content,
                text: message.text_content,
            },
        };
        serde_json::to_string(&request_body)
            .map_err(|e| AppError::internal(format!("腾讯云请求体序列化失败: {}", e)))
    }

    /// 发送腾讯云 HTTP 请求并返回状态码与响应体
    async fn send_tencent_http_request(
        &self,
        payload: &str,
        timestamp: i64,
        authorization: &str,
    ) -> Result<(reqwest::StatusCode, String), AppError> {
        // V15 P1 20.1-A：注入 traceparent 到腾讯云 SES 出站请求
        let traceparent = crate::observability::trace_context::traceparent_from_current_span();
        let response = self
            .http_client
            .post(TENCENT_SES_API_URL)
            .header("X-TC-Action", "SendMail")
            .header("X-TC-Region", "ap-guangzhou")
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Version", "2020-10-02")
            .header("Authorization", authorization)
            .header("Content-Type", "application/json; charset=utf-8")
            .header(
                crate::observability::trace_context::TRACEPARENT_HEADER,
                traceparent,
            )
            .body(payload.to_string())
            .send()
            .await
            .map_err(|e| AppError::internal(format!("腾讯云邮件发送请求失败: {}", e)))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "未知错误".to_string());
        Ok((status, body))
    }

    /// 解析腾讯云响应：HTTP 2xx 且 body 无 Error 字段才算成功
    fn handle_tencent_response(status: reqwest::StatusCode, body: String) -> Result<(), AppError> {
        if status.is_success() {
            if body.contains("\"Error\"") {
                Err(AppError::internal(format!(
                    "腾讯云邮件发送业务失败: {}",
                    body
                )))
            } else {
                Ok(())
            }
        } else {
            Err(AppError::internal(format!(
                "腾讯云邮件发送失败: HTTP {} - {}",
                status, body
            )))
        }
    }

    /// 解析阿里云 api_key 为 (AccessKeyId, AccessKeySecret)
    fn split_aliyun_credentials(&self) -> Option<(String, String)> {
        let key = self.config.api_key.as_str();
        let idx = key.find(':')?;
        if idx == 0 || idx == key.len() - 1 {
            return None;
        }
        Some((key[..idx].to_string(), key[idx + 1..].to_string()))
    }

    /// 解析腾讯云 api_key 为 (SecretId, SecretKey)
    fn split_tencent_credentials(&self) -> Option<(String, String)> {
        let key = self.config.api_key.as_str();
        let idx = key.find(':')?;
        if idx == 0 || idx == key.len() - 1 {
            return None;
        }
        Some((key[..idx].to_string(), key[idx + 1..].to_string()))
    }

    /// 阿里云 RPC V1 签名算法
    /// 规则：1. 将公共参数 + 业务参数合并，按参数名 ASCII 字典序排序；2. 每个参数 URL 编码后用 `&` 拼接成 canonicalized query string；3. 待签名字符串 = `GET&%2F&` + URL编码(canonicalized query string)；4. Signature = BASE64(HMAC-SHA1(待签名字符串, AccessKeySecret + "&"))
    fn aliyun_sign(
        &self,
        biz_params: &[(&str, String)],
        access_key_secret: &str,
    ) -> Result<String, AppError> {
        // 公共参数
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let signature_nonce = format!(
            "{:016x}{:08x}",
            Utc::now().timestamp_millis() as u64,
            fastrand::u32(0..u32::MAX)
        );

        let mut all_params: Vec<(&str, String)> = vec![
            ("Format", "JSON".to_string()),
            ("Version", "2015-11-23".to_string()),
            (
                "AccessKeyId",
                self.split_aliyun_credentials()
                    .map(|(k, _)| k)
                    .unwrap_or_default(),
            ),
            ("SignatureMethod", "HMAC-SHA1".to_string()),
            ("Timestamp", timestamp),
            ("SignatureVersion", "1.0".to_string()),
            ("SignatureNonce", signature_nonce),
        ];
        all_params.extend(biz_params.iter().cloned());

        // 按参数名 ASCII 字典序排序（参数名相同时按值排序）
        // 批次 415：修复 needless_borrow，b.0 已是 &str 无需额外取地址（b.1 是 String 需保留 &）
        all_params.sort_by(|a, b| a.0.cmp(b.0).then_with(|| a.1.cmp(&b.1)));

        // 构造规范化请求字符串：每个参数 URL 编码后用 `&` 拼接
        let canonicalized: String = all_params
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    utf8_percent_encode(k, ALIYUN_SIGNATURE_ENCODE_SET),
                    utf8_percent_encode(v, ALIYUN_SIGNATURE_ENCODE_SET)
                )
            })
            .collect::<Vec<_>>()
            .join("&");

        // 待签名字符串：GET&%2F&<URL编码的规范化字符串>
        let string_to_sign = format!(
            "GET&%2F&{}",
            utf8_percent_encode(&canonicalized, ALIYUN_SIGNATURE_ENCODE_SET)
        );

        // HMAC-SHA1（key = AccessKeySecret + "&"）
        let mut mac = HmacSha1::new_from_slice(format!("{}&", access_key_secret).as_bytes())
            .map_err(|e| AppError::internal(format!("阿里云 HMAC 初始化失败: {}", e)))?;
        mac.update(string_to_sign.as_bytes());
        let signature = BASE64_STANDARD.encode(mac.finalize().into_bytes());

        Ok(signature)
    }

    /// 腾讯云 TC3-HMAC-SHA256 V3 签名算法
    /// 规则：1. CanonicalRequest = Method\nURI\nQueryString\nCanonicalHeaders\nSignedHeaders\nHashedPayload；2. CredentialScope = Date/Service/Version/tc3_request；3. StringToSign = "TC3-HMAC-SHA256\nTimestamp\nCredentialScope\nHashedCanonicalRequest"；4. SecretDate = HMAC-SHA256(Date, "TC3_SECRET_KEY")；5. SecretService = HMAC-SHA256(SecretDate, Service)；6. SecretSigning = HMAC-SHA256(SecretService, "tc3_request")；7. Signature = HEX(HMAC-SHA256(SecretSigning, StringToSign))；注意：region 不参与 V3 签名（仅出现在 X-TC-Region 请求头），故函数不接收 region 参数。；批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `TencentSignParams`，；消除 `clippy::too_many_arguments` 警告。
    fn tencent_sign(&self, params: TencentSignParams<'_>) -> Result<String, AppError> {
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(params.timestamp, 0)
            .ok_or_else(|| AppError::internal("腾讯云签名时间戳无效"))?
            .format("%Y-%m-%d")
            .to_string();

        // 1. CanonicalRequest
        // POST 请求 URI 为 "/"，QueryString 为空
        let canonical_headers = format!(
            "content-type:application/json; charset=utf-8\nhost:{}\nx-tc-action:{}\n",
            "ses.tencentcloudapi.com",
            params.action.to_lowercase()
        );
        let signed_headers = "content-type;host;x-tc-action";
        let hashed_payload = hex::encode(Sha256::digest(params.payload.as_bytes()));
        let canonical_request = format!(
            "POST\n/\n\n{}\n{}\n{}",
            canonical_headers, signed_headers, hashed_payload
        );

        // 2. StringToSign
        let credential_scope =
            format!("{}/{}/{}/tc3_request", date, params.service, params.version);
        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{}\n{}\n{}",
            params.timestamp, credential_scope, hashed_canonical_request
        );

        // 3. 多层 HMAC-SHA256（派生密钥链：SecretKey -> SecretDate -> SecretService -> SecretSigning）
        // 批次 413：使用 as_slice() 显式转换为 &[u8]，避免 &Vec<u8> 触发 clippy::needless_reference
        let secret_date = hmac_sha256_bytes(params.secret_key.as_bytes(), date.as_bytes());
        let secret_service = hmac_sha256_bytes(secret_date.as_slice(), params.service.as_bytes());
        let secret_signing = hmac_sha256_bytes(secret_service.as_slice(), b"tc3_request");
        let signature = hex::encode(hmac_sha256_bytes(
            secret_signing.as_slice(),
            string_to_sign.as_bytes(),
        ));

        // 4. Authorization
        Ok(format!(
            "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            params.secret_id, credential_scope, signed_headers, signature
        ))
    }

    /// 检查邮件服务是否可用
    pub async fn health_check(&self) -> Result<bool, AppError> {
        match self.config.provider.as_str() {
            "sendgrid" => {
                // V15 P1 20.1-A：健康检查也注入 traceparent
                let traceparent =
                    crate::observability::trace_context::traceparent_from_current_span();
                let response = self
                    .http_client
                    .get("https://api.sendgrid.com/v3/user/profile")
                    .header("Authorization", format!("Bearer {}", self.config.api_key))
                    .header(
                        crate::observability::trace_context::TRACEPARENT_HEADER,
                        traceparent,
                    )
                    .send()
                    .await
                    .map_err(|e| AppError::internal(format!("健康检查请求失败: {}", e)))?;
                Ok(response.status().is_success())
            }
            "aliyun" => {
                // 阿里云 DirectMail 无独立健康检查接口；
                // 通过配置校验（api_key 冒号分隔且两部分非空）作为可达性检查
                Ok(self.split_aliyun_credentials().is_some())
            }
            "tencent" => {
                // 腾讯云 SES 无独立健康检查接口；
                // 通过配置校验（api_key 冒号分隔且两部分非空）作为可达性检查
                Ok(self.split_tencent_credentials().is_some())
            }
            _ => Ok(true),
        }
    }
}

/// 计算 HMAC-SHA256，返回字节数组
/// L-12 修复（批次 376 v13 复审）：消除 spawn 任务内的 expect 调用。；HMAC-SHA256 接受任意长度密钥，new_from_slice 永远返回 Ok，；但为防御性编程，改为 match + error 日志兜底（不触发 panic）。；理论不可达的失败路径返回空 Vec，不影响业务正确性。
fn hmac_sha256_bytes(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = match HmacSha256::new_from_slice(key) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(error = %e, "HMAC-SHA256 new_from_slice 失败（理论不可达），返回空结果");
            return Vec::new();
        }
    };
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// 邮件模板
pub struct EmailTemplate;

impl EmailTemplate {
    /// 生成通知邮件 HTML 模板（纯文本正文途）。
    /// 入参：`content` 是纯文本正文，不含任何排版标记，本函数整体转义后注入。
    /// 传给谁：`EmailService::send_notification_email`。
    /// 需要模板自带 `<p>`、`<strong>` 一类标记的调用方一律走
    /// `Self::notification_template_with_safe_body`（可信正文途），两途由函数名区分，
    /// 选错就会把标记转义成收件人可见的文本，用例可直接证伪。
    pub fn notification_template(title: &str, content: &str, action_url: Option<&str>) -> String {
        Self::notification_template_with_safe_body(title, &escape_html(content), action_url)
    }

    /// 生成通知邮件 HTML 模板（可信正文途）。
    /// 入参：`safe_body` 是调用方已完成值转义的 HTML 片段，本函数不再对它做二次转义；
    /// `title` 与 `action_url` 仍由本函数负责转义。
    /// 传给谁：`Self::order_notification`、`Self::inventory_alert` 这类"模板自带标记 + 插值已转义"
    /// 的构造点；返回值交给 `EmailService` 的发信出口。
    /// 存什么：只拼装结构，不改写 `safe_body` 的一个字符。
    pub fn notification_template_with_safe_body(
        title: &str,
        safe_body: &str,
        action_url: Option<&str>,
    ) -> String {
        let safe_title = escape_html(title);
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
            safe_title, safe_title, safe_body, action_button
        )
    }

    /// 生成订单通知邮件。
    /// 入参：`order_no` 订单号（人工可录入）、`status` 状态中文串、`detail_url` 详情页路径。
    /// 传给谁：`EmailTemplate::notification_template_with_safe_body`（可信正文途），
    /// 再由 `EventNotificationService` 交给 `EmailService::send_html_email`。
    /// 存什么：订单号与状态先按 HTML 文本上下文转义，再拼进带 `<strong>` 标记的片段，
    /// 标记本身不再被二次转义。
    pub fn order_notification(order_no: &str, status: &str, detail_url: &str) -> String {
        let safe_order_no = escape_html(order_no);
        let safe_status = escape_html(status);
        let content = format!(
            r#"<p>您好，</p>
            <p>您的订单 <strong>{}</strong> 状态已更新为 <strong>{}</strong>。</p>
            <p>请登录系统查看详细信息。</p>"#,
            safe_order_no, safe_status
        );
        Self::notification_template_with_safe_body("订单状态更新", &content, Some(detail_url))
    }

    /// V15 P0-S14 死代码清理（批次 461）：删除 approval_notification 邮件模板方法
    /// 原因：唯一调用方 EventNotificationService::notify_pending_approval 已被删除，；该方法变为真实死代码。依据死代码处理规范第六章「真正未使用的项应显式删除」。
    //
    /// 生成库存预警邮件。
    /// 入参：`product_name` 产品名（人工录入）、`current_stock` 与 `threshold` 为数量展示串。
    /// 传给谁：`EmailTemplate::notification_template_with_safe_body`（可信正文途）。
    /// 存什么：三个值先转义成文本，再拼进 `<strong>` 标记片段，避免把排版标记二次转义。
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
        Self::notification_template_with_safe_body("库存预警通知", &content, None)
    }
}
