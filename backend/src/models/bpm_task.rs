#![allow(dead_code)]
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

    /// 业务类型（按类型判别被引用表，多态松散引用）
    pub business_type: Option<String>,
    /// 业务单据 ID（与 bpm_process_instance.business_id 同源同值）
    pub business_id: Option<i64>,

    // ===== 以下四列生效 DDL 为 JSONB（migration system/mod.rs:96-100）=====
    // 无 column_type 属性时 SeaORM 按 Vec<T> 推断为 PG 数组类型，读 JSONB 列即
    // ColumnDecode、写整链带病；范本见 models/lab_dip_sample.rs formula_detail。
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub assignee_ids: Option<Vec<i32>>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub assignee_names: Option<Vec<String>>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub candidate_role_ids: Option<Vec<i32>>,

    #[sea_orm(column_type = "JsonBinary", nullable)]
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
