#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色号价格实体（P0-5 扩展版）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "product_color_prices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub product_id: i64,
    pub color_id: i64,
    pub currency: String,
    pub base_price: Decimal,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub customer_level: Option<String>,
    pub min_quantity: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // P0-5 新增字段
    pub max_quantity: Option<Decimal>,
    pub customer_id: Option<i64>,
    pub season: Option<String>,
    pub is_active: bool,
    pub priority: i32,
    pub created_by: Option<i64>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approval_status: String,
    pub tenant_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
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
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(has_many = "super::color_price_history::Entity")]
    History,
    #[sea_orm(has_many = "super::color_price_tier::Entity")]
    Tiers,
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

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::color_price_history::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::History.def()
    }
}

impl Related<super::color_price_tier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tiers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
