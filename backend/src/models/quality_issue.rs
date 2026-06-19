#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 质量异常实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_issues")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub custom_order_id: i64,
    pub process_node_id: Option<i64>,
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub discovered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub status: String,
    pub tenant_id: i64,
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
    #[sea_orm(
        belongs_to = "super::process_node::Entity",
        from = "Column::ProcessNodeId",
        to = "super::process_node::Column::Id"
    )]
    ProcessNode,
}

impl Related<super::custom_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
