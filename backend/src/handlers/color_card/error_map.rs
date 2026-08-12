//! 色卡 Handler 错误转换辅助
//!
//! V15 P0-F03 重构：删除 BorrowError 转换（borrow 模式已废弃）

use crate::services::color_card_crud_service::CrudError;
use crate::services::color_card_item_service::ItemError;
use crate::utils::error::AppError;

/// CRUD 错误转 AppError
pub fn crud_err(e: CrudError) -> AppError {
    match e {
        CrudError::NotFound => AppError::not_found("色卡不存在"),
        CrudError::InvalidState => AppError::business("当前状态不允许此操作"),
        CrudError::Validation(msg) => AppError::validation(msg),
        CrudError::Database(e) => AppError::database(e.to_string()),
    }
}

/// 色号错误转 AppError
pub fn item_err(e: ItemError) -> AppError {
    match e {
        ItemError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        ItemError::ItemNotFound => AppError::not_found("色号不存在"),
        ItemError::InvalidState => AppError::business("当前色卡状态不允许此操作"),
        ItemError::Validation(msg) => AppError::validation(msg),
        ItemError::Database(e) => AppError::database(e.to_string()),
    }
}
