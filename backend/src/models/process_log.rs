#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工艺日志实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "process_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub process_node_id: i64,
    pub action: String,
    pub operator_id: Option<i64>,
    pub before_status: Option<String>,
    pub after_status: Option<String>,
    pub log_time: DateTime<Utc>,
    pub log_content: Option<String>,
    pub attachments: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::process_node::Entity",
        from = "Column::ProcessNodeId",
        to = "super::process_node::Column::Id"
    )]
    ProcessNode,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OperatorId",
        to = "super::user::Column::Id"
    )]
    Operator,
}

impl Related<super::process_node::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessNode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
