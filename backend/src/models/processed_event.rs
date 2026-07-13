//! 事件幂等去重 Entity
//!
//! 对应数据库表：processed_events
//!
//! B-P1-8 修复（批次 365 v13 复审）：用于事件消费者幂等校验，防止重复消费导致
//! 重复生成凭证、重复更新状态等副作用。主键 (consumer_id, event_key) 保证同一消费者
//! 对同一事件键仅处理一次。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "processed_events")]
pub struct Model {
    /// 消费者标识（如 "inventory_finance_bridge"、"event_bus_main"）
    #[sea_orm(primary_key, auto_increment = false)]
    pub consumer_id: String,
    /// 幂等键（如 "inventory_txn:123"、"sales_shipped:456"）
    #[sea_orm(primary_key, auto_increment = false)]
    pub event_key: String,
    /// BusinessEvent 变体名，便于排查
    pub event_type: String,
    /// 处理时间
    pub processed_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
