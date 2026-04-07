#![allow(dead_code, unused_imports, unused_variables)]
//! 付款申请 Model
//!
//! 应付管理模块的付款申请实体

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

// =====================================================
// 付款申请 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_payment_request")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 付款申请单号（PR20260315001）
    #[sea_orm(unique)]
    pub request_no: String,

    /// 申请日期
    pub request_date: NaiveDate,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 付款类型：PREPAYMENT=预付款，PROGRESS=进度款，FINAL=尾款，WARRANTY=质保金
    #[sea_orm(column_type = "String(N(20))")]
    pub payment_type: String,

    /// 付款方式：TT=电汇，LC=信用证，DP=付款交单，DA=承兑交单，CHECK=支票，CASH=现金
    #[sea_orm(column_type = "String(N(20))")]
    pub payment_method: String,

    /// 申请金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub request_amount: Decimal,

    /// 审批状态：DRAFT=草稿，APPROVING=审批中，APPROVED=已审批，REJECTED=已拒绝
    #[sea_orm(column_type = "String(N(20))", default = "'DRAFT'")]
    pub approval_status: String,

    /// 币种
    #[sea_orm(column_type = "String(N(10))", default = "'CNY'")]
    pub currency: String,

    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", default = "1.000000")]
    pub exchange_rate: Decimal,

    /// 外币金额
    pub request_amount_foreign: Option<Decimal>,

    /// 期望付款日期
    pub expected_payment_date: Option<NaiveDate>,

    /// 收款银行
    pub bank_name: Option<String>,

    /// 收款账号
    pub bank_account: Option<String>,

    /// 收款账户名
    pub bank_account_name: Option<String>,

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

    /// 提交人 ID
    pub submitted_by: Option<i32>,

    /// 提交时间
    pub submitted_at: Option<DateTime<Utc>>,

    /// 审批人 ID
    pub approved_by: Option<i32>,

    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,

    /// 拒绝人 ID
    pub rejected_by: Option<i32>,

    /// 拒绝时间
    pub rejected_at: Option<DateTime<Utc>>,

    /// 拒绝原因
    pub rejected_reason: Option<String>,
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
        from = "Column::SubmittedBy",
        to = "super::user::Column::Id"
    )]
    SubmittedByUser,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    ApprovedByUser,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RejectedBy",
        to = "super::user::Column::Id"
    )]
    RejectedByUser,

    #[sea_orm(
        has_many = "super::ap_payment_request_item::Entity",
        from = "Column::Id",
        to = "super::ap_payment_request_item::Column::RequestId"
    )]
    PaymentRequestItems,
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
