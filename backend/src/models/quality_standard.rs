#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_standards")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub standard_name: String,
    pub standard_code: String,
    pub standard_type: String,
    pub product_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub version: String,
    pub previous_version_id: Option<i32>,
    pub content: String,
    pub technical_requirements: Option<String>,
    pub testing_methods: Option<String>,
    pub acceptance_criteria: Option<String>,
    #[sea_orm(column_type = "Date")]
    pub effective_date: NaiveDate,
    pub expiry_date: Option<NaiveDate>,
    pub status: String,
    pub approved_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
