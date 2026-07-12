#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "webhooks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub url: String,
    pub events: String,
    pub secret: Option<String>,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
    pub retry_count: i32,
    /// 最后一次发送的原始业务负载（批次 251 修复：retry 重投原始数据用）
    pub last_payload: Option<String>,
    /// 最后一次发送的事件类型（批次 251 修复：retry 重投原始事件用）
    pub last_event: Option<String>,
    /// Webhook 所有者用户 ID（批次 320 M-4 修复：IDOR 防护）
    /// - NULL：系统级 webhook（历史数据，所有认证用户可访问，向后兼容）
    /// - 非 NULL：用户私有 webhook，仅所有者可操作
    pub user_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
