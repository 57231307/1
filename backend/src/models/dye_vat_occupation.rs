#![allow(dead_code)]
//! 染缸占用记录 Model（V15 P2 B05-P2-6 创建）
//!
//! 表 dye_vat_occupation：记录染缸设备占用与释放（缸号进入 dyeing 状态时占用，
//! 离开 dyeing 状态时释放），支持设备资源调度与产能可视化。
//! 唯一约束：同一 vat_id 同时只能有一条 status='occupied' 的记录。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 染缸占用状态常量（status 字段值）
pub mod occupation_status {
    /// 已占用：染缸正在被某缸号使用
    pub const OCCUPIED: &str = "occupied";

    /// 已释放：染缸已释放，可被其他缸号占用
    pub const RELEASED: &str = "released";
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_vat_occupation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 染缸/设备 ID（关联 work_center_equipment 或 dye_vat 主数据）
    pub vat_id: i32,
    /// 占用该染缸的缸号 ID（关联 dye_batch.id）
    pub batch_id: i32,
    /// 冗余缸号编号（便于报表查询，避免 join）
    pub batch_no: Option<String>,
    /// 占用时间（缸号进入 dyeing 状态时）
    pub occupied_at: DateTime<Utc>,
    /// 释放时间（缸号离开 dyeing 状态时）
    pub released_at: Option<DateTime<Utc>>,
    /// 状态：occupied / released
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::BatchId",
        to = "super::dye_batch::Column::Id"
    )]
    DyeBatch,
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
