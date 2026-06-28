#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 主备隔离切换事件表 Model
//!
//! 记录每次主备切换、熔断、回切事件。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 主备隔离事件表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "failover_event")]
pub struct Model {
    /// 主键
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 功能名
    pub function_name: String,

    /// 事件类型
    pub event_type: String,

    /// 原状态
    pub from_state: Option<String>,

    /// 目标状态
    pub to_state: Option<String>,

    /// 事件原因
    pub reason: Option<String>,

    /// 调用延迟（毫秒）
    pub latency_ms: Option<i32>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// DTO：事件响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEventDto {
    pub id: i64,
    pub function_name: String,
    pub event_type: String,
    pub from_state: Option<String>,
    pub to_state: Option<String>,
    pub reason: Option<String>,
    pub latency_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl From<Model> for FailoverEventDto {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            function_name: m.function_name,
            event_type: m.event_type,
            from_state: m.from_state,
            to_state: m.to_state,
            reason: m.reason,
            latency_ms: m.latency_ms,
            created_at: m.created_at,
        }
    }
}
