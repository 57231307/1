#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单明细实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotation_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub quotation_id: i64,

    pub product_id: i64,
    pub color_id: Option<i64>,
    pub color_code: Option<String>,
    pub pantone_code: Option<String>,
    pub cncs_code: Option<String>,

    pub specification: Option<String>,
    pub unit: String,

    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub unit_price_with_tax: Decimal,
    pub amount: Decimal,
    pub amount_with_tax: Decimal,

    pub tier_pricing: Option<Json>,
    pub discount_rate: Option<Decimal>,
    pub discount_amount: Option<Decimal>,

    pub notes: Option<String>,
    pub sequence: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_quotation::Entity",
        from = "Column::QuotationId",
        to = "super::sales_quotation::Column::Id"
    )]
    Quotation,
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

impl Related<super::sales_quotation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotation.def()
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
