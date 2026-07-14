//! 通用输入校验工具模块
//!
//! 批次 98 P2-B 修复（v5 复审）：抽取 handler 中重复的 `validate_amount_range` 到统一模块，
//! 并追加 `round_dp(2)` 精度校验，避免 Decimal 字段从 String parse 后精度溢出。

use rust_decimal::Decimal;
use validator::ValidationError;

/// 金额范围 + 精度校验
///
/// - 范围：(0, 10 亿]，金额必须为正且不超过 10 亿
/// - 精度：`round_dp(2)`，金额最多 2 位小数（货币精度规范）
///
/// 用于 `validator::Validate` 派生宏的 `#[validate(custom(function = "crate::utils::validator::validate_amount_range"))]`。
///
/// # 错误
///
/// - `"金额必须为正且不超过10亿"`：金额 <= 0 或 > 10 亿
/// - `"金额精度不能超过2位小数"`：金额小数位 > 2
pub fn validate_amount_range(amount: &Decimal) -> Result<(), ValidationError> {
    let zero = Decimal::ZERO;
    let max = Decimal::new(1_000_000_000, 0); // 10 亿

    if *amount <= zero || *amount > max {
        return Err(ValidationError::new("金额必须为正且不超过10亿"));
    }

    // 批次 98 P2-B 修复（v5 复审）：精度校验，金额最多 2 位小数
    // 防止 Decimal 字段从 String parse 后小数位超长（如 "1.234567"）导致 DB 存储精度漂移
    if amount.round_dp(2) != *amount {
        return Err(ValidationError::new("金额精度不能超过2位小数"));
    }

    Ok(())
}

/// 金额范围 + 精度校验（Option 版本）
///
/// 批次 414 技术债务修复：为 `Option<Decimal>` 字段提供的校验函数。
/// - `None`：直接通过（表示"未提供"，由 service 层处理语义）
/// - `Some(amount)`：委托给 `validate_amount_range` 校验
///
/// 用于 `validator::Validate` 派生宏的 `#[validate(custom(function = "crate::utils::validator::validate_amount_range_opt"))]`。
pub fn validate_amount_range_opt(amount: &Option<Decimal>) -> Result<(), ValidationError> {
    match amount {
        None => Ok(()),
        Some(v) => validate_amount_range(v),
    }
}
