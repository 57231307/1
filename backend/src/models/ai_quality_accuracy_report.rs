#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 质量预测准确率报告表实体（V15 P1 2.4 + 8.3 对账）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_quality_accuracy_reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 报告周期（"YYYY-MM"）
    pub report_period: String,
    pub total_predictions: i32,
    pub correct_predictions: i32,
    pub accuracy_rate: Option<Decimal>,
    pub precision_score: Option<Decimal>,
    pub recall_score: Option<Decimal>,
    pub f1_score: Option<Decimal>,
    pub mismatch_cases_json: Option<Json>,
    pub generated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
