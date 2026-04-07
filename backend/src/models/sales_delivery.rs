#![allow(dead_code, unused_imports, unused_variables)]
//! 销售交货 Model
//!
//! 销售交货模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售交货 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_delivery")]
pub struct Model {
    /// 交货单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 交货单号
    #[sea_orm(unique)]
    pub delivery_no: String,

    /// 销售订单 ID（外键）
    pub order_id: i32,

    /// 客户 ID（外键）
    pub customer_id: i32,

    /// 交货日期
    pub delivery_date: NaiveDate,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 交货状态：DRAFT=草稿，SHIPPED=已发货，DELIVERED=已交货，PARTIAL=部分交货
    pub status: String,

    /// 总数量
    pub total_quantity: Decimal,

    /// 总金额
    pub total_amount: Decimal,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 销售交货关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 交货 - 客户（多对一）
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,

    /// 交货 - 仓库（多对一）
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,

    /// 交货 - 销售订单（多对一）
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::OrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,

    /// 交货 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
