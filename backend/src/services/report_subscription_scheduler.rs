//! 报表订阅调度任务（P0-D16，Batch 488）
//!
//! 设计依据：审计报告 batch-16 P0-16-1 — `report_subscription` 表有 `next_run_at`
//! 字段但无后台调度任务触发，订阅即使配置了也不会自动执行。
//!
//! 实现要点：
//! - 每 60 秒（可配 `REPORT_SUBSCRIPTION_SCHEDULER_INTERVAL_SECS`）扫描
//!   `is_enabled=true AND status='ACTIVE' AND next_run_at <= now()` 的订阅；
//! - 对每条订阅发送 HTML 邮件通知（含报表查看链接），更新 `last_run_at` /
//!   `last_run_status` / `last_run_error` / `run_count` / `next_run_at`；
//! - 邮件服务通过 `EmailService::from_env()` 创建，未配置时跳过邮件发送但
//!   仍更新订阅执行状态（避免 next_run_at 永远停留在过去）；
//! - 默认启用，可通过 `REPORT_SUBSCRIPTION_SCHEDULER_ENABLED=false` 关闭。
//!
//! 参考模板：`services/crm/recycle_executor.rs`（无门控）+
//! `services/failover_service.rs::FailoverMonitor`（带 env 门控）。

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use tracing::{info, warn};

use crate::models::report_subscription::{
    ActiveModel, Column, Entity as ReportSubscriptionEntity, Model as ReportSubscriptionModel,
};
use crate::services::email_service::EmailService;
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每分钟扫描一次到期订阅
const DEFAULT_INTERVAL_SECS: u64 = 60;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 60;

/// 单次扫描最多处理的订阅数量 — 防止极端积压场景下长时间占用 DB
const MAX_SUBSCRIPTIONS_PER_SCAN: u64 = 100;

/// 报表订阅调度任务
pub struct ReportSubscriptionScheduler {
    db: Arc<DatabaseConnection>,
    email_service: Option<EmailService>,
    frontend_base_url: String,
}

impl ReportSubscriptionScheduler {
    /// 创建调度器实例。
    ///
    /// 邮件服务通过 `EmailService::from_env()` 创建：
    /// - 已配置 `EMAIL_PROVIDER` / `EMAIL_API_KEY` / `EMAIL_FROM` → 创建成功；
    /// - 未配置 → 返回 `None`，调度器仍会扫描并更新订阅状态，但跳过邮件发送。
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let email_service = EmailService::from_env();
        if email_service.is_none() {
            warn!(
                "报表订阅调度器：未配置 EMAIL_PROVIDER/EMAIL_API_KEY/EMAIL_FROM，\
                 邮件通知将被跳过（订阅状态仍会更新）"
            );
        }
        let frontend_base_url = std::env::var("BINGXI_FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:5173".to_string());
        Self {
            db,
            email_service,
            frontend_base_url,
        }
    }

    /// 执行一次扫描：查询到期订阅并逐个处理。
    ///
    /// 返回本次扫描处理的订阅数量。
    pub async fn run_once(&self) -> Result<u64, AppError> {
        let now = Utc::now();

        // 查询到期订阅：启用 + 活跃 + next_run_at <= now
        let due_subscriptions = ReportSubscriptionEntity::find()
            .filter(Column::IsEnabled.eq(true))
            .filter(Column::Status.eq("ACTIVE"))
            .filter(Column::NextRunAt.lte(now))
            .order_by_asc(Column::NextRunAt)
            .limit(MAX_SUBSCRIPTIONS_PER_SCAN)
            .all(&*self.db)
            .await?;

        let count = due_subscriptions.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        info!(
            "报表订阅调度器：扫描到 {} 条到期订阅，开始处理",
            count
        );

        for sub in due_subscriptions {
            // 单条订阅失败不影响其他订阅执行
            if let Err(e) = self.execute_subscription(&sub).await {
                warn!(
                    subscription_id = sub.id,
                    subscription_name = %sub.name,
                    error = %e,
                    "报表订阅执行失败"
                );
                if let Err(update_err) = self
                    .update_subscription_status(sub.id, false, Some(format!("{}", e)))
                    .await
                {
                    warn!(
                        subscription_id = sub.id,
                        error = %update_err,
                        "更新订阅失败状态时再次失败"
                    );
                }
            }
        }

        info!("报表订阅调度器：本轮扫描完成，处理 {} 条订阅", count);
        Ok(count)
    }

    /// 执行单条订阅：发送邮件通知 + 更新订阅状态。
    async fn execute_subscription(&self, sub: &ReportSubscriptionModel) -> Result<(), AppError> {
        let now = Utc::now();
        let recipients = self.extract_recipients(sub)?;
        let subject = format!("【报表订阅】{} 已到期执行", sub.name);
        let report_url = format!("{}/report/subscriptions/{}", self.frontend_base_url, sub.id);
        let html_content = format!(
            "<html><body>\
             <h2>报表订阅通知</h2>\
             <p>订阅名称：<strong>{}</strong></p>\
             <p>订阅频率：{}</p>\
             <p>导出格式：{}</p>\
             <p>触发时间：{}</p>\
             <p>请点击以下链接查看报表：</p>\
             <p><a href=\"{}\">{}</a></p>\
             <hr><p style=\"color:#888;font-size:12px;\">本邮件由系统自动发送，请勿回复。</p>\
             </body></html>",
            sub.name, sub.frequency, sub.export_format, now, report_url, report_url
        );

        // 邮件发送：未配置 EmailService 时跳过（仍更新订阅状态）
        if let Some(email_svc) = &self.email_service {
            if !recipients.is_empty() {
                email_svc
                    .send_html_email(recipients.clone(), subject.clone(), html_content.clone())
                    .await?;
            } else {
                warn!(
                    subscription_id = sub.id,
                    "订阅 recipients 为空，跳过邮件发送"
                );
            }
        }

        // 更新订阅状态为成功
        self.update_subscription_status(sub.id, true, None).await?;

        Ok(())
    }

    /// 从订阅 `recipients`（JSON 数组）提取邮箱列表。
    fn extract_recipients(
        &self,
        sub: &ReportSubscriptionModel,
    ) -> Result<Vec<String>, AppError> {
        let recipients: Vec<String> = match &sub.recipients {
            serde_json::Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            serde_json::Value::Object(_) => {
                return Err(AppError::business(format!(
                    "订阅 {} 的 recipients 字段格式错误（应为字符串数组，实际为 object）",
                    sub.id
                )));
            }
            _ => Vec::new(),
        };
        Ok(recipients)
    }

    /// 更新订阅执行状态：last_run_at / last_run_status / last_run_error / run_count / next_run_at。
    async fn update_subscription_status(
        &self,
        id: i32,
        success: bool,
        error: Option<String>,
    ) -> Result<(), AppError> {
        let existing = ReportSubscriptionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("订阅 {} 不存在", id)))?;

        // 在 Model 转为 ActiveModel 之前调用 calculate_next_run（Model 方法）
        let next_run = existing.calculate_next_run();
        let new_run_count = existing.run_count + 1;
        let now = Utc::now();

        let mut active_model: ActiveModel = existing.into();
        active_model.last_run_at = Set(Some(now));
        active_model.last_run_status = Set(Some(if success {
            "success".to_string()
        } else {
            "failed".to_string()
        }));
        active_model.last_run_error = Set(error);
        active_model.run_count = Set(new_run_count);
        active_model.next_run_at = Set(next_run);
        active_model.updated_at = Set(now);

        active_model.update(&*self.db).await?;
        Ok(())
    }

    /// 启动后台调度任务（参考 RecycleExecutor 模式）。
    ///
    /// 启动后先延迟 `INITIAL_DELAY_SECS` 秒（避免与启动初始化争抢 DB），
    /// 然后以 `REPORT_SUBSCRIPTION_SCHEDULER_INTERVAL_SECS`（默认 60 秒）为间隔循环执行。
    ///
    /// 环境变量门控：
    /// - `REPORT_SUBSCRIPTION_SCHEDULER_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；
    /// - `REPORT_SUBSCRIPTION_SCHEDULER_INTERVAL_SECS`（默认 60）— 扫描间隔。
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("REPORT_SUBSCRIPTION_SCHEDULER_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                info!("报表订阅调度器：环境变量 REPORT_SUBSCRIPTION_SCHEDULER_ENABLED=false，跳过启动");
                return;
            }

            let interval_secs = std::env::var("REPORT_SUBSCRIPTION_SCHEDULER_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            // 启动初始延迟
            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "报表订阅调度器：后台任务已启动（每 {} 秒扫描一次到期订阅）",
                interval_secs
            );

            loop {
                match self.run_once().await {
                    Ok(count) if count > 0 => {
                        info!(count, "报表订阅调度器：本轮处理 {} 条订阅", count);
                    }
                    Ok(_) => {
                        // 无到期订阅，静默
                    }
                    Err(e) => {
                        warn!(error = %e, "报表订阅调度器：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
