#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

/// 库存流水实体模型（面料行业）
/// 记录每一笔库存变动，支持批次、色号追溯
use serde::{Serialize, Deserialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_transactions")]
pub struct Model {
    /// 流水 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 交易类型：采购入库/生产入库/销售出库/调拨/盘点/调整
    pub transaction_type: String,
    /// 产品 ID
    pub product_id: i32,
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,

    /// 数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_kg: Decimal,

    /// 来源单据类型
    pub source_bill_type: Option<String>,
    /// 来源单据号
    pub source_bill_no: Option<String>,
    /// 来源单据 ID
    pub source_bill_id: Option<i32>,

    /// 变化前数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_before_meters: Option<Decimal>,
    /// 变化前数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_before_kg: Option<Decimal>,
    /// 变化后数量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_after_meters: Option<Decimal>,
    /// 变化后数量（公斤）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_after_kg: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,
    /// 创建人 ID
    pub created_by: Option<i32>,
    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
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
