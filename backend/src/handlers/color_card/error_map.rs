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
        // InvalidState 是色卡主体状态门（draft 之外不可改字段），文案为不含内部 token/记录 ID
        // 的固定规则，沿用脱敏 `business`（判据：不确定一律 business）。
        // Validation 承载提交字段取值/格式/必填（色卡类型枚举、HEX 格式、RGB 范围），
        // 用 validation_displayable 按 error.rs 规则外显真实原因。
        CrudError::InvalidState => AppError::business("当前状态不允许此操作"),
        CrudError::Validation(msg) => AppError::validation_displayable(msg),
        CrudError::AuditLog(msg) => AppError::database(msg),
        CrudError::Database(e) => AppError::database(e.to_string()),
    }
}

/// 色号错误转 AppError
pub fn item_err(e: ItemError) -> AppError {
    match e {
        ItemError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        ItemError::ItemNotFound => AppError::not_found("色号不存在"),
        // 色号维护状态门：拒绝依据是"仅草稿态色卡可以维护色号"这一公开业务规则，
        // 文案不含内部状态 token（'draft' 等词表值）与记录 ID ⇒ 满足 business_displayable
        // 安全边界，外显真实原因。状态门失败属用户可自助修正（先把卡留在草稿态再维护色号），
        // 不应降级成人畜无害、无从排查的脱敏"业务处理失败"。
        ItemError::InvalidState => AppError::business_displayable("只有草稿态色卡可以维护色号"),
        ItemError::Validation(msg) => AppError::validation_displayable(msg),
        ItemError::Database(e) => AppError::database(e.to_string()),
    }
}
