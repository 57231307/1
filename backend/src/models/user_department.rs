use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 用户部门关联表（一人多部门，主部门 + 兼职）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_departments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 用户 ID（关联 users.id）
    pub user_id: i32,
    /// 部门 ID（关联 departments.id）
    pub department_id: i32,
    /// 是否主部门（true=主部门，false=兼职部门）
    pub is_primary: bool,
    /// 兼职开始日期（NULL 表示无固定期限）
    pub start_date: Option<chrono::NaiveDate>,
    /// 兼职结束日期（NULL 表示无固定期限）
    pub end_date: Option<chrono::NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::department::Entity",
        from = "Column::DepartmentId",
        to = "super::department::Column::Id"
    )]
    Department,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Department.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
