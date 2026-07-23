//! BI 分析 DTO 子模块（bi_analysis_ops/types）
//!
//! 批次 490 D10-3a 拆分：从原 `bi_analysis_service.rs` L24-230 迁移。
//! 包含 8 个对外 response struct（BiResponse + 7 个业务 DTO）+ 13 个 FromQueryResult 中间结构
//! + 1 个内部传递结构（KpiCurrentMetrics）。

use rust_decimal::Decimal;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

// ============================================================================
// 通用响应包装
// ============================================================================

/// 通用响应包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> BiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data,
        }
    }
}

// ============================================================================
// 对外 Response DTO
// ============================================================================

/// 时间序列点
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeSeriesPoint {
    /// 周期标识（YYYY-MM-DD / YYYY-MM / YYYY-Q1 / YYYY）
    pub period: String,
    /// 销售额
    pub total_amount: f64,
    /// 订单数
    pub order_count: i64,
    /// 销售数量
    pub quantity: f64,
    /// 利润
    pub profit_amount: f64,
}

/// 客户排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomerRank {
    pub customer_id: i64,
    pub customer_name: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub percentage: f64,
}

/// 产品排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductRank {
    pub product_id: i64,
    pub product_name: String,
    pub product_code: String,
    pub category: String,
    pub total_amount: f64,
    pub quantity: f64,
    pub order_count: i64,
}

/// 区域统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionStat {
    pub region: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub customer_count: i64,
}

/// 品类统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStat {
    pub category: String,
    pub total_amount: f64,
    pub percentage: f64,
}

/// 利润分析
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfitAnalysis {
    pub total_revenue: f64,
    pub total_cost: f64,
    pub total_profit: f64,
    pub gross_margin: f64,
    pub order_count: i64,
    pub avg_order_value: f64,
}

/// KPI 概览
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KpiSummary {
    /// 总销售额
    pub total_sales: f64,
    /// 订单数
    pub order_count: i64,
    /// 客户数
    pub customer_count: i64,
    /// 客单价
    pub avg_order_value: f64,
    /// 同比增长率（与上一年同期）
    pub yoy_growth: f64,
    /// 环比增长率（与上月）
    pub mom_growth: f64,
}

// ============================================================================
// FromQueryResult 中间结构（仅 ops 子模块内部使用，不对外 re-export）
// ============================================================================

#[derive(Debug, FromQueryResult)]
pub(crate) struct TimeSeriesRow {
    pub(crate) period: String,
    pub(crate) total_amount: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
    pub(crate) quantity: Option<Decimal>,
    pub(crate) profit_amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct CustomerRankRow {
    pub(crate) customer_id: i32,
    pub(crate) customer_name: String,
    pub(crate) total_amount: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct ProductRankRow {
    pub(crate) product_id: i32,
    pub(crate) product_name: String,
    pub(crate) product_code: String,
    pub(crate) category: Option<String>,
    pub(crate) total_amount: Option<Decimal>,
    pub(crate) quantity: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct RegionStatRow {
    pub(crate) region: String,
    pub(crate) total_amount: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
    pub(crate) customer_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct CategoryStatRow {
    pub(crate) category: String,
    pub(crate) total_amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct ProfitRow {
    pub(crate) total_revenue: Option<Decimal>,
    pub(crate) total_cost: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct KpiRow {
    pub(crate) total_sales: Option<Decimal>,
    pub(crate) order_count: Option<i64>,
    pub(crate) customer_count: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct CustomerOrderRow {
    pub(crate) order_id: i32,
    pub(crate) amount: Option<Decimal>,
    pub(crate) order_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct ProductOrderRow {
    pub(crate) order_id: i32,
    pub(crate) quantity: Option<Decimal>,
    pub(crate) amount: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct TotalRow {
    pub(crate) total: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct YoYRow {
    pub(crate) this_year: Option<Decimal>,
    pub(crate) last_year: Option<Decimal>,
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct MoMRow {
    pub(crate) this_month: Option<Decimal>,
    pub(crate) last_month: Option<Decimal>,
}

/// KPI 当前周期指标（内部传递用）
pub(crate) struct KpiCurrentMetrics {
    pub(crate) total_sales: f64,
    pub(crate) order_count: i64,
    pub(crate) customer_count: i64,
    pub(crate) avg_order_value: f64,
}

/// 透视矩阵行（v11 批次 144 P1-3：动态 SQL 透视矩阵）
#[derive(Debug, FromQueryResult)]
pub(crate) struct PivotRow {
    pub(crate) row_key: Option<String>,
    pub(crate) row_label: Option<String>,
    pub(crate) col_key: Option<String>,
    pub(crate) col_label: Option<String>,
    pub(crate) measure_value: Option<Decimal>,
}
