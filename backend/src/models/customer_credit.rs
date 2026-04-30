#![allow(dead_code, unused_imports, unused_variables)]
//! 客户信用 Entity
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_credit_ratings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Decimal,
    pub used_credit: Decimal,
    pub available_credit: Decimal,
    pub credit_days: Option<i32>,
    pub last_assessment_date: Option<NaiveDate>,
    pub next_assessment_date: Option<NaiveDate>,
    pub status: String,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
