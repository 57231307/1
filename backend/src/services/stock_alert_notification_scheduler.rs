//! 库存告警通知调度任务（V15 P1 batch-18 缺陷 7.2）
//!
//! 设计依据：审计报告 batch-18 P1-7.2 — `get_stock_alerts` API 仅返回告警列表供前端查询，
//! 无主动推送（站内信/邮件），告警可能被忽略导致补货延误。
//!
//! 实现要点：
//! - 默认每 6 小时扫描一次 `inventory_stocks` 全量记录，调用 `compute_alert_type` 派生告警类型；
//! - 对非 `normal` 告警项，调用 `EventNotificationService::notify_inventory_alert` 推送
//!   站内信 + 邮件给 admin/manager 角色用户；
//! - 24h 去重：通过 `notification_service.dedup_key`（key 含 product_id + alert_type + 日期）
//!   保证同产品同类型告警 24h 内仅推送一次；
//! - 通知优先级由 EventNotificationService 内部按 INVENTORY 业务类型决定（Urgent）。
//!
//! 参考模板：`services/color_card_issue_scheduler.rs`（带 env 门控 + 审计服务可选）。

use std::sync::Arc;

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{info, warn};

use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::product::Entity as ProductEntity;
use crate::services::event_notification_service::EventNotificationService;
use crate::services::inventory_stock_query::compute_alert_type;
use crate::services::stock_alert::ALERT_TYPE_NORMAL;
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每 6 小时扫描一次
const DEFAULT_INTERVAL_SECS: u64 = 6 * 3600;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 180;

/// 单次扫描最多处理的告警数量 — 防止极端积压场景下长时间占用 DB
const MAX_ALERTS_PER_SCAN: u64 = 500;

/// 库存告警通知调度器
pub struct StockAlertNotificationScheduler {
    db: Arc<DatabaseConnection>,
}

impl StockAlertNotificationScheduler {
    /// 创建调度器实例。
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 执行一次扫描：查询全部库存记录，对派生告警项推送通知。
    ///
    /// 返回本次扫描发送通知的告警数量。
    pub async fn run_once(&self) -> Result<u64, AppError> {
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::QuantityAvailable.gte(rust_decimal::Decimal::ZERO))
            .all(&*self.db)
            .await?;

        let mut alert_count: u64 = 0;
        let mut processed: u64 = 0;
        for stock in stocks {
            if processed >= MAX_ALERTS_PER_SCAN {
                warn!(
                    processed,
                    max = MAX_ALERTS_PER_SCAN,
                    "库存告警通知：达到单次扫描上限，剩余告警下次循环处理"
                );
                break;
            }
            processed += 1;

            let alert_type = compute_alert_type(&stock);
            if alert_type == ALERT_TYPE_NORMAL {
                continue;
            }

            if let Err(e) = self.notify_stock_alert(&stock, alert_type).await {
                warn!(
                    stock_id = stock.id,
                    alert_type,
                    error = %e,
                    "库存告警通知：单条告警推送失败，跳过继续"
                );
                continue;
            }
            alert_count += 1;
        }

        if alert_count > 0 {
            info!(
                alert_count,
                processed,
                "库存告警通知：本轮扫描完成，发送 {} 条告警通知（共扫描 {} 条库存）",
                alert_count,
                processed
            );
        }
        Ok(alert_count)
    }

    /// 推送单条库存告警通知给 admin/manager 角色用户。
    async fn notify_stock_alert(
        &self,
        stock: &inventory_stock::Model,
        alert_type: &str,
    ) -> Result<(), AppError> {
        let product_name = self.fetch_product_name(stock.product_id).await;
        let notify_user_ids = self.fetch_admin_manager_user_ids().await;
        if notify_user_ids.is_empty() {
            return Ok(());
        }

        let alert_desc = Self::alert_desc(alert_type);
        let current_stock = format!("{}", stock.quantity_available);
        let threshold = format!(
            "补货点 {} / 上限 {}（告警类型：{}）",
            stock.reorder_point, stock.max_stock_point, alert_desc
        );

        let event_notifier = EventNotificationService::new(self.db.clone());
        for &user_id in &notify_user_ids {
            if let Err(e) = event_notifier
                .notify_inventory_alert(
                    user_id,
                    &product_name,
                    stock.product_id,
                    &current_stock,
                    &threshold,
                )
                .await
            {
                warn!(
                    user_id,
                    product_id = stock.product_id,
                    alert_type,
                    error = %e,
                    "库存告警通知：发送给用户失败，跳过该用户继续"
                );
            }
        }
        Ok(())
    }

    /// 拉取产品名称（失败时返回 fallback 字符串）。
    async fn fetch_product_name(&self, product_id: i32) -> String {
        match ProductEntity::find_by_id(product_id).one(&*self.db).await {
            Ok(Some(p)) => p.name,
            _ => format!("产品#{}", product_id),
        }
    }

    /// 拉取 admin/manager 角色用户 ID 列表（与 event_bus_ops::listener::fetch_admin_manager_user_ids 对齐）。
    async fn fetch_admin_manager_user_ids(&self) -> Vec<i32> {
        use crate::models::role::{self as role_model, Entity as RoleEntity};
        use crate::models::user::{Column as UserColumn, Entity as UserEntity};

        let roles = match RoleEntity::find()
            .filter(role_model::Column::Code.is_in(vec!["admin", "manager", "warehouse_manager"]))
            .all(&*self.db)
            .await
        {
            Ok(rs) => rs,
            Err(e) => {
                warn!(error = %e, "库存告警通知：拉取角色失败，返回空列表");
                return Vec::new();
            }
        };
        if roles.is_empty() {
            return Vec::new();
        }
        let role_ids: Vec<i32> = roles.iter().map(|r| r.id).collect();

        let users = match UserEntity::find()
            .filter(UserColumn::RoleId.is_in(role_ids))
            .all(&*self.db)
            .await
        {
            Ok(us) => us,
            Err(e) => {
                warn!(error = %e, "库存告警通知：拉取用户列表失败，返回空列表");
                return Vec::new();
            }
        };

        let mut user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();
        user_ids.sort_unstable();
        user_ids.dedup();
        user_ids
    }

    /// 告警类型中文描述（与 AlertType::desc 保持一致）。
    fn alert_desc(alert_type: &str) -> &'static str {
        match alert_type {
            "out_of_stock" => "缺货",
            "low_stock" => "低于下限",
            "over_stock" => "高于上限",
            "expiring" => "即将过期",
            "slow_moving" => "滞销",
            "discrepancy" => "盘点差异",
            _ => "未知告警",
        }
    }

    /// 启动后台调度任务（参考 ColorCardIssueExpiryScheduler 模式）。
    ///
    /// 启动后先延迟 `INITIAL_DELAY_SECS` 秒（避免与启动初始化争抢 DB），
    /// 然后以 `STOCK_ALERT_NOTIFICATION_INTERVAL_SECS`（默认 6 小时）为间隔循环执行。
    ///
    /// 环境变量门控：
    /// - `STOCK_ALERT_NOTIFICATION_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；
    /// - `STOCK_ALERT_NOTIFICATION_INTERVAL_SECS`（默认 21600=6h）— 扫描间隔。
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("STOCK_ALERT_NOTIFICATION_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                info!(
                    "库存告警通知调度器：环境变量 STOCK_ALERT_NOTIFICATION_ENABLED=false，跳过启动"
                );
                return;
            }

            let interval_secs = std::env::var("STOCK_ALERT_NOTIFICATION_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "库存告警通知调度器：后台任务已启动（每 {} 秒扫描一次全量库存并推送告警通知）",
                interval_secs
            );

            loop {
                match self.run_once().await {
                    Ok(count) if count > 0 => {
                        info!(count, "库存告警通知调度器：本轮发送 {} 条告警通知", count);
                    }
                    Ok(_) => {
                        // 无告警，静默
                    }
                    Err(e) => {
                        warn!(error = %e, "库存告警通知调度器：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_desc() {
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("out_of_stock"),
            "缺货"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("low_stock"),
            "低于下限"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("over_stock"),
            "高于上限"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("expiring"),
            "即将过期"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("slow_moving"),
            "滞销"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("discrepancy"),
            "盘点差异"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("unknown"),
            "未知告警"
        );
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_INTERVAL_SECS, 21600);
        assert_eq!(INITIAL_DELAY_SECS, 180);
        assert_eq!(MAX_ALERTS_PER_SCAN, 500);
    }
}
