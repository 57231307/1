#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 季节性调价规则实体（P0-5）
///
/// 按季节自动调价（春夏 / 秋冬 / 节日）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "seasonal_price_rules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub rule_name: String,
    pub season: String,
    pub product_category_id: Option<i64>,
    pub adjustment_type: String,
    pub adjustment_value: Decimal,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub is_active: bool,
    pub description: Option<String>,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_category::Entity",
        from = "Column::ProductCategoryId",
        to = "super::product_category::Column::Id"
    )]
    ProductCategory,
}

impl Related<super::product_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductCategory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
