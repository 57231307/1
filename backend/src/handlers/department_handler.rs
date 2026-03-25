use axum::{
    extract::{Path, Query, State},
    Json,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use serde::Deserialize;

use crate::models::department;
use crate::services::department_service::DepartmentService;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::error::AppError;
use crate::services::department_service::DepartmentTreeNode;

/// 查询参数 - 部门列表
#[derive(Debug, Deserialize)]
pub struct DepartmentListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建部门请求
#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// 更新部门请求
#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

/// 获取部门列表
pub async fn list_departments(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<DepartmentListQuery>,
) -> Result<Json<ApiResponse<Vec<department::Model>>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(10);

    let department_service = DepartmentService::new(db.clone());
    let (departments, total) = department_service.list_departments(
        page,
        page_size,
        query.parent_id,
        query.search,
    ).await?;

    Ok(Json(PaginatedResponse::new(departments, total, page, page_size).into()))
}

/// 获取部门详情
pub async fn get_department(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<department::Model>>, AppError> {
    let department_service = DepartmentService::new(db.clone());
    let department = department_service.get_department(id).await?;
    Ok(Json(ApiResponse::success(department)))
}

/// 创建部门
pub async fn create_department(
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<Json<ApiResponse<department::Model>>, AppError> {
    let department_service = DepartmentService::new(db.clone());
    let department = department_service.create_department(
        req.name,
        req.description,
        req.parent_id,
    ).await?;
    Ok(Json(ApiResponse::success_with_msg(department, "部门创建成功")))
}

/// 更新部门
pub async fn update_department(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateDepartmentRequest>,
) -> Result<Json<ApiResponse<department::Model>>, AppError> {
    let department_service = DepartmentService::new(db.clone());
    let department = department_service.update_department(
        id,
        req.name,
        req.description,
        req.parent_id,
    ).await?;
    Ok(Json(ApiResponse::success_with_msg(department, "部门更新成功")))
}

/// 删除部门
pub async fn delete_department(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let department_service = DepartmentService::new(db.clone());
    department_service.delete_department(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "部门删除成功")))
}

/// 获取部门树形结构
pub async fn get_department_tree(
    State(db): State<Arc<DatabaseConnection>>,
) -> Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>, AppError> {
    let department_service = DepartmentService::new(db.clone());
    let tree = department_service.get_department_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}