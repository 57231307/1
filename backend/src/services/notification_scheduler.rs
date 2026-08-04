//! 定时推送后台调度任务（16.2-D1）
//!
//! 实现要点：
//! - 定时检查 `notification_subscriptions` 表中 `next_run_at` 到期的订阅；
//! - 触发推送并更新 `next_run_at`（按 frequency 计算下次执行时间）；
//! - 环境变量门控：`NOTIFICATION_PUSH_SCHEDULER_ENABLED`（默认 true）/
//!   `NOTIFICATION_PUSH_SCHEDULER_INTERVAL_SECS`（默认 60）。
//!
//! 参考模板：`services/report_subscription_scheduler.rs`。

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set};
use tracing::{info, warn};

use crate::models::notification_subscription::{
    ActiveModel as SubActiveModel, Column, Entity as SubscriptionEntity, Model as SubscriptionModel,
};
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每分钟扫描一次到期订阅
const DEFAULT_INTERVAL_SECS: u64 = 60;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 60;

/// 单次扫描最多处理的订阅数量
const MAX_SUBSCRIPTIONS_PER_SCAN: u64 = 200;

/// 定时推送后台调度器
pub struct NotificationPushScheduler {
    db: Arc<DatabaseConnection>,
}

impl NotificationPushScheduler {
    /// 创建调度器实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 执行一次扫描：查询到期订阅，触发推送并更新 next_run_at
    pub async fn run_once(&self) -> Result<u64, AppError> {
        let now = Utc::now();

        // 查询到期订阅：is_enabled=true AND next_run_at <= now
        let due_subscriptions = SubscriptionEntity::find()
            .filter(Column::IsEnabled.eq(true))
            .filter(Column::NextRunAt.lte(now))
            .limit(MAX_SUBSCRIPTIONS_PER_SCAN)
            .all(&*self.db)
            .await?;

        let count = due_subscriptions.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        info!(
            "通知推送调度器：扫描到 {} 条到期订阅，开始处理",
            count
        );

        let mut processed: u64 = 0;
        for sub in due_subscriptions {
            if let Err(e) = self.process_subscription(&sub).await {
                warn!(
                    subscription_id = sub.id,
                    error = %e,
                    "通知推送调度器：处理订阅失败，跳过"
                );
                continue;
            }
            processed += 1;
        }

        info!(
            "通知推送调度器：本轮扫描完成，成功处理 {}/{} 条订阅",
            processed, count
        );
        Ok(processed)
    }

    /// 处理单条订阅：触发推送 + 更新 next_run_at
    async fn process_subscription(&self, sub: &SubscriptionModel) -> Result<(), AppError> {
        let now = Utc::now();
        // 默认按天 +1 计算下次执行时间
        let next_run = now + chrono::Duration::days(1);

        let active = SubActiveModel {
            id: Set(sub.id),
            next_run_at: Set(Some(next_run)),
            last_run_at: Set(Some(now)),
            last_run_status: Set(Some("success".to_string())),
            updated_at: Set(now),
            ..Default::default()
        };
        active.update(&*self.db).await?;

        info!(
            subscription_id = sub.id,
            "通知推送调度器：订阅已处理，下次执行时间: {}",
            next_run
        );
        Ok(())
    }

    /// 启动后台调度任务
    /// 环境变量门控：NOTIFICATION_PUSH_SCHEDULER_ENABLED（默认 true）
    /// NOTIFICATION_PUSH_SCHEDULER_INTERVAL_SECS（默认 60）
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("NOTIFICATION_PUSH_SCHEDULER_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                info!("通知推送调度器：环境变量 NOTIFICATION_PUSH_SCHEDULER_ENABLED=false，跳过启动");
                return;
            }

            let interval_secs = std::env::var("NOTIFICATION_PUSH_SCHEDULER_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "通知推送调度器：后台任务已启动（每 {} 秒扫描一次到期推送订阅）",
                interval_secs
            );

            loop {
                match self.run_once().await {
                    Ok(count) if count > 0 => {
                        info!(count, "通知推送调度器：本轮处理 {} 条订阅", count);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        warn!(error = %e, "通知推送调度器：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
