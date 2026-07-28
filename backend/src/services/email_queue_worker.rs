//! 邮件队列后台 Worker（V15 P1 batch-16 缺陷 6.1/6.2/6.3 修复）
//!
//! 设计依据：审计报告 batch-16 缺陷 6.1 / 6.2 —
//! - 缺陷 6.1：邮件发送同步阻塞，无异步队列
//! - 缺陷 6.2：邮件失败重试机制不完整，无重试调度任务
//!
//! 实现要点：
//! - 后台 worker 每 60 秒（可配 `EMAIL_QUEUE_WORKER_INTERVAL_SECS`）扫描
//!   `status = PENDING AND retry_count < MAX_RETRY_COUNT AND (next_retry_at IS NULL OR next_retry_at <= now())`
//!   的邮件；
//! - 对每封邮件调用 `mark_as_sending` 乐观锁避免并发重复发送；
//! - 通过 `EmailService::from_env()` 创建邮件服务（未配置时仅记录 FAILED）；
//! - 成功 → SENT；失败 → `increment_retry` 接入指数退避（60s/300s/1800s），超过 3 次转入 FAILED 死信；
//! - 默认启用，可通过 `EMAIL_QUEUE_WORKER_ENABLED=false` 关闭。

use std::sync::Arc;

use chrono::Utc;
use sea_orm::DatabaseConnection;
use tracing::{info, warn};

use crate::models::email_log::Model as EmailLogModel;
use crate::services::email_log_service::{EmailLogService, MAX_RETRY_COUNT};
use crate::services::email_service::{EmailMessage, EmailService};
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每分钟扫描一次到期邮件
const DEFAULT_INTERVAL_SECS: u64 = 60;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 30;

/// 单次扫描最多处理的邮件数量 — 防止极端积压场景下长时间占用 DB
const MAX_EMAILS_PER_SCAN: u64 = 50;

/// 邮件队列后台 Worker
pub struct EmailQueueWorker {
    db: Arc<DatabaseConnection>,
    email_service: Option<EmailService>,
}

impl EmailQueueWorker {
    /// 创建 Worker 实例
    /// 邮件服务通过 `EmailService::from_env()` 创建：已配置 `EMAIL_PROVIDER` / `EMAIL_API_KEY` / `EMAIL_FROM` → 创建成功；未配置 → 返回 `None`，Worker 仍会扫描但所有 PENDING 邮件直接标记为 FAILED；（避免无效邮件在队列中无限堆积）。
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let email_service = EmailService::from_env();
        if email_service.is_none() {
            warn!(
                "邮件队列 Worker：未配置 EMAIL_PROVIDER/EMAIL_API_KEY/EMAIL_FROM，\
                 PENDING 邮件将直接标记为 FAILED（无法实际发送）"
            );
        }
        Self { db, email_service }
    }

    /// 执行一次扫描：查询到期邮件并逐个发送（返回本次扫描处理的邮件数量。）
    pub async fn run_once(&self) -> Result<u64, AppError> {
        let log_service = EmailLogService::new(self.db.clone());

        // 查询待发送邮件：PENDING + retry_count < MAX + (next_retry_at IS NULL OR <= now)
        let pending_emails = log_service
            .list_pending_for_retry(MAX_EMAILS_PER_SCAN)
            .await?;

        if pending_emails.is_empty() {
            return Ok(0);
        }

        let mut processed: u64 = 0;
        for email_log in pending_emails {
            match self.process_one_email(&log_service, &email_log).await {
                Ok(true) => processed += 1,
                Ok(false) => {
                    // 乐观锁失败：被其他 worker 实例先处理了，跳过
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        email_log_id = email_log.id,
                        "邮件队列 Worker：处理邮件失败，下次扫描继续"
                    );
                }
            }
        }

        Ok(processed)
    }

    /// 处理单封邮件：乐观锁标记 SENDING → 实际发送 → 更新状态（返回值：true 表示本次处理（发送成功或失败都算处理），false 表示乐观锁失败被跳过。）
    async fn process_one_email(
        &self,
        log_service: &EmailLogService,
        email_log: &EmailLogModel,
    ) -> Result<bool, AppError> {
        // 缺陷 6.1 修复：乐观锁 — 仅当当前状态仍为 PENDING 时才更新为 SENDING
        let acquired = log_service.mark_as_sending(email_log.id).await?;
        if !acquired {
            return Ok(false);
        }

        let email_service = match &self.email_service {
            Some(svc) => svc,
            None => {
                // 邮件服务未配置：直接标记为 FAILED，避免无效邮件在队列中堆积
                log_service
                    .update_status(
                        email_log.id,
                        "FAILED",
                        Some(
                            "邮件服务未配置（EMAIL_PROVIDER/EMAIL_API_KEY/EMAIL_FROM 缺失）"
                                .to_string(),
                        ),
                        None,
                    )
                    .await?;
                return Ok(true);
            }
        };

        // 构造 EmailMessage（含附件）
        let message = self.build_email_message(email_log)?;

        // 实际发送
        match email_service.send_email(message).await {
            Ok(_) => {
                log_service
                    .update_status(
                        email_log.id,
                        "SENT",
                        None,
                        Some(uuid::Uuid::new_v4().to_string()),
                    )
                    .await?;
                info!(
                    email_log_id = email_log.id,
                    recipients = %email_log.recipients,
                    "邮件队列 Worker：发送成功"
                );
            }
            Err(e) => {
                let err_msg = e.to_string();
                warn!(
                    email_log_id = email_log.id,
                    error = %err_msg,
                    "邮件队列 Worker：发送失败，调用 increment_retry 调度重试"
                );
                // 先更新为 FAILED + error_message，然后 increment_retry 会根据重试次数
                // 决定是否转为 PENDING（带 next_retry_at）或保持 FAILED 死信
                log_service
                    .update_status(email_log.id, "FAILED", Some(err_msg), None)
                    .await?;
                if let Err(retry_err) = log_service.increment_retry(email_log.id).await {
                    warn!(
                        error = %retry_err,
                        email_log_id = email_log.id,
                        "邮件队列 Worker：increment_retry 失败（不影响本次失败处理）"
                    );
                }
            }
        }

        Ok(true)
    }

    /// 从 EmailLogModel 构造 EmailMessage，包含附件解码
    fn build_email_message(&self, email_log: &EmailLogModel) -> Result<EmailMessage, AppError> {
        use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
        use base64::Engine;
        use std::collections::HashMap;

        let to: Vec<String> = email_log
            .recipients
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let cc = email_log.cc.as_ref().map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        });

        let bcc = email_log.bcc.as_ref().map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        });

        // 附件解码：从 JSON 数组还原为 HashMap<filename, Vec<u8>>
        let attachments = if let Some(attachments_json) = &email_log.attachments {
            if let Some(arr) = attachments_json.as_array() {
                let mut map: HashMap<String, Vec<u8>> = HashMap::new();
                for item in arr {
                    let filename = item
                        .get("filename")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let content_base64 = item
                        .get("content_base64")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !content_base64.is_empty() {
                        let content = BASE64_STANDARD.decode(content_base64).map_err(|e| {
                            AppError::internal(format!(
                                "附件 '{}' Base64 解码失败: {}",
                                filename, e
                            ))
                        })?;
                        map.insert(filename, content);
                    }
                }
                if map.is_empty() {
                    None
                } else {
                    Some(map)
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(EmailMessage {
            to,
            cc,
            bcc,
            subject: email_log.subject.clone(),
            html_content: email_log.html_content.clone().or(email_log.body.clone()),
            text_content: email_log.text_content.clone(),
            attachments,
        })
    }

    /// 启动后台调度任务（参考 ReportSubscriptionScheduler 模式）
    /// 环境变量门控：`EMAIL_QUEUE_WORKER_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；`EMAIL_QUEUE_WORKER_INTERVAL_SECS`（默认 60）— 扫描间隔。
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("EMAIL_QUEUE_WORKER_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                info!("邮件队列 Worker：环境变量 EMAIL_QUEUE_WORKER_ENABLED=false，跳过启动");
                return;
            }

            let interval_secs = std::env::var("EMAIL_QUEUE_WORKER_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            // 启动初始延迟
            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                max_emails_per_scan = MAX_EMAILS_PER_SCAN,
                max_retry_count = MAX_RETRY_COUNT,
                "邮件队列 Worker：后台任务已启动（每 {} 秒扫描一次 PENDING 邮件）",
                interval_secs
            );

            loop {
                let scan_start = Utc::now();
                match self.run_once().await {
                    Ok(count) if count > 0 => {
                        info!(
                            count,
                            elapsed_ms = (Utc::now() - scan_start).num_milliseconds(),
                            "邮件队列 Worker：本轮处理 {} 封邮件",
                            count
                        );
                    }
                    Ok(_) => {
                        // 无到期邮件，静默
                    }
                    Err(e) => {
                        warn!(error = %e, "邮件队列 Worker：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
