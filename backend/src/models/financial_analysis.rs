#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "financial_indicators")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub indicator_name: String,
    pub indicator_code: String,
    pub indicator_type: String,
    pub formula: Option<String>,
    pub unit: Option<String>,
    pub status: String,
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
