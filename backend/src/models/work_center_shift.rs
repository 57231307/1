#![allow(dead_code)]
//! 工作中心-班次关联 Model
//!
//! P1 batch-18 缺陷 11.1：建立工作中心-班次关联表，替代硬编码 default_shifts_for_type

use chrono::{DateTime, NaiveTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_center_shift")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工作中心 ID（外键）
    pub work_center_id: i32,
    /// 班次名称（如 早班/中班/晚班）
    pub shift_name: String,
    /// 开始时间
    pub start_time: NaiveTime,
    /// 结束时间
    pub end_time: NaiveTime,
    /// 是否启用
    pub is_active: bool,
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
}

impl Related<super::work_center::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkCenter.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
