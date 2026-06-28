//! 定制订单响应 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 定制订单列表响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomOrderListItem {
    pub id: i64,
    pub order_no: String,
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub spec: String,
    pub quantity: Decimal,
    pub unit: String,
    pub status: String,
    pub expected_delivery_date: Option<NaiveDate>,
    pub actual_delivery_date: Option<NaiveDate>,
    pub total_amount: Option<Decimal>,
    pub currency: String,
    pub sales_order_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// 定制订单详情响应（包含节点/异常/售后）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomOrderDetail {
    pub id: i64,
    pub order_no: String,
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub spec: String,
    pub quantity: Decimal,
    pub unit: String,
    pub custom_requirements: serde_json::Value,
    pub yarn_spec: Option<String>,
    pub dye_method: Option<String>,
    pub finishing_method: Option<String>,
    pub status: String,
    pub expected_delivery_date: Option<NaiveDate>,
    pub actual_delivery_date: Option<NaiveDate>,
    pub sales_order_id: Option<i64>,
    pub total_amount: Option<Decimal>,
    pub currency: String,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    /// 工艺节点列表
    pub process_nodes: Vec<ProcessNodeInfo>,

    /// 质量异常列表
    pub quality_issues: Vec<QualityIssueInfo>,

    /// 售后工单列表
    pub after_sales: Vec<AfterSalesInfo>,
}

/// 工艺节点信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessNodeInfo {
    pub id: i64,
    pub node_type: String,
    pub node_name: String,
    pub sequence: i32,
    pub status: String,
    pub planned_start_date: Option<DateTime<Utc>>,
    pub planned_end_date: Option<DateTime<Utc>>,
    pub actual_start_date: Option<DateTime<Utc>>,
    pub actual_end_date: Option<DateTime<Utc>>,
    pub operator_id: Option<i64>,
    pub notes: Option<String>,
}

/// 质量异常信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityIssueInfo {
    pub id: i64,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub discovered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub status: String,
}

/// 售后工单信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AfterSalesInfo {
    pub id: i64,
    pub issue_type: String,
    pub description: String,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub refund_amount: Option<Decimal>,
}

/// 工艺流程时间线（节点 + 日志合并）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessTimeline {
    pub order_id: i64,
    pub order_no: String,
    pub current_status: String,
    pub nodes: Vec<ProcessNodeWithLogs>,
}

/// 工艺节点（含日志）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessNodeWithLogs {
    pub id: i64,
    pub node_type: String,
    pub node_name: String,
    pub sequence: i32,
    pub status: String,
    pub planned_start_date: Option<DateTime<Utc>>,
    pub planned_end_date: Option<DateTime<Utc>>,
    pub actual_start_date: Option<DateTime<Utc>>,
    pub actual_end_date: Option<DateTime<Utc>>,
    pub logs: Vec<ProcessLogInfo>,
}

/// 工艺日志
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessLogInfo {
    pub id: i64,
    pub action: String,
    pub operator_id: Option<i64>,
    pub before_status: Option<String>,
    pub after_status: Option<String>,
    pub log_time: DateTime<Utc>,
    pub log_content: Option<String>,
    pub attachments: Vec<String>,
}

/// 分页响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}
