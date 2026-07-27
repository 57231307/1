//! 工作中心-设备关联 Model
//!
//! P1 batch-18 缺陷 11.1：建立工作中心-设备关联表，支持按设备维度精细产能分析

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_center_equipment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工作中心 ID（外键）
    pub work_center_id: i32,
    /// 设备名称
    pub equipment_name: String,
    /// 设备编码
    pub equipment_code: Option<String>,
    /// 状态：active/maintenance/inactive
    pub status: String,
    /// 每小时产能
    #[sea_orm(column_type = "Decimal(Some((10, 2)))", nullable)]
    pub capacity_per_hour: Option<Decimal>,
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
