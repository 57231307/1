//! 财务预警扫描调度器
//!
//! 周期性调用 [`crate::services::finance_alert_service::FinanceAlertService::trigger_scan`]
//! 全量扫描四类候选（应收逾期 / 库存积压 / 现金缺口 / 预算超支）并幂等生成 active 预警：
//! 同类型同目标已有未关闭（active/acknowledged）预警时跳过，不重复建行。
//! 系统触发无触发人（triggered_by=None），不为每条预警附带生成站内通知；
//! 预警消费入口是财务预警查询接口（前端列表/详情）。
//!
//! 生命周期：与其余启动期定时任务同一套框架——spawn 句柄注册进 MAIN_BACKGROUND_TASKS，
//! 循环内用 tokio::select! 同时等待扫描间隔与全局取消 Token，停机时优雅退出（abort 兜底）。
//!
//! 入参（开关/间隔/首轮延迟）全部来自配置层 `background_jobs.finance_alert_scan` 段
//! （见 [`crate::config::settings::FinanceAlertScanJob`]），源码不写魔法数字。

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::settings::FinanceAlertScanJob;
use crate::models::finance_alert_dto::TriggerScanRequest;
use crate::services::finance_alert_service::FinanceAlertService;

/// 财务预警扫描调度器
pub struct FinanceAlertScanScheduler {
    db: Arc<DatabaseConnection>,
    job: FinanceAlertScanJob,
}

impl FinanceAlertScanScheduler {
    pub fn new(db: Arc<DatabaseConnection>, job: FinanceAlertScanJob) -> Self {
        Self { db, job }
    }

    /// 执行一次全量扫描（四类 alert_type=None），成功打印命中总数与分类型新建计数，失败显式上抛给调度循环记 warn
    pub async fn scan_once(&self) -> Result<usize, String> {
        let service = FinanceAlertService::new(self.db.clone());
        let created = service
            .trigger_scan(TriggerScanRequest { alert_type: None }, None)
            .await
            .map_err(|e| format!("财务预警扫描失败: {e}"))?;
        let mut by_type: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for alert in &created {
            *by_type.entry(alert.alert_type.clone()).or_insert(0) += 1;
        }
        info!(
            created_total = created.len(),
            created_by_type = ?by_type,
            "财务预警扫描调度：本轮成功（四类全量幂等扫描）"
        );
        Ok(created.len())
    }

    /// 启动后台调度循环（受全局取消 Token 控制），返回 JoinHandle 供启动链注册与停机 abort。
    pub fn start_background_task(
        self: Arc<Self>,
        cancel: CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(self.job.initial_delay_secs)).await;
            let interval = std::time::Duration::from_secs(self.job.interval_secs);
            info!(
                interval_secs = self.job.interval_secs,
                "财务预警扫描调度运行中（每 {} 秒扫描应收逾期/库存积压/现金缺口/预算超支四类候选）",
                self.job.interval_secs
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        match self.scan_once().await {
                            Ok(_) => {}
                            Err(e) => warn!(error = %e, "财务预警扫描调度：本轮失败，下一轮继续"),
                        }
                    }
                    _ = cancel.cancelled() => {
                        info!("财务预警扫描调度收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }
}
