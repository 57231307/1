//! 应付单 Model
//!
//! 应付管理模块包含以下实体：
//! - ap_invoice: 应付单
//! - ap_payment_request: 付款申请
//! - ap_payment_request_item: 付款申请明细
//! - ap_payment: 付款单
//! - ap_verification: 应付核销
//! - ap_verification_item: 核销明细
//! - ap_reconciliation: 供应商对账单

use sea_orm::entity::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// =====================================================
// 应付单 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_invoice")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 应付单号（AP20260315001）
    #[sea_orm(unique)]
    pub invoice_no: String,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 应付类型：PURCHASE=采购应付，EXPENSE=费用应付，OTHER=其他
    #[sea_orm(column_type = "String(N(20))")]
    pub invoice_type: String,

    /// 来源类型：PURCHASE_RECEIPT=采购入库，PURCHASE_RETURN=采购退货，MANUAL=手工录入
    pub source_type: Option<String>,

    /// 来源单 ID（采购入库单 ID 或退货单 ID）
    pub source_id: Option<i32>,

    /// 应付日期
    pub invoice_date: NaiveDate,

    /// 到期日期
    pub due_date: NaiveDate,

    /// 账期（天）
    #[sea_orm(default = "30")]
    pub payment_terms: i32,

    /// 应付金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub amount: Decimal,

    /// 已付金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub paid_amount: Decimal,

    /// 未付金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub unpaid_amount: Decimal,

    /// 应付状态：DRAFT=草稿，AUDITED=已审核，PARTIAL_PAID=部分付款，PAID=已付清，CANCELLED=已取消
    #[sea_orm(column_type = "String(N(20))", default = "'DRAFT'")]
    pub invoice_status: String,

    /// 币种
    #[sea_orm(column_type = "String(N(10))", default = "'CNY'")]
    pub currency: String,

    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", default = "1.000000")]
    pub exchange_rate: Decimal,

    /// 外币金额
    pub amount_foreign: Option<Decimal>,

    /// 税额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub tax_amount: Decimal,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub created_at: DateTime<Utc>,

    /// 更新人 ID
    pub updated_by: Option<i32>,

    /// 更新时间
    #[sea_orm(auto_time_on_create = true, auto_time_on_update = true)]
    pub updated_at: DateTime<Utc>,

    /// 审核人 ID
    pub approved_by: Option<i32>,

    /// 审核时间
    pub approved_at: Option<DateTime<Utc>>,

    /// 取消人 ID
    pub cancelled_by: Option<i32>,

    /// 取消时间
    pub cancelled_at: Option<DateTime<Utc>>,

    /// 取消原因
    pub cancelled_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UpdatedBy",
        to = "super::user::Column::Id"
    )]
    UpdatedByUser,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    ApprovedByUser,

    #[sea_orm(
        has_many = "super::ap_payment_request_item::Entity",
        from = "Column::Id",
        to = "super::ap_payment_request_item::Column::InvoiceId"
    )]
    PaymentRequestItems,

    #[sea_orm(
        has_many = "super::ap_verification_item::Entity",
        from = "Column::Id",
        to = "super::ap_verification_item::Column::InvoiceId"
    )]
    VerificationItems,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
