//! 采购退货 Model
//!
//! 采购退货模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 采购退货 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_return")]
pub struct Model {
    /// 退货单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 退货单号
    #[sea_orm(unique)]
    pub return_no: String,

    /// 采购订单 ID（外键）
    pub purchase_order_id: Option<i32>,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

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
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 采购退货关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::purchase_return_item::Entity")]
    Items,

    /// 采购退货 - 供应商（多对一）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,

    /// 采购退货 - 仓库（多对一）
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,

    /// 采购退货 - 采购订单（多对一）
    #[sea_orm(
        belongs_to = "super::purchase_order::Entity",
        from = "Column::PurchaseOrderId",
        to = "super::purchase_order::Column::Id"
    )]
    PurchaseOrder,

    /// 采购退货 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl Related<super::purchase_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PurchaseOrder.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl Related<super::purchase_return_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
