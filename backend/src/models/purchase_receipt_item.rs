#![allow(dead_code, unused_imports, unused_variables)]
//! 采购入库明细 Model
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_receipt_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub receipt_id: i32,
    pub order_item_id: Option<i32>,
    pub line_no: i32,
    pub product_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((8, 2)))")]
    pub gram_weight: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((8, 2)))")]
    pub width: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity_alt: Option<Decimal>,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub amount: Option<Decimal>,
    pub location_code: Option<String>,
    pub package_no: Option<String>,
    pub production_date: Option<NaiveDate>,
    pub shelf_life: Option<i32>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub internal_dye_lot_id: Option<i32>,
    pub internal_dye_lot_no: Option<String>,
    pub internal_piece_ids: Option<Vec<i32>>,
    pub internal_piece_nos: Option<Vec<String>>,
    pub supplier_dye_lot_no: Option<String>,
    pub supplier_piece_nos: Option<Vec<String>>,
    pub batch_conversion_log_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_receipt::Entity",
        from = "Column::ReceiptId",
        to = "super::purchase_receipt::Column::Id"
    )]
    Receipt,
    #[sea_orm(
        belongs_to = "super::purchase_order_item::Entity",
        from = "Column::OrderItemId",
        to = "super::purchase_order_item::Column::Id"
    )]
    OrderItem,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::purchase_receipt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Receipt.def()
    }
}

impl Related<super::purchase_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItem.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
