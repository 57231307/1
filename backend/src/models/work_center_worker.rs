#![allow(dead_code)]
//! 工作中心-人员关联 Model
//!
//! P1 batch-18 缺陷 11.1：建立工作中心-人员关联表（含多技能），支持人员技能矩阵管理

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_center_worker")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工作中心 ID（外键）
    pub work_center_id: i32,
    /// 用户 ID（外键到 users 表）
    pub user_id: i32,
    /// 多技能标签（JSON 数组，如 ["dyeing", "finishing", "inspection"]）
    pub skills: Option<serde_json::Value>,
    /// 是否为该工作中心的主要人员
    pub is_primary: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::work_center::Entity",
        from = "Column::WorkCenterId",
        to = "super::work_center::Column::Id"
    )]
    WorkCenter,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::work_center::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkCenter.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
