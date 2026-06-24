//! 定制订单更新 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-custom-order-design.md

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 添加工艺节点 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct CreateProcessNodeDto {
    /// 节点类型：yarn_purchasing / dyeing / finishing / delivery / after_sales
    #[validate(length(min = 1, max = 30))]
    pub node_type: String,

    /// 节点名称
    #[validate(length(min = 1, max = 100))]
    pub node_name: String,

    /// 顺序（1-5）
    #[validate(range(min = 1, max = 5))]
    pub sequence: i32,

    /// 计划开始时间
    pub planned_start_date: Option<chrono::DateTime<chrono::Utc>>,

    /// 计划结束时间
    pub planned_end_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// 更新工艺节点 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateProcessNodeDto {
    pub status: Option<String>,
    pub operator_id: Option<i64>,
    pub actual_start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub actual_end_date: Option<chrono::DateTime<chrono::Utc>>,
    pub notes: Option<String>,
}

/// 推进工艺节点 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdvanceNodeDto {
    pub action: String,
    pub operator_id: i64,
    pub notes: Option<String>,
    pub attachments: Option<Vec<String>>,
}

/// 添加工艺日志 DTO
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AddProcessLogDto {
    pub action: String,
    pub operator_id: i64,
    pub before_status: Option<String>,
    pub after_status: Option<String>,
    pub log_content: Option<String>,
    pub attachments: Option<Vec<String>>,
}
