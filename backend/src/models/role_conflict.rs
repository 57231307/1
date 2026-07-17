use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// V15 P0-S05/P0-S23：SoD 职责分离互斥角色对 model
// 记录互斥角色对，防止用户同时持有冲突角色
// P0-S23 已真实接入业务：user_service.check_role_conflict_for_user + init_service.create_default_role_conflicts

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "role_conflicts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 互斥角色 A 的 code（保证 role_a_code < role_b_code）
    pub role_a_code: String,
    /// 互斥角色 B 的 code
    pub role_b_code: String,
    /// 冲突类型（sod = 职责分离）
    pub conflict_type: String,
    /// 冲突描述
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
