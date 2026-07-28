#![allow(dead_code)]
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
    #[sea_orm(
        column_name = "total_amount",
        column_type = "Decimal(Some((14, 2)))",
        default_value = "0"
    )]
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

/// 缺陷 6.1：L1 审批允许的角色（常规调拨 < 1万元）
/// 包含仓库一线操作员及更高层级角色
pub const L1_APPROVER_ROLES: &[&str] = &[
    "admin",
    "gm",
    "deputy_gm",
    "warehouse_manager",
    "warehouse_clerk",
    "inventory_manager",
];

/// 缺陷 6.1：L2 审批允许的角色（经理级，1万-10万元）
pub const L2_APPROVER_ROLES: &[&str] =
    &["admin", "gm", "deputy_gm", "warehouse_manager", "inventory_manager"];

/// 缺陷 6.1：L3 审批允许的角色（总监级，> 10万元）
pub const L3_APPROVER_ROLES: &[&str] = &["admin", "gm", "deputy_gm"];

/// 缺陷 6.1：根据审批层级校验角色是否有权审批
/// L1：仓库一线及更高层级；L2：经理级及以上；L3：总监级及以上
pub fn can_approve_at_level(role_code: &str, level: &str) -> bool {
    let allowed: &[&str] = match level {
        APPROVAL_LEVEL_L1 => L1_APPROVER_ROLES,
        APPROVAL_LEVEL_L2 => L2_APPROVER_ROLES,
        APPROVAL_LEVEL_L3 => L3_APPROVER_ROLES,
        _ => return false,
    };
    allowed.contains(&role_code)
}
