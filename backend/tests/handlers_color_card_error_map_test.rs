//! 色卡错误映射单元测试（crud_err / item_err 各变体 → AppError 族与出参脱敏口径）
//!
//! 覆盖目标：
//! - crud_err 5 个变体的错误映射
//! - item_err 5 个变体的错误映射
use bingxi_backend::handlers::color_card::error_map::*;
use bingxi_backend::services::color_card_crud_service::*;
use bingxi_backend::services::color_card_item_service::*;
use bingxi_backend::utils::error::AppError;

/// 审计写入失败不得再冒充"用户输入不合法"：必须是 DATABASE_ERROR（500）且真实原因只进日志
#[test]
fn test_crud_err_audit_log_maps_to_database_not_validation() {
    let err = crud_err(CrudError::AuditLog(
        "update_with_audit failed: connection reset by peer".to_string(),
    ));
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "审计写入失败属服务端持久化缺陷，族必须是 DATABASE_ERROR"
    );
    assert!(
        err.to_string().contains("connection reset"),
        "Display（日志侧）必须保留真实原因，实际={}",
        err
    );
    // DatabaseError 的出参恒为脱敏常量 err_msg::DB_ERROR_PUBLIC（utils/messages.rs:40，
    // 取值「数据库错误」；「数据库操作失败」是 DB_OP_FAIL:86，只作 From<DbErr> 兜底分类的
    // **内部**文案，不是出参），断错值会让本用例恒红。
    assert_eq!(
        err.to_response().message,
        "数据库错误",
        "出参不得把内部错误原文推给用户"
    );
}

/// test_crud_err_not_foundys
/// NotFound → NOT_FOUND / HTTP 404，出参为脱敏常量 err_msg::NOT_FOUND_PUBLIC（messages.rs:42）
#[test]
fn test_crud_err_not_foundys() {
    let err = crud_err(CrudError::NotFound);
    let msg = err.to_string();
    assert!(
        msg.contains("色卡不存在"),
        "NotFound 应映射为'色卡不存在'，实际：{}",
        msg
    );
    assert_eq!(err.error_code(), "NOT_FOUND");
    assert_eq!(err.to_response().message, "资源未找到");
}

/// test_crud_err_invalid_stateys
/// InvalidState 是**状态门**（非提交字段校验）⇒ 归 BUSINESS_ERROR 族；
/// 用脱敏 `business` 构造 ⇒ 出参恒为 err_msg::BUSINESS_PUBLIC（messages.rs:43），
/// 真实判定依据只进日志（error.rs:34「不确定的一律用 business」）。
#[test]
fn test_crud_err_invalid_stateys() {
    let err = crud_err(CrudError::InvalidState);
    let msg = err.to_string();
    assert!(
        msg.contains("当前状态不允许此操作"),
        "InvalidState 应映射为'当前状态不允许此操作'，实际：{}",
        msg
    );
    assert!(matches!(err, AppError::BusinessError(_)));
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().message, "业务处理失败");
}

/// test_crud_err_validationys
/// `CrudError::Validation` 通道承载提交字段取值/格式/必填，映射点为
/// `validation_displayable`（handlers/color_card/error_map.rs:18）——族仍是 VALIDATION_ERROR，
/// 出参 message 外显真实原因（与同域 ItemError::Validation 的口径一致）。
#[test]
fn test_crud_err_validationys() {
    let err = crud_err(CrudError::Validation("字段不能为空".to_string()));
    let msg = err.to_string();
    assert!(
        msg.contains("字段不能为空"),
        "Validation 应透传原始消息，实际：{}",
        msg
    );
    assert!(
        matches!(err, AppError::ValidationErrorDisplayable(_)),
        "提交字段校验必须归校验族且可外显，实际={err:?}"
    );
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert_eq!(
        err.to_response().message,
        "字段不能为空",
        "出参必须外显真实校验原因，不得回潮脱敏常量"
    );
}

/// test_crud_err_databaseys
/// Database → DATABASE_ERROR（脱敏出参「数据库错误」），Display 侧保留原始错误供日志
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
    assert_eq!(err.error_code(), "DATABASE_ERROR");
    assert_eq!(err.to_response().message, "数据库错误");
}

/// test_item_errsybtys
/// 逐变体钉「族 + 出参文案」：NotFound 类 → NOT_FOUND（脱敏）；
/// 色号维护状态门（ItemError::InvalidState）→ BUSINESS_ERROR 且**外显**公开业务规则
/// 「只有草稿态色卡可以维护色号」（纯规则、不含内部状态 token/记录 ID，满足
/// business_displayable 安全边界，error_map.rs）；
/// 提交字段校验 → VALIDATION_ERROR 且**外显**真实原因（error_map.rs 用 validation_displayable）。
#[test]
fn test_item_errsybtys() {
    let not_found = item_err(ItemError::ColorCardNotFound);
    let msg = not_found.to_string();
    assert!(
        msg.contains("色卡不存在"),
        "ColorCardNotFound 映射错误：{}",
        msg
    );
    assert_eq!(not_found.error_code(), "NOT_FOUND");
    assert_eq!(not_found.to_response().message, "资源未找到");

    let item_not_found = item_err(ItemError::ItemNotFound);
    let msg = item_not_found.to_string();
    assert!(msg.contains("色号不存在"), "ItemNotFound 映射错误：{}", msg);
    assert_eq!(item_not_found.error_code(), "NOT_FOUND");
    assert_eq!(item_not_found.to_response().message, "资源未找到");

    let state = item_err(ItemError::InvalidState);
    let msg = state.to_string();
    assert!(
        msg.contains("只有草稿态色卡可以维护色号"),
        "InvalidState 映射错误：{}",
        msg
    );
    assert!(
        matches!(state, AppError::BusinessErrorDisplayable(_)),
        "色号维护状态门是公开业务规则，必须归 business 族且可外显，实际={state:?}"
    );
    assert_eq!(state.error_code(), "BUSINESS_ERROR");
    // 状态门拒绝原因必须随出参外显（不得回潮成脱敏常量"业务处理失败"）
    assert_eq!(state.to_response().message, "只有草稿态色卡可以维护色号");
    // 外显文案不得泄露内部状态 token（'draft' 等小写词表值不得出现在出参 message）
    assert!(
        !state.to_response().message.contains("draft")
            && !state.to_response().message.contains("active"),
        "出参文案只描述公开规则，不得回显内部状态 token：{}",
        state.to_response().message
    );

    let validation = item_err(ItemError::Validation("色号重复".to_string()));
    let msg = validation.to_string();
    assert!(msg.contains("色号重复"), "Validation 映射错误：{}", msg);
    assert!(matches!(
        validation,
        AppError::ValidationErrorDisplayable(_)
    ));
    assert_eq!(validation.error_code(), "VALIDATION_ERROR");
    // displayable 校验族：出参必须携带真实原因（回潮成"请求参数验证失败"即红）
    assert_eq!(validation.to_response().message, "色号重复");

    let db_err = item_err(ItemError::Database(sea_orm::DbErr::Custom(
        "锁超时".to_string(),
    )));
    let msg = db_err.to_string();
    assert!(msg.contains("锁超时"), "Database 映射错误：{}", msg);
    assert_eq!(db_err.error_code(), "DATABASE_ERROR");
    assert_eq!(db_err.to_response().message, "数据库错误");
}
