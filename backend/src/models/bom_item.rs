#![allow(dead_code)]

//! BOM明细 Model
//!
//! BOM物料清单明细项

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BOM明细 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bom_items")]
pub struct Model {
    /// BOM明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// BOM ID（外键）
    pub bom_id: i32,

    /// 物料 ID
    pub material_id: i32,

    /// 用量
    pub quantity: Decimal,

    /// 单位
    pub unit: Option<String>,

    /// 损耗率（0-1）
    pub scrap_rate: Option<Decimal>,

    /// 排序号
    pub sort_order: Option<i32>,

    /// 是否删除

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// BOM明细关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// BOM明细 - BOM（多对一）
    #[sea_orm(
        belongs_to = "super::bom::Entity",
        from = "Column::BomId",
        to = "super::bom::Column::Id"
    )]
    Bom,
}

impl Related<super::bom::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Bom.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
