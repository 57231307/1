//! BPM 流程实例 Model
//!
//! BPM 流程实例模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// BPM 流程实例 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "bpm_process_instances")]
pub struct Model {
    /// 流程实例 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流程定义 ID（外键）
    pub process_definition_id: i32,

    /// 实例编码
    pub instance_no: String,

    /// 申请人 ID
    pub applicant_id: i32,

    /// 当前状态：PENDING=待处理，PROCESSING=处理中，COMPLETED=已完成，TERMINATED=已终止
    pub status: String,

    /// 业务类型
    pub business_type: Option<String>,

    /// 业务 ID
    pub business_id: Option<i32>,

    /// 业务编号
    pub business_no: Option<String>,

    /// 流程节点
    pub current_node: Option<String>,

    /// 流程变量（JSON）
    pub variables: Option<serde_json::Value>,

    /// 开始时间
    pub start_time: DateTime<Utc>,

    /// 结束时间
    pub end_time: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// BPM 流程实例关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 实例 - 流程定义（多对一）
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
