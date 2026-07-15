//! 委外加工发料明细模型（outsourcing_order_item 表）
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算
//! 真实业务：每个委外订单可发出多种物料（坯布/棉纱等），按面料四维标识追溯
//! 面料四维标识：product_id + color_no + dye_lot_no + batch_no

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 委外加工发料明细模型
///
/// 真实业务要点：
/// - 一个委外订单可发出多种物料
/// - 按面料四维标识（product_id+color_no+dye_lot_no+batch_no）追溯
/// - 关联库存流水 ID，便于库存账实核对
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "outsourcing_order_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 委外订单 ID（外键 → outsourcing_order）
    pub outsourcing_order_id: i32,
    /// 发出的物料 ID（外键 → products）
    pub product_id: i32,
    /// 色号
    pub color_no: Option<String>,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 匹号（面料四维标识 product_id+color_no+dye_lot_no+batch_no）
    pub batch_no: Option<String>,
    /// 发出仓库 ID（外键 → warehouses）
    pub warehouse_id: Option<i32>,
    /// 发出数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub quantity: Decimal,
    /// 单位
    pub unit: String,
    /// 单位成本
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub unit_cost: Decimal,
    /// 明细总成本 = quantity × unit_cost
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub total_cost: Decimal,
    /// 关联库存流水 ID（可空）
    pub inventory_transaction_id: Option<i32>,
    /// 备注
    pub remarks: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联委外订单
    #[sea_orm(
        belongs_to = "super::outsourcing_order::Entity",
        from = "Column::OutsourcingOrderId",
        to = "super::outsourcing_order::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OutsourcingOrder,
    /// 关联物料
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Product,
    /// 关联仓库
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Warehouse,
}

impl Related<super::outsourcing_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutsourcingOrder.def()
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
