#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工艺节点实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "process_nodes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub custom_order_id: i64,
    pub node_type: String,
    pub node_name: String,
    pub sequence: i32,
    pub status: String,
    pub planned_start_date: Option<DateTime<Utc>>,
    pub planned_end_date: Option<DateTime<Utc>>,
    pub actual_start_date: Option<DateTime<Utc>>,
    pub actual_end_date: Option<DateTime<Utc>>,
    pub operator_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::custom_order::Entity",
        from = "Column::CustomOrderId",
        to = "super::custom_order::Column::Id"
    )]
    CustomOrder,
    #[sea_orm(has_many = "super::process_log::Entity")]
    Logs,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OperatorId",
        to = "super::user::Column::Id"
    )]
    Operator,
}

impl Related<super::custom_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomOrder.def()
    }
}

impl Related<super::process_log::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Logs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
