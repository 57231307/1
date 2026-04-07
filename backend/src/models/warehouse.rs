#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "warehouses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub status: String,
    pub warehouse_type: Option<String>,
    pub temperature_control: Option<bool>,
    pub humidity_control: Option<bool>,
    pub fire_protection_level: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::inventory_stock::Entity")]
    InventoryStock,
}

impl Related<super::inventory_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InventoryStock.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
