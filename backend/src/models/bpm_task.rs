#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! BPM 任务 Model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BPM 任务 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bpm_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub task_no: String,

    pub instance_id: i32,

    pub process_definition_id: i32,

    pub node_id: String,

    pub node_name: String,

    pub node_type: String,

    pub task_type: Option<String>,

    pub status: Option<String>,

    pub priority: Option<String>,

    pub assignee_ids: Option<Vec<i32>>,

    pub assignee_names: Option<Vec<String>>,

    pub candidate_role_ids: Option<Vec<i32>>,

    pub candidate_user_ids: Option<Vec<i32>>,

    pub actual_handler_id: Option<i32>,

    pub actual_handler_name: Option<String>,

    pub action: Option<String>,

    pub approval_opinion: Option<String>,

    pub attachment_urls: Option<Vec<String>>,

    pub handled_at: Option<DateTime<Utc>>,

    pub duration_seconds: Option<i64>,

    pub due_date: Option<DateTime<Utc>>,

    pub is_overdue: Option<bool>,

    pub overdue_days: Option<i32>,

    pub form_data: Option<serde_json::Value>,

    pub task_variables: Option<serde_json::Value>,

    pub created_at: Option<DateTime<Utc>>,

    pub updated_at: Option<DateTime<Utc>>,

    pub remarks: Option<String>,
}

/// BPM 任务关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bpm_process_instance::Entity",
        from = "Column::InstanceId",
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
