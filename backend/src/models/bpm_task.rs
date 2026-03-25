//! BPM 任务 Model
//!
//! BPM 任务模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BPM 任务 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bpm_tasks")]
pub struct Model {
    /// 任务 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流程实例 ID（外键）
    pub process_instance_id: i32,

    /// 任务编码
    pub task_no: String,

    /// 任务名称
    pub name: String,

    /// 任务类型
    pub task_type: String,

    /// 节点 ID
    pub node_id: String,

    /// 节点名称
    pub node_name: String,

    /// 处理人 ID
    pub assignee_id: Option<i32>,

    /// 候选用户 IDs（JSON 数组）
    pub candidate_ids: Option<serde_json::Value>,

    /// 任务状态：PENDING=待处理，COMPLETED=已完成，REJECTED=已拒绝
    pub status: String,

    /// 业务类型
    pub business_type: Option<String>,

    /// 业务 ID
    pub business_id: Option<i32>,

    /// 处理意见
    pub comment: Option<String>,

    /// 处理时间
    pub completed_at: Option<DateTime<Utc>>,

    /// 到期时间
    pub due_time: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// BPM 任务关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 任务 - 流程实例（多对一）
    #[sea_orm(
        belongs_to = "super::bpm_process_instance::Entity",
        from = "Column::ProcessInstanceId",
        to = "super::bpm_process_instance::Column::Id"
    )]
    ProcessInstance,
}

impl Related<super::bpm_process_instance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessInstance.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
