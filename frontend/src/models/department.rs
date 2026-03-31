//! 部门模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Department {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub manager: Option<String>,
    pub phone: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DepartmentListResponse {
    pub departments: Vec<Department>,
    pub total: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<i32>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}
