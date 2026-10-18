#![allow(dead_code)]
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// SoD 职责分离互斥角色对 model
// 记录互斥角色对，防止用户同时持有冲突角色。
// 内置默认只覆盖财务侧钱账分管：建表种子写在 migration/src/domain/finance/，
// 其中 2 对「生产↔质量」默认已由 migration/src/domain/role_conflicts_drop_production_sod/
// 撤除（纺织厂一人多岗为常态，生产与质量不得由系统内置硬互斥）；企业自配互斥走管理面，
// 初始化链不再写本表。
// 真实消费方：user_service::check_role_conflict_for_user（换角色写口校验）
// 与 GET /roles/conflicts 列表端点（handlers/role_handler.rs）。

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
