#![allow(dead_code)]
//! 定时推送订阅 Model（16.2-D1）
//!
//! 对应数据库表：notification_subscriptions
//! 存储用户的定时推送订阅配置，支持按频率自动触发推送。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notification_subscriptions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 用户 ID
    pub user_id: i32,
    /// 订阅名称
    pub name: String,
    /// 业务类型（如 stock_alert / order_update 等）
    pub business_type: String,
    /// 推送渠道（internal / email / sms / webhook）
    pub channel: String,
    /// 是否启用
    pub is_enabled: bool,
    /// 下次执行时间
    pub next_run_at: Option<DateTime<Utc>>,
    /// 上次执行时间
    pub last_run_at: Option<DateTime<Utc>>,
    /// 上次执行状态
    pub last_run_status: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
