#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_inspection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub inspection_no: String,
    pub receipt_id: Option<i32>,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub inspection_date: NaiveDate,
    pub inspector_id: Option<i32>,
    pub inspection_type: Option<String>,
    pub sample_size: Option<Decimal>,
    pub defect_count: Option<i32>,
    pub pass_quantity: Option<Decimal>,
    pub reject_quantity: Option<Decimal>,
    pub inspection_status: Option<String>,
    pub inspection_result: Option<String>,
    pub quality_score: Option<Decimal>,
    pub defect_description: Option<String>,
    pub attachment_urls: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completed_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
