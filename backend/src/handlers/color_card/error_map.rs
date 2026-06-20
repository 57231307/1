//! 色卡 Handler 错误转换辅助
//!
//! 任务编号: P14 批 2 I-3 第 9 批（拆分原 handlers/color_card_handler.rs）
//! 把 CrudError / ItemError / BorrowError 转 AppError 集中到一处
//! 行为完全保持一致（仅结构重构）

use crate::services::color_card_borrow_service::BorrowError;
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

/// 借出错误转 AppError
pub fn borrow_err(e: BorrowError) -> AppError {
    match e {
        BorrowError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        BorrowError::RecordNotFound => AppError::not_found("借出记录不存在"),
        BorrowError::InvalidState(msg) => AppError::business(msg),
        BorrowError::Validation(msg) => AppError::validation(msg),
        BorrowError::Database(e) => AppError::database(e.to_string()),
    }
}
