//! 坏账核销审批 Model（V15 P0-B02 Batch 481 创建）
//!
//! 表 bad_debt_writeoffs：二级审批流（申请人→财务经理→总经理）+ 核销执行
//! 状态机：pending → finance_approved → approved（核销完成）
//!                 → rejected（任一级拒绝）
//!                 → cancelled（申请人取消）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bad_debt_writeoffs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub customer_id: i64,
    pub ar_invoice_id: i32,
    pub writeoff_amount: Decimal,
    pub reason: String,
    pub applicant_user_id: i32,
    pub applicant_username: String,
    pub applicant_at: DateTime<Utc>,
    /// 当前审批层级：1=待财务经理 / 2=待总经理
    pub approval_level: i16,
    /// 状态：pending / finance_approved / approved / rejected / cancelled
    pub approval_status: String,
    pub finance_manager_id: Option<i32>,
    pub finance_manager_at: Option<DateTime<Utc>>,
    pub finance_manager_comment: Option<String>,
    pub general_manager_id: Option<i32>,
    pub general_manager_at: Option<DateTime<Utc>>,
    pub general_manager_comment: Option<String>,
    pub voucher_id: Option<i64>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancel_reason: Option<String>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::ar_invoice::Entity",
        from = "Column::ArInvoiceId",
        to = "super::ar_invoice::Column::Id"
    )]
    ArInvoice,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApplicantUserId",
        to = "super::user::Column::Id"
    )]
    Applicant,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::ar_invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ArInvoice.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Applicant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
