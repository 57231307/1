//! 排程相关数据传输对象（DTO）
//!
//! 从 `services/scheduling_service.rs` 迁移的纯数据结构，
//! 供 service 子模块与 handler 共享。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 排程工单（已排程的生产订单）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledOrder {
    pub id: i32,
    pub order_id: i32,
    pub order_no: String,
    pub product_id: i32,
    pub quantity: Decimal,
    pub work_center_id: i32,
    pub work_center_name: String,
    pub planned_start: NaiveDate,
    pub planned_end: NaiveDate,
    pub start_time: NaiveDate,
    pub end_time: NaiveDate,
    pub actual_start: Option<NaiveDate>,
    pub actual_end: Option<NaiveDate>,
    pub status: String,
    pub priority: i32,
    pub dependencies: Vec<i32>,
}

/// 时间槽
#[allow(dead_code, reason = "预留")]
#[derive(Debug, Clone)]
pub struct TimeSlot {
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub work_center_id: i32,
}

/// 排程冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConflict {
    pub order_id: i32,
    pub order_no: Option<String>,
    pub work_center_id: i32,
    pub work_center_name: Option<String>,
    pub conflict_type: String,
    pub description: String,
    pub severity: Option<String>,
    pub conflicting_order_id: Option<i32>,
    pub conflicting_order_no: Option<String>,
}

/// 甘特图项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttItemDto {
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

/// 甘特图数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GanttData {
    pub items: Vec<GanttItemDto>,
    pub work_centers: Vec<WorkCenterInfo>,
    pub date_range: Option<DateRange>,
    pub schedule_details: Option<Vec<ScheduleDetail>>,
}

/// 工作中心信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenterInfo {
    pub id: i32,
    pub name: String,
    pub code: Option<String>,
    pub status: Option<String>,
}

/// 日期范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// 自动排程请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScheduleRequest {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub work_center_ids: Option<Vec<i32>>,
    pub algo: String,
}

/// 自动排程结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoScheduleResult {
    pub scheduled_count: i32,
    pub conflicts: Vec<ScheduleConflict>,
    pub gantt_data: GanttData,
    pub total_orders: Option<i32>,
    pub scheduled_orders: Option<Vec<ScheduleDetail>>,
    pub unscheduled_orders: Option<Vec<ScheduleDetail>>,
    pub schedule_details: Option<Vec<ScheduleDetail>>,
    pub id: Option<i32>,
    pub batch_no: Option<String>,
}

/// 排程详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleDetail {
    pub order_id: i32,
    pub order_no: Option<String>,
    pub work_center_id: i32,
    pub work_center_name: Option<String>,
    pub planned_start: NaiveDate,
    pub planned_end: NaiveDate,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<String>,
}

/// 调整排程请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustScheduleRequest {
    pub order_id: Option<i32>,
    pub new_start: Option<NaiveDate>,
    pub new_end: Option<NaiveDate>,
    pub work_center_id: Option<i32>,
    pub adjust_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub priority: Option<i32>,
}

/// 排程查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledOrderQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub work_center_id: Option<i32>,
    pub status: Option<String>,
    pub page: u64,
    pub page_size: u64,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
}
