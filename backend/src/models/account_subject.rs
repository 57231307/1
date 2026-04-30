#![allow(dead_code, unused_imports, unused_variables)]
//! 会计科目 Entity
//!
//! 对应数据库表：account_subjects

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account_subjects")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub code: String,
    pub name: String,
    pub level: i32,
    pub parent_id: Option<i32>,
    pub full_code: Option<String>,

    // 余额属性
    pub balance_direction: Option<String>,
    pub initial_balance_debit: Decimal,
    pub initial_balance_credit: Decimal,
    pub current_period_debit: Decimal,
    pub current_period_credit: Decimal,
    pub ending_balance_debit: Decimal,
    pub ending_balance_credit: Decimal,

    // 辅助核算
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_department: bool,
    pub assist_employee: bool,
    pub assist_project: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub assist_dye_lot: bool,
    pub assist_grade: bool,
    pub assist_workshop: bool,

    // 双计量单位
    pub enable_dual_unit: bool,
    pub primary_unit: Option<String>,
    pub secondary_unit: Option<String>,

    // 控制属性
    pub is_cash_account: bool,
    pub is_bank_account: bool,
    pub allow_manual_entry: bool,
    pub require_summary: bool,

    // 状态
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::account_subject::Entity",
        from = "Column::ParentId",
        to = "super::account_subject::Column::Id"
    )]
    Parent,
    #[sea_orm(
        has_many = "super::account_subject::Entity",
        from = "Column::Id",
        to = "super::account_subject::Column::ParentId"
    )]
    Children,
}

impl Related<super::account_subject::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
