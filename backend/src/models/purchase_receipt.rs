#![allow(dead_code)]
//! 采购入库 Model
//!
//! 采购入库模块包含以下实体：
//! - purchase_receipt: 采购入库单主表
//! - purchase_receipt_item: 采购入库明细表

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

// =====================================================
// 采购入库 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_receipt")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 入库单号（GR20260315001）
    #[sea_orm(unique)]
    pub receipt_no: String,

    /// 采购订单 ID（外键）
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 入库日期
    pub receipt_date: NaiveDate,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 收货部门 ID
    pub department_id: Option<i32>,

    /// 收货人 ID
    pub receiver_id: Option<i32>,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 质检状态（词表唯一来源 models/status/purchase_inventory.rs 的 purchase_receipt_inspection，
    /// 大写四态）：PENDING=待检验，PASSED=质检合格（允许后续入库/结算流转），
    /// REJECTED=质检不合格（走让步接收或退货流程），
    /// CONCESSION_ACCEPTED=让步接收（特采降级接收，不放行入库/结算，须复检改判）。
    /// 取值集与 DB CHECK `chk_purchase_receipt_inspection_status` 逐项相等。
    /// 本列没有 INSPECTING/FAILED 取值。
    #[sea_orm(column_type = "String(N(20))", default = "'PENDING'")]
    pub inspection_status: String,

    /// 入库状态：DRAFT=草稿，CONFIRMED=已确认，CANCELLED=已取消
    #[sea_orm(column_type = "String(N(20))", default = "'DRAFT'")]
    pub receipt_status: String,

    /// 总入库数量（主单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", default = "0.0000")]
    pub total_quantity: Decimal,

    /// 总入库数量（辅助单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))", default = "0.0000")]
    pub total_quantity_alt: Decimal,

    /// 总金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub total_amount: Decimal,

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

    /// 确认时间
    pub confirmed_at: Option<DateTime<Utc>>,

    /// 确认人 ID
    pub confirmed_by: Option<i32>,

    /// 最近一次让步接收理由（专用真实列，禁挪用 notes）；写入方 concede_receipt
    pub concession_reason: Option<String>,

    /// 最近一次让步接收操作人（服务端会话派生，请求体不承载身份）；写入方 concede_receipt
    pub concession_by: Option<i32>,

    /// 最近一次让步接收时间；写入方 concede_receipt
    pub concession_at: Option<DateTime<Utc>>,

    /// 最近一次复检改判理由（专用真实列）；写入方 rejudge_receipt
    pub rejudge_reason: Option<String>,

    /// 最近一次复检改判操作人（服务端会话派生）；写入方 rejudge_receipt
    pub rejudge_by: Option<i32>,

    /// 最近一次复检改判时间；写入方 rejudge_receipt
    pub rejudge_at: Option<DateTime<Utc>>,

    /// 复检改判累计次数；写入方 rejudge_receipt 累加（INTEGER NOT NULL DEFAULT 0）
    pub rejudge_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联采购订单
    #[sea_orm(
        belongs_to = "crate::models::purchase_order::Entity",
        from = "Column::OrderId",
        to = "crate::models::purchase_order::Column::Id"
    )]
    Order,

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

    /// 关联收货人
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::ReceiverId",
        to = "crate::models::user::Column::Id"
    )]
    Receiver,

    /// 关联质检员
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::InspectorId",
        to = "crate::models::user::Column::Id"
    )]
    Inspector,

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

    /// 关联确认人
    #[sea_orm(
        belongs_to = "crate::models::user::Entity",
        from = "Column::ConfirmedBy",
        to = "crate::models::user::Column::Id"
    )]
    Confirmer,

    /// 关联入库明细
    #[sea_orm(has_many = "super::purchase_receipt_item::Entity")]
    ReceiptItems,
}

impl Related<super::purchase_receipt_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ReceiptItems.def()
    }
}

impl Related<crate::models::purchase_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
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
        Relation::Receiver.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
