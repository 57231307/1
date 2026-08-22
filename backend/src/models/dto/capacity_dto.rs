//! 产能分析相关 DTO
//!
//! 从 capacity_service 迁移出的数据传输对象

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 可用产能查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCapacity {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub date_from: NaiveDate,
    pub date_to: NaiveDate,
    pub total_capacity: Decimal,
    pub used_capacity: Decimal,
    pub available_capacity: Decimal,
    pub load_rate: Decimal,
}

/// 产能负荷预警事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOverloadAlert {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub current_demand: Decimal,
    pub load_rate: Decimal,
    pub threshold: Decimal,
    pub alert_level: String,
    pub message: String,
}

/// 工作中心产能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkCenterCapacity {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub work_center_type: Option<String>,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub status: String,
    pub shifts: Vec<ShiftInfo>,
}

/// 班次信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftInfo {
    pub shift_name: String,
    pub start_time: String,
    pub end_time: String,
    pub capacity_ratio: Decimal,
}

/// 产能负荷分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityLoadItem {
    pub work_center_id: i32,
    pub work_center_code: String,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub capacity_unit: Option<String>,
    pub planned_quantity: Decimal,
    pub in_progress_quantity: Decimal,
    pub total_demand: Decimal,
    pub load_rate: Decimal,
    pub status: String,
    /// batch-18 P2-4：缺口量（total_demand - daily_capacity，仅超载时 > 0）
    #[serde(default)]
    pub gap_quantity: Decimal,
    /// batch-18 P2-4：扩产/外包建议
    #[serde(default)]
    pub suggestions: Vec<BottleneckSuggestion>,
}

/// batch-18 P2-4：扩产/外包建议类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    /// 扩产建议：增加班次/设备/工时
    Expansion,
    /// 外包建议：委外加工
    Outsourcing,
    /// 转移建议：将负荷转移到空闲工作中心
    Transfer,
}

/// batch-18 P2-4：瓶颈建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckSuggestion {
    pub suggestion_type: SuggestionType,
    pub description: String,
    pub suggested_quantity: Decimal,
    pub priority: String,
}

/// 产能概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityOverview {
    pub total_work_centers: i64,
    pub active_work_centers: i64,
    pub total_daily_capacity: Decimal,
    pub total_planned_demand: Decimal,
    pub overall_load_rate: Decimal,
    pub bottleneck_work_centers: Vec<CapacityLoadItem>,
    pub overloaded_count: i64,
    pub idle_count: i64,
}

/// 产能负荷查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct LoadAnalysisQuery {
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    // v11 批次 149 P2-A：接入 work_center_id filter（load_analysis 方法中使用）
    pub work_center_id: Option<i32>,
}

/// 产能预测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityForecast {
    pub work_center_id: i32,
    pub work_center_name: String,
    pub daily_capacity: Decimal,
    pub forecast_days: i32,
    pub total_capacity: Decimal,
    pub predicted_demand: Decimal,
    pub predicted_available: Decimal,
    pub predicted_load_rate: Decimal,
    pub confidence: f64,
}

/// 创建工作中心输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkCenterInput {
    pub code: Option<String>,
    pub name: String,
    pub work_center_type: Option<String>,
    pub daily_capacity: Option<Decimal>,
    pub capacity_unit: Option<String>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

/// 更新工作中心输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkCenterInput {
    pub code: Option<String>,
    pub name: Option<String>,
    pub work_center_type: Option<String>,
    pub daily_capacity: Option<Decimal>,
    pub capacity_unit: Option<String>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}
