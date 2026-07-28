#![allow(dead_code)]
//! 角色关系 Model（V15 P1 12.2）
//!
//! 支持角色继承（inherit）与互斥（mutual_exclusive）关系
//! - 继承：parent_role_code 继承 child_role_code 的所有权限
//! - 互斥：parent_role_code 与 child_role_code 不可同时分配给同一用户

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 角色关系实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "role_relations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 父角色编码（对于 inherit：继承方；对于 mutual_exclusive：互斥方 A）
    pub parent_role_code: String,
    /// 子角色编码（对于 inherit：被继承方；对于 mutual_exclusive：互斥方 B）
    pub child_role_code: String,
    /// 关系类型：inherit（继承）/ mutual_exclusive（互斥）
    pub relation_type: String,
    /// 关系描述
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 关系类型常量
pub mod relation_type {
    pub const INHERIT: &str = "inherit";
    pub const MUTUAL_EXCLUSIVE: &str = "mutual_exclusive";
}
