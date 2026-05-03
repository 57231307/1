#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Serialize, Deserialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_transfer_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: Decimal,
    pub shipped_quantity: Decimal,
    pub received_quantity: Decimal,
    pub unit_cost: Option<Decimal>,
    pub notes: Option<String>,
    pub is_deleted: bool,
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
