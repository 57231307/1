use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// B-P1-7 修复（批次 384 v13 复审）：事件死信队列模型
// 事件处理失败超过最大重试次数后的持久化记录，供人工排查或补偿处理。

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "event_dead_letters")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 事件类型（BusinessEvent 变体名）
    pub event_type: String,
    /// 事件 payload（JSON 序列化）
    pub event_payload: serde_json::Value,
    /// 首次失败原因摘要
    pub failure_reason: String,
    /// 最后一次失败的完整错误信息
    pub last_error: Option<String>,
    /// 已重试次数
    pub retry_count: i32,
    /// 最大重试次数（默认 5）
    pub max_retries: i32,
    /// 状态：PENDING（待重试）/ DEAD（已入死信，待人工处理）/ RESOLVED（已人工处理）
    pub status: String,
    /// 首次失败时间
    pub first_failed_at: chrono::DateTime<chrono::Utc>,
    /// 最后重试时间
    pub last_retry_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 已处理时间
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 处理人
    pub resolved_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 死信状态常量
pub mod status {
    /// 待重试
    pub const PENDING: &str = "PENDING";
    /// 已入死信，待人工处理
    pub const DEAD: &str = "DEAD";
    /// 已人工处理
    pub const RESOLVED: &str = "RESOLVED";
}
