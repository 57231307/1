#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "product_supplier_mappings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub is_primary: bool,
    pub priority: i32,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub supplier_price: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub min_order_quantity: Option<Decimal>,
    pub lead_time: Option<i32>,
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
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::product_color::Entity",
        from = "Column::ProductColorId",
        to = "super::product_color::Column::Id"
    )]
    ProductColor,
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,
    #[sea_orm(
        belongs_to = "super::supplier_product::Entity",
        from = "Column::SupplierProductId",
        to = "super::supplier_product::Column::Id"
    )]
    SupplierProduct,
    #[sea_orm(
        belongs_to = "super::supplier_product_color::Entity",
        from = "Column::SupplierProductColorId",
        to = "super::supplier_product_color::Column::Id"
    )]
    SupplierProductColor,
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductColor.def()
    }
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::supplier_product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierProduct.def()
    }
}

impl Related<super::supplier_product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierProductColor.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
