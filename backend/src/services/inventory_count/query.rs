//! 库存盘点查询（list_counts、get_count_detail、list_items）
//!
//! 这些方法通过 InventoryCountService::list_counts 等公开 API 访问
//! 实现位于父模块 InventoryCountService（保持向后兼容）

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::{InventoryCountDetail, InventoryCountItemDetail};
use crate::models::dto::PageRequest;
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

/// 列表查询参数
#[derive(Debug, Clone, Default)]
pub struct CountListQuery {
    pub page: PageRequest,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub keyword: Option<String>,
    pub start_date: Option<chrono::DateTime<chrono::Utc>>,
    pub end_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// 列表查询实现（占位）
/// 实际实现仍在 InventoryCountService::list_counts 中
pub async fn list_counts(
    _db: Arc<DatabaseConnection>,
    _query: CountListQuery,
) -> Result<PaginatedResponse<InventoryCountDetail>, AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::list_counts".to_string(),
    ))
}

/// 详情查询实现（占位）
pub async fn get_count_detail(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::get_count_detail".to_string(),
    ))
}

/// 明细项列表（占位）
pub async fn list_items(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<Vec<InventoryCountItemDetail>, AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::list_items".to_string(),
    ))
}
