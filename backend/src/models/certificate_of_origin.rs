#![allow(dead_code)]
//! 出口产地证模型
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "certificate_of_origin")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub certificate_no: String,
    pub inspection_id: Option<i32>,
    pub product_name: String,
    pub hs_code: String,
    pub origin_country: String,
    pub destination_country: String,
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub quantity: Decimal,
    pub unit: String,
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub invoice_amount: Option<Decimal>,
    pub certificate_type: String,
    pub issue_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
