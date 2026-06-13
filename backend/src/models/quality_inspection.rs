#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_inspection_standards")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub standard_name: String,
    pub standard_code: String,
    pub product_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub inspection_type: String,
    pub inspection_items: Option<serde_json::Value>,
    pub sampling_method: Option<String>,
    pub sampling_rate: Option<rust_decimal::Decimal>,
    pub acceptance_criteria: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
