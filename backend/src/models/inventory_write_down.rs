#![allow(dead_code)]
//! 存货跌价准备模型（V15 P2 B08-P2-6）
//!
//! 依据：《企业会计准则第 1 号——存货》
//! 业务：季节性降价跌价准备 / 呆滞面料跌价准备 / 过期染料助剂跌价准备

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_write_downs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub write_down_type: String,
    pub inventory_stock_id: Option<i32>,
    pub product_id: Option<i32>,
    pub chemical_id: Option<i32>,
    pub original_cost: Decimal,
    pub net_realizable_value: Decimal,
    pub write_down_amount: Decimal,
    pub provision_date: chrono::NaiveDate,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
