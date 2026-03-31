use crate::models::department::{
    CreateDepartmentRequest, Department, DepartmentListResponse, UpdateDepartmentRequest,
};
use crate::services::api::ApiService;

pub struct DepartmentService;

impl DepartmentService {
    pub async fn list_departments() -> Result<DepartmentListResponse, String> {
        ApiService::get::<DepartmentListResponse>("/departments").await
    }

    pub async fn get_department(id: i32) -> Result<Department, String> {
        ApiService::get::<Department>(&format!("/departments/{}", id)).await
    }

    pub async fn create_department(req: CreateDepartmentRequest) -> Result<Department, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/departments", &payload).await
    }

    pub async fn update_department(id: i32, req: UpdateDepartmentRequest) -> Result<Department, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/departments/{}", id), &payload).await
    }

    pub async fn delete_department(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/departments/{}", id)).await
    }

    pub async fn get_department_tree() -> Result<Vec<serde_json::Value>, String> {
        ApiService::get::<Vec<serde_json::Value>>("/departments/tree").await
    }
}
