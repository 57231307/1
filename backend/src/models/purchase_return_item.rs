use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_return_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub product_id: i32,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity_alt: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price_foreign: Decimal,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub discount_percent: Decimal,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub tax_percent: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub subtotal: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub tax_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub discount_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_return::Entity",
        from = "Column::ReturnId",
        to = "super::purchase_return::Column::Id"
    )]
    Return,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::purchase_return::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Return.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
