#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库存实体模型（面料行业版）
/// 包含批次、色号、缸号、等级、双计量单位等面料行业特色字段
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_stocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub warehouse_id: i32,
    pub product_id: i32,
    pub quantity_on_hand: Decimal,
    pub quantity_available: Decimal,
    pub quantity_reserved: Decimal,
    pub quantity_incoming: Decimal,
    pub reorder_point: Decimal,
    pub reorder_quantity: Decimal,
    pub bin_location: Option<String>,
    pub last_count_date: Option<DateTime<Utc>>,
    pub last_movement_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // ========== 面料行业特色字段 ==========
    /// 批次号（必填）
    pub batch_no: String,
    /// 色号（必填）
    pub color_no: String,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 等级：一等品/二等品/等外品
    pub grade: String,
    /// 生产日期
    pub production_date: Option<DateTime<Utc>>,
    /// 保质期
    pub expiry_date: Option<DateTime<Utc>>,

    /// 数量（米）- 主计量单位
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_meters: Decimal,
    /// 数量（公斤）- 辅计量单位
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_kg: Decimal,
    /// 克重（g/m²）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub gram_weight: Option<Decimal>,
    /// 幅宽（cm）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub width: Option<Decimal>,

    /// 库位 ID
    pub location_id: Option<i32>,
    /// 货架号
    pub shelf_no: Option<String>,
    /// 层号
    pub layer_no: Option<String>,

    /// 库存状态：正常/冻结/待检
    pub stock_status: String,
    /// 质量状态：合格/不合格/待检
    pub quality_status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
