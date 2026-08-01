#![allow(dead_code)]
//! 女职工三期保护记录模型（V15 P2 B08-25）
//!
//! 依据：《女职工劳动保护特别规定》《劳动法》第 58-63 条
//! 业务：孕期/产期/哺乳期保护记录

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "female_worker_protection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub worker_id: i32,
    pub protection_type: String,
    pub expected_start_date: Option<NaiveDate>,
    pub expected_end_date: Option<NaiveDate>,
    pub actual_start_date: Option<NaiveDate>,
    pub actual_end_date: Option<NaiveDate>,
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
