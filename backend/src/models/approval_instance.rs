use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "approval_instances")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub template_id: i32,
    pub resource_id: i32,
    pub status: String,
    pub current_step_order: i32,
    pub applicant_id: i32,
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
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApplicantId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
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

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
