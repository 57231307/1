use crate::models::department::{
    CreateDepartmentRequest, Department, DepartmentListResponse, UpdateDepartmentRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct DepartmentService;

impl CrudService for DepartmentService {
    type Model = Department;
    type ListResponse = DepartmentListResponse;
    type CreateRequest = CreateDepartmentRequest;
    type UpdateRequest = UpdateDepartmentRequest;

    fn base_path() -> &'static str {
        "/departments"
    }
}

// 保留特定业务方法
impl DepartmentService {
    pub async fn get_department_tree() -> Result<Vec<serde_json::Value>, String> {
        ApiService::get::<Vec<serde_json::Value>>("/departments/tree").await
    }
}
