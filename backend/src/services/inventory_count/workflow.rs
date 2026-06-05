//! 库存盘点工作流（approve_count, complete_count）
//!
//! 实际实现位于 InventoryCountService

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::InventoryCountDetail;
use crate::utils::error::AppError;

pub async fn approve_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _approver: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "请使用 InventoryCountService::approve_count".to_string(),
    ))
}

pub async fn complete_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "请使用 InventoryCountService::complete_count".to_string(),
    ))
}
