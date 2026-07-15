#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 库存匹数 Model
//!
//! 库存匹数模块
//! v14 批次 416：与 SQL 表 001_consolidated_schema.sql 完全同步

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库存匹数 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_piece")]
pub struct Model {
    /// 匹数 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 匹号（内部编码，唯一性由 (dye_lot_id, piece_no) 联合唯一约束保证）
    pub piece_no: String,

    /// 缸号 ID（外键，关联 batch_dye_lot；NOT NULL）
    /// v14 批次 416 新增：原 Rust 模型缺失此字段，导致 INSERT 时违反 NOT NULL 约束
    pub dye_lot_id: i32,

    /// 供应商匹号（外部编码）
    pub supplier_piece_no: Option<String>,

    /// 长度（米）
    pub length: Decimal,

    /// 重量（千克）
    pub weight: Option<Decimal>,

    /// 幅宽（cm）
    pub width: Option<Decimal>,

    /// 克重（g/m²）
    pub gram_weight: Option<Decimal>,

    /// 库位号
    pub position_no: Option<String>,

    /// 包号
    pub package_no: Option<String>,

    /// 生产日期
    pub production_date: Option<NaiveDate>,

    /// 保质期（天）
    pub shelf_life: Option<i32>,

    /// 质检状态（pending/inspecting/passed/failed）
    pub quality_status: Option<String>,

    /// 库存状态（available/reserved/locked/sold）
    pub inventory_status: Option<String>,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 备注
    pub remarks: Option<String>,

    /// 条码
    pub barcode: Option<String>,

    // ========== v14 批次 416：迁移 032 补齐的 DB 缺失字段 ==========

    /// 产品 ID（外键，通过 dye_lot_id 可间接关联，但 Rust 模型直接引用便于查询）
    pub product_id: i32,

    /// 批号（面料行业批号，与缸号配合使用）
    pub batch_no: String,

    // ========== v14 批次 419：面料行业追溯字段（F-P0-2 修复） ==========
    /// 色号（面料行业追溯字段，冗余存储便于直接查询，无需 JOIN batch_dye_lot 表）
    pub color_no: Option<String>,
    /// 缸号字符串（面料行业追溯字段，冗余存储便于直接查询）
    pub dye_lot_no: Option<String>,

    /// 母卷 ID（如果是拆分或剪裁而来的布卷，指向原始布卷 ID）
    pub parent_piece_id: Option<i32>,

    /// 库位 ID（外键）
    pub location_id: Option<i32>,

    /// 扫码类型（SHIP=扫码发货，INVENTORY=扫码盘库）
    pub scan_type: Option<String>,

    /// 状态：参考 crate::models::status::inventory_piece 模块常量
    /// AVAILABLE=可用，RESERVED=已预留，SHIPPED=已发货，DEFECT=缺陷，UNAVAILABLE=不可用
    pub status: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    /// 创建人 ID
    pub created_by: Option<i32>,

    /// 更新人 ID
    pub updated_by: Option<i32>,
}

/// 库存匹数关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 匹数 - 缸号批次（多对一）
    /// v14 批次 416 新增：dye_lot_id 外键关联 batch_dye_lot
    #[sea_orm(
        belongs_to = "super::batch_dye_lot::Entity",
        from = "Column::DyeLotId",
        to = "super::batch_dye_lot::Column::Id"
    )]
    DyeLot,

    /// 匹数 - 产品（多对一）
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,

    /// 匹数 - 仓库（多对一）
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
}

impl Related<super::batch_dye_lot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeLot.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
