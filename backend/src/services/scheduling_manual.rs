//! P9-2 排程手动调整子模块
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 手动调整工单顺序
//! - 锁定/解锁工位
//! - 重新排程

use super::scheduling_service::SchedulingService;
use crate::models::production_order::Entity as ProductionOrderEntity;
use crate::services::scheduling_service::{AdjustScheduleRequest, ScheduleDetail};
use crate::utils::error::AppError;
use chrono::Utc;

/// P9-2 标记：手动调整子模块路径
pub const P92_MANUAL_MODULE: &str = "scheduling_manual";

/// 调整类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustType {
    MoveUp,
    MoveDown,
    MoveTop,
    MoveBottom,
    Lock,
    Unlock,
}

impl AdjustType {
    pub fn desc(&self) -> &'static str {
        match self {
            Self::MoveUp => "上移",
            Self::MoveDown => "下移",
            Self::MoveTop => "置顶",
            Self::MoveBottom => "置底",
            Self::Lock => "锁定",
            Self::Unlock => "解锁",
        }
    }
}

impl SchedulingService {
    // adjust_schedule
    // 内容来自原 scheduling_service.rs L531-582

    pub async fn adjust_schedule(
        &self,
        order_id: i32,
        req: AdjustScheduleRequest,
    ) -> Result<ScheduleDetail, AppError> {
        use sea_orm::EntityTrait;
        use sea_orm::{ActiveModelTrait, Set};

        let order = ProductionOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("生产订单不存在"))?;

        // batch-18 P3：检查锁定状态
        if order.status == "locked" {
            return Err(AppError::business("已锁定的订单不能调整"));
        }

        // batch-18 P3：冲突检测 - 检查同一工作中心是否有时间重叠
        if let (Some(wc_id), Some(start), Some(end)) = (
            req.work_center_id.or(order.work_center_id),
            req.start_date.or(order.planned_start_date),
            req.end_date.or(order.planned_end_date),
        ) {
            let conflicts = self
                .detect_schedule_conflicts(order_id, wc_id, start, end)
                .await?;
            if !conflicts.is_empty() {
                tracing::warn!(
                    order_id = order_id,
                    conflict_count = conflicts.len(),
                    "排程调整存在冲突"
                );
            }
        }

        use crate::models::production_order::ActiveModel;
        let mut active: ActiveModel = order.clone().into();

        if let Some(wc_id) = req.work_center_id {
            active.work_center_id = Set(Some(wc_id));
        }
        if let Some(start) = req.start_date {
            active.planned_start_date = Set(Some(start));
        }
        if let Some(end) = req.end_date {
            active.planned_end_date = Set(Some(end));
        }
        if let Some(priority) = req.priority {
            active.priority = Set(priority);
        }

        active.updated_at = Set(Utc::now());

        let updated = active.update(&*self.db).await?;

        let wc_name = self
            .get_work_center_name(updated.work_center_id)
            .await
            .unwrap_or_else(|| "未知".to_string());

        let today = Utc::now().date_naive();
        Ok(ScheduleDetail {
            order_id: updated.id,
            order_no: Some(updated.order_no.clone()),
            work_center_id: updated.work_center_id.unwrap_or(0),
            work_center_name: Some(wc_name),
            planned_start: updated.planned_start_date.unwrap_or(today),
            planned_end: updated.planned_end_date.unwrap_or(today),
            start_date: updated.planned_start_date,
            end_date: updated.planned_end_date,
            status: Some(updated.status.clone()),
        })
    }

    /// batch-18 P3：检测排程冲突
    async fn detect_schedule_conflicts(
        &self,
        exclude_order_id: i32,
        work_center_id: i32,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<i32>, AppError> {
        use crate::models::production_order::Column;
        use sea_orm::{ColumnTrait, QueryFilter};

        let conflicting_orders = ProductionOrderEntity::find()
            .filter(Column::WorkCenterId.eq(Some(work_center_id)))
            .filter(Column::Id.ne(exclude_order_id))
            .filter(Column::Status.is_in(vec!["SCHEDULED", "IN_PROGRESS"]))
            .filter(Column::PlannedStartDate.lt(end_date))
            .filter(Column::PlannedEndDate.gt(start_date))
            .all(&*self.db)
            .await?;

        Ok(conflicting_orders.into_iter().map(|o| o.id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_desc() {
        assert_eq!(AdjustType::MoveUp.desc(), "上移");
        assert_eq!(AdjustType::MoveDown.desc(), "下移");
        assert_eq!(AdjustType::MoveTop.desc(), "置顶");
        assert_eq!(AdjustType::MoveBottom.desc(), "置底");
        assert_eq!(AdjustType::Lock.desc(), "锁定");
        assert_eq!(AdjustType::Unlock.desc(), "解锁");
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_MANUAL_MODULE, "scheduling_manual");
    }
}
