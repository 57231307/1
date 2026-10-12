#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateProcessDefinitionRequest {
    // 入参字段名桥接：前端 api/bpm-enhanced.ts 的 ProcessDefinition 契约与 GET 列表出参别名
    // 均用 `process_name`/`process_key`（见 bpm_definition_handler.rs model_to_frontend_json）。
    // 出参已补齐实体真源键 code/name，但入参此前只认 code/name，导致 UI 建单提交的
    // {process_key, process_name} 反序列化缺必填 name/code → 400。这里按 handler 注释声明的
    // “入参出参字段名差异由 handler 层负责转换”补齐反序列化别名；Rust 字段名与内部构造
    // （create_version 复制记录仍用 name/code）保持不变。
    #[serde(alias = "process_name")]
    pub name: String,
    #[serde(alias = "process_key")]
    pub code: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub version: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateProcessDefinitionRequest {
    #[serde(alias = "process_name")]
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub config: Option<serde_json::Value>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProcessDefinitionQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateVersionRequest {
    pub version: String,
    pub config: Option<serde_json::Value>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateBpmTemplateRequest {
    pub template_name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TemplateQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StartProcessRequest {
    pub process_key: String,
    pub business_type: String,
    pub business_id: i64,
    pub title: String,
    pub initiator_id: i32,
    pub initiator_name: String,
    pub initiator_department_id: Option<i32>,
    pub priority: Option<String>,
    pub form_data: Option<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StartProcessResponse {
    pub instance_id: i32,
    pub instance_no: String,
    pub task_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApproveTaskRequest {
    pub task_id: i32,
    pub handler_id: i32,
    pub handler_name: String,
    pub action: String, // "approve", "reject"
    pub approval_opinion: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    pub user_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 撤回流程实例请求 DTO（批次 157d-3 新增）
#[derive(Debug, Deserialize, Serialize)]
pub struct CancelInstanceRequest {
    /// 撤回原因（选填）
    pub cancel_reason: Option<String>,
}
