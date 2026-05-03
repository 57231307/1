#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "supplier_products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub supplier_id: i32,
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub unit: String,
    pub is_enabled: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<i32>,
    pub updated_by: Option<i32>,
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,
    #[sea_orm(has_many = "super::supplier_product_color::Entity")]
    SupplierProductColors,
    #[sea_orm(has_many = "super::product_supplier_mapping::Entity")]
    ProductSupplierMappings,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::supplier_product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierProductColors.def()
    }
}

impl Related<super::product_supplier_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductSupplierMappings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
