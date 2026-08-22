//! 产量工资 DTO 模块
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 纯数据传输对象（DTO），从 wage_service facade 迁移而来，
//! Service struct 与 impl 块保留在 wage_service / wage_ops 子模块。

use rust_decimal::Decimal;
use serde::Deserialize;

// ============================================================================
// 工序工价请求 DTO
// ============================================================================

/// 创建工价请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWageRateRequest {
    pub process_route_id: i32,
    pub wage_type: Option<String>,
    pub piece_price: Option<Decimal>,
    pub time_price: Option<Decimal>,
    pub grade_a_ratio: Option<Decimal>,
    pub grade_b_ratio: Option<Decimal>,
    pub grade_c_ratio: Option<Decimal>,
    pub effective_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工价请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWageRateRequest {
    pub wage_type: Option<String>,
    pub piece_price: Option<Decimal>,
    pub time_price: Option<Decimal>,
    pub grade_a_ratio: Option<Decimal>,
    pub grade_b_ratio: Option<Decimal>,
    pub grade_c_ratio: Option<Decimal>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
}

/// 工价查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct WageRateQuery {
    pub route_code: Option<String>,
    pub process_route_id: Option<i32>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 工资记录请求 DTO
// ============================================================================

/// 创建工资记录请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateWageRecordRequest {
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub workshop: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新工资记录请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWageRecordRequest {
    pub workshop: Option<String>,
    pub remarks: Option<String>,
}

/// 工资计算请求（触发计算）
#[derive(Debug, Clone, Deserialize)]
pub struct CalculateWageRequest {
    /// 重新计算（删除已有明细重新生成）
    pub recalculate: Option<bool>,
}

/// 工资记录查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct WageRecordQuery {
    pub record_no: Option<String>,
    pub workshop: Option<String>,
    pub status: Option<String>,
    pub period_start: Option<chrono::NaiveDate>,
    pub period_end: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
