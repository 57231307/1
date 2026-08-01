#![allow(dead_code)]
//! 安全告警日志模型（V15 P2 B11-P2-9）
//!
//! 业务：安全告警记录，保留 7 年
//! 依据：《网络安全法》《数据安全法》

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "security_alert_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub source_ip: Option<String>,
    pub user_id: Option<i32>,
    pub details: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
