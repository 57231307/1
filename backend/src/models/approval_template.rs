use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "approval_templates")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    #[sea_orm(unique)]
    pub resource_type: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::approval_node::Entity")]
    ApprovalNode,
    #[sea_orm(has_many = "super::approval_instance::Entity")]
    ApprovalInstance,
}

impl Related<super::approval_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalNode.def()
    }
}

impl Related<super::approval_instance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalInstance.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
