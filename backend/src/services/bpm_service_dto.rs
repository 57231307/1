//! BPM 服务相关 DTO 结构体（业务关系/审批链/监控统计/流程详情）
//!
//! 拆分自 bpm_service.rs：原 4 个 DTO 单独成文件便于引用。

use crate::models::{bpm_process_instance, bpm_task};

/// BPM business relation info
/// BPM business relation info
#[derive(Debug, serde::Serialize)]
pub struct BpmBusinessRelation {
    pub has_process: bool,
    pub instance_id: i32,
    pub instance_no: String,
    pub process_status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub task_count: i32,
    pub completed_tasks: i32,
    pub pending_tasks: i32,
}

/// 审批链节点信息
#[derive(Debug, serde::Serialize)]
pub struct ApprovalChainNode {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub assignee_id: Option<i32>,
    pub assignee_name: Option<String>,
    pub status: String,
    pub comment: Option<String>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub due_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// 流程监控统计
#[derive(Debug, serde::Serialize)]
pub struct ProcessMonitorStats {
    pub total_instances: i64,
    pub processing_instances: i64,
    pub completed_instances: i64,
    pub terminated_instances: i64,
    pub total_tasks: i64,
    pub pending_tasks: i64,
    pub completed_tasks: i64,
    pub rejected_tasks: i64,
    pub avg_process_duration_minutes: Option<f64>,
}

/// 流程实例详情
#[derive(Debug, serde::Serialize)]
pub struct ProcessInstanceDetail {
    pub instance: bpm_process_instance::Model,
    pub definition_name: String,
    pub tasks: Vec<bpm_task::Model>,
    pub approval_chain: Vec<ApprovalChainNode>,
}
