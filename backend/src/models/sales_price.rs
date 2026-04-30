#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_prices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: Decimal,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Decimal,
    pub price_type: String,
    pub price_level: Option<String>,
    #[sea_orm(column_type = "Date")]
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
