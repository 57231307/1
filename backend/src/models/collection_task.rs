//! 催收任务 Model（V15 P0-B03 Batch 481 创建）
//!
//! 表 collection_tasks：按账龄自动生成催收任务，分配销售员，记录催收结果
//! 状态机：pending → in_progress → completed / cancelled

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "collection_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub task_no: String,
    pub customer_id: i64,
    pub ar_invoice_id: Option<i32>,
    pub overdue_amount: Decimal,
    pub overdue_days: i32,
    /// 催收类型：phone / visit / email / letter
    pub task_type: String,
    /// 优先级：low / normal / high / urgent
    pub priority: String,
    pub due_date: NaiveDate,
    pub assigned_to: i32,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<i32>,
    /// 状态：pending / in_progress / completed / cancelled
    pub status: String,
    pub contact_result: Option<String>,
    pub contact_at: Option<DateTime<Utc>>,
    pub next_action_date: Option<NaiveDate>,
    pub next_action_type: Option<String>,
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
        from = "Column::AssignedTo",
        to = "super::user::Column::Id"
    )]
    AssignedTo,
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
        Relation::AssignedTo.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
