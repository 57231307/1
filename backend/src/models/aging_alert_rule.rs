#![allow(dead_code)]
//! 账龄预警规则 Entity
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "aging_alert_rules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 规则名称
    pub rule_name: String,
    /// 规则编码
    pub rule_code: String,
    /// 账龄区间（current/30_days/60_days/90_days/180_days/1_year/over_1_year）
    pub aging_bucket: String,
    /// 阈值天数
    pub threshold_days: i32,
    /// 阈值金额
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub threshold_amount: Option<Decimal>,
    /// 预警级别（info/warning/critical）
    pub alert_level: String,
    /// 通知方式（system/email/sms）
    pub notify_method: String,
    /// 通知角色列表
    pub notify_roles: Option<Vec<String>>,
    /// 是否启用
    pub is_active: bool,
    /// 备注
    pub remarks: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
