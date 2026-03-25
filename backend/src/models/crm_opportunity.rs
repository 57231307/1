//! CRM 商机 Model
//!
//! CRM 商机模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CRM 商机 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_opportunities")]
pub struct Model {
    /// 商机 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 商机编号
    #[sea_orm(unique)]
    pub opportunity_no: String,

    /// 商机名称
    pub name: String,

    /// 客户 ID（外键）
    pub customer_id: Option<i32>,

    /// 线索 ID（外键）
    pub lead_id: Option<i32>,

    /// 商机金额
    pub amount: Decimal,

    /// 预计成交日期
    pub expected_close_date: Option<NaiveDate>,

    /// 商机阶段：PROSPECT=意向，NEGOTIATION=谈判中，CLOSED_WON=成交，CLOSED_LOST=失败
    pub stage: String,

    /// 商机来源
    pub source: Option<String>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// CRM 商机关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
