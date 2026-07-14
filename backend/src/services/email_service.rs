//! 邮件服务
//!
//! 提供邮件发送功能，支持通过 HTTP API 调用第三方邮件服务
//! 可配置为使用 SendGrid、阿里云邮件推送、腾讯云邮件等服务

use crate::utils::error::AppError;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

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
    /// 安全约束（H-2 修复 + P1-3 死字段清理，2026-06-25 综合审计）：
    /// - 禁止从 `EMAIL_API_URL` 环境变量读取 API URL，防止环境变量注入导致 API Key 被
    ///   发送到攻击者控制的服务器。
    /// - 各服务商 API URL 使用硬编码的官方地址（如 SENDGRID_API_URL 常量）。
    /// - EmailConfig.api_url 字段已删除，不再保留可被误用的自定义 URL 入口。
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
        // SENDGRID_API_URL 是 &'static str，IntoUrl 已为 &str 实现，直接传值。
        let api_url: &str = SENDGRID_API_URL;

        let response = self
            .http_client
            .post(api_url)
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
    ///
    /// 真实接入阿里云 DirectMail SingleSendMail API：
    /// - API 端点：https://dm.aliyuncs.com/
    /// - 签名算法：RPC V1（HMAC-SHA1 + Base64）
    /// - 文档：https://help.aliyun.com/document_detail/29434.html
    ///
    /// api_key 格式：`<AccessKeyId>:<AccessKeySecret>`，冒号分隔两部分。
    async fn send_via_aliyun(&self, message: EmailMessage) -> Result<(), AppError> {
        // 解析 api_key：格式为 "<AccessKeyId>:<AccessKeySecret>"
        let (_, access_key_secret) = self
            .split_aliyun_credentials()
            .ok_or_else(|| AppError::business("阿里云邮件配置 api_key 格式错误，应为 <AccessKeyId>:<AccessKeySecret>"))?;

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

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("阿里云邮件发送请求失败: {}", e)))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "未知错误".to_string());

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
    ///
    /// 真实接入腾讯云邮件服务 SES SendMail API：
    /// - API 端点：https://ses.tencentcloudapi.com/
    /// - 签名算法：TC3-HMAC-SHA256（V3 签名）
    /// - 文档：https://cloud.tencent.com/document/product/1288/51034
    ///
    /// api_key 格式：`<SecretId>:<SecretKey>`，冒号分隔两部分。
    async fn send_via_tencent(&self, message: EmailMessage) -> Result<(), AppError> {
        // 解析 api_key：格式为 "<SecretId>:<SecretKey>"
        let (secret_id, secret_key) = self
            .split_tencent_credentials()
            .ok_or_else(|| AppError::business("腾讯云邮件配置 api_key 格式错误，应为 <SecretId>:<SecretKey>"))?;

        // 构造请求体（SendMail API 入参）
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

        let payload = serde_json::to_string(&request_body)
            .map_err(|e| AppError::internal(format!("腾讯云请求体序列化失败: {}", e)))?;

        // 计算 V3 签名
        let timestamp = Utc::now().timestamp();
        let authorization = self.tencent_sign(
            "SendMail",
            "ses",
            "2020-10-02",
            timestamp,
            &payload,
            &secret_id,
            &secret_key,
        )?;

        let response = self
            .http_client
            .post(TENCENT_SES_API_URL)
            .header("X-TC-Action", "SendMail")
            .header("X-TC-Region", "ap-guangzhou")
            .header("X-TC-Timestamp", timestamp.to_string())
            .header("X-TC-Version", "2020-10-02")
            .header("Authorization", authorization)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(payload)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("腾讯云邮件发送请求失败: {}", e)))?;

        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "未知错误".to_string());

        if status.is_success() {
            // 腾讯云业务错误码在 200 响应 body 的 Response.Error 字段中
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
        Some((
            key[..idx].to_string(),
            key[idx + 1..].to_string(),
        ))
    }

    /// 解析腾讯云 api_key 为 (SecretId, SecretKey)
    fn split_tencent_credentials(&self) -> Option<(String, String)> {
        let key = self.config.api_key.as_str();
        let idx = key.find(':')?;
        if idx == 0 || idx == key.len() - 1 {
            return None;
        }
        Some((
            key[..idx].to_string(),
            key[idx + 1..].to_string(),
        ))
    }

    /// 阿里云 RPC V1 签名算法
    ///
    /// 规则：
    /// 1. 将公共参数 + 业务参数合并，按参数名 ASCII 字典序排序
    /// 2. 每个参数 URL 编码后用 `&` 拼接成 canonicalized query string
    /// 3. 待签名字符串 = `GET&%2F&` + URL编码(canonicalized query string)
    /// 4. Signature = BASE64(HMAC-SHA1(待签名字符串, AccessKeySecret + "&"))
    fn aliyun_sign(
        &self,
        biz_params: &[(&str, String)],
        access_key_secret: &str,
    ) -> Result<String, AppError> {
        // 公共参数
        let timestamp = Utc::now()
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let signature_nonce = format!(
            "{:016x}{:08x}",
            Utc::now().timestamp_millis() as u64,
            fastrand::u32(0..u32::MAX)
        );

        let mut all_params: Vec<(&str, String)> = vec![
            ("Format", "JSON".to_string()),
            ("Version", "2015-11-23".to_string()),
            ("AccessKeyId", self.split_aliyun_credentials().map(|(k, _)| k).unwrap_or_default()),
            ("SignatureMethod", "HMAC-SHA1".to_string()),
            ("Timestamp", timestamp),
            ("SignatureVersion", "1.0".to_string()),
            ("SignatureNonce", signature_nonce),
        ];
        all_params.extend(biz_params.iter().cloned());

        // 按参数名 ASCII 字典序排序（参数名相同时按值排序）
        all_params.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

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
    ///
    /// 规则：
    /// 1. CanonicalRequest = Method\nURI\nQueryString\nCanonicalHeaders\nSignedHeaders\nHashedPayload
    /// 2. CredentialScope = Date/Service/Version/tc3_request
    /// 3. StringToSign = "TC3-HMAC-SHA256\nTimestamp\nCredentialScope\nHashedCanonicalRequest"
    /// 4. SecretDate = HMAC-SHA256(Date, "TC3_SECRET_KEY")
    /// 5. SecretService = HMAC-SHA256(SecretDate, Service)
    /// 6. SecretSigning = HMAC-SHA256(SecretService, "tc3_request")
    /// 7. Signature = HEX(HMAC-SHA256(SecretSigning, StringToSign))
    ///
    /// 注意：region 不参与 V3 签名（仅出现在 X-TC-Region 请求头），故函数不接收 region 参数。
    #[allow(clippy::too_many_arguments)] // TODO(tech-debt): 腾讯云 V3 签名固定参数集，无法进一步聚合
    fn tencent_sign(
        &self,
        action: &str,
        service: &str,
        version: &str,
        timestamp: i64,
        payload: &str,
        secret_id: &str,
        secret_key: &str,
    ) -> Result<String, AppError> {
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
            .ok_or_else(|| AppError::internal("腾讯云签名时间戳无效"))?
            .format("%Y-%m-%d")
            .to_string();

        // 1. CanonicalRequest
        // POST 请求 URI 为 "/"，QueryString 为空
        let canonical_headers = format!(
            "content-type:application/json; charset=utf-8\nhost:{}\nx-tc-action:{}\n",
            "ses.tencentcloudapi.com",
            action.to_lowercase()
        );
        let signed_headers = "content-type;host;x-tc-action";
        let hashed_payload = hex::encode(Sha256::digest(payload.as_bytes()));
        let canonical_request = format!(
            "POST\n/\n\n{}\n{}\n{}",
            canonical_headers, signed_headers, hashed_payload
        );

        // 2. StringToSign
        let credential_scope = format!("{}/{}/{}/tc3_request", date, service, version);
        let hashed_canonical_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, hashed_canonical_request
        );

        // 3. 多层 HMAC-SHA256（派生密钥链：SecretKey -> SecretDate -> SecretService -> SecretSigning）
        let secret_date = hmac_sha256_bytes(secret_key.as_bytes(), date.as_bytes());
        let secret_service = hmac_sha256_bytes(&secret_date, service.as_bytes());
        let secret_signing = hmac_sha256_bytes(&secret_service, b"tc3_request");
        let signature = hex::encode(hmac_sha256_bytes(&secret_signing, string_to_sign.as_bytes()));

        // 4. Authorization
        Ok(format!(
            "TC3-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            secret_id, credential_scope, signed_headers, signature
        ))
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
///
/// L-12 修复（批次 376 v13 复审）：消除 spawn 任务内的 expect 调用。
/// HMAC-SHA256 接受任意长度密钥，new_from_slice 永远返回 Ok，
/// 但为防御性编程，改为 match + error 日志兜底（不触发 panic）。
/// 理论不可达的失败路径返回空 Vec，不影响业务正确性。
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
}
