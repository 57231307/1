#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 客户专属价格实体（P0-5）
///
/// 战略客户 / 大客户协议价（最高优先级）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_color_prices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub customer_id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub special_price: Decimal,
    pub discount_percent: Option<Decimal>,
    pub currency: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub notes: Option<String>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::product_color::Entity",
        from = "Column::ColorId",
        to = "super::product_color::Column::Id"
    )]
    Color,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Color.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
