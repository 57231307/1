#![allow(dead_code)]
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
    /// 已发货数量（销售发货时累计）
    pub quantity_shipped: Decimal,
    pub quantity_incoming: Decimal,
    pub reorder_point: Decimal,
    /// 库存上限（高于此值触发 OverStock 告警，0 表示未设置）
    ///
    /// v11 批次 144 P1-4：新增字段，用于 compute_alert_type 判定"高于上限"告警。
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub max_stock_point: Decimal,
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

    /// 乐观锁版本号
    pub version: i32,

    // P1 batch-18 缺陷 7.1：补货策略字段
    /// 补货策略：reorder_point（订货点法）/ eoq（经济订货量）/ mrp（物料需求计划）
    #[sea_orm(default_value = "reorder_point")]
    pub replenishment_strategy: String,
}

// P1 batch-18 缺陷 7.1：补货策略常量
pub const REPLENISHMENT_REORDER_POINT: &str = "reorder_point";
pub const REPLENISHMENT_EOQ: &str = "eoq";
pub const REPLENISHMENT_MRP: &str = "mrp";

/// P1 batch-18 缺陷 7.1：根据补货策略计算建议采购量
/// - reorder_point：固定补货量 = reorder_quantity
/// - eoq：经济订货量 = √(2 * 年需求 * 订货成本 / 单位存储成本)
/// - mrp：由 MRP 引擎按 BOM 展开，此处返回缺口量（reorder_quantity 兜底）
pub fn compute_replenishment_qty(
    strategy: &str,
    reorder_quantity: Decimal,
    annual_demand: Option<Decimal>,
    order_cost: Option<Decimal>,
    holding_cost: Option<Decimal>,
) -> Decimal {
    match strategy {
        REPLENISHMENT_EOQ => {
            let d = annual_demand.unwrap_or(Decimal::ZERO);
            let s = order_cost.unwrap_or(Decimal::ZERO);
            let h = holding_cost.unwrap_or(Decimal::ZERO);
            if d.is_zero() || s.is_zero() || h.is_zero() {
                return reorder_quantity;
            }
            let two_ds = Decimal::from(2) * d * s;
            let eoq = (two_ds / h).to_string();
            let sqrt_val: f64 = eoq.parse().unwrap_or(0.0);
            let result = sqrt_val.sqrt();
            Decimal::try_from(result).unwrap_or(reorder_quantity)
        }
        REPLENISHMENT_MRP => reorder_quantity,
        _ => reorder_quantity,
    }
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
