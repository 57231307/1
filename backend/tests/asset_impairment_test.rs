#![allow(dead_code)]
//! 资产减值测试 Entity
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "asset_impairment_tests")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub asset_id: i32,
    pub test_date: NaiveDate,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub carrying_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub recoverable_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub impairment_loss: Decimal,
    pub test_basis: String,
    pub notes: Option<String>,
    pub status: String,
    pub reviewed_by: Option<i32>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
