#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_reconciliations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub reconciliation_no: String,
    pub reconciliation_date: NaiveDate,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub closing_balance: Decimal,
    pub reconciliation_status: Option<String>,
    pub confirmed_by_customer: Option<bool>,
    pub dispute_reason: Option<String>,
    pub confirmed_by: Option<i32>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
