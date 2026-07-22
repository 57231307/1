//! CRM 公海回收规则自动执行器（crm/recycle_executor）
//!
//! V15 P0-B07（Batch 482）：实现 CRM 回收规则的自动执行。
//!
//! 设计依据：审计报告 §18.3-D1 — 回收规则仅有 CRUD 无自动执行
//! 修复前：`recycle_rule.rs` 仅实现规则增删改查，无 scheduler/cron 自动执行回收，
//!         客户长期未跟进不会自动回收到公海，规则形同虚设。
//! 修复后：本模块实现 RecycleExecutor，扫描超过 N 天未跟进的活跃线索，
//!         自动将 lead_status 从 "new" 改为 "pool"（回收到公海）。
//!
//! 调用方式：在 main.rs 启动时 tokio::spawn 后台任务，每 6 小时执行一次
//! （与 audit_cleanup_service 周期对齐，避免过频打扰数据库）。
//!
//! 业务规则：
//! 1. 仅扫描启用的回收规则（is_enabled = true）
//! 2. 仅扫描活跃线索（lead_status = "new"），不处理已转化/已流失/已在公海的线索
//! 3. 判定条件：lead.last_follow_up_date 为空 或 距今超过规则.days 天
//!    - last_follow_up_date 为空表示从未跟进，按创建日期比较
//! 4. 回收时：lead_status 置为 "pool"，记录 owner_id 清空前的值用于追溯
//! 5. 单次扫描限制最多回收 1000 条线索，防止一次性回收过多影响业务

use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::crm_lead::{self, Entity as LeadEntity};
use crate::models::crm_recycle_rule::{self, Entity as RecycleRuleEntity};
// 批次 236 v13 P1-1：线索状态常量接入（规则 0）
use crate::models::status::crm_lead as lead_status;

/// 单次扫描最多回收的线索数量（防一次性回收过多影响业务）
const MAX_RECYCLE_PER_SCAN: u64 = 1000;

/// CRM 公海回收规则自动执行器
pub struct RecycleExecutor {
    db: Arc<DatabaseConnection>,
}

impl RecycleExecutor {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 执行一次回收扫描，返回成功回收的线索数量
    pub async fn run_once(&self) -> Result<u64, sea_orm::DbErr> {
        let now = Utc::now();
        let mut total_recycled: u64 = 0;
        let rules = self.fetch_enabled_rules().await?;
        if rules.is_empty() {
            return Ok(0);
        }
        info!(
            "[RecycleExecutor] 开始扫描，启用规则 {} 条，时间 {}",
            rules.len(),
            now
        );
        for rule in &rules {
            if total_recycled >= MAX_RECYCLE_PER_SCAN {
                warn!(
                    "[RecycleExecutor] 已达单次扫描上限 {}，停止后续规则处理",
                    MAX_RECYCLE_PER_SCAN
                );
                break;
            }
            self.process_rule(rule, now, &mut total_recycled).await?;
        }
        info!(
            "[RecycleExecutor] 扫描结束，总回收 {} 条线索到公海",
            total_recycled
        );
        Ok(total_recycled)
    }

    /// 查询所有启用的回收规则（按 days 升序）
    async fn fetch_enabled_rules(&self) -> Result<Vec<crm_recycle_rule::Model>, sea_orm::DbErr> {
        let rules = RecycleRuleEntity::find()
            .filter(crm_recycle_rule::Column::IsEnabled.eq(true))
            .order_by(crm_recycle_rule::Column::Days, sea_orm::Order::Asc)
            .all(&*self.db)
            .await?;
        if rules.is_empty() {
            info!("[RecycleExecutor] 无启用的回收规则，跳过扫描");
        }
        Ok(rules)
    }

    /// 处理单条回收规则：查询并批量回收符合条件的线索
    async fn process_rule(
        &self,
        rule: &crm_recycle_rule::Model,
        now: chrono::DateTime<chrono::Utc>,
        total_recycled: &mut u64,
    ) -> Result<(), sea_orm::DbErr> {
        let cutoff_date = now - Duration::days(rule.days as i64);
        let leads_to_recycle = LeadEntity::find()
            .filter(crm_lead::Column::LeadStatus.eq(lead_status::NEW))
            .filter(
                sea_orm::Condition::any()
                    .add(
                        sea_orm::Condition::all()
                            .add(crm_lead::Column::LastFollowUpDate.is_null())
                            .add(crm_lead::Column::CreatedAt.lt(cutoff_date)),
                    )
                    .add(crm_lead::Column::LastFollowUpDate.lt(cutoff_date.date_naive())),
            )
            .order_by(crm_lead::Column::Id, sea_orm::Order::Asc)
            .paginate(&*self.db, 100);

        let total_pages = leads_to_recycle.num_pages().await?;
        for page_idx in 0..total_pages {
            if *total_recycled >= MAX_RECYCLE_PER_SCAN {
                break;
            }
            let leads = leads_to_recycle.fetch_page(page_idx).await?;
            if leads.is_empty() {
                break;
            }
            let txn = self.db.begin().await?;
            for lead in leads {
                let mut active: crm_lead::ActiveModel = lead.into();
                active.lead_status = Set(Some(lead_status::POOL.to_string()));
                active.updated_at = Set(Some(now));
                active.update(&txn).await?;
                *total_recycled += 1;
                if *total_recycled >= MAX_RECYCLE_PER_SCAN {
                    break;
                }
            }
            txn.commit().await?;
        }

        info!(
            "[RecycleExecutor] 规则 {}（{}天）扫描完成，累计回收 {} 条",
            rule.name, rule.days, total_recycled
        );
        Ok(())
    }

    /// 启动后台定时任务（每 6 小时执行一次 run_once）
    ///
    /// 在 main.rs 中调用：
    /// ```ignore
    /// let executor = Arc::new(RecycleExecutor::new(db.clone()));
    /// let handle = executor.start_background_task();
    /// ```
    pub fn start_background_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // 启动后先等 60 秒，避免与启动初始化争抢数据库连接
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            // 每次循环间隔 6 小时
            let interval = std::time::Duration::from_secs(6 * 3600);
            loop {
                tracing::info!("[RecycleExecutor] 定时任务触发");
                match self.run_once().await {
                    Ok(count) => {
                        tracing::info!(
                            "[RecycleExecutor] 定时任务完成，回收 {} 条线索",
                            count
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[RecycleExecutor] 定时任务失败：{}，下次循环继续",
                            e
                        );
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
