//! 核销明细 Model
//!
//! 应付管理模块的核销明细实体

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// =====================================================
// 核销明细 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_verification_item")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 核销单 ID（外键）
    pub verification_id: i32,

    /// 应付单 ID（外键）
    pub invoice_id: i32,

    /// 付款单 ID（外键）
    pub payment_id: i32,

    /// 核销金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub verify_amount: Decimal,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ap_verification::Entity",
        from = "Column::VerificationId",
        to = "super::ap_verification::Column::Id"
    )]
    Verification,

    #[sea_orm(
        belongs_to = "super::ap_invoice::Entity",
        from = "Column::InvoiceId",
        to = "super::ap_invoice::Column::Id"
    )]
    Invoice,

    #[sea_orm(
        belongs_to = "super::ap_payment::Entity",
        from = "Column::PaymentId",
        to = "super::ap_payment::Column::Id"
    )]
    Payment,
}

impl Related<super::ap_verification::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Verification.def()
    }
}

impl Related<super::ap_invoice::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Invoice.def()
    }
}

impl Related<super::ap_payment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Payment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
