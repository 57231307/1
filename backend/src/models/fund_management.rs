#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fund_accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub account_name: String,
    pub account_no: String,
    pub account_type: String,
    pub bank_name: Option<String>,
    pub currency: String,
    pub balance: Decimal,
    pub available_balance: Decimal,
    pub frozen_balance: Decimal,
    pub status: String,
    #[sea_orm(column_type = "Date")]
    pub opened_date: Option<NaiveDate>,
    pub remark: Option<String>,
    #[sea_orm(column_type = "Timestamp")]
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
