use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "approval_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub instance_id: i32,
    pub node_id: Option<i32>,
    pub approver_id: i32,
    pub action: String,
    pub comments: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::approval_instance::Entity",
        from = "Column::InstanceId",
        to = "super::approval_instance::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ApprovalInstance,
    #[sea_orm(
        belongs_to = "super::approval_node::Entity",
        from = "Column::NodeId",
        to = "super::approval_node::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ApprovalNode,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApproverId",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    User,
}

impl Related<super::approval_instance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalInstance.def()
    }
}

impl Related<super::approval_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApprovalNode.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
