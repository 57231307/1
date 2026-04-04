use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "supplier_product_colors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub supplier_product_id: i32,
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub extra_cost: Decimal,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remarks: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::supplier_product::Entity",
        from = "Column::SupplierProductId",
        to = "super::supplier_product::Column::Id"
    )]
    SupplierProduct,
    #[sea_orm(has_many = "super::product_supplier_mapping::Entity")]
    ProductSupplierMappings,
}

impl Related<super::supplier_product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SupplierProduct.def()
    }
}

impl Related<super::product_supplier_mapping::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductSupplierMappings.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
