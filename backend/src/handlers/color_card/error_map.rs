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

#[cfg(test)]
mod tests {
    //! 色卡错误映射单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - crud_err 4 个变体的错误映射
    //! - item_err 5 个变体的错误映射
    //! - borrow_err 5 个变体的错误映射

    use super::*;

    /// 测试_crud_err_not_found映射
    ///
    /// CrudError::NotFound → AppError::not_found，消息含"色卡不存在"
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
    ///
    /// CrudError::InvalidState → AppError::business，消息含"当前状态不允许此操作"
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
    ///
    /// CrudError::Validation(msg) → AppError::validation，消息透传原始内容
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
    ///
    /// CrudError::Database(DbErr) → AppError::database，消息含数据库错误描述
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
    ///
    /// 覆盖 ItemError 5 个变体的错误映射
    #[test]
    fn 测试_item_err所有变体映射() {
        // ColorCardNotFound → 色卡不存在
        let msg = item_err(ItemError::ColorCardNotFound).to_string();
        assert!(msg.contains("色卡不存在"), "ColorCardNotFound 映射错误：{}", msg);

        // ItemNotFound → 色号不存在
        let msg = item_err(ItemError::ItemNotFound).to_string();
        assert!(msg.contains("色号不存在"), "ItemNotFound 映射错误：{}", msg);

        // InvalidState → 当前色卡状态不允许此操作
        let msg = item_err(ItemError::InvalidState).to_string();
        assert!(
            msg.contains("当前色卡状态不允许此操作"),
            "InvalidState 映射错误：{}",
            msg
        );

        // Validation → 透传消息
        let msg = item_err(ItemError::Validation("色号重复".to_string())).to_string();
        assert!(msg.contains("色号重复"), "Validation 映射错误：{}", msg);

        // Database → 包含原始错误
        let msg = item_err(ItemError::Database(sea_orm::DbErr::Custom("锁超时".to_string()))).to_string();
        assert!(msg.contains("锁超时"), "Database 映射错误：{}", msg);
    }

    /// 测试_borrow_err所有变体映射
    ///
    /// 覆盖 BorrowError 5 个变体的错误映射
    #[test]
    fn 测试_borrow_err所有变体映射() {
        // ColorCardNotFound → 色卡不存在
        let msg = borrow_err(BorrowError::ColorCardNotFound).to_string();
        assert!(msg.contains("色卡不存在"), "ColorCardNotFound 映射错误：{}", msg);

        // RecordNotFound → 借出记录不存在
        let msg = borrow_err(BorrowError::RecordNotFound).to_string();
        assert!(
            msg.contains("借出记录不存在"),
            "RecordNotFound 映射错误：{}",
            msg
        );

        // InvalidState(msg) → 透传消息
        let msg = borrow_err(BorrowError::InvalidState("已归还不可重复操作".to_string())).to_string();
        assert!(
            msg.contains("已归还不可重复操作"),
            "InvalidState 映射错误：{}",
            msg
        );

        // Validation(msg) → 透传消息
        let msg = borrow_err(BorrowError::Validation("借出数量无效".to_string())).to_string();
        assert!(msg.contains("借出数量无效"), "Validation 映射错误：{}", msg);

        // Database → 包含原始错误
        let msg = borrow_err(BorrowError::Database(sea_orm::DbErr::Custom("死锁".to_string()))).to_string();
        assert!(msg.contains("死锁"), "Database 映射错误：{}", msg);
    }
}
