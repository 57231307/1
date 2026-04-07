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
#[sea_orm(table_name = "locations")]
pub struct Model {
    /// 库位 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 库位编码
    pub location_code: String,

    /// 货架编号
    pub shelf_no: String,

    /// 层编号
    pub layer_no: String,

    /// 位置编号
    pub position_no: String,

    /// 最大容量
    pub max_capacity: Decimal,

    /// 当前使用量
    pub current_usage: Decimal,

    /// 备注
    pub remarks: Option<String>,

    /// 是否启用
    pub is_active: bool,

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
