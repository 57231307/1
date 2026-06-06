//! 库存盘点查询（list_counts、get_count_detail、list_items）
//!
//! 这些方法为占位实现，真实业务逻辑应通过 `crate::services::inventory_count::*` 子模块调用。
//! 当前模块结构：
//! - `query`     — 列表查询与详情查询
//! - `commands`  — 增删改操作
//! - `workflow`  — 审批与完成
//! - `items`     — 明细项管理

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
///
/// 真实实现请使用 `crate::services::inventory_count::query::list_counts`
pub async fn list_counts(
    _db: Arc<DatabaseConnection>,
    _query: CountListQuery,
) -> Result<PaginatedResponse<InventoryCountDetail>, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 详情查询实现（占位）
pub async fn get_count_detail(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 明细项列表（占位）
pub async fn list_items(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<Vec<InventoryCountItemDetail>, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}
