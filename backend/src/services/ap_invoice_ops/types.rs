//! 应付单 DTO 定义子模块（ap_invoice_ops/types）
//!
//! 批次 490 D10-4b 拆分：从原 `ap_invoice_service.rs` 迁移 6 个 DTO。
//! 包含 CreateApInvoiceRequest / UpdateApInvoiceRequest（手工创建/更新请求）和
//! AgingAnalysisItem / BalanceSummary / StatusStatItem / ApInvoiceStatistics（报表统计）。
//!
//! 校验纯函数（validate_positive_decimal / validate_non_negative_decimal /
//! validate_exchange_rate）保留在 facade，本模块 DTOs 通过
//! `#[validate(custom(function = "crate::services::ap_invoice_service::validate_*"))]`
//! 全路径引用 facade 的校验函数（与 crate::utils::validator 用法一致）。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建应付单请求
///
/// TS-S-5 安全加固（2026-06-26）：补齐 exchange_rate / amount / currency / notes / attachment_urls 校验，
/// 防止手工传入 0.01 汇率（P0-1）或负数金额。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApInvoiceRequest {
    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 应付类型
    #[validate(length(min = 1, max = 20, message = "发票号码长度必须在1到20个字符之间"))]
    pub invoice_type: Option<String>,

    /// 应付日期
    pub invoice_date: Option<NaiveDate>,

    /// 到期日期
    pub due_date: Option<NaiveDate>,

    /// 账期（天）
    #[validate(range(min = 0, max = 365, message = "账期必须在0到365天之间"))]
    pub payment_terms: Option<i32>,

    /// 应付金额（必须为正数）
    #[validate(custom(function = "crate::services::ap_invoice_service::validate_positive_decimal"))]
    pub amount: Option<Decimal>,

    /// 币种（ISO 4217 三字母代码）
    #[validate(length(equal = 3, message = "币种必须为 ISO 4217 三字母代码"))]
    pub currency: Option<String>,

    /// 汇率（必须大于 0，防止 P0-1 历史缺陷的 0.01 汇率再次发生）
    #[validate(custom(function = "crate::services::ap_invoice_service::validate_exchange_rate"))]
    pub exchange_rate: Option<Decimal>,

    /// 税额（必须非负）
    #[validate(custom(function = "crate::services::ap_invoice_service::validate_non_negative_decimal"))]
    pub tax_amount: Option<Decimal>,

    /// 备注
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    #[validate(length(max = 10, message = "附件数量不能超过10个"))]
    pub attachment_urls: Option<Vec<String>>,
}

/// 更新应付单请求
///
/// TS-S-5 安全加固（2026-06-26）：补齐字段校验，与 CreateApInvoiceRequest 保持一致。
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateApInvoiceRequest {
    /// 应付类型
    #[validate(length(min = 1, max = 20, message = "发票号码长度必须在1到20个字符之间"))]
    pub invoice_type: Option<String>,

    /// 应付日期
    pub invoice_date: Option<NaiveDate>,

    /// 到期日期
    pub due_date: Option<NaiveDate>,

    /// 账期（天）
    #[validate(range(min = 0, max = 365, message = "账期必须在0到365天之间"))]
    pub payment_terms: Option<i32>,

    /// 应付金额（必须为正数）
    #[validate(custom(function = "crate::services::ap_invoice_service::validate_positive_decimal"))]
    pub amount: Option<Decimal>,

    /// 备注
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    #[validate(length(max = 10, message = "附件数量不能超过10个"))]
    pub attachment_urls: Option<Vec<String>>,
}

// =====================================================
// 报表统计 DTO
// =====================================================

/// 账龄分析项
#[derive(Debug, Serialize, Deserialize)]
pub struct AgingAnalysisItem {
    /// 账龄区间
    pub aging_bucket: String,

    /// 应付单数量
    pub invoice_count: i64,

    /// 总金额
    pub total_amount: Decimal,
}

/// 应付余额汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSummary {
    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 应付单数量
    pub invoice_count: i64,
}

/// 应付状态分布项（批次 133 v9 复审 P1）
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusStatItem {
    /// 状态（DRAFT/AUDITED/PARTIAL_PAID/PAID/CANCELLED）
    pub status: String,
    /// 该状态下的应付单数量
    pub invoice_count: i64,
    /// 该状态下的应付总金额
    pub total_amount: Decimal,
}

/// 应付统计报表（批次 133 v9 复审 P1）
///
/// 综合 3 个维度的统计：
/// 1. 余额汇总（balance_summary）：总应付 / 已付 / 未付 / 数量
/// 2. 账龄分析（aging_analysis）：按账龄区间分组的未付清应付单
/// 3. 状态分布（status_distribution）：按状态分组的应付单
#[derive(Debug, Serialize, Deserialize)]
pub struct ApInvoiceStatistics {
    /// 余额汇总
    pub balance_summary: BalanceSummary,
    /// 账龄分析
    pub aging_analysis: Vec<AgingAnalysisItem>,
    /// 状态分布
    pub status_distribution: Vec<StatusStatItem>,
}
