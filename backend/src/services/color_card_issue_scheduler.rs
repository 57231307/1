//! 色卡发放过期检查定时任务（V15 P1 缺陷 10.5-1）
//!
//! 设计依据：审计报告 batch-09 P1-10.5-1 — 已发放色卡超过有效期后不会自动标记为过期，
//! 状态机无法闭环。
//!
//! 实现要点：
//! - 每日 02:00 扫描 `color_card_issues` 表中 `status='issued'` 且
//!   `expected_return_date < today` 的记录；
//! - 对每条过期记录：标记为 `cancelled`（remark="系统自动取消（超过预计归还日期）"），
//!   并恢复色卡 `issued_quantity`（库存联动，与 cancel_issue 一致）；
//! - 事务化：单条记录失败不影响其他记录处理；
//! - 审计日志：每条自动取消记录审计日志（resource_type=color_card_issue_expiry）。
//!
//! 环境变量门控：
//! - `COLOR_CARD_ISSUE_EXPIRY_CHECK_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；
//! - `COLOR_CARD_ISSUE_EXPIRY_CHECK_INTERVAL_SECS`（默认 86400=24h）— 扫描间隔。
//!
//! 参考模板：`services/report_subscription_scheduler.rs`（带 env 门控）。

use std::sync::Arc;

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use tracing::{info, warn};

use crate::models::audit_log::{OperationType, Severity};
use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_issue::{self, ActiveModel as IssueActive, Entity as IssueEntity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每 24 小时扫描一次
const DEFAULT_INTERVAL_SECS: u64 = 86400;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 120;

/// 单次扫描最多处理的过期记录数量 — 防止极端积压场景下长时间占用 DB
const MAX_EXPIRED_PER_SCAN: u64 = 500;

/// 色卡发放过期检查调度器
pub struct ColorCardIssueExpiryScheduler {
    db: Arc<DatabaseConnection>,
    audit_service: Option<Arc<AuditLogService>>,
}

impl ColorCardIssueExpiryScheduler {
    /// 创建调度器实例（audit_service 为可选：传入时记录审计日志，None 时仅执行业务逻辑。）
    pub fn new(db: Arc<DatabaseConnection>, audit_service: Option<Arc<AuditLogService>>) -> Self {
        Self { db, audit_service }
    }

    /// 执行一次扫描：查询过期的发放记录并逐个标记为 cancelled（返回本次扫描处理的过期记录数量。）
    pub async fn run_once(&self) -> Result<u64, AppError> {
        let today = Utc::now().date_naive();

        // 查询过期记录：status='issued' AND expected_return_date < today AND is_deleted=false
        let overdue_issues = IssueEntity::find()
            .filter(color_card_issue::Column::Status.eq("issued"))
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::ExpectedReturnDate.lt(today))
            .filter(color_card_issue::Column::ExpectedReturnDate.is_not_null())
            .order_by_asc(color_card_issue::Column::ExpectedReturnDate)
            .limit(MAX_EXPIRED_PER_SCAN)
            .all(&*self.db)
            .await?;

        let count = overdue_issues.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        info!(
            "色卡发放过期检查：扫描到 {} 条过期记录（expected_return_date < {}），开始处理",
            count, today
        );

        let mut success_count: u64 = 0;
        for issue in overdue_issues {
            // 单条记录失败不影响其他记录处理
            match self.cancel_expired_issue(&issue).await {
                Ok(_) => success_count += 1,
                Err(e) => {
                    warn!(
                        issue_id = issue.id,
                        color_card_id = issue.color_card_id,
                        expected_return_date = ?issue.expected_return_date,
                        error = %e,
                        "色卡发放过期检查：处理单条记录失败，跳过继续"
                    );
                }
            }
        }

        info!(
            "色卡发放过期检查：本轮扫描完成，处理 {} / {} 条过期记录",
            success_count, count
        );
        Ok(success_count)
    }

    /// 取消单条过期发放记录（事务化）
    /// 业务逻辑（与 cancel_issue 一致）：1. 更新发放记录 status='cancelled'，remark 追加"系统自动取消（超过预计归还日期）"；2. 恢复色卡 issued_quantity（-= issue_qty）；3. 记录审计日志（best-effort）
    async fn cancel_expired_issue(&self, issue: &color_card_issue::Model) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        // 锁定发放记录（避免与并发 cancel/return 冲突）
        let existing = IssueEntity::find_by_id(issue.id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发放记录 {} 不存在", issue.id)))?;

        // 双重校验：状态可能已被并发操作变更
        if existing.status != "issued" {
            return Err(AppError::business(format!(
                "发放记录 {} 状态为 {}（非 issued），跳过自动取消",
                existing.id, existing.status
            )));
        }

        let now = Utc::now();
        let auto_cancel_remark = format!(
            "系统自动取消（超过预计归还日期 {}）",
            existing.expected_return_date.unwrap_or_default()
        );
        let new_remark = match &existing.remark {
            Some(r) if !r.is_empty() => format!("{}\n{}", r, auto_cancel_remark),
            _ => auto_cancel_remark,
        };

        // 1. 更新发放记录
        let mut active: IssueActive = existing.clone().into();
        active.status = Set("cancelled".to_string());
        active.remark = Set(Some(new_remark));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;

        // 2. 恢复色卡 issued_quantity（库存联动）
        let card = ColorCardEntity::find_by_id(existing.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("色卡 {} 不存在", existing.color_card_id))
            })?;
        let new_issued = (card.issued_quantity - existing.issue_qty).max(0);
        let mut card_active: color_card::ActiveModel = card.into();
        card_active.issued_quantity = Set(new_issued);
        card_active.updated_at = Set(now);
        card_active.update(&txn).await?;

        txn.commit().await?;

        // 3. 记录审计日志（best-effort，不阻塞业务）
        if let Some(audit_svc) = &self.audit_service {
            let event = AuditEvent {
                user_id: None,
                username: Some("system_scheduler".to_string()),
                operation_type: OperationType::Delete,
                severity: Severity::Warn,
                resource_type: Some("color_card_issue_expiry".to_string()),
                resource_id: Some(updated.id.to_string()),
                resource_name: Some(format!("色卡发放记录#{}（自动过期取消）", updated.id)),
                description: Some(format!(
                    "定时任务自动取消过期发放记录：issue_id={}，色卡ID={}，客户ID={}，\
                     预计归还日期={:?}",
                    updated.id,
                    updated.color_card_id,
                    updated.customer_id,
                    updated.expected_return_date
                )),
                request_method: None,
                request_path: None,
                before_snapshot: Some(serde_json::json!({
                    "issue_id": existing.id,
                    "status": existing.status,
                    "expected_return_date": existing.expected_return_date,
                })),
                after_snapshot: Some(serde_json::json!({
                    "issue_id": updated.id,
                    "status": updated.status,
                    "remark": updated.remark,
                })),
            };
            audit_svc.clone().record_async(event, None);
        }

        Ok(())
    }

    /// 启动后台调度任务（参考 ReportSubscriptionScheduler 模式）
    /// 启动后先延迟 `INITIAL_DELAY_SECS` 秒（避免与启动初始化争抢 DB），；然后以 `COLOR_CARD_ISSUE_EXPIRY_CHECK_INTERVAL_SECS`（默认 86400 秒=24h）为间隔循环执行。；环境变量门控：`COLOR_CARD_ISSUE_EXPIRY_CHECK_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；`COLOR_CARD_ISSUE_EXPIRY_CHECK_INTERVAL_SECS`（默认 86400）— 扫描间隔。
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let enabled = std::env::var("COLOR_CARD_ISSUE_EXPIRY_CHECK_ENABLED")
                .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);
            if !enabled {
                info!(
                    "色卡发放过期检查：环境变量 COLOR_CARD_ISSUE_EXPIRY_CHECK_ENABLED=false，跳过启动"
                );
                return;
            }

            let interval_secs = std::env::var("COLOR_CARD_ISSUE_EXPIRY_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            // 启动初始延迟
            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "色卡发放过期检查：后台任务已启动（每 {} 秒扫描一次过期发放记录，默认每日 02:00 等效）",
                interval_secs
            );

            loop {
                match self.run_once().await {
                    Ok(count) if count > 0 => {
                        info!(count, "色卡发放过期检查：本轮处理 {} 条过期记录", count);
                    }
                    Ok(_) => {
                        // 无过期记录，静默
                    }
                    Err(e) => {
                        warn!(error = %e, "色卡发放过期检查：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
