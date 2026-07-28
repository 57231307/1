use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_transfers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: DateTime<Utc>,
    pub status: String,
    pub total_quantity: Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // P1 batch-18 缺陷 6.1：调拨分级审批
    #[sea_orm(column_name = "approval_level")]
    pub approval_level: Option<String>,
    #[sea_orm(column_name = "approved_by_role")]
    pub approved_by_role: Option<String>,
    #[sea_orm(column_name = "total_amount", column_type = "Decimal(Some((14, 2)))", default_value = "0")]
    pub total_amount: Decimal,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::FromWarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    FromWarehouse,
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::ToWarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    ToWarehouse,
    #[sea_orm(has_many = "super::inventory_transfer_item::Entity")]
    Items,
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FromWarehouse.def()
    }
}

impl Related<super::inventory_transfer_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 缺陷 6.1：调拨审批层级常量
pub const APPROVAL_LEVEL_L1: &str = "L1";
pub const APPROVAL_LEVEL_L2: &str = "L2";
pub const APPROVAL_LEVEL_L3: &str = "L3";

/// 缺陷 6.1：调拨金额阈值（元）- L1: < 1万；L2: 1万-10万；L3: > 10万
/// 注：Decimal::new 非 const fn，使用函数返回避免 const 上下文限制
pub fn l1_amount_threshold() -> Decimal {
    Decimal::new(10000, 0)
}
pub fn l2_amount_threshold() -> Decimal {
    Decimal::new(100000, 0)
}

/// 缺陷 6.1：根据调拨总金额计算审批层级
pub fn determine_approval_level(total_amount: Decimal) -> &'static str {
    if total_amount >= l2_amount_threshold() {
        APPROVAL_LEVEL_L3
    } else if total_amount >= l1_amount_threshold() {
        APPROVAL_LEVEL_L2
    } else {
        APPROVAL_LEVEL_L1
    }
}
