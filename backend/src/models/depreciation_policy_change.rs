#![allow(dead_code)]
//! 折旧政策变更 Entity
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "depreciation_policy_changes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub asset_id: i32,
    pub change_date: NaiveDate,
    pub old_method: String,
    pub new_method: String,
    pub old_useful_life: Option<i32>,
    pub new_useful_life: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub old_salvage_rate: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub new_salvage_rate: Option<Decimal>,
    pub reason: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub status: String,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
