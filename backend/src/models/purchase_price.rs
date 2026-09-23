#![allow(dead_code)]
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_prices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    #[sea_orm(column_type = "Date")]
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联产品（product_id -> products.id）
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,

    /// 关联供应商（supplier_id -> suppliers.id）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,
}

impl ActiveModelBehavior for ActiveModel {}
