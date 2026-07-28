#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 模型评估指标表实体（V15 P1 3.4）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_model_evaluations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub model_version_id: i32,
    pub evaluation_date: DateTime<Utc>,
    pub accuracy: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1_score: Option<Decimal>,
    pub sample_count: i32,
    pub evaluation_report: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ai_model_version::Entity",
        from = "Column::ModelVersionId",
        to = "super::ai_model_version::Column::Id"
    )]
    ModelVersion,
}

impl Related<super::ai_model_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ModelVersion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
