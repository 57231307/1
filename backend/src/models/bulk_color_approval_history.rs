//! 大货批色状态变更历史 Model（V15 P1-10 创建）
//!
//! 表 bulk_color_approval_history：记录每次状态变更的快照，支持客户投诉追溯、
//! 内部责任界定、合规审计三大业务场景。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

/// 大货批色状态变更历史实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bulk_color_approval_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 关联批色记录
    pub bulk_color_approval_id: i64,
    /// 变更前状态（首次创建时为 None）
    pub from_status: Option<String>,
    /// 变更后状态
    pub to_status: String,
    /// 操作人用户 ID
    pub operator_id: Option<i32>,
    /// 变更原因
    pub reason: Option<String>,
    /// 变更后记录完整快照 JSON
    pub snapshot: Option<Json>,
    pub created_at: DateTime<Utc>,
}

/// 关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 历史 - 批色记录（多对一）
    #[sea_orm(
        belongs_to = "super::bulk_color_approval::Entity",
        from = "Column::BulkColorApprovalId",
        to = "super::bulk_color_approval::Column::Id"
    )]
    BulkColorApproval,
}

impl Related<super::bulk_color_approval::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BulkColorApproval.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
