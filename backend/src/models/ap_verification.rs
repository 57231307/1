//! 应付核销 Model
//!
//! 应付管理模块的应付核销实体

use sea_orm::entity::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

// =====================================================
// 应付核销 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_verification")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 核销单号（VER20260315001）
    #[sea_orm(unique)]
    pub verification_no: String,

    /// 核销日期
    pub verification_date: NaiveDate,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 核销方式：AUTO=自动核销，MANUAL=手工核销
    #[sea_orm(column_type = "String(N(20))")]
    pub verification_type: String,

    /// 核销总金额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_amount: Decimal,

    /// 核销状态：COMPLETED=已完成，CANCELLED=已取消
    #[sea_orm(column_type = "String(N(20))", default = "'COMPLETED'")]
    pub verification_status: String,

    /// 备注
    pub notes: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub created_at: DateTime<Utc>,

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
        from = "Column::CancelledBy",
        to = "super::user::Column::Id"
    )]
    CancelledByUser,

    #[sea_orm(
        has_many = "super::ap_verification_item::Entity",
        from = "Column::Id",
        to = "super::ap_verification_item::Column::VerificationId"
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
