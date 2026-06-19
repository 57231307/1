//! 质量异常 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 上报质量异常请求
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct ReportQualityIssueDto {
    pub custom_order_id: i64,
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
    pub operator_id: i64,
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
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
