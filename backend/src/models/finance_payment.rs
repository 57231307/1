#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "finance_payments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub payment_no: String,
    pub invoice_id: Option<i32>,
    pub payment_date: DateTime<Utc>,
    pub amount: Decimal,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
