#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "currencies")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub code: String,
    pub name: String,
    pub symbol: Option<String>,
    pub precision: Option<i32>,
    pub is_base: Option<bool>,
    pub is_active: Option<bool>,
    pub is_deleted: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
