//! 色卡仓储管理 - 借出 DTO
//!
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 借出色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct BorrowColorCardDto {
    /// 色卡 ID
    pub color_card_id: i64,

    /// 借出客户 ID
    pub customer_id: i64,

    /// 经办员工 ID（默认当前用户）
    pub borrowed_by: Option<i64>,

    /// 预计归还时间
    pub expected_return_at: Option<DateTime<Utc>>,

    /// 用途
    pub purpose: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 归还色卡请求 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ReturnColorCardDto {
    /// 实际归还时间（默认 now）
    pub actual_return_at: Option<DateTime<Utc>>,

    /// 备注
    pub notes: Option<String>,
}

/// 登记遗失请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct MarkLostColorCardDto {
    /// 赔付金额（必填 > 0，Decimal 自定义校验：range 验证器不支持 Decimal）
    #[validate(custom(function = "validate_decimal_positive", message = "赔付金额必须大于 0"))]
    pub compensation_amount: Decimal,

    /// 遗失原因
    pub notes: Option<String>,
}

/// 自定义校验：Decimal 必须 > 0
fn validate_decimal_positive(v: &Decimal) -> Result<(), validator::ValidationError> {
    if *v > Decimal::ZERO {
        Ok(())
    } else {
        Err(validator::ValidationError::new("decimal_positive"))
    }
}

/// 标记损坏请求 DTO
#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct MarkDamagedColorCardDto {
    /// 赔付金额（损坏也可能产生赔付，可选）
    pub compensation_amount: Option<Decimal>,

    /// 损坏原因
    pub notes: Option<String>,
}

/// 借出历史查询参数
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListBorrowRecordsQuery {
    pub color_card_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    /// 起始时间
    pub from_date: Option<DateTime<Utc>>,
    /// 结束时间
    pub to_date: Option<DateTime<Utc>>,
}

/// 色卡列表查询参数
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListColorCardsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub card_type: Option<String>,
    pub season: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}
