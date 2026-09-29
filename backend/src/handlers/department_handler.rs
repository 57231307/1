use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use axum::{Json, extract::State};
use serde::Deserialize;
use validator::Validate;

use crate::services::department_service::DepartmentService;
use crate::services::department_service::DepartmentTreeNode;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数 - 部门列表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct DepartmentListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建部门请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDepartmentRequest {
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: String,
    /// 部门编码（契约对齐：前端 DepartmentCreateRequest.code；不传时后端自动生成 DEPT_时间戳）
    #[validate(length(max = 50, message = "部门编码最长50字符"))]
    pub code: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub manager_id: Option<i32>,
    /// 排序号（契约对齐：前端 DepartmentCreateRequest.sort_order；不传时默认 0）
    pub sort_order: Option<i32>,
}

/// 更新部门请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDepartmentRequest {
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub manager_id: Option<i32>,
    /// 排序号（契约对齐：前端 DepartmentUpdateRequest.sort_order）
    pub sort_order: Option<i32>,
    #[serde(alias = "status", alias = "is_active")]
    pub is_active: Option<bool>,
}

crate::define_crud_handlers!(
    DepartmentService,
    CreateDepartmentRequest,
    UpdateDepartmentRequest,
    DepartmentListQuery,
    i32
);

/// 获取部门树形结构 (定制化额外路由)
pub async fn get_department_tree(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>, AppError> {
    let department_service = DepartmentService::new(state.db.clone());
    let tree = department_service.get_department_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}
