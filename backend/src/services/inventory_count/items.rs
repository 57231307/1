//! 库存盘点明细项管理（add_item, update_item, delete_item）
//!
//! 占位实现，真实业务逻辑请使用 `crate::services::inventory_count::items::*` 子模块。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::{InventoryCountDetail, InventoryCountItemRequest};
use crate::utils::error::AppError;

/// 添加明细项（占位）
pub async fn add_item(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _req: InventoryCountItemRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 更新明细项（占位）
pub async fn update_item(
    _db: Arc<DatabaseConnection>,
    _item_id: i32,
    _req: InventoryCountItemRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 删除明细项（占位）
pub async fn delete_item(_db: Arc<DatabaseConnection>, _item_id: i32) -> Result<(), AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}
