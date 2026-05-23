#![allow(dead_code)]

//! 资金转账记录 Model

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 资金转账记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fund_transfers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_no: String,
    pub from_account_id: Option<i32>,
    pub to_account_id: Option<i32>,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub amount: Decimal,
    pub transfer_type: String,
    pub transfer_date: NaiveDate,
    pub purpose: Option<String>,
    pub status: Option<String>,
    pub applied_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTimeWithTimeZone>,
    pub executed_at: Option<DateTimeWithTimeZone>,
    pub remark: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::fund_account::Entity",
        from = "Column::FromAccountId",
        to = "super::fund_account::Column::Id"
    )]
    FromAccount,
    #[sea_orm(
        belongs_to = "super::fund_account::Entity",
        from = "Column::ToAccountId",
        to = "super::fund_account::Column::Id"
    )]
    ToAccount,
}

impl ActiveModelBehavior for ActiveModel {}
