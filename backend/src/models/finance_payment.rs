#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "finance_payments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub payment_no: String,
    pub invoice_id: Option<i32>,
    pub payment_date: DateTime<Utc>,
    pub amount: Decimal,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
