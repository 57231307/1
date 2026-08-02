#![allow(dead_code)]
//! 安全生产事故报告模型（V15 P2 B08-P2-9）
//!
//! 依据：《安全生产法》《生产安全事故报告和调查处理条例》
//! 业务：安全生产事故记录、报告、调查
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "safety_accident_reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub accident_no: String,
    pub accident_level: String,
    pub accident_date: NaiveDate,
    pub location: Option<String>,
    pub description: String,
    pub casualties: i32,
    pub direct_loss: Option<Decimal>,
    pub cause: Option<String>,
    pub measures: Option<String>,
    pub reporter_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
