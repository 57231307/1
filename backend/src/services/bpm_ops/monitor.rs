//! 流程监控统计子模块（bpm_ops/monitor）
//!
//! 从原 `bpm_service.rs` 迁移 3 个方法：
//! - `get_monitor_stats`：获取流程监控统计（实例/任务计数 + 平均处理时长）
//! - `get_pending_tasks_for_monitor`：获取待处理任务列表（分页）
//! - `list_instances_for_monitor`：获取流程实例列表（分页，可按状态过滤）

use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};

use crate::models::dto::PageResponse;
use crate::models::status::bpm_instance as instance_status;
use crate::models::status::bpm_task as task_status;
use crate::models::{bpm_process_instance, bpm_task};
use crate::services::bpm_service::BpmService;
use crate::services::bpm_service_dto::ProcessMonitorStats;
use crate::utils::error::AppError;

impl BpmService {
    // ========== 流程监控功能 ==========

    /// 获取流程监控统计
    pub async fn get_monitor_stats(&self) -> Result<ProcessMonitorStats, AppError> {
        let total_instances = bpm_process_instance::Entity::find()
            .count(&*self.db)
            .await?;

        let processing_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq(instance_status::PROCESSING))
            .count(&*self.db)
            .await?;

        let completed_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq(instance_status::COMPLETED))
            .count(&*self.db)
            .await?;

        let terminated_instances = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq(instance_status::TERMINATED))
            .count(&*self.db)
            .await?;

        let total_tasks = bpm_task::Entity::find().count(&*self.db).await?;

        let pending_tasks = bpm_task::Entity::find()
            // P0 3-4 修复：任务状态写入为小写（pending/completed/rejected），
            // 查询需用小写匹配，原大写导致统计永远返回 0
            .filter(bpm_task::Column::Status.eq(task_status::PENDING))
            .count(&*self.db)
            .await?;

        let completed_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::Status.eq(task_status::COMPLETED))
            .count(&*self.db)
            .await?;

        let rejected_tasks = bpm_task::Entity::find()
            .filter(bpm_task::Column::Status.eq(task_status::REJECTED))
            .count(&*self.db)
            .await?;

        // 计算平均流程处理时长（分钟）
        let avg_duration = bpm_process_instance::Entity::find()
            .filter(bpm_process_instance::Column::Status.eq(instance_status::COMPLETED))
            .filter(bpm_process_instance::Column::CompletedAt.is_not_null())
            .select_only()
            .column_as(
                sea_orm::sea_query::Expr::cust(
                    "AVG(EXTRACT(EPOCH FROM (completed_at - started_at)) / 60)",
                ),
                "avg_duration",
            )
            .into_tuple::<Option<f64>>()
            .one(&*self.db)
            .await?
            .flatten();

        Ok(ProcessMonitorStats {
            total_instances: total_instances as i64,
            processing_instances: processing_instances as i64,
            completed_instances: completed_instances as i64,
            terminated_instances: terminated_instances as i64,
            total_tasks: total_tasks as i64,
            pending_tasks: pending_tasks as i64,
            completed_tasks: completed_tasks as i64,
            rejected_tasks: rejected_tasks as i64,
            avg_process_duration_minutes: avg_duration,
        })
    }

    /// 获取待处理任务列表（用于监控）
    pub async fn get_pending_tasks_for_monitor(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<PageResponse<bpm_task::Model>, AppError> {
        // P0 3-4 修复：任务状态写入为小写，查询用小写匹配
        let stmt = bpm_task::Entity::find().filter(bpm_task::Column::Status.eq(task_status::PENDING));

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }

    /// 获取流程实例列表（用于监控）
    pub async fn list_instances_for_monitor(
        &self,
        status: Option<String>,
        page: u64,
        page_size: u64,
    ) -> Result<PageResponse<bpm_process_instance::Model>, AppError> {
        let mut stmt = bpm_process_instance::Entity::find();

        if let Some(s) = status {
            stmt = stmt.filter(bpm_process_instance::Column::Status.eq(s));
        }

        stmt = stmt.order_by_desc(bpm_process_instance::Column::CreatedAt);

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        let total_pages = if total == 0 {
            0
        } else {
            total.div_ceil(page_size)
        };
        Ok(PageResponse {
            data: items,
            total,
            page,
            page_size,
            total_pages,
        })
    }
}
