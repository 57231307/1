#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 缸号映射 Model
//!
//! v14 批次 416：与 SQL 表 001_consolidated_schema.sql 完全同步
//! 原模型字段 dye_batch_id/lot_no 在 SQL 表中不存在，已替换为正确字段

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 缸号映射 Entity（内部缸号 <-> 供应商缸号）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_lot_mapping")]
pub struct Model {
    /// 映射 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 内部缸号（NOT NULL）
    pub internal_dye_lot_no: String,

    /// 供应商缸号（NOT NULL）
    pub supplier_dye_lot_no: String,

    /// 供应商 ID（外键，NOT NULL）
    pub supplier_id: i32,

    /// 成品编码（可选，用于更精确的映射）
    pub product_code: Option<String>,

    /// 色号（可选）
    pub color_no: Option<String>,

    /// 内部缸号 ID（外键，关联 batch_dye_lot）
    pub batch_dye_lot_id: Option<i32>,

    /// 是否启用
    pub is_active: bool,

    /// 映射日期（NOT NULL）
    pub mapping_date: NaiveDate,

    /// 验证状态（pending/validated/failed）
    pub validation_status: String,

    /// 验证时间
    pub validated_at: Option<DateTime<Utc>>,

    /// 验证人 ID
    pub validated_by: Option<i32>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    /// 创建人 ID
    pub created_by: Option<i32>,

    /// 更新人 ID
    pub updated_by: Option<i32>,
}

/// 缸号映射关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 映射 - 供应商（多对一）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,

    /// 映射 - 缸号批次（多对一，可选关联）
    #[sea_orm(
        belongs_to = "super::batch_dye_lot::Entity",
        from = "Column::BatchDyeLotId",
        to = "super::batch_dye_lot::Column::Id"
    )]
    BatchDyeLot,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::batch_dye_lot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BatchDyeLot.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
