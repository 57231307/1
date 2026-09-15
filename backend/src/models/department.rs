use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 部门实体（组织树节点，支持父子层级和负责人关联）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "departments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub manager_id: Option<i32>,
    /// 负责人姓名（契约对齐：前端「负责人」列读取 manager_name；
    /// 非数据库列，由 DepartmentService::list/get/create 查 users 表填充）
    #[sea_orm(ignore)]
    pub manager_name: Option<String>,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::department::Entity",
        from = "Column::ParentId",
        to = "super::department::Column::Id"
    )]
    Parent,
    #[sea_orm(has_many = "super::department::Entity")]
    Children,
    #[sea_orm(has_many = "super::user::Entity")]
    Users,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
