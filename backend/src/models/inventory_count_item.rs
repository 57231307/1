#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库存盘点明细 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_count_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 盘点单 ID
    pub count_id: i32,

    /// 库存 ID
    pub stock_id: i32,

    /// 产品 ID
    pub product_id: i32,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 条码编号
    pub barcode: Option<String>,

    /// 卷长
    pub roll_length: Option<String>,

    /// 入库缸号
    pub dye_lot_no: Option<String>,

    /// 盘点前数量（账面数量）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity_before: Decimal,

    /// 实际盘点数量
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity_actual: Decimal,

    /// 差异数量（实际 - 账面）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity_difference: Decimal,

    /// 单位成本
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub unit_cost: Decimal,

    /// 总成本差异
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_cost: Decimal,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库存盘点明细 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::inventory_count::Entity",
        from = "Column::CountId",
        to = "super::inventory_count::Column::Id"
    )]
    Count,
    #[sea_orm(
        belongs_to = "super::inventory_stock::Entity",
        from = "Column::StockId",
        to = "super::inventory_stock::Column::Id"
    )]
    Stock,
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

impl Related<super::inventory_count::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Count.def()
    }
}

impl Related<super::inventory_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Stock.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
