//! 账户余额 Model
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 账户余额 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "account_balances")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 科目ID
    pub subject_id: i32,

    /// 会计期间 (YYYY-MM)
    pub period: String,

    /// 期初余额(借方)
    pub initial_balance_debit: Decimal,

    /// 期初余额(贷方)
    pub initial_balance_credit: Decimal,

    /// 本期发生额(借方)
    pub current_period_debit: Decimal,

    /// 本期发生额(贷方)
    pub current_period_credit: Decimal,

    /// 期末余额(借方)
    pub ending_balance_debit: Decimal,

    /// 期末余额(贷方)
    pub ending_balance_credit: Decimal,

    /// 创建时间
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: DateTime<Utc>,

    /// 更新时间
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
