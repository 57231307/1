#![allow(dead_code)]
//! 质量异常 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 上报质量异常请求
///
/// 契约修复（与任务 #148 CreateAfterSalesDto 先例同构）：`custom_order_id`（异常
/// 归属）由路由 `POST /custom-orders/{orderId}/issues` 的 path 参数权威提供，不再
/// 属于请求体字段。此前该字段为非 Option 必填且无 serde default，而 handler 又在
/// 反序列化成功后用 path 值覆盖 body 值——body 携带本无任何语义，前端不发该键却
/// 必在反序列化层报 missing field（上报 100% 失败）。若客户端仍在 body 发送
/// `custom_order_id`（含伪造他人订单 ID），serde 默认忽略未知字段，归属一律以
/// path 为准（越权防护不变，对齐 handlers/color_card/items.rs::create_color_item
/// 的 `service.create(id, dto)` 范式）。
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct ReportQualityIssueDto {
    pub process_node_id: Option<i64>,

    /// 异常类型：color_diff(色差) / color_fastness(色牢度) / spec(规格不符) / damage(破损) / other
    #[validate(length(min = 1, max = 50))]
    pub issue_type: String,

    /// 严重度：low / medium / high / critical
    #[validate(length(min = 1, max = 20))]
    pub severity: String,

    /// 描述
    #[validate(length(min = 1))]
    pub description: String,

    /// 色差 ΔE 值（GB/T 26377-2022 颜色标准，可选）
    pub color_delta_e: Option<Decimal>,

    /// 色牢度等级（ISO 105 标准，可选 1-5）
    pub color_fastness_grade: Option<i32>,
}

/// 解决质量异常请求
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResolveQualityIssueDto {
    pub resolution: String,
}

/// 质量异常详情
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QualityIssueDetail {
    pub id: i64,
    pub custom_order_id: i64,
    pub process_node_id: Option<i64>,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub discovered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
