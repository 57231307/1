use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub table_name: String,
    pub record_id: i32,
    pub action: String, // CREATE, UPDATE, DELETE
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub old_data: Option<serde_json::Value>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub new_data: Option<serde_json::Value>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub changed_fields: Option<serde_json::Value>,
    pub user_id: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
