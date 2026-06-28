#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 质量预测历史表实体（P2-4）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_quality_predictions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub request_id: String,
    pub product_id: Option<i64>,
    pub inspection_type: String,
    pub window_days: i32,
    pub total_inspections: i64,
    pub avg_qualification_rate: Decimal,
    pub trend: String,
    pub trend_rate: Decimal,
    pub risk_score: i16,
    pub risk_level: String,
    pub confidence: Decimal,
    pub top_issues_json: Option<Json>,
    pub recommendations_json: Option<Json>,
    pub period_breakdown_json: Option<Json>,
    pub source: String,
    pub is_acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<i64>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::AcknowledgedBy",
        to = "super::user::Column::Id"
    )]
    AcknowledgedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
