#![allow(dead_code)]

//! 成本分析 Entity
//!
//! 对应数据库表：cost_analyses

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cost_analyses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub analysis_no: String,
    pub analysis_date: NaiveDate,

    // 分析维度
    pub period: String,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub workshop: Option<String>,

    // 成本数据
    pub total_direct_material: Decimal,
    pub total_direct_labor: Decimal,
    pub total_overhead: Decimal,
    pub total_processing_fee: Decimal,
    pub total_dyeing_fee: Decimal,
    pub total_cost: Decimal,

    // 产量数据
    pub total_output_meters: Option<Decimal>,
    pub total_output_kg: Option<Decimal>,

    // 单位成本
    pub avg_unit_cost_meters: Option<Decimal>,
    pub avg_unit_cost_kg: Option<Decimal>,

    // 对比分析
    pub standard_cost: Option<Decimal>,
    pub variance: Option<Decimal>,
    pub variance_rate: Option<Decimal>,

    // 分析结论
    pub conclusion: Option<String>,

    pub created_by: i32,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
