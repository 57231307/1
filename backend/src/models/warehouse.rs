#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "warehouses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub warehouse_code: String,
    pub name: String,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: Option<String>,
    pub postal_code: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub manager_id: Option<i32>,
    pub is_active: bool,
    pub notes: Option<String>,
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
