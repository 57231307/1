#![allow(dead_code, unused_imports, unused_variables)]
//! 供应商对账单 Model
//!
//! 应付管理模块的供应商对账实体

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

// =====================================================
// 供应商对账单 Entity
// =====================================================
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ap_reconciliation")]
pub struct Model {
    /// 主键 ID
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 对账单号（REC20260315001）
    #[sea_orm(unique)]
    pub reconciliation_no: String,

    /// 供应商 ID（外键）
    pub supplier_id: i32,

    /// 对账开始日期
    pub start_date: NaiveDate,

    /// 对账结束日期
    pub end_date: NaiveDate,

    /// 期初余额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub opening_balance: Decimal,

    /// 本期应付合计
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub total_invoice: Decimal,

    /// 本期付款合计
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub total_payment: Decimal,

    /// 期末余额
    #[sea_orm(column_type = "Decimal(Some((18, 2)))", default = "0.00")]
    pub closing_balance: Decimal,

    /// 对账状态：PENDING=待确认，CONFIRMED=已确认，DISPUTED=有争议
    #[sea_orm(column_type = "String(N(20))", default = "'PENDING'")]
    pub reconciliation_status: String,

    /// 备注
    pub notes: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    #[sea_orm(auto_time_on_create = true)]
    pub created_at: DateTime<Utc>,

    /// 更新时间
    #[sea_orm(auto_time_on_update = true)]
    pub updated_at: DateTime<Utc>,

    /// 确认人 ID（供应商确认）
    pub confirmed_by: Option<i32>,

    /// 确认时间
    pub confirmed_at: Option<DateTime<Utc>>,

    /// 争议人 ID
    pub disputed_by: Option<i32>,

    /// 争议时间
    pub disputed_at: Option<DateTime<Utc>>,

    /// 争议原因
    pub disputed_reason: Option<String>,
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
        from = "Column::ConfirmedBy",
        to = "super::user::Column::Id"
    )]
    ConfirmedByUser,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::DisputedBy",
        to = "super::user::Column::Id"
    )]
    DisputedByUser,
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
