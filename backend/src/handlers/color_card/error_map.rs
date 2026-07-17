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

#[cfg(test)]
mod tests {
    //! 色卡错误映射单元测试（批次 394 补测，V15 P0-F03 删除 borrow_err 测试）
    //!
    //! 覆盖目标：
    //! - crud_err 4 个变体的错误映射
    //! - item_err 5 个变体的错误映射

    use super::*;

    /// 测试_crud_err_not_found映射
    #[test]
    fn 测试_crud_err_not_found映射() {
        let err = crud_err(CrudError::NotFound);
        let msg = err.to_string();
        assert!(
            msg.contains("色卡不存在"),
            "NotFound 应映射为'色卡不存在'，实际：{}",
            msg
        );
    }

    /// 测试_crud_err_invalid_state映射
    #[test]
    fn 测试_crud_err_invalid_state映射() {
        let err = crud_err(CrudError::InvalidState);
        let msg = err.to_string();
        assert!(
            msg.contains("当前状态不允许此操作"),
            "InvalidState 应映射为'当前状态不允许此操作'，实际：{}",
            msg
        );
    }

    /// 测试_crud_err_validation映射
    #[test]
    fn 测试_crud_err_validation映射() {
        let err = crud_err(CrudError::Validation("字段不能为空".to_string()));
        let msg = err.to_string();
        assert!(
            msg.contains("字段不能为空"),
            "Validation 应透传原始消息，实际：{}",
            msg
        );
    }

    /// 测试_crud_err_database映射
    #[test]
    fn 测试_crud_err_database映射() {
        let db_err = sea_orm::DbErr::Custom("连接超时".to_string());
        let err = crud_err(CrudError::Database(db_err));
        let msg = err.to_string();
        assert!(
            msg.contains("连接超时"),
            "Database 应包含原始错误描述，实际：{}",
            msg
        );
    }

    /// 测试_item_err所有变体映射
    #[test]
    fn 测试_item_err所有变体映射() {
        let msg = item_err(ItemError::ColorCardNotFound).to_string();
        assert!(msg.contains("色卡不存在"), "ColorCardNotFound 映射错误：{}", msg);

        let msg = item_err(ItemError::ItemNotFound).to_string();
        assert!(msg.contains("色号不存在"), "ItemNotFound 映射错误：{}", msg);

        let msg = item_err(ItemError::InvalidState).to_string();
        assert!(
            msg.contains("当前色卡状态不允许此操作"),
            "InvalidState 映射错误：{}",
            msg
        );

        let msg = item_err(ItemError::Validation("色号重复".to_string())).to_string();
        assert!(msg.contains("色号重复"), "Validation 映射错误：{}", msg);

        let msg = item_err(ItemError::Database(sea_orm::DbErr::Custom("锁超时".to_string()))).to_string();
        assert!(msg.contains("锁超时"), "Database 映射错误：{}", msg);
    }
}
