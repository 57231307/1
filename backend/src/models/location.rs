#![allow(dead_code, unused_imports, unused_variables)]
//! 库位 Model
//!
//! 仓库库位管理模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库位 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "warehouse_locations")]
pub struct Model {
    /// 库位 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 库位编码
    pub location_code: String,

    /// 库位类型：平面库/货架库/卷装库
    pub location_type: Option<String>,

    /// 最大重量
    pub max_weight: Option<Decimal>,

    /// 最大高度
    pub max_height: Option<Decimal>,

    /// 是否批次管理
    pub is_batch_managed: Option<bool>,

    /// 是否色号管理
    pub is_color_managed: Option<bool>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库位 Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
