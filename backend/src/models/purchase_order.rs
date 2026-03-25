//! 采购订单 Model
//!
//! 采购订单模块包含以下实体：
//! - purchase_order: 采购订单主表
//! - purchase_order_item: 采购订单明细表

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

// =====================================================
// 采购订单 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_order")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 订单编号（PO20260315001）
    #[sea_orm(unique)]
    pub order_no: String,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 订单日期
    pub order_date: NaiveDate,

    /// 预计交货日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 实际交货日期
    pub actual_delivery_date: Option<NaiveDate>,

    /// 入库仓库 ID
    pub warehouse_id: i32,

    /// 采购部门 ID
    pub department_id: i32,

    /// 采购员 ID
    pub purchaser_id: i32,

    /// 币种
    #[sea_orm(column_type = "String(N(10))", default = "'CNY'")]
    pub currency: String,

    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", default = "1.000000")]
    pub exchange_rate: Decimal,

    /// 订单总金额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub total_amount: Decimal,

    /// 订单总金额（外币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub total_amount_foreign: Decimal,

    /// 总数量（主单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", default = "0.0000")]
    pub total_quantity: Decimal,

    /// 总数量（辅助单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", default = "0.0000")]
    pub total_quantity_alt: Decimal,

    /// 订单状态：DRAFT=草稿，SUBMITTED=已提交，APPROVED=已审批，REJECTED=已拒绝，PARTIAL_RECEIVED=部分入库，COMPLETED=已完成，CLOSED=已关闭
    #[sea_orm(column_type = "String(N(20))", default = "'DRAFT'")]
    pub order_status: String,

    /// 付款条件
    pub payment_terms: Option<String>,

    /// 运输条款
    pub shipping_terms: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新人 ID
    pub updated_by: Option<i32>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    /// 审批人 ID
    pub approved_by: Option<i32>,

    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,

    /// 拒绝原因
    pub rejected_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联供应商
    #[sea_orm(
        belongs_to = "crate::models::supplier::Entity",
        from = "Column::SupplierId",
        to = "crate::models::supplier::Column::Id"
    )]
    Supplier,

    /// 关联仓库
    #[sea_orm(
        belongs_to = "crate::models::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "crate::models::warehouse::Column::Id"
    )]
    Warehouse,

    /// 关联部门
    #[sea_orm(
        belongs_to = "crate::models::department::Entity",
        from = "Column::DepartmentId",
        to = "crate::models::department::Column::Id"
    )]
    Department,

    /// 关联采购员
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::PurchaserId",
        to = "crate::models::user::Column::Id"
    )]
    Purchaser,

    /// 关联创建人
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::CreatedBy",
        to = "crate::models::user::Column::Id"
    )]
    Creator,

    /// 关联更新人
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::UpdatedBy",
        to = "crate::models::user::Column::Id"
    )]
    Updater,

    /// 关联审批人
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::ApprovedBy",
        to = "crate::models::user::Column::Id"
    )]
    Approver,

    /// 关联订单明细
    #[sea_orm(has_many = "super::purchase_order_item::Entity")]
    OrderItems,
}

impl Related<super::purchase_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItems.def()
    }
}

impl Related<crate::models::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<crate::models::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl Related<crate::models::department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Department.def()
    }
}

impl Related<crate::models::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Purchaser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
