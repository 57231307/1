#![allow(dead_code)]
//! 资产分类 Entity
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "asset_categories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub category_code: String,
    pub category_name: String,
    pub parent_id: Option<i32>,
    /// 默认使用年限（月）
    pub default_useful_life: Option<i32>,
    /// 默认折旧方法（直线法/双倍余额递减法/年数总和法）
    pub default_depreciation_method: Option<String>,
    /// 默认残值率
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub default_salvage_rate: Option<Decimal>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
