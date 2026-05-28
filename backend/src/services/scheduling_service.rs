//! 生产排程 Service
//!
//! 提供基于优先级和产能的自动排程、甘特图数据生成、冲突检测及手动调整功能

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use sea_orm::DatabaseConnection;
use sea_orm::Set;

use crate::models::production_order::{
    Entity as ProductionOrderEntity, Model as ProductionOrderModel,
};
use crate::models::scheduling_result::{ActiveModel as SchedulingActiveModel, Entity as SchedulingResultEntity};
use crate::models::work_center::{Entity as WorkCenterEntity, Model as WorkCenterModel};
use crate::utils::error::AppError;
use crate::services::capacity_service::CapacityService;

/// 排程工单
#[derive(Debug, Clone)]
pub struct ScheduledOrder {
    pub order_id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub quantity: Decimal,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_time: NaiveDate,
    pub end_time: NaiveDate,
    pub priority: i32,
    pub status: String,
    pub dependencies: Vec<i32>,
}

/// 工作中心产能信息
#[derive(Debug, Clone)]
pub struct WorkCenterCapacity {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub daily_capacity: Decimal,
    pub status: String,
}

/// 排程时间槽
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub work_center_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub order_id: Option<i32>,
    pub order_no: Option<String>,
    pub is_available: bool,
}

/// 排程冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConflict {
    pub conflict_type: String,
    pub order_id: i32,
    pub order_no: String,
    pub conflicting_order_id: Option<i32>,
    pub conflicting_order_no: Option<String>,
    pub work_center_id: Option<i32>,
    pub description: String,
    pub severity: String,
}

/// 甘特图数据项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttItem {
    pub id: String,
    pub order_id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub duration_days: i64,
    pub progress: f64,
    pub status: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
}

/// 甘特图响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttData {
    pub items: Vec<GanttItem>,
    pub work_centers: Vec<WorkCenterInfo>,
    pub date_range: DateRange,
}

/// 工作中心信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenterInfo {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: String,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 自动排程请求
#[derive(Debug, Clone)]
pub struct AutoScheduleRequest {
    pub work_center_ids: Option<Vec<i32>>,
    pub start_date: Option<NaiveDate>,
    pub strategy: Option<String>,
}

/// 自动排程结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScheduleResult {
    pub total_orders: i32,
    pub scheduled_orders: i32,
    pub unscheduled_orders: i32,
    pub conflicts: Vec<ScheduleConflict>,
    pub gantt_data: GanttData,
    pub schedule_details: Vec<ScheduleDetail>,
}

/// 排程明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDetail {
    pub order_id: i32,
    pub order_no: String,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
}

/// 手动调整排程请求
#[derive(Debug, Clone)]
pub struct AdjustScheduleRequest {
    pub work_center_id: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub priority: Option<i32>,
}

/// 排程工单列表查询参数
#[derive(Debug, Clone)]
pub struct ScheduledOrderQuery {
    pub work_center_id: Option<i32>,
    pub status: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    pub page: u64,
    pub page_size: u64,
}

/// 排程 Service
pub struct SchedulingService {
    db: Arc<DatabaseConnection>,
    capacity_service: Arc<CapacityService>,
}

impl SchedulingService {
    pub fn new(db: Arc<DatabaseConnection>, capacity_service: Arc<CapacityService>) -> Self {
        Self { db, capacity_service }
    }

    /// 自动排程 - 基于优先级和产能
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
            let available = wc_available_capacity.get(&wc_id).copied().unwrap_or(Decimal::ZERO);
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

            let has_overlap = schedule.iter().any(|(s, e, _, _)| {
                !(assigned_end < *s || assigned_start > *e)
            });

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
    pub async fn get_gantt_data(
        &self,
        work_center_id: Option<i32>,
        date_from: Option<NaiveDate>,
        date_to: Option<NaiveDate>,
    ) -> Result<GanttData, AppError> {
        let mut orders = ProductionOrderEntity::find()
            .filter(
                crate::models::production_order::Column::Status
                    .ne("CANCELLED"),
            )
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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

                let wc_name = o
                    .work_center_id
                    .map(|_| "未知".to_string())
                    .unwrap_or_default();

                Some(ScheduleDetail {
                    order_id: o.id,
                    order_no: o.order_no.clone(),
                    work_center_id: o.work_center_id.unwrap_or(0),
                    work_center_name: wc_name,
                    start_date: start,
                    end_date: end,
                    status: o.status.clone(),
                })
            })
            .collect();

        let work_centers = self.load_active_work_centers(&None).await?;
        let gantt = self.build_gantt_data(&scheduled_details, &work_centers);
        Ok(gantt)
    }

    /// 检测排程冲突
    pub async fn detect_conflicts(&self) -> Result<Vec<ScheduleConflict>, AppError> {
        let orders = ProductionOrderEntity::find()
            .filter(
                crate::models::production_order::Column::Status
                    .ne("CANCELLED"),
            )
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
    pub async fn adjust_schedule(
        &self,
        order_id: i32,
        req: AdjustScheduleRequest,
    ) -> Result<ScheduleDetail, AppError> {
        use sea_orm::ActiveModelTrait;
        use sea_orm::Set;

        let order = ProductionOrderEntity::find_by_id(order_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("生产订单不存在".to_string()))?;

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

        let updated = active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let wc_name = self
            .get_work_center_name(updated.work_center_id)
            .await
            .unwrap_or_else(|| "未知".to_string());

        Ok(ScheduleDetail {
            order_id: updated.id,
            order_no: updated.order_no.clone(),
            work_center_id: updated.work_center_id.unwrap_or(0),
            work_center_name: wc_name,
            start_date: updated.planned_start_date.unwrap_or(Utc::now().date_naive()),
            end_date: updated.planned_end_date.unwrap_or(Utc::now().date_naive()),
            status: updated.status.clone(),
        })
    }

    /// 获取排程工单列表
    pub async fn list_scheduled_orders(
        &self,
        query: ScheduledOrderQuery,
    ) -> Result<(Vec<ScheduledOrder>, u64), AppError> {
        let mut select = ProductionOrderEntity::find()
            .filter(
                crate::models::production_order::Column::Status
                    .ne("CANCELLED"),
            );

        if let Some(wc_id) = query.work_center_id {
            select = select.filter(
                crate::models::production_order::Column::WorkCenterId.eq(wc_id),
            );
        }

        if let Some(status) = query.status {
            select = select.filter(crate::models::production_order::Column::Status.eq(status));
        }

        if let Some(date_from) = query.date_from {
            select = select.filter(
                crate::models::production_order::Column::PlannedEndDate
                    .gte(date_from),
            );
        }

        if let Some(date_to) = query.date_to {
            select = select.filter(
                crate::models::production_order::Column::PlannedStartDate.lte(date_to),
            );
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let orders = select
            .order_by_asc(crate::models::production_order::Column::Priority)
            .paginate(&*self.db, query.page_size)
            .fetch_page(query.page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut results: Vec<ScheduledOrder> = Vec::new();
        for order in orders {
            let wc_name = self
                .get_work_center_name(order.work_center_id)
                .await
                .unwrap_or_else(|| "未分配".to_string());

            results.push(ScheduledOrder {
                order_id: order.id,
                order_no: order.order_no,
                product_id: order.product_id,
                quantity: order.planned_quantity,
                work_center_id: order.work_center_id.unwrap_or(0),
                work_center_name: wc_name,
                start_time: order.planned_start_date.unwrap_or(Utc::now().date_naive()),
                end_time: order.planned_end_date.unwrap_or(Utc::now().date_naive()),
                priority: order.priority,
                status: order.status,
                dependencies: Vec::new(),
            });
        }

        Ok((results, total))
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
            .map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    /// 加载待排程工单
    async fn load_pending_orders(&self) -> Result<Vec<ProductionOrderModel>, AppError> {
        ProductionOrderEntity::find()
            .filter(
                crate::models::production_order::Column::Status
                    .eq("DRAFT"),
            )
            .order_by_asc(crate::models::production_order::Column::Priority)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))
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
        let end_candidate = candidate + Duration::days(days_needed - 1);

        loop {
            let has_overlap = schedule.iter().any(|(s, e, _, _)| {
                !(end_candidate < *s || candidate > *e)
            });

            if !has_overlap {
                return candidate;
            }

            let next_start = schedule
                .iter()
                .filter(|(_s, e, _, _)| *e >= candidate)
                .map(|(_, e, _, _)| *e + Duration::days(1))
                .min()
                .unwrap_or(candidate + Duration::days(1));

            candidate = next_start;
            let new_end = candidate + Duration::days(days_needed - 1);

            if new_end > candidate + Duration::days(365) {
                return candidate;
            }
        }
    }

    /// 构建甘特图数据
    fn build_gantt_data(
        &self,
        details: &[ScheduleDetail],
        work_centers: &[WorkCenterModel],
    ) -> GanttData {
        let items: Vec<GanttItem> = details
            .iter()
            .map(|d| {
                let duration = (d.end_date - d.start_date).num_days() + 1;
                let progress = match d.status.as_str() {
                    "COMPLETED" => 100.0,
                    "IN_PROGRESS" => 50.0,
                    "SCHEDULED" => 0.0,
                    _ => 0.0,
                };

                GanttItem {
                    id: format!("order_{}", d.order_id),
                    order_id: d.order_id,
                    order_no: d.order_no.clone(),
                    product_id: 0,
                    work_center_id: d.work_center_id,
                    work_center_name: d.work_center_name.clone(),
                    start_date: d.start_date,
                    end_date: d.end_date,
                    duration_days: duration,
                    progress,
                    status: d.status.clone(),
                    priority: 0,
                    dependencies: Vec::new(),
                }
            })
            .collect();

        let wc_infos: Vec<WorkCenterInfo> = work_centers
            .iter()
            .map(|wc| WorkCenterInfo {
                id: wc.id,
                code: wc.code.clone(),
                name: wc.name.clone(),
                status: wc.status.clone(),
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
            date_range,
        }
    }

    /// 获取工作中心名称
    async fn get_work_center_name(&self, wc_id: Option<i32>) -> Option<String> {
        let wc_id = wc_id?;
        let wc = WorkCenterEntity::find_by_id(wc_id)
            .one(&*self.db)
            .await
            .ok()
            .flatten()?;
        Some(wc.name)
    }

    /// 持久化排程结果
    pub async fn save_schedule_result(
        &self,
        result: &AutoScheduleResult,
        strategy: &str,
        user_id: i32,
        user_name: &str,
        remarks: Option<String>,
    ) -> Result<crate::models::scheduling_result::Model, AppError> {
        let now = Utc::now();
        let batch_no = format!("SCH-{}-{}", now.format("%Y%m%d%H%M%S"), fastrand::u32(100000..999999));

        // 计算日期范围
        let (start_date, end_date) = if result.schedule_details.is_empty() {
            (now.date_naive(), now.date_naive())
        } else {
            let min_start = result.schedule_details.iter().map(|d| d.start_date).min().unwrap_or(now.date_naive());
            let max_end = result.schedule_details.iter().map(|d| d.end_date).max().unwrap_or(now.date_naive());
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
            schedule_details: Set(Some(serde_json::to_value(&result.schedule_details).unwrap_or_default())),
            gantt_data: Set(Some(serde_json::to_value(&result.gantt_data).unwrap_or_default())),
            conflicts: Set(Some(serde_json::to_value(&result.conflicts).unwrap_or_default())),
            created_by: Set(user_id),
            created_by_name: Set(Some(user_name.to_string())),
            remarks: Set(remarks),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取排程历史记录
    pub async fn get_schedule_history(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<crate::models::scheduling_result::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let paginator = SchedulingResultEntity::find()
            .order_by_desc(crate::models::scheduling_result::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let items = paginator.fetch_page(page - 1).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((items, total))
    }

    /// 获取排程结果详情
    pub async fn get_schedule_result(
        &self,
        id: i32,
    ) -> Result<Option<crate::models::scheduling_result::Model>, AppError> {
        let model = SchedulingResultEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 确认排程结果
    pub async fn confirm_schedule_result(
        &self,
        id: i32,
        _user_id: i32,
    ) -> Result<crate::models::scheduling_result::Model, AppError> {
        let model = SchedulingResultEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("排程结果不存在".to_string()))?;

        if model.status != "DRAFT" {
            return Err(AppError::BusinessError("只有草稿状态的排程结果可以确认".to_string()));
        }

        let mut active_model: SchedulingActiveModel = model.into();
        active_model.status = Set("CONFIRMED".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }
}
