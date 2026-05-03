#![allow(dead_code)]

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库存预留 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_reservations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 销售订单 ID
    pub order_id: i32,

    /// 产品 ID
    pub product_id: i32,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 预留数量
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity: Decimal,

    /// 预留状态：pending-待处理，locked-已锁定，released-已释放，used-已使用
    pub status: String,

    /// 预留时间
    pub reserved_at: DateTime<Utc>,

    /// 释放时间
    pub released_at: Option<DateTime<Utc>>,

    /// 备注
    pub notes: Option<String>,

    /// 创建人
    pub created_by: Option<i32>,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库存预留 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::OrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,
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

impl ActiveModelBehavior for ActiveModel {}
