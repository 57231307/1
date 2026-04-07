#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_code: String,
    pub item_name: String,
    pub parent_id: Option<i32>,
    pub item_type: String,
    pub level: i32,
    pub status: String,
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
