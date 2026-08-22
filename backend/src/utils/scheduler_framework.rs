//! 统一任务调度框架 trait + 注册中心
//!
//! A.6.1：项目存在 5 个独立 scheduler/worker（notification_scheduler /
//! report_subscription_scheduler / stock_alert_notification_scheduler /
//! color_card_issue_scheduler / email_queue_worker），各自为政，样板重复。
//! 本 trait 提供统一接口，各 scheduler 适配后可统一注册、启动、停止。
//!
//! 使用方式：
//! ```
//! use crate::utils::scheduler_framework::{Scheduler, SchedulerRegistry};
//!
//! struct MyScheduler { cancel: tokio_util::sync::CancellationToken }
//! impl Scheduler for MyScheduler {
//!     fn name(&self) -> &str { "my-scheduler" }
//!     fn interval_secs(&self) -> u64 { 3600 }
//!     async fn run_once(&self) -> Result<(), String> {
//!         // 执行一次调度任务
//!         Ok(())
//!     }
//! }
//!
//! // 注册并启动
//! let mut registry = SchedulerRegistry::new();
//! registry.register(Arc::new(MyScheduler { cancel: token.clone() }));
//! registry.start_all().await;
//! ```

use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// 调度器统一接口（A.6.1）
///
/// 各域 scheduler 适配此 trait 后，可统一注册到 SchedulerRegistry，
/// 由注册中心统一管理启动、停止和生命周期。
// 后续接入 SchedulerRegistry/StateMachine 时会使用
#[allow(dead_code)]
#[async_trait::async_trait]
pub trait Scheduler: Send + Sync {
    /// 调度器名称（用于日志和监控）
    fn name(&self) -> &str;

    /// 执行间隔（秒）
    fn interval_secs(&self) -> u64;

    /// 执行一次调度任务
    ///
    /// 实现方负责具体业务逻辑，返回 Err 时记录日志但不中断调度循环。
    async fn run_once(&self) -> Result<(), String>;
}

/// 调度器注册中心（A.6.1）
///
/// 统一管理所有调度器的注册、启动和停止。
/// 替代各 service 中各自 tokio::spawn 的样板代码。
// 后续接入 SchedulerRegistry/StateMachine 时会使用
#[allow(dead_code)]
pub struct SchedulerRegistry {
    schedulers: Vec<Arc<dyn Scheduler>>,
}

// 后续接入 SchedulerRegistry/StateMachine 时会使用
#[allow(dead_code)]
impl SchedulerRegistry {
    /// 创建空注册中心
    pub fn new() -> Self {
        Self {
            schedulers: Vec::new(),
        }
    }

    /// 注册调度器
    pub fn register(&mut self, scheduler: Arc<dyn Scheduler>) {
        tracing::info!(
            scheduler = scheduler.name(),
            interval_secs = scheduler.interval_secs(),
            "调度器已注册"
        );
        self.schedulers.push(scheduler);
    }

    /// 启动所有调度器（各自独立的 tokio::spawn 循环，受 CancellationToken 控制）
    ///
    /// 返回所有 spawn 的 JoinHandle，供 graceful shutdown 时 abort。
    pub async fn start_all(&self, cancel: CancellationToken) -> Vec<tokio::task::JoinHandle<()>> {
        let mut handles = Vec::new();
        for scheduler in &self.schedulers {
            let s = scheduler.clone();
            let token = cancel.clone();
            let name = s.name().to_string();
            let interval = std::time::Duration::from_secs(s.interval_secs());

            let handle = tokio::spawn(async move {
                tracing::info!(scheduler = %name, "调度器启动");
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {
                            if let Err(e) = s.run_once().await {
                                tracing::warn!(scheduler = %name, error = %e, "调度器执行失败");
                            }
                        }
                        _ = token.cancelled() => {
                            tracing::info!(scheduler = %name, "调度器收到取消信号，优雅退出");
                            break;
                        }
                    }
                }
            });
            handles.push(handle);
        }
        handles
    }

    /// 已注册调度器数量
    pub fn count(&self) -> usize {
        self.schedulers.len()
    }
}

impl Default for SchedulerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
