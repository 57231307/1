#![allow(dead_code)]

//! BPM 流程实例 Model

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BPM 流程实例 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bpm_process_instance")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub instance_no: String,

    pub process_definition_id: i32,

    pub business_type: String,

    pub business_id: i32,

    pub title: String,

    pub priority: Option<String>,

    pub current_node_id: Option<String>,

    pub current_node_name: Option<String>,

    pub status: Option<String>,

    pub initiator_id: i32,

    pub initiator_name: String,

    pub initiator_department_id: Option<i32>,

    pub current_handler_ids: Option<Vec<i32>>,

    pub current_handler_names: Option<Vec<String>>,

    pub form_data: Option<serde_json::Value>,

    pub variables: Option<serde_json::Value>,

    pub started_at: Option<DateTime<Utc>>,

    pub completed_at: Option<DateTime<Utc>>,

    pub duration_seconds: Option<i64>,

    pub created_at: Option<DateTime<Utc>>,

    pub updated_at: Option<DateTime<Utc>>,

    pub remarks: Option<String>,
}

/// BPM 流程实例关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::bpm_process_definition::Entity",
        from = "Column::ProcessDefinitionId",
        to = "super::bpm_process_definition::Column::Id"
    )]
    ProcessDefinition,
}

impl Related<super::bpm_process_definition::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessDefinition.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
