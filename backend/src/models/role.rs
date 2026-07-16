#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "roles")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub permissions: Option<String>,
    pub is_system: bool,
    /// V15 P0-S01 新增：数据范围（行级数据权限）
    /// 取值：all（全部数据）/ dept（本部门数据）/ self（仅本人数据）
    /// 默认 'self'，确保最小权限原则。
    pub data_scope: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user::Entity")]
    Users,
    #[sea_orm(has_many = "super::role_permission::Entity")]
    RolePermissions,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Users.def()
    }
}

impl Related<super::role_permission::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RolePermissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
