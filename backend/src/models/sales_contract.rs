#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 销售合同 Entity
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_contracts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub contract_no: String,
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub signed_date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub delivery_date: Option<NaiveDate>,
    pub delivery_location: Option<String>,
    pub status: String,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
