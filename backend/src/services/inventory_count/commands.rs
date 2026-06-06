//! 库存盘点增删改（create_count, update_count, delete_count）
//!
//! 占位实现，真实业务逻辑请使用 `crate::services::inventory_count::commands::*` 子模块。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::{
    CreateInventoryCountRequest, InventoryCountDetail, UpdateInventoryCountRequest,
};
use crate::utils::error::AppError;

/// 创建库存盘点（占位）
pub async fn create_count(
    _db: Arc<DatabaseConnection>,
    _req: CreateInventoryCountRequest,
    _operator: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 更新库存盘点（占位）
pub async fn update_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _req: UpdateInventoryCountRequest,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 删除库存盘点（占位）
pub async fn delete_count(_db: Arc<DatabaseConnection>, _count_id: i32) -> Result<(), AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}
