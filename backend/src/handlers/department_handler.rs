use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use serde::Deserialize;
use validator::Validate;

use crate::models::department;
use crate::services::department_service::DepartmentService;
use crate::services::department_service::DepartmentTreeNode;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 部门列表
#[derive(Debug, Deserialize, Validate)]
pub struct DepartmentListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建部门请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDepartmentRequest {
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// 更新部门请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDepartmentRequest {
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
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
) -> Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>, AppError> {
    let department_service = DepartmentService::new(state.db.clone());
    let tree = department_service.get_department_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}
