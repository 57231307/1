#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_statistics")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub statistic_type: String,
    pub period: String,
    pub dimension_type: String,
    pub dimension_id: Option<i32>,
    pub dimension_name: Option<String>,
    pub order_count: i32,
    pub total_amount: Decimal,
    pub total_qty: Decimal,
    pub total_cost: Decimal,
    pub gross_profit: Decimal,
    pub gross_profit_rate: Decimal,
    pub avg_order_value: Decimal,
    #[sea_orm(column_type = "Timestamp")]
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
