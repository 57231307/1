#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 工艺优化历史表实体（P2-4）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_process_optimizations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub request_id: String,
    pub color_no: String,
    pub color_name: Option<String>,
    pub fabric_type: String,
    pub dye_type: Option<String>,
    pub recommended_temperature: Decimal,
    pub recommended_time_minutes: i32,
    pub recommended_ph_value: Decimal,
    pub recommended_liquor_ratio: Decimal,
    pub similar_cases: i32,
    pub confidence: Decimal,
    pub source: String,
    pub reason: Option<String>,
    pub candidates_json: Option<Json>,
    pub is_applied: bool,
    pub applied_at: Option<DateTime<Utc>>,
    pub applied_by: Option<i64>,
    pub feedback_score: Option<i16>,
    pub feedback_remark: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::AppliedBy",
        to = "super::user::Column::Id"
    )]
    AppliedByUser,
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
