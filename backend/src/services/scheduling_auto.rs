//! P9-2 排程自动调度子模块
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 基于优先级和产能的自动排程
//! - 排程冲突检测
//! - 排程结果保存

use super::scheduling_service::SchedulingService;
use crate::models::production_order::{Entity as ProductionOrderEntity, Model as ProductionOrderModel};
use crate::models::scheduling_result::ActiveModel as SchedulingActiveModel;
use crate::models::work_center::{Entity as WorkCenterEntity, Model as WorkCenterModel};
use crate::services::capacity_service::WorkCenterCapacity;
use crate::services::scheduling_service::{
    AutoScheduleRequest, AutoScheduleResult, DateRange, GanttData, ScheduleConflict, ScheduleDetail,
    WorkCenterInfo,
};
use crate::utils::error::AppError;
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::collections::HashMap;

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
            return Ok(build_empty_schedule_result(&work_centers));
        }
        let total_orders = pending_orders.len();
        let sorted_orders = sort_orders_by_strategy(pending_orders, req.algo.as_str());
        let wc_capacity = build_wc_capacity_map(&work_centers);
        let mut wc_schedule = build_empty_wc_schedule(&wc_capacity);
        let mut wc_available_capacity = build_wc_available_capacity(&work_centers);
        let (mut conflicts, mut scheduled_details, mut scheduled_count) =
            (Vec::new(), Vec::new(), 0);
        for order in &sorted_orders {
            if self.schedule_single_order(
                order,
                &work_centers,
                &wc_capacity,
                &mut wc_schedule,
                &mut wc_available_capacity,
                req.start_date,
                &mut conflicts,
                &mut scheduled_details,
            ) {
                scheduled_count += 1;
            }
        }
        let gantt_data = self.build_gantt_data(&scheduled_details, &work_centers);
        Ok(build_schedule_result(
            scheduled_count,
            total_orders,
            scheduled_details,
            conflicts,
            gantt_data,
        ))
    }

    /// 处理单个工单的排程：校验工作中心/产能/时间槽，更新状态并返回是否成功排程。
    fn schedule_single_order(
        &self,
        order: &ProductionOrderModel,
        work_centers: &[WorkCenterModel],
        wc_capacity: &HashMap<i32, WorkCenterCapacity>,
        wc_schedule: &mut HashMap<i32, Vec<(NaiveDate, NaiveDate, i32, String)>>,
        wc_available_capacity: &mut HashMap<i32, Decimal>,
        start_date: NaiveDate,
        conflicts: &mut Vec<ScheduleConflict>,
        scheduled_details: &mut Vec<ScheduleDetail>,
    ) -> bool {
        let quantity = order.planned_quantity;
        let wc_id =
            order
                .work_center_id
                .unwrap_or_else(|| work_centers.first().map(|wc| wc.id).unwrap_or(0));
        if wc_id == 0 || !wc_capacity.contains_key(&wc_id) {
            conflicts.push(Self::build_no_work_center_conflict(order));
            return false;
        }
        let cap = &wc_capacity[&wc_id];
        if quantity.is_zero() {
            return false;
        }
        let available = wc_available_capacity.get(&wc_id).copied().unwrap_or(Decimal::ZERO);
        if quantity > available {
            conflicts.push(Self::build_capacity_insufficient_conflict(order, wc_id, cap, available));
            return false;
        }
        wc_available_capacity.insert(wc_id, available - quantity);
        let days_needed = Self::compute_days_needed(quantity, cap.daily_capacity).max(1);
        let schedule = wc_schedule.entry(wc_id).or_default();
        let assigned_start = self.find_earliest_slot(schedule, start_date, days_needed);
        let assigned_end = assigned_start + Duration::days(days_needed - 1);
        let has_overlap = schedule
            .iter()
            .any(|(s, e, _, _)| !(assigned_end < *s || assigned_start > *e));
        if has_overlap {
            conflicts.push(Self::build_time_overlap_conflict(order, wc_id, cap));
        }
        schedule.push((assigned_start, assigned_end, order.id, order.order_no.clone()));
        scheduled_details.push(Self::build_scheduled_detail(order, wc_id, cap, assigned_start, assigned_end));
        true
    }

    /// 构造单工单排程明细（SCHEDULED 状态）
    fn build_scheduled_detail(
        order: &ProductionOrderModel,
        wc_id: i32,
        cap: &WorkCenterCapacity,
        start: NaiveDate,
        end: NaiveDate,
    ) -> ScheduleDetail {
        ScheduleDetail {
            order_id: order.id,
            order_no: Some(order.order_no.clone()),
            work_center_id: wc_id,
            work_center_name: Some(cap.name.clone()),
            planned_start: start,
            planned_end: end,
            start_date: Some(start),
            end_date: Some(end),
            status: Some("SCHEDULED".to_string()),
        }
    }

    /// 计算工单所需加工天数（按数量/日产能向上取整，最小 1 天）。
    fn compute_days_needed(quantity: Decimal, daily_capacity: Decimal) -> i64 {
        if daily_capacity.is_zero() {
            return 1;
        }
        let d = quantity / daily_capacity;
        let rounded = d.round();
        rounded.to_string().parse::<i64>().unwrap_or(1)
    }

    /// 构造"未指定有效工作中心"冲突。
    fn build_no_work_center_conflict(order: &ProductionOrderModel) -> ScheduleConflict {
        ScheduleConflict {
            conflict_type: "NO_WORK_CENTER".to_string(),
            order_id: order.id,
            order_no: Some(order.order_no.clone()),
            conflicting_order_id: None,
            conflicting_order_no: None,
            work_center_id: 0,
            work_center_name: None,
            description: format!("工单 {} 未指定有效工作中心", order.order_no),
            severity: Some("HIGH".to_string()),
        }
    }

    /// 构造"工作中心产能不足"冲突。
    fn build_capacity_insufficient_conflict(
        order: &ProductionOrderModel,
        wc_id: i32,
        cap: &WorkCenterCapacity,
        available: Decimal,
    ) -> ScheduleConflict {
        ScheduleConflict {
            conflict_type: "CAPACITY_INSUFFICIENT".to_string(),
            order_id: order.id,
            order_no: Some(order.order_no.clone()),
            conflicting_order_id: None,
            conflicting_order_no: None,
            work_center_id: wc_id,
            work_center_name: Some(cap.name.clone()),
            description: format!(
                "工单 {} 需要产能 {}，工作中心 {} 可用产能不足（剩余 {}）",
                order.order_no, order.planned_quantity, cap.name, available
            ),
            severity: Some("HIGH".to_string()),
        }
    }

    /// 构造"工作时间重叠"冲突。
    fn build_time_overlap_conflict(
        order: &ProductionOrderModel,
        wc_id: i32,
        cap: &WorkCenterCapacity,
    ) -> ScheduleConflict {
        ScheduleConflict {
            conflict_type: "TIME_OVERLAP".to_string(),
            order_id: order.id,
            order_no: Some(order.order_no.clone()),
            conflicting_order_id: None,
            conflicting_order_no: None,
            work_center_id: wc_id,
            work_center_name: Some(cap.name.clone()),
            description: format!("工单 {} 在工作中心 {} 存在时间重叠", order.order_no, wc_id),
            severity: Some("MEDIUM".to_string()),
        }
    }

    /// 检测排程冲突
    pub async fn detect_conflicts(&self) -> Result<Vec<ScheduleConflict>, AppError> {
        let orders = ProductionOrderEntity::find()
            .filter(crate::models::production_order::Column::Status.ne("CANCELLED"))
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await?;

        let mut wc_orders: HashMap<i32, Vec<&ProductionOrderModel>> = HashMap::new();
        for order in &orders {
            if let Some(wc_id) = order.work_center_id {
                wc_orders.entry(wc_id).or_default().push(order);
            }
        }

        let mut conflicts = Self::detect_time_overlap_conflicts(&wc_orders);
        conflicts.extend(Self::detect_missing_date_conflicts(&orders));
        conflicts.extend(Self::detect_invalid_date_conflicts(&orders));
        Ok(conflicts)
    }

    /// 检测同一工作中心的时间重叠冲突
    fn detect_time_overlap_conflicts(
        wc_orders: &HashMap<i32, Vec<&ProductionOrderModel>>,
    ) -> Vec<ScheduleConflict> {
        let mut conflicts = Vec::new();
        for (wc_id, wc_order_list) in wc_orders {
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
                                order_no: Some(a.order_no.clone()),
                                conflicting_order_id: Some(b.id),
                                conflicting_order_no: Some(b.order_no.clone()),
                                work_center_id: *wc_id,
                                work_center_name: None,
                                description: format!(
                                    "工单 {} 和 {} 在工作中心 {} 时间重叠",
                                    a.order_no, b.order_no, wc_id
                                ),
                                severity: Some("HIGH".to_string()),
                            });
                        }
                    }
                }
            }
        }
        conflicts
    }

    /// 检测缺少计划日期的工单
    fn detect_missing_date_conflicts(orders: &[ProductionOrderModel]) -> Vec<ScheduleConflict> {
        let mut conflicts = Vec::new();
        for order in orders {
            if order.planned_start_date.is_none() || order.planned_end_date.is_none() {
                conflicts.push(ScheduleConflict {
                    conflict_type: "MISSING_DATES".to_string(),
                    order_id: order.id,
                    order_no: Some(order.order_no.clone()),
                    conflicting_order_id: None,
                    conflicting_order_no: None,
                    work_center_id: order.work_center_id.unwrap_or(0),
                    work_center_name: None,
                    description: format!("工单 {} 缺少计划日期", order.order_no),
                    severity: Some("MEDIUM".to_string()),
                });
            }
        }
        conflicts
    }

    /// 检测结束日期早于开始日期的工单
    fn detect_invalid_date_conflicts(orders: &[ProductionOrderModel]) -> Vec<ScheduleConflict> {
        let mut conflicts = Vec::new();
        for order in orders {
            if let (Some(start), Some(end)) = (order.planned_start_date, order.planned_end_date) {
                if end < start {
                    conflicts.push(ScheduleConflict {
                        conflict_type: "INVALID_DATES".to_string(),
                        order_id: order.id,
                        order_no: Some(order.order_no.clone()),
                        conflicting_order_id: None,
                        conflicting_order_no: None,
                        work_center_id: order.work_center_id.unwrap_or(0),
                        work_center_name: None,
                        description: format!("工单 {} 结束日期早于开始日期", order.order_no),
                        severity: Some("HIGH".to_string()),
                    });
                }
            }
        }
        conflicts
    }

    /// 保存排程结果
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
        // P3 维度 3 修复（批次 87）：消除 unwrap，改用 if let 显式模式匹配
        let (start_date, end_date) = if result
            .schedule_details
            .as_ref()
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            (now.date_naive(), now.date_naive())
        } else if let Some(details) = result.schedule_details.as_ref() {
            let min_start = details
                .iter()
                .map(|d| d.start_date.unwrap_or(d.planned_start))
                .min()
                .unwrap_or(now.date_naive());
            let max_end = details
                .iter()
                .map(|d| d.end_date.unwrap_or(d.planned_end))
                .max()
                .unwrap_or(now.date_naive());
            (min_start, max_end)
        } else {
            (now.date_naive(), now.date_naive())
        };

        let active_model = SchedulingActiveModel {
            id: Default::default(),
            batch_no: Set(batch_no),
            strategy: Set(strategy.to_string()),
            status: Set("DRAFT".to_string()),
            total_orders: Set(result.total_orders.unwrap_or(0)),
            scheduled_orders: Set(result.scheduled_count),
            unscheduled_orders: Set(
                result.unscheduled_orders.as_ref().map(|v| v.len() as i32).unwrap_or(0),
            ),
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

    /// 加载活跃工作中心
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
}

/// 构造空排程结果（无待排程工单时返回）。
fn build_empty_schedule_result(work_centers: &[WorkCenterModel]) -> AutoScheduleResult {
    AutoScheduleResult {
        scheduled_count: 0,
        conflicts: Vec::new(),
        gantt_data: GanttData {
            items: Vec::new(),
            work_centers: work_centers
                .iter()
                .map(|wc| WorkCenterInfo {
                    id: wc.id,
                    code: Some(wc.code.clone()),
                    name: wc.name.clone(),
                    status: Some(wc.status.clone()),
                })
                .collect(),
            date_range: Some(DateRange {
                start: Utc::now().date_naive(),
                end: Utc::now().date_naive(),
            }),
            schedule_details: None,
        },
        total_orders: Some(0),
        scheduled_orders: Some(Vec::new()),
        unscheduled_orders: Some(Vec::new()),
        schedule_details: Some(Vec::new()),
        id: None,
        batch_no: None,
    }
}

/// 按调度策略排序工单（priority/fifo/earliest_due/默认 priority）。
fn sort_orders_by_strategy(
    orders: Vec<ProductionOrderModel>,
    strategy: &str,
) -> Vec<ProductionOrderModel> {
    let mut sorted = orders;
    match strategy {
        "priority" => sorted.sort_by_key(|o| o.priority),
        "fifo" => sorted.sort_by_key(|o| o.created_at),
        "earliest_due" => sorted.sort_by_key(|o| o.planned_end_date.unwrap_or(NaiveDate::MAX)),
        _ => sorted.sort_by_key(|o| o.priority),
    }
    sorted
}

/// 构造工作中心产能映射（id -> WorkCenterCapacity）。
fn build_wc_capacity_map(
    work_centers: &[WorkCenterModel],
) -> HashMap<i32, WorkCenterCapacity> {
    let mut map = HashMap::new();
    for wc in work_centers {
        let daily_cap = wc.daily_capacity.unwrap_or(Decimal::new(100, 0));
        map.insert(
            wc.id,
            WorkCenterCapacity {
                id: wc.id,
                code: wc.code.clone(),
                name: wc.name.clone(),
                work_center_type: wc.work_center_type.clone(),
                daily_capacity: daily_cap,
                capacity_unit: Some("件".to_string()),
                status: wc.status.clone(),
                shifts: Vec::new(),
            },
        );
    }
    map
}

/// 构造空的工作中心排程表（id -> 空时间槽 Vec）。
fn build_empty_wc_schedule(
    wc_capacity: &HashMap<i32, WorkCenterCapacity>,
) -> HashMap<i32, Vec<(NaiveDate, NaiveDate, i32, String)>> {
    wc_capacity
        .keys()
        .map(|&wc_id| (wc_id, Vec::new()))
        .collect()
}

/// 构造工作中心 30 天可用总产能映射（id -> 总可用产能）。
fn build_wc_available_capacity(work_centers: &[WorkCenterModel]) -> HashMap<i32, Decimal> {
    let mut map = HashMap::new();
    for wc in work_centers {
        let daily_cap = wc.daily_capacity.unwrap_or(Decimal::new(100, 0));
        map.insert(wc.id, daily_cap * Decimal::from(30));
    }
    map
}

/// 构造最终排程结果（含 scheduled_orders 克隆与 schedule_details 移动）。
fn build_schedule_result(
    scheduled_count: i32,
    total_orders: usize,
    scheduled_details: Vec<ScheduleDetail>,
    conflicts: Vec<ScheduleConflict>,
    gantt_data: GanttData,
) -> AutoScheduleResult {
    AutoScheduleResult {
        scheduled_count,
        total_orders: Some(total_orders as i32),
        scheduled_orders: Some(scheduled_details.clone()),
        unscheduled_orders: Some(vec![]),
        schedule_details: Some(scheduled_details),
        conflicts,
        gantt_data,
        id: None,
        batch_no: None,
    }
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
