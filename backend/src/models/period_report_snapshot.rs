#![allow(dead_code)]
//! 期末报表快照 Entity
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "period_report_snapshots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 会计期间 ID
    pub period_id: i32,
    /// 报表类型（balance_sheet/income_statement/cash_flow/trial_balance）
    pub report_type: String,
    /// 报表数据（JSON）
    pub report_data: Json,
    /// 快照哈希（SHA-256，防篡改）
    pub snapshot_hash: String,
    /// 创建人 ID
    pub created_by: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 快照 - 会计期间（多对一）
    #[sea_orm(
        belongs_to = "super::accounting_period::Entity",
        from = "Column::PeriodId",
        to = "super::accounting_period::Column::Id"
    )]
    AccountingPeriod,
}

impl Related<super::accounting_period::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AccountingPeriod.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
