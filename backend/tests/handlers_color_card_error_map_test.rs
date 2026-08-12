#[cfg(test)]
mod tests {
    //! 色卡错误映射单元测试（批次 394 补测，V15 P0-F03 删除 borrow_err 测试）
    //!
    //! 覆盖目标：
    //! - crud_err 4 个变体的错误映射
    //! - item_err 5 个变体的错误映射
    use bingxi_backend::handlers::color_card::error_map::*;
    use bingxi_backend::services::color_card_item_service::*;
    use bingxi_backend::services::custom_order_crud_service::*;


    /// test_crud_err_not_foundys
    #[test]
    fn test_crud_err_not_foundys() {
        let err = crud_err(CrudError::NotFound);
        let msg = err.to_string();
        assert!(
            msg.contains("色卡不存在"),
            "NotFound 应映射为'色卡不存在'，实际：{}",
            msg
        );
    }

    /// test_crud_err_invalid_stateys
    #[test]
    fn test_crud_err_invalid_stateys() {
        let err = crud_err(CrudError::InvalidState);
        let msg = err.to_string();
        assert!(
            msg.contains("当前状态不允许此操作"),
            "InvalidState 应映射为'当前状态不允许此操作'，实际：{}",
            msg
        );
    }

    /// test_crud_err_validationys
    #[test]
    fn test_crud_err_validationys() {
        let err = crud_err(CrudError::Validation("字段不能为空".to_string()));
        let msg = err.to_string();
        assert!(
            msg.contains("字段不能为空"),
            "Validation 应透传原始消息，实际：{}",
            msg
        );
    }

    /// test_crud_err_databaseys
    #[test]
    fn test_crud_err_databaseys() {
        let db_err = sea_orm::DbErr::Custom("连接超时".to_string());
        let err = crud_err(CrudError::Database(db_err));
        let msg = err.to_string();
        assert!(
            msg.contains("连接超时"),
            "Database 应包含原始错误描述，实际：{}",
            msg
        );
    }

    /// test_item_errsybtys
    #[test]
    fn test_item_errsybtys() {
        let msg = item_err(ItemError::ColorCardNotFound).to_string();
        assert!(
            msg.contains("色卡不存在"),
            "ColorCardNotFound 映射错误：{}",
            msg
        );

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

        let msg = item_err(ItemError::Database(sea_orm::DbErr::Custom(
            "锁超时".to_string(),
        )))
        .to_string();
        assert!(msg.contains("锁超时"), "Database 映射错误：{}", msg);
    }
}