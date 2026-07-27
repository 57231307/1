use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// AI 模型版本管理表实体（V15 P1 3.1 + 10.2 模型变更审计）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ai_model_versions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub model_name: String,
    pub version: String,
    pub algorithm: String,
    pub parameters_json: Option<Json>,
    pub training_date: Option<chrono::NaiveDate>,
    pub training_dataset_size: Option<i32>,
    pub accuracy_metrics_json: Option<Json>,
    /// draft / active / retired / archived
    pub status: String,
    pub changed_by: Option<i32>,
    pub change_reason: Option<String>,
    /// pending / approved / rejected
    pub approval_status: String,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ChangedBy",
        to = "super::user::Column::Id"
    )]
    ChangedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    ApprovedByUser,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChangedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
