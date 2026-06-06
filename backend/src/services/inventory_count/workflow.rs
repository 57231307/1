//! 库存盘点工作流（approve_count, complete_count）
//!
//! 占位实现，真实业务逻辑请使用 `crate::services::inventory_count::workflow::*` 子模块。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use super::types::InventoryCountDetail;
use crate::utils::error::AppError;

/// 审核库存盘点（占位）
pub async fn approve_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
    _approver: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}

/// 完成库存盘点（占位）
pub async fn complete_count(
    _db: Arc<DatabaseConnection>,
    _count_id: i32,
) -> Result<InventoryCountDetail, AppError> {
    Err(AppError::NotImplemented(
        "该功能正在开发中，请等待 inventory_count 子模块完整实现".to_string(),
    ))
}
