#![allow(dead_code)]
//! 预算版本 Entity
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub plan_id: i32,
    pub version_no: String,
    pub version_name: String,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_amount: Decimal,
    pub status: String,
    pub change_reason: Option<String>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
