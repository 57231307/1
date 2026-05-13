#![allow(dead_code)]

//! 销售退货 Model
//!
//! 销售退货模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售退货 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_return")]
pub struct Model {
    /// 退货单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 退货单号
    #[sea_orm(unique)]
    pub return_no: String,

    /// 销售订单 ID（外键）
    pub sales_order_id: Option<i32>,

    /// 客户 ID（外键）
    pub customer_id: i32,

    /// 退货日期
    pub return_date: NaiveDate,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 退货原因
    pub reason: String,

    /// 状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，COMPLETED=已完成
    pub status: String,

    /// 退货总金额
    pub total_amount: Decimal,

    /// 备注
    pub remarks: Option<String>,

    /// 审批人 ID
    pub approved_by: Option<i32>,

    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,

    /// 拒绝原因
    pub rejected_reason: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 销售退货关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sales_return_item::Entity")]
    Items,

    /// 销售退货 - 客户（多对一）
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,

    /// 销售退货 - 仓库（多对一）
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,

    /// 销售退货 - 销售订单（多对一）
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::SalesOrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,

    /// 销售退货 - 用户（创建人，多对一）
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

impl Related<super::sales_return_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
