#![allow(dead_code)]
//! 定制订单响应 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::FromQueryResult;
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
    /// 批次 88 PH-1 占位符实现：订单备注
    pub notes: Option<String>,
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
    /// 批次 88 PH-1 占位符实现：订单备注
    pub notes: Option<String>,

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
    pub operator_id: Option<i32>,
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
///
/// 同时作为售后读侧查询的 `into_model` 视图对象：字段名 = after_sales 实体列名 +
/// JOIN 别名 `customer_name`（列名自动映射，缺列会 TryGetError 而非静默，
/// 读侧必须带 LEFT JOIN + column_as 提供全部字段来源）
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct AfterSalesInfo {
    pub id: i64,
    pub issue_type: String,
    /// 实体 `after_sales.customer_id` 为 NOT NULL i32，出参如实非可选
    pub customer_id: i32,
    /// 客户名：读侧 `LEFT JOIN customers` + `column_as(customers.customer_name)` 富化。
    /// `customer_id` 本身 NOT NULL 且为外键，但关联行缺失时 LEFT JOIN 产生 NULL，
    /// 故本字段为 Option（忠实回显 JOIN 结果，禁止以 id 或拼装名填充）
    pub customer_name: Option<String>,
    pub description: String,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub refund_amount: Option<Decimal>,
    /// V15 P0-B12：关联质量异常 ID（trigger_quality_investigation 触发后回填）
    pub quality_issue_id: Option<i64>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因分类（quality/logistics/customer_preference/other）
    pub reason_category: Option<String>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因明细（结构化子类）
    pub reason_detail: Option<String>,
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
    pub operator_id: Option<i32>,
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
