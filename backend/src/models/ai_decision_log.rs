#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 决策审计日志专用表实体（V15 P1 10.1）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_decision_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// process_optimization / quality_prediction / sales_forecast / ...
    pub decision_type: String,
    pub model_version_id: Option<i32>,
    pub input_json: Option<Json>,
    pub output_json: Option<Json>,
    pub user_id: Option<i32>,
    pub ip_address: Option<String>,
    pub latency_ms: Option<i32>,
    pub confidence: Option<Decimal>,
    pub source: Option<String>,
    pub degraded: bool,
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
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::ai_model_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ModelVersion.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
