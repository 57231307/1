use crate::services::api::ApiService;

/// 部门数据模型
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

/// 部门列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DepartmentListResponse {
    pub departments: Vec<Department>,
    pub total: u64,
}

/// 创建部门请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub code: String,
    pub parent_id: Option<i32>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}

/// 更新部门请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<i32>,
    pub manager: Option<String>,
    pub phone: Option<String>,
}

/// 部门服务
pub struct DepartmentService;

impl DepartmentService {
    pub async fn list_departments() -> Result<DepartmentListResponse, String> {
        ApiService::get::<DepartmentListResponse>("/api/v1/erp/departments").await
    }

    pub async fn get_department(id: i32) -> Result<Department, String> {
        ApiService::get::<Department>(&format!("/api/v1/erp/departments/{}", id)).await
    }

    pub async fn create_department(req: CreateDepartmentRequest) -> Result<Department, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/departments", &payload).await
    }

    pub async fn update_department(id: i32, req: UpdateDepartmentRequest) -> Result<Department, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/api/v1/erp/departments/{}", id), &payload).await
    }

    pub async fn delete_department(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/departments/{}", id)).await
    }

    /// 获取部门树形结构
    pub async fn get_department_tree() -> Result<Vec<serde_json::Value>, String> {
        ApiService::get::<Vec<serde_json::Value>>("/api/v1/erp/departments/tree").await
    }
}
