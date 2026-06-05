//! 库存盘点明细项管理（add_item, update_item, delete_item）

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::{InventoryCountDetail, InventoryCountItemRequest};
use crate::utils::error::AppError;

pub async fn add_item(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _req: InventoryCountItemRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "请使用 InventoryCountService::add_item".to_string(),
    ))
}

pub async fn update_item(
    _db: Arc<DatabaseConnection>,
    _item_id: i32,
    _req: InventoryCountItemRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "请使用 InventoryCountService::update_item".to_string(),
    ))
}

pub async fn delete_item(_db: Arc<DatabaseConnection>, _item_id: i32) -> Result<(), AppError> {
    Err(AppError::NotImplemented(
        "请使用 InventoryCountService::delete_item".to_string(),
    ))
}
