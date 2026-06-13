#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

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
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
