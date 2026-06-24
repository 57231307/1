//! P9-2 排程查询与甘特图子模块
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 排程甘特图数据生成
//! - 排程查询
//! - 排程历史查询
//! - 排程结果确认

use super::scheduling_service::SchedulingService;
use crate::models::production_order::Entity as ProductionOrderEntity;
use crate::models::scheduling_result::{ActiveModel as SchedulingActiveModel, Entity as SchedulingResultEntity};
use crate::models::work_center::{Entity as WorkCenterEntity, Model as WorkCenterModel};
use crate::utils::error::AppError;
use crate::services::scheduling_service::{
    DateRange, GanttData, GanttItemDto, ScheduleDetail, ScheduledOrder,
    ScheduledOrderQuery, WorkCenterInfo,
};
use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};

/// P9-2 标记：排程查询子模块路径
pub const P92_QRY_MODULE: &str = "scheduling_query";

/// 甘特图任务项
#[derive(Debug, Clone)]
pub struct GanttItem {
    pub task_id: i32,
    pub name: String,
    pub start: chrono::NaiveDate,
    pub end: chrono::NaiveDate,
    pub progress: i32,
}

impl GanttItem {
    pub fn duration_days(&self) -> i64 {
        (self.end - self.start).num_days()
    }

    pub fn desc(&self) -> String {
        format!(
            "{}（{} → {}，{} 天，进度 {}%）",
            self.name,
            self.start,
            self.end,
            self.duration_days(),
            self.progress
        )
    }
}

impl SchedulingService {
    // get_gantt_data + list_scheduled_orders + get_schedule_history + get_schedule_result + confirm_schedule_result
    // 内容来自原 scheduling_service.rs L387-445 + L583-794 + L862-947
    // 私有 fn: build_gantt_data + get_work_center_name

    pub async fn get_gantt_data(
        &self,
        work_center_id: Option<i32>,
        date_from: Option<NaiveDate>,
        date_to: Option<NaiveDate>,
    ) -> Result<GanttData, AppError> {
        let mut orders = ProductionOrderEntity::find()
            .filter(crate::models::production_order::Column::Status.ne("CANCELLED"))
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await?;

        if let Some(wc_id) = work_center_id {
            orders.retain(|o| o.work_center_id == Some(wc_id));
        }

        let scheduled_details: Vec<ScheduleDetail> = orders
            .iter()
            .filter_map(|o| {
                let start = o.planned_start_date?;
                let end = o.planned_end_date?;

                if let Some(df) = date_from {
                    if end < df {
                        return None;
                    }
                }
                if let Some(dt) = date_to {
                    if start > dt {
                        return None;
                    }
                }

                // work_center_id 为 None 时表示"未指定"，有 ID 时由于当前闭包无法 await DB 查询，
                // 暂时 fallback 到"未知"（待后续重构为批量查询）
                let wc_name = if o.work_center_id.is_none() {
                    "未指定".to_string()
                } else {
                    "未知".to_string()
                };

                Some(ScheduleDetail {
                    order_id: o.id,
                    order_no: Some(o.order_no.clone()),
                    work_center_id: o.work_center_id.unwrap_or(0),
                    work_center_name: Some(wc_name),
                    planned_start: start,
                    planned_end: end,
                    start_date: Some(start),
                    end_date: Some(end),
                    status: Some(o.status.clone()),
                })
            })
            .collect();

        // 从生产订单关联的 work_center_id 列表批量查询工作中心，避免 N+1
        let work_center_ids: Vec<i32> = orders
            .iter()
            .filter_map(|o| o.work_center_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        let work_centers = if work_center_ids.is_empty() {
            Vec::new()
        } else {
            WorkCenterEntity::find()
                .filter(crate::models::work_center::Column::Id.is_in(work_center_ids))
                .all(&*self.db)
                .await?
        };
        let gantt = self.build_gantt_data(&scheduled_details, &work_centers);
        Ok(gantt)
    }

    /// 检测排程冲突
    pub async fn list_scheduled_orders(
        &self,
        query: ScheduledOrderQuery,
    ) -> Result<(Vec<ScheduledOrder>, u64), AppError> {
        let mut select = ProductionOrderEntity::find()
            .filter(crate::models::production_order::Column::Status.ne("CANCELLED"));

        if let Some(wc_id) = query.work_center_id {
            select = select.filter(crate::models::production_order::Column::WorkCenterId.eq(wc_id));
        }

        if let Some(status) = query.status {
            select = select.filter(crate::models::production_order::Column::Status.eq(status));
        }

        if let Some(date_from) = query.date_from {
            select = select
                .filter(crate::models::production_order::Column::PlannedEndDate.gte(date_from));
        }

        if let Some(date_to) = query.date_to {
            select = select
                .filter(crate::models::production_order::Column::PlannedStartDate.lte(date_to));
        }

        let total = select.clone().count(&*self.db).await?;

        let orders = select
            .order_by_asc(crate::models::production_order::Column::Priority)
            .paginate(&*self.db, query.page_size)
            .fetch_page(query.page - 1)
            .await?;

        let mut results: Vec<ScheduledOrder> = Vec::new();
        for order in orders {
            let wc_name = self
                .get_work_center_name(order.work_center_id)
                .await
                .unwrap_or_else(|| "未分配".to_string());

            results.push(ScheduledOrder {
                id: order.id,
                order_id: order.id,
                order_no: order.order_no,
                product_id: order.product_id,
                quantity: order.planned_quantity,
                work_center_id: order.work_center_id.unwrap_or(0),
                work_center_name: wc_name,
                planned_start: order.planned_start_date.unwrap_or(Utc::now().date_naive()),
                planned_end: order.planned_end_date.unwrap_or(Utc::now().date_naive()),
                start_time: order.planned_start_date.unwrap_or(Utc::now().date_naive()),
                end_time: order.planned_end_date.unwrap_or(Utc::now().date_naive()),
                actual_start: order.actual_start_date,
                actual_end: order.actual_end_date,
                status: order.status,
                priority: order.priority,
                dependencies: Vec::new(),
            });
        }

        Ok((results, total))
    }

    /// 持久化排程结果
    pub async fn get_schedule_history(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<crate::models::scheduling_result::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let paginator = SchedulingResultEntity::find()
            .order_by_desc(crate::models::scheduling_result::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;

        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取排程结果详情
    pub async fn get_schedule_result(
        &self,
        id: i32,
    ) -> Result<Option<crate::models::scheduling_result::Model>, AppError> {
        let model = SchedulingResultEntity::find_by_id(id)
            .one(&*self.db)
            .await?;

        Ok(model)
    }

    /// 确认排程结果并应用到生产订单
    pub async fn confirm_schedule_result(
        &self,
        id: i32,
        _user_id: i32,
    ) -> Result<crate::models::scheduling_result::Model, AppError> {
        let model = SchedulingResultEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("排程结果不存在"))?;

        if model.status != "DRAFT" {
            return Err(AppError::business(
                "只有草稿状态的排程结果可以确认".to_string(),
            ));
        }

        // 使用事务保护批量更新生产订单
        let txn = self.db.begin().await?;

        // 解析排程明细并应用到生产订单
        if let Some(details_json) = &model.schedule_details {
            if let Ok(details) = serde_json::from_value::<Vec<ScheduleDetail>>(details_json.clone())
            {
                for detail in &details {
                    if let Ok(Some(order)) = ProductionOrderEntity::find_by_id(detail.order_id)
                        .one(&txn)
                        .await
                        .map_err(|e| AppError::database(e.to_string()))
                    {
                        use crate::models::production_order::ActiveModel;
                        let mut active: ActiveModel = order.into();
                        active.planned_start_date = Set(detail.start_date);
                        active.planned_end_date = Set(detail.end_date);
                        active.work_center_id = Set(Some(detail.work_center_id));
                        // 自动将DRAFT状态更新为SCHEDULED
                        if let sea_orm::ActiveValue::Set(s) = &active.status {
                            if s == "DRAFT" {
                                active.status = Set("SCHEDULED".to_string());
                            }
                        }
                        active.updated_at = Set(Utc::now());
                        active.update(&txn).await?;
                    }
                }
            }
        }

        let mut active_model: SchedulingActiveModel = model.into();
        active_model.status = Set("CONFIRMED".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&txn).await?;

        txn.commit().await?;

        Ok(updated)
    }

    pub(crate) fn build_gantt_data(
        &self,
        details: &[ScheduleDetail],
        work_centers: &[WorkCenterModel],
    ) -> GanttData {
        let items: Vec<GanttItemDto> = details
            .iter()
            .map(|d| {
                let start = d.start_date.unwrap_or(d.planned_start);
                let end = d.end_date.unwrap_or(d.planned_end);
                let duration = (end - start).num_days() + 1;
                let progress = match d.status.as_deref().unwrap_or("") {
                    "COMPLETED" => 100.0,
                    "IN_PROGRESS" => 50.0,
                    "SCHEDULED" => 0.0,
                    _ => 0.0,
                };

                GanttItemDto {
                    id: format!("order_{}", d.order_id),
                    order_id: d.order_id,
                    order_no: d.order_no.clone().unwrap_or_default(),
                    product_id: 0,
                    work_center_id: d.work_center_id,
                    work_center_name: d.work_center_name.clone().unwrap_or_default(),
                    start_date: start,
                    end_date: end,
                    duration_days: duration,
                    progress,
                    status: d.status.clone().unwrap_or_default(),
                    priority: 0,
                    dependencies: Vec::new(),
                }
            })
            .collect();

        let wc_infos: Vec<WorkCenterInfo> = work_centers
            .iter()
            .map(|wc| WorkCenterInfo {
                id: wc.id,
                code: Some(wc.code.clone()),
                name: wc.name.clone(),
                status: Some(wc.status.clone()),
            })
            .collect();

        let today = Utc::now().date_naive();
        let date_range = if items.is_empty() {
            DateRange {
                start: today,
                end: today,
            }
        } else {
            let min_start = items.iter().map(|i| i.start_date).min().unwrap_or(today);
            let max_end = items.iter().map(|i| i.end_date).max().unwrap_or(today);
            DateRange {
                start: min_start,
                end: max_end,
            }
        };

        GanttData {
            items,
            work_centers: wc_infos,
            date_range: Some(date_range),
            schedule_details: None,
        }
    }

    /// 获取工作中心名称
    pub(crate) async fn get_work_center_name(&self, wc_id: Option<i32>) -> Option<String> {
        let wc_id = wc_id?;
        let wc = WorkCenterEntity::find_by_id(wc_id)
            .one(&*self.db)
            .await
            .ok()
            .flatten()?;
        Some(wc.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gantt_duration() {
        let item = GanttItem {
            task_id: 1,
            name: "工单 A".to_string(),
            start: crate::ymd!(2026, 6, 1),
            end: crate::ymd!(2026, 6, 8),
            progress: 50,
        };
        assert_eq!(item.duration_days(), 7);
        assert!(item.desc().contains("工单 A"));
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_QRY_MODULE, "scheduling_query");
    }
}
