//! 通用输入校验工具模块
//!
//! 批次 98 P2-B 修复（v5 复审）：抽取 handler 中重复的 `validate_amount_range` 到统一模块，
//! 并追加 `round_dp(2)` 精度校验，避免 Decimal 字段从 String parse 后精度溢出。

use rust_decimal::Decimal;
use validator::ValidationError;

/// 金额范围 + 精度校验（范围 (0, 10亿]，精度 round_dp(2) 最多 2 位小数；用于 Validate 派生宏 custom；错误："金额必须为正且不超过10亿"/"金额精度不能超过2位小数"）
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

/// 信用额度范围 + 精度校验（允许 0；批次 414 为 CreditRatingRequestDto 提供，与 validate_amount_range 区别为允许 0 表示置零暂停信用；范围 [0, 10亿]，精度 round_dp(2)；validator 框架对 Option<T> 自动解包 None 跳过）
pub fn validate_credit_limit_range(amount: &Decimal) -> Result<(), ValidationError> {
    let max = Decimal::new(1_000_000_000, 0); // 10 亿

    if *amount < Decimal::ZERO || *amount > max {
        return Err(ValidationError::new("信用额度不能为负且不超过10亿"));
    }

    if amount.round_dp(2) != *amount {
        return Err(ValidationError::new("信用额度精度不能超过2位小数"));
    }

    Ok(())
}
