use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "supplier_evaluation_indicators")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub indicator_name: String,
    pub indicator_code: String,
    pub category: String,
    pub weight: Decimal,
    pub max_score: i32,
    pub evaluation_method: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
