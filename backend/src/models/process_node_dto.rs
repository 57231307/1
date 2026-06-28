//! 工艺节点 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 工艺节点完整信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessNodeDetail {
    pub id: i64,
    pub custom_order_id: i64,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 工艺节点列表查询参数
#[derive(Debug, Deserialize, Clone, Default)]
pub struct ProcessNodeQuery {
    pub custom_order_id: Option<i64>,
    pub status: Option<String>,
    pub node_type: Option<String>,
}

/// 工艺节点状态推进请求
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct AdvanceNodeRequest {
    /// 目标状态：pending / in_progress / completed / blocked
    #[validate(length(min = 1, max = 20))]
    pub target_status: String,

    /// 操作人 ID
    pub operator_id: i64,

    /// 备注
    pub notes: Option<String>,
}
