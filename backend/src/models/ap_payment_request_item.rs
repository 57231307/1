//! 付款申请明细 Model
//!
//! 应付管理模块的付款申请明细实体

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// =====================================================
// 付款申请明细 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_payment_request_item")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 付款申请 ID（外键）
    pub request_id: i32,

    /// 应付单 ID（外键）
    pub invoice_id: i32,

    /// 申请金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub apply_amount: Decimal,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ap_payment_request::Entity",
        from = "Column::RequestId",
        to = "super::ap_payment_request::Column::Id"
    )]
    PaymentRequest,

    #[sea_orm(
        belongs_to = "super::ap_invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::ap_invoice::Column::Id"
    )]
    Invoice,
}

impl Related<super::ap_payment_request::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PaymentRequest.def()
    }
}

impl Related<super::ap_invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
