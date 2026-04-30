#![allow(dead_code, unused_imports, unused_variables)]
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "unqualified_products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_name = "unqualified_no")]
    pub unqualified_no: String,
    #[sea_orm(column_name = "inspection_id")]
    pub inspection_id: Option<i32>,
    #[sea_orm(column_name = "product_id")]
    pub product_id: i32,
    #[sea_orm(column_name = "batch_no")]
    pub batch_no: Option<String>,
    #[sea_orm(column_name = "unqualified_qty")]
    pub unqualified_qty: Decimal,
    #[sea_orm(column_name = "unqualified_reason")]
    pub unqualified_reason: String,
    #[sea_orm(column_name = "handling_method")]
    pub handling_method: String,
    #[sea_orm(column_name = "handling_status")]
    pub handling_status: String,
    #[sea_orm(column_name = "handling_by")]
    pub handling_by: Option<i32>,
    #[sea_orm(column_name = "handling_at")]
    pub handling_at: Option<chrono::NaiveDateTime>,
    pub remark: Option<String>,
    #[sea_orm(column_name = "created_at")]
    pub created_at: chrono::NaiveDateTime,
    #[sea_orm(column_name = "updated_at")]
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use rust_decimal::Decimal;
