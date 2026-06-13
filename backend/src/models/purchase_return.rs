#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_return")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub return_no: String,
    pub receipt_id: Option<i32>,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub return_date: NaiveDate,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub reason_type: Option<String>,
    pub reason_detail: Option<String>,
    pub return_status: Option<String>,
    pub total_quantity: Option<Decimal>,
    pub total_quantity_alt: Option<Decimal>,
    pub total_amount: Option<Decimal>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_by: Option<i32>,
    pub updated_at: DateTime<Utc>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
