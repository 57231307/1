#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_order_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub discount_percent: Decimal,
    pub tax_percent: Decimal,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub shipped_quantity: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sea_orm(column_separator = '_')]
    pub color_no: String,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<Decimal>,
    pub color_extra_cost: Decimal,
    pub grade_price_diff: Decimal,
    pub final_price: Option<Decimal>,
    pub shipped_quantity_meters: Decimal,
    pub shipped_quantity_kg: Decimal,
    pub paper_tube_weight: Option<Decimal>,
    pub is_net_weight: Option<bool>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::OrderId",
        to = "super::sales_order::Column::Id"
    )]
    Order,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub struct SalesOrderItemToCustomer;

impl Linked for SalesOrderItemToCustomer {
    type FromEntity = Entity;
    type ToEntity = super::customer::Entity;

    fn link(&self) -> Vec<RelationDef> {
        vec![
            Relation::Order.def(),
            super::sales_order::Relation::Customer.def(),
        ]
    }
}
