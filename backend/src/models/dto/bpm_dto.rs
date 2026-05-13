use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct StartProcessRequest {
    pub process_key: String,
    pub business_type: String,
    pub business_id: i32,
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
    pub user_id: i32,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
