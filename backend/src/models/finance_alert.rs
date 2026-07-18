//! 财务预警 Model（V15 P0-B04 Batch 481 创建）
//!
//! 表 finance_alerts：4 类财务预警（应收超额/库存积压/现金流不足/预算超支）
//! 状态机：active → acknowledged → resolved / expired

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "finance_alerts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub alert_no: String,
    /// 预警类型：ar_overdue / inventory_backlog / cash_flow_shortage / budget_overrun
    pub alert_type: String,
    /// 预警级别：info / warning / critical
    pub alert_level: String,
    pub title: String,
    pub content: String,
    pub target_module: Option<String>,
    pub target_id: Option<i64>,
    pub threshold_value: Option<Decimal>,
    pub actual_value: Option<Decimal>,
    pub value_unit: Option<String>,
    pub triggered_at: DateTime<Utc>,
    pub triggered_by: Option<i32>,
    /// 状态：active / acknowledged / resolved / expired
    pub status: String,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<i32>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<i32>,
    pub resolve_note: Option<String>,
    pub expired_at: Option<DateTime<Utc>>,
    pub notification_id: Option<i32>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::TriggeredBy",
        to = "super::user::Column::Id"
    )]
    TriggeredBy,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TriggeredBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
