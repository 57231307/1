#![allow(dead_code)]

//! 汇率 Model
//!
//! 汇率历史记录

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 汇率 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "exchange_rates")]
pub struct Model {
    /// 汇率 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 源币种代码
    pub from_currency: String,

    /// 目标币种代码
    pub to_currency: String,

    /// 汇率
    pub rate: Decimal,

    /// 生效日期
    pub effective_date: NaiveDate,

    /// 数据来源
    pub source: Option<String>,

    /// 是否删除

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 汇率关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
