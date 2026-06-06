//! 库存盘点增删改（create_count, update_count, delete_count）
//!
//! 占位实现，真实代码在 InventoryCountService 中

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::{CreateInventoryCountRequest, InventoryCountDetail, UpdateInventoryCountRequest};
use crate::utils::error::AppError;

pub async fn create_count(
    _db: Arc<DatabaseConnection>,
    _req: CreateInventoryCountRequest,
    _operator: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::create_count".to_string(),
    ))
}

pub async fn update_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _req: UpdateInventoryCountRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::update_count".to_string(),
    ))
}

pub async fn delete_count(_db: Arc<DatabaseConnection>, _count_id: i32) -> Result<(), AppError> {
    Err(AppError::BusinessError(
        "请使用 InventoryCountService::delete_count".to_string(),
    ))
}
