#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "inventory_transfer_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub stock_id: Option<i32>,
    pub barcode: Option<String>,
    pub roll_length: Option<String>,
    pub dye_lot_no: Option<String>,
    pub quantity: Decimal,
    pub shipped_quantity: Decimal,
    pub received_quantity: Decimal,
    pub unit_cost: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::inventory_transfer::Entity",
        from = "Column::TransferId",
        to = "super::inventory_transfer::Column::Id"
    )]
    Transfer,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::inventory_transfer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Transfer.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
