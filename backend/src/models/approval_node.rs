use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "approval_nodes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub template_id: i32,
    pub step_order: i32,
    pub approver_role_id: Option<i32>,
    pub approver_user_id: Option<i32>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub condition_expr: Option<serde_json::Value>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::approval_template::Entity",
        from = "Column::TemplateId",
        to = "super::approval_template::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ApprovalTemplate,
    #[sea_orm(has_many = "super::approval_log::Entity")]
    ApprovalLog,
}

impl Related<super::approval_template::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalTemplate.def()
    }
}

impl Related<super::approval_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalLog.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
