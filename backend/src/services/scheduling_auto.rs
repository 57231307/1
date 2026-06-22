//! P9-2 排程自动调度子模块
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 基于优先级和产能的自动排程
//! - 排程冲突检测
//! - 排程结果保存

use super::scheduling_service::SchedulingService;
use crate::models::production_order::{self, Entity as ProductionOrderEntity, Model as ProductionOrderModel};
use crate::services::scheduling_service::{AutoScheduleRequest, AutoScheduleResult, ScheduleConflict};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::sync::Arc;

/// P9-2 标记：自动排程子模块路径
pub const P92_AUTO_MODULE: &str = "scheduling_auto";

/// 排程算法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingAlgo {
    Fifo,
    Priority,
    Spt,
    Edd,
}

impl SchedulingAlgo {
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Fifo => "先进先出",
            Self::Priority => "优先级优先",
            Self::Spt => "最短加工时间",
            Self::Edd => "最早交货期",
        }
    }
}

impl SchedulingService {
    // auto_schedule / detect_conflicts / save_schedule_result
    // 内容来自原 scheduling_service.rs L186-386 + L446-530 + L795-861
    // 私有 fn: load_active_work_centers / load_pending_orders / find_earliest_slot

    pub async fn auto_schedule(
        &self,
        req: AutoScheduleRequest,
    ) -> Result<AutoScheduleResult, AppError> {
        let work_centers = self.load_active_work_centers(&req.work_center_ids).await?;
        let pending_orders = self.load_pending_orders().await?;

        if pending_orders.is_empty() {
            return Ok(AutoScheduleResult {
                total_orders: 0,
                scheduled_orders: 0,
                unscheduled_orders: 0,
                conflicts: Vec::new(),
                gantt_data: GanttData {
                    items: Vec::new(),
                    work_centers: work_centers
                        .iter()
                        .map(|wc| WorkCenterInfo {
                            id: wc.id,
                            code: wc.code.clone(),
                            name: wc.name.clone(),
                            status: wc.status.clone(),
                        })
                        .collect(),
                    date_range: DateRange {
                        start: Utc::now().date_naive(),
                        end: Utc::now().date_naive(),
                    },
                },
                schedule_details: Vec::new(),
            });
        }

        let mut sorted_orders = pending_orders.clone();
        let strategy = req.strategy.as_deref().unwrap_or("priority");
        match strategy {
            "priority" => sorted_orders.sort_by_key(|o| o.priority),
            "fifo" => sorted_orders.sort_by_key(|o| o.created_at),
            "earliest_due" => {
                sorted_orders.sort_by_key(|o| o.planned_end_date.unwrap_or(NaiveDate::MAX));
            }
            _ => sorted_orders.sort_by_key(|o| o.priority),
        }

        let mut wc_capacity: HashMap<i32, WorkCenterCapacity> = HashMap::new();
        for wc in &work_centers {
            let daily_cap = wc.daily_capacity.unwrap_or(Decimal::new(100, 0));
            wc_capacity.insert(
                wc.id,
                WorkCenterCapacity {
                    id: wc.id,
                    code: wc.code.clone(),
                    name: wc.name.clone(),
                    daily_capacity: daily_cap,
                    status: wc.status.clone(),
                },
            );
        }

        let mut wc_schedule: HashMap<i32, Vec<(NaiveDate, NaiveDate, i32, String)>> =
            HashMap::new();
        for wc_id in wc_capacity.keys() {
            wc_schedule.insert(*wc_id, Vec::new());
        }

        let start_date = req.start_date.unwrap_or(Utc::now().date_naive());
        let mut scheduled_details: Vec<ScheduleDetail> = Vec::new();
        let mut conflicts: Vec<ScheduleConflict> = Vec::new();
        let mut scheduled_count = 0;

        // 获取每个工作中心的可用产能信息
        let mut wc_available_capacity: HashMap<i32, Decimal> = HashMap::new();
        for wc in &work_centers {
            let daily_cap = wc.daily_capacity.unwrap_or(Decimal::new(100, 0));
            // 假设排程周期为30天，计算总可用产能
            let total_capacity = daily_cap * Decimal::from(30);
            wc_available_capacity.insert(wc.id, total_capacity);
        }

        for order in &sorted_orders {
            let quantity = order.planned_quantity;
            let wc_id = order.work_center_id.unwrap_or_else(|| {
                if let Some(first_wc) = work_centers.first() {
                    first_wc.id
                } else {
                    0
                }
            });

            if wc_id == 0 || !wc_capacity.contains_key(&wc_id) {
                conflicts.push(ScheduleConflict {
                    conflict_type: "NO_WORK_CENTER".to_string(),
                    order_id: order.id,
                    order_no: order.order_no.clone(),
                    conflicting_order_id: None,
                    conflicting_order_no: None,
                    work_center_id: None,
                    description: format!("工单 {} 未指定有效工作中心", order.order_no),
                    severity: "HIGH".to_string(),
                });
                continue;
            }

            let cap = &wc_capacity[&wc_id];
            if quantity.is_zero() {
                continue;
            }

            // 检查工作中心可用产能是否充足
            let available = wc_available_capacity
                .get(&wc_id)
                .copied()
                .unwrap_or(Decimal::ZERO);
            if quantity > available {
                conflicts.push(ScheduleConflict {
                    conflict_type: "CAPACITY_INSUFFICIENT".to_string(),
                    order_id: order.id,
                    order_no: order.order_no.clone(),
                    conflicting_order_id: None,
                    conflicting_order_no: None,
                    work_center_id: Some(wc_id),
                    description: format!(
                        "工单 {} 需要产能 {}，工作中心 {} 可用产能不足（剩余 {}）",
                        order.order_no, quantity, cap.name, available
                    ),
                    severity: "HIGH".to_string(),
                });
                continue;
            }

            // 更新工作中心已用产能
            wc_available_capacity.insert(wc_id, available - quantity);

            let days_needed = if cap.daily_capacity.is_zero() {
                1
            } else {
                let d = quantity / cap.daily_capacity;
                let rounded = d.round();
                let val = rounded.to_string().parse::<i64>().unwrap_or(1);
                val.max(1)
            };
            let days_needed = days_needed.max(1);

            let schedule = wc_schedule.entry(wc_id).or_default();
            let assigned_start = self.find_earliest_slot(schedule, start_date, days_needed);
            let assigned_end = assigned_start + Duration::days(days_needed - 1);

            let has_overlap = schedule
                .iter()
                .any(|(s, e, _, _)| !(assigned_end < *s || assigned_start > *e));

            if has_overlap {
                conflicts.push(ScheduleConflict {
                    conflict_type: "TIME_OVERLAP".to_string(),
                    order_id: order.id,
                    order_no: order.order_no.clone(),
                    conflicting_order_id: None,
                    conflicting_order_no: None,
                    work_center_id: Some(wc_id),
                    description: format!(
                        "工单 {} 在工作中心 {} 存在时间重叠",
                        order.order_no, wc_id
                    ),
                    severity: "MEDIUM".to_string(),
                });
            }

            schedule.push((
                assigned_start,
                assigned_end,
                order.id,
                order.order_no.clone(),
            ));

            let wc_name = cap.name.clone();
            scheduled_details.push(ScheduleDetail {
                order_id: order.id,
                order_no: order.order_no.clone(),
                work_center_id: wc_id,
                work_center_name: wc_name,
                start_date: assigned_start,
                end_date: assigned_end,
                status: "SCHEDULED".to_string(),
            });

            scheduled_count += 1;
        }

        let gantt_data = self.build_gantt_data(&scheduled_details, &work_centers);

        Ok(AutoScheduleResult {
            total_orders: pending_orders.len() as i32,
            scheduled_orders: scheduled_count,
            unscheduled_orders: pending_orders.len() as i32 - scheduled_count,
            conflicts,
            gantt_data,
            schedule_details: scheduled_details,
        })
    }

    /// 获取甘特图数据

    pub async fn detect_conflicts(&self) -> Result<Vec<ScheduleConflict>, AppError> {
        let orders = ProductionOrderEntity::find()
            .filter(crate::models::production_order::Column::Status.ne("CANCELLED"))
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await?;

        let mut conflicts: Vec<ScheduleConflict> = Vec::new();
        let mut wc_orders: HashMap<i32, Vec<&ProductionOrderModel>> = HashMap::new();

        for order in &orders {
            if let Some(wc_id) = order.work_center_id {
                wc_orders.entry(wc_id).or_default().push(order);
            }
        }

        for (wc_id, wc_order_list) in &wc_orders {
            let mut sorted = wc_order_list.clone();
            sorted.sort_by_key(|o| o.planned_start_date.unwrap_or(NaiveDate::MAX));

            for i in 0..sorted.len() {
                for j in (i + 1)..sorted.len() {
                    let a = sorted[i];
                    let b = sorted[j];

                    if let (Some(a_start), Some(a_end), Some(b_start), Some(b_end)) = (
                        a.planned_start_date,
                        a.planned_end_date,
                        b.planned_start_date,
                        b.planned_end_date,
                    ) {
                        if !(b_start > a_end || a_start > b_end) {
                            conflicts.push(ScheduleConflict {
                                conflict_type: "TIME_OVERLAP".to_string(),
                                order_id: a.id,
                                order_no: a.order_no.clone(),
                                conflicting_order_id: Some(b.id),
                                conflicting_order_no: Some(b.order_no.clone()),
                                work_center_id: Some(*wc_id),
                                description: format!(
                                    "工单 {} 和 {} 在工作中心 {} 时间重叠",
                                    a.order_no, b.order_no, wc_id
                                ),
                                severity: "HIGH".to_string(),
                            });
                        }
                    }
                }
            }
        }

        for order in &orders {
            if order.planned_start_date.is_none() || order.planned_end_date.is_none() {
                conflicts.push(ScheduleConflict {
                    conflict_type: "MISSING_DATES".to_string(),
                    order_id: order.id,
                    order_no: order.order_no.clone(),
                    conflicting_order_id: None,
                    conflicting_order_no: None,
                    work_center_id: order.work_center_id,
                    description: format!("工单 {} 缺少计划日期", order.order_no),
                    severity: "MEDIUM".to_string(),
                });
            }

            if let (Some(start), Some(end)) = (order.planned_start_date, order.planned_end_date) {
                if end < start {
                    conflicts.push(ScheduleConflict {
                        conflict_type: "INVALID_DATES".to_string(),
                        order_id: order.id,
                        order_no: order.order_no.clone(),
                        conflicting_order_id: None,
                        conflicting_order_no: None,
                        work_center_id: order.work_center_id,
                        description: format!("工单 {} 结束日期早于开始日期", order.order_no),
                        severity: "HIGH".to_string(),
                    });
                }
            }
        }

        Ok(conflicts)
    }

    /// 手动调整排程

    pub async fn save_schedule_result(
        &self,
        result: &AutoScheduleResult,
        strategy: &str,
        user_id: i32,
        user_name: &str,
        remarks: Option<String>,
    ) -> Result<crate::models::scheduling_result::Model, AppError> {
        let now = Utc::now();
        let batch_no = format!(
            "SCH-{}-{}",
            now.format("%Y%m%d%H%M%S"),
            crate::utils::random::random_6_digit()
        );

        // 计算日期范围
        let (start_date, end_date) = if result.schedule_details.is_empty() {
            (now.date_naive(), now.date_naive())
        } else {
            let min_start = result
                .schedule_details
                .iter()
                .map(|d| d.start_date)
                .min()
                .unwrap_or(now.date_naive());
            let max_end = result
                .schedule_details
                .iter()
                .map(|d| d.end_date)
                .max()
                .unwrap_or(now.date_naive());
            (min_start, max_end)
        };

        let active_model = SchedulingActiveModel {
            id: Default::default(),
            batch_no: Set(batch_no),
            strategy: Set(strategy.to_string()),
            status: Set("DRAFT".to_string()),
            total_orders: Set(result.total_orders),
            scheduled_orders: Set(result.scheduled_orders),
            unscheduled_orders: Set(result.unscheduled_orders),
            conflict_count: Set(result.conflicts.len() as i32),
            schedule_start_date: Set(start_date),
            schedule_end_date: Set(end_date),
            schedule_details: Set(Some(
                serde_json::to_value(&result.schedule_details).unwrap_or_default(),
            )),
            gantt_data: Set(Some(
                serde_json::to_value(&result.gantt_data).unwrap_or_default(),
            )),
            conflicts: Set(Some(
                serde_json::to_value(&result.conflicts).unwrap_or_default(),
            )),
            created_by: Set(user_id),
            created_by_name: Set(Some(user_name.to_string())),
            remarks: Set(remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 获取排程历史记录

    async fn load_active_work_centers(
        &self,
        ids: &Option<Vec<i32>>,
    ) -> Result<Vec<WorkCenterModel>, AppError> {
        let mut query = WorkCenterEntity::find()
            .filter(crate::models::work_center::Column::Status.eq("ACTIVE"));

        if let Some(id_list) = ids {
            if !id_list.is_empty() {
                query = query.filter(crate::models::work_center::Column::Id.is_in(id_list.clone()));
            }
        }

        query
            .order_by_asc(crate::models::work_center::Column::Code)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    /// 加载待排程工单

    async fn load_pending_orders(&self) -> Result<Vec<ProductionOrderModel>, AppError> {
        ProductionOrderEntity::find()
            .filter(crate::models::production_order::Column::Status.eq("DRAFT"))
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    /// 查找最早可用时间槽

    fn find_earliest_slot(
        &self,
        schedule: &[(NaiveDate, NaiveDate, i32, String)],
        start_date: NaiveDate,
        days_needed: i64,
    ) -> NaiveDate {
        if schedule.is_empty() {
            return start_date;
        }

        let mut candidate = start_date;
        let max_iterations = 365; // 防止无限循环
        let mut iterations = 0;

        loop {
            let end_candidate = candidate + Duration::days(days_needed - 1);

            let has_overlap = schedule
                .iter()
                .any(|(s, e, _, _)| !(end_candidate < *s || candidate > *e));

            if !has_overlap {
                return candidate;
            }

            // 找到下一个可用时间槽
            let next_start = schedule
                .iter()
                .filter(|(_s, e, _, _)| *e >= candidate)
                .map(|(_, e, _, _)| *e + Duration::days(1))
                .min()
                .unwrap_or(candidate + Duration::days(1));

            candidate = next_start;

            iterations += 1;
            if iterations >= max_iterations {
                // 超过最大迭代次数，返回当前候选日期（避免无限循环）
                return candidate;
            }
        }
    }

    /// 构建甘特图数据
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algo_desc() {
        assert_eq!(SchedulingAlgo::Fifo.desc(), "先进先出");
        assert_eq!(SchedulingAlgo::Priority.desc(), "优先级优先");
        assert_eq!(SchedulingAlgo::Spt.desc(), "最短加工时间");
        assert_eq!(SchedulingAlgo::Edd.desc(), "最早交货期");
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_AUTO_MODULE, "scheduling_auto");
    }
}
