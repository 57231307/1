#![allow(dead_code)]

//! 报表订阅 Model
//!
//! 存储报表订阅配置，支持定时生成和发送报表

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 订阅频率
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum SubscriptionFrequency {
    /// 每天
    #[sea_orm(string_value = "DAILY")]
    Daily,
    /// 每周
    #[sea_orm(string_value = "WEEKLY")]
    Weekly,
    /// 每月
    #[sea_orm(string_value = "MONTHLY")]
    Monthly,
}

/// 订阅状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum SubscriptionStatus {
    /// 启用
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    /// 禁用
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
    /// 暂停
    #[sea_orm(string_value = "PAUSED")]
    Paused,
}

/// 报表订阅 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "report_subscriptions")]
pub struct Model {
    /// 订阅 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 租户 ID
    pub tenant_id: i32,

    /// 订阅名称
    pub name: String,

    /// 关联的报表模板 ID
    pub template_id: i32,

    /// 订阅频率
    pub frequency: String,

    /// 收件人邮箱列表（JSON数组格式）
    pub recipients: Json,

    /// 导出格式（pdf/excel/csv）
    pub export_format: String,

    /// 是否启用
    pub is_enabled: bool,

    /// 状态
    pub status: String,

    /// 下次执行时间
    pub next_run_at: Option<DateTime<Utc>>,

    /// 上次执行时间
    pub last_run_at: Option<DateTime<Utc>>,

    /// 上次执行状态（success/failed）
    pub last_run_status: Option<String>,

    /// 上次执行错误信息
    pub last_run_error: Option<String>,

    /// 执行次数
    pub run_count: i32,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 报表订阅关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// 计算下次执行时间
    pub fn calculate_next_run(&self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match self.frequency.as_str() {
            "DAILY" => Some(now + chrono::Duration::days(1)),
            "WEEKLY" => Some(now + chrono::Duration::weeks(1)),
            "MONTHLY" => Some(now + chrono::Duration::days(30)),
            _ => None,
        }
    }
}
