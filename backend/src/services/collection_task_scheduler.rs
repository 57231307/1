//! 催收任务自动生成调度器
//!
//! 周期性调用 [`crate::services::collection_task_service::CollectionTaskService::auto_generate_tasks`]
//! 扫描逾期未收发票、按客户聚合、幂等创建 pending 催收任务（客户已有未关闭任务时跳过）。
//!
//! 受派人口径（重要业务约束）：`collection_tasks.assigned_to` 是 NOT NULL 且外键引用 users.id，
//! 每条自动任务必须落到一个真实存在的具体催收人名下。"自动生成后默认派给谁"属部署口径，
//! 代码不写死任何用户：受派人固定取配置 `background_jobs.collection_task_auto_generate.assign_user_id`
//! （由启动链校验为真实存在的 users.id 后才挂载本调度）。
//!
//! 生命周期：与其余启动期定时任务同框架（MAIN_BACKGROUND_TASKS 注册 + 全局取消 Token 优雅退出）。

use std::sync::Arc;

use sea_orm::{DatabaseConnection, EntityTrait};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::settings::CollectionTaskAutoGenerateJob;
use crate::models::collection_task_dto::AutoGenerateTasksRequest;
use crate::models::user;
use crate::services::collection_task_service::CollectionTaskService;

/// 催收任务自动生成调度器
pub struct CollectionTaskAutoGenerateScheduler {
    db: Arc<DatabaseConnection>,
    job: CollectionTaskAutoGenerateJob,
    /// 受派人用户 ID（由启动链校验存在后注入；assigned_to/assigned_by 均落到该用户）
    assign_user_id: i32,
}

impl CollectionTaskAutoGenerateScheduler {
    pub fn new(
        db: Arc<DatabaseConnection>,
        job: CollectionTaskAutoGenerateJob,
        assign_user_id: i32,
    ) -> Self {
        Self {
            db,
            job,
            assign_user_id,
        }
    }

    /// 执行一次自动生成，成功打印新建任务数，失败显式上抛给调度循环记 warn
    pub async fn scan_once(&self) -> Result<usize, String> {
        // 轮内预检受派人仍存在：assigned_to 是 NOT NULL 外键，若配置的用户在运行期被删除，
        // 整批建单会外键失败——此处以显式原因拦截，把"修配置"指给运维而不是抛裸外键错误。
        let assign_exists = user::Entity::find_by_id(self.assign_user_id)
            .one(&*self.db)
            .await
            .map_err(|e| format!("查询受派人用户失败: {e}"))?
            .is_some();
        if !assign_exists {
            return Err(format!(
                "配置的催收受派人用户 {} 在 users 表不存在，本轮中止（请修正 background_jobs.collection_task_auto_generate.assign_user_id）",
                self.assign_user_id
            ));
        }
        let service = CollectionTaskService::new(self.db.clone());
        let req = AutoGenerateTasksRequest {
            min_overdue_days: self.job.min_overdue_days,
            as_of_date: None,
        };
        let created = service
            .auto_generate_tasks(req, self.assign_user_id)
            .await
            .map_err(|e| format!("催收任务自动生成失败: {e}"))?;
        info!(
            created_total = created.len(),
            assign_user_id = self.assign_user_id,
            min_overdue_days = ?self.job.min_overdue_days,
            "催收任务自动生成调度：本轮成功（按客户聚合幂等建单）"
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
                assign_user_id = self.assign_user_id,
                "催收任务自动生成调度运行中（每 {} 秒扫描逾期发票并按客户聚合建单）",
                self.job.interval_secs
            );
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        match self.scan_once().await {
                            Ok(_) => {}
                            Err(e) => warn!(error = %e, "催收任务自动生成调度：本轮失败，下一轮继续"),
                        }
                    }
                    _ = cancel.cancelled() => {
                        info!("催收任务自动生成调度收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }
}
