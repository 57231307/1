//! 排程核心服务（scheduling_service）
//!
//! 包含排程的 DTO + struct + new() 入口。
//! 9 个 pub fn 已按 P9-2 拆分到 3 个子模块：
//! - `scheduling_auto`   排程自动调度（auto_schedule / detect_conflicts / save_schedule_result）
//! - `scheduling_manual` 排程手动调整（adjust_schedule）
//! - `scheduling_query`  排程查询与甘特图（get_gantt_data / list_scheduled_orders / get_schedule_history / get_schedule_result / confirm_schedule_result）
//!
//! 通过 `impl SchedulingService` 跨文件分布，所有方法调用方代码路径不变。

use chrono::NaiveDate;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 排程工单（已排程的生产订单）
#[derive(Debug, Clone)]
pub struct ScheduledOrder {
    pub id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub work_center_id: i32,
    pub planned_start: NaiveDate,
    pub planned_end: NaiveDate,
    pub actual_start: Option<NaiveDate>,
    pub actual_end: Option<NaiveDate>,
    pub status: String,
    pub priority: i32,
}

/// 工作中心产能
#[derive(Debug, Clone)]
pub struct WorkCenterCapacity {
    pub work_center_id: i32,
    pub work_center_name: String,
    pub daily_capacity: i32,
    pub utilization: f64,
}

/// 时间槽
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub work_center_id: i32,
}

/// 排程冲突
#[derive(Debug, Clone)]
pub struct ScheduleConflict {
    pub order_id: i32,
    pub work_center_id: i32,
    pub conflict_type: String,
    pub description: String,
}

/// 甘特图项
#[derive(Debug, Clone)]
pub struct GanttItemDto {
    pub order_id: i32,
    pub order_no: String,
    pub work_center: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub progress: i32,
}

/// 甘特图数据
#[derive(Debug, Clone)]
pub struct GanttData {
    pub items: Vec<GanttItemDto>,
    pub work_centers: Vec<WorkCenterInfo>,
}

/// 工作中心信息
#[derive(Debug, Clone)]
pub struct WorkCenterInfo {
    pub id: i32,
    pub name: String,
}

/// 日期范围
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 自动排程请求
#[derive(Debug, Clone)]
pub struct AutoScheduleRequest {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub work_center_ids: Option<Vec<i32>>,
    pub algo: String,
}

/// 自动排程结果
#[derive(Debug, Clone)]
pub struct AutoScheduleResult {
    pub scheduled_count: i32,
    pub conflicts: Vec<ScheduleConflict>,
    pub gantt_data: GanttData,
}

/// 排程详情
#[derive(Debug, Clone)]
pub struct ScheduleDetail {
    pub order_id: i32,
    pub order_no: String,
    pub work_center_id: i32,
    pub planned_start: NaiveDate,
    pub planned_end: NaiveDate,
}

/// 调整排程请求
#[derive(Debug, Clone)]
pub struct AdjustScheduleRequest {
    pub order_id: i32,
    pub new_start: NaiveDate,
    pub new_end: NaiveDate,
    pub work_center_id: Option<i32>,
    pub adjust_type: String,
}

/// 排程查询
#[derive(Debug, Clone)]
pub struct ScheduledOrderQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub work_center_id: Option<i32>,
    pub status: Option<String>,
    pub page: u64,
    pub page_size: u64,
}

/// 排程服务
pub struct SchedulingService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl SchedulingService {
    /// 创建排程服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
