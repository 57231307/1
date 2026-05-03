#![allow(dead_code)]

//! 付款单 Model
//!
//! 应付管理模块的付款单实体

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

// =====================================================
// 付款单 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_payment")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 付款单号（PAY20260315001）
    #[sea_orm(unique)]
    pub payment_no: String,

    /// 付款日期
    pub payment_date: NaiveDate,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 付款申请 ID（外键）
    pub request_id: Option<i32>,

    /// 付款方式：TT/LC/DP/DA/CHECK/CASH
    #[sea_orm(column_type = "String(N(20))")]
    pub payment_method: String,

    /// 付款金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub payment_amount: Decimal,

    /// 付款状态：REGISTERED=已登记，CONFIRMED=已确认
    #[sea_orm(column_type = "String(N(20))", default = "'REGISTERED'")]
    pub payment_status: String,

    /// 币种
    #[sea_orm(column_type = "String(N(10))", default = "'CNY'")]
    pub currency: String,

    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((18, 6)))", default = "1.000000")]
    pub exchange_rate: Decimal,

    /// 外币金额
    pub payment_amount_foreign: Option<Decimal>,

    /// 付款银行
    pub bank_name: Option<String>,

    /// 付款账号
    pub bank_account: Option<String>,

    /// 交易流水号
    pub transaction_no: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表（付款凭证）
    pub attachment_urls: Option<Vec<String>>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新人 ID
    pub updated_by: Option<i32>,

    /// 更新时间
    #[sea_orm(auto_time_on_create = true, auto_time_on_update = true)]
    pub updated_at: DateTime<Utc>,

    /// 确认人 ID
    pub confirmed_by: Option<i32>,

    /// 确认时间
    pub confirmed_at: Option<DateTime<Utc>>,
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
        belongs_to = "super::ap_payment_request::Entity",
        from = "Column::RequestId",
        to = "super::ap_payment_request::Column::Id"
    )]
    PaymentRequest,

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
        from = "Column::ConfirmedBy",
        to = "super::user::Column::Id"
    )]
    ConfirmedByUser,

    #[sea_orm(
        has_many = "super::ap_verification_item::Entity",
        from = "Column::Id",
        to = "super::ap_verification_item::Column::PaymentId"
    )]
    VerificationItems,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::ap_payment_request::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentRequest.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedByUser.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
