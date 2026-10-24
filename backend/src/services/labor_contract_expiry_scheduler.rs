//! 劳动合同到期扫描调度器
//!
//! 周期性调用 [`crate::services::labor_contract_service::LaborContractService::scan_expiry_warnings`]
//! 生成到期/临近到期预警。该扫描带真实写入副作用：对已过期合同把状态从 active 自动流转为 expired
//! （《劳动合同法》口径下的到期收口），因此无人触发时线上过期合同状态永不闭环——本调度补齐触发方。
//!
//! 系统级全量扫描：headless 调度没有请求人，故 data_scope 传 None（不按归属人过滤，覆盖全部有效合同），
//! 与合规巡检语义一致。调用方仅消费本服务已有的公开方法，不改动 [`crate::services::labor_contract_service`]
//!（该文件归属其它车道）。
//!
//! 生命周期：与其余启动期定时任务同框架（MAIN_BACKGROUND_TASKS 注册 + 全局取消 Token 优雅退出）。
//! 入参来自配置层 `background_jobs.labor_contract_expiry_scan` 段。

use std::sync::Arc;

use sea_orm::DatabaseConnection;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::settings::LaborContractExpiryScanJob;
use crate::services::labor_contract_service::{ContractExpiryLevel, LaborContractService};

/// 劳动合同到期扫描调度器
pub struct LaborContractExpiryScheduler {
    db: Arc<DatabaseConnection>,
    job: LaborContractExpiryScanJob,
}

impl LaborContractExpiryScheduler {
    pub fn new(db: Arc<DatabaseConnection>, job: LaborContractExpiryScanJob) -> Self {
        Self { db, job }
    }

    /// 执行一次全量到期扫描，按预警档位分组计数打印（到期/90-60-30 天档位），失败显式上抛给调度循环记 warn
    pub async fn scan_once(&self) -> Result<usize, String> {
        let service = LaborContractService::new(self.db.clone());
        let warnings = service
            .scan_expiry_warnings(None)
            .await
            .map_err(|e| format!("劳动合同到期扫描失败: {e}"))?;

        let expired = warnings
            .iter()
            .filter(|w| matches!(w.level, ContractExpiryLevel::Expired))
            .count();
        let warn_90 = warnings
            .iter()
            .filter(|w| matches!(w.level, ContractExpiryLevel::Warning90Days))
            .count();
        let warn_60 = warnings
            .iter()
            .filter(|w| matches!(w.level, ContractExpiryLevel::Warning60Days))
            .count();
        let warn_30 = warnings
            .iter()
            .filter(|w| matches!(w.level, ContractExpiryLevel::Warning30Days))
            .count();

        info!(
            warning_total = warnings.len(),
            expired,
            warn_90,
            warn_60,
            warn_30,
            "劳动合同到期扫描调度：本轮成功（已对到期合同执行状态流转）"
        );
        Ok(warnings.len())
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
                "劳动合同到期扫描调度运行中（每 {} 秒扫描一次到期/临近到期合同，已过期自动流转 expired）",
                self.job.interval_secs
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        match self.scan_once().await {
                            Ok(_) => {}
                            Err(e) => warn!(error = %e, "劳动合同到期扫描调度：本轮失败，下一轮继续"),
                        }
                    }
                    _ = cancel.cancelled() => {
                        info!("劳动合同到期扫描调度收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }
}
