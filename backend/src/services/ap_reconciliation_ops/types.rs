//! 供应商对账 DTO 定义子模块（ap_reconciliation_ops/types）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移 4 个 DTO。
//! 包含生成对账请求、供应商应付汇总、自动对账结果、发票关联信息。
//! facade 通过 `pub use` 重新导出测试与外部实际使用的 DTO。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 生成对账单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct GenerateReconciliationRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 对账开始日期
    pub start_date: NaiveDate,

    /// 对账结束日期
    pub end_date: NaiveDate,

    /// 备注
    pub notes: Option<String>,
}

/// 供应商应付汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct SupplierApSummary {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 供应商编码
    pub supplier_code: String,

    /// 供应商名称
    pub supplier_name: String,

    /// 应付单总数
    pub total_invoice_count: i64,

    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 已付清应付单数量
    pub paid_invoice_count: i64,

    /// 部分付款应付单数量
    pub partial_paid_invoice_count: i64,

    /// 逾期应付单数量
    pub overdue_invoice_count: i64,

    /// 逾期金额
    pub overdue_amount: Decimal,
}

/// 自动对账结果
// 批次 23（2026-06-29 v5 P0-1 修复补充）：新增 Clone 派生，支持 lock().await.clone() 模式
#[derive(Debug, Clone, Serialize)]
pub struct AutoReconciliationResult {
    pub reconciliation_id: i32,
    pub reconciliation_no: String,
    pub supplier_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoice: Decimal,
    pub total_payment: Decimal,
    pub closing_balance: Decimal,
    pub invoice_count: usize,
    pub payment_count: usize,
    pub status: String,
    pub message: String,
}

/// 发票关联信息
#[derive(Debug, Serialize)]
pub struct InvoiceRelationInfo {
    pub invoice_id: i32,
    pub invoice_no: String,
    pub source_type: String,
    pub source_id: i32,
    pub source_no: Option<String>,
    pub supplier_id: i32,
    pub amount: Decimal,
    pub status: String,
}
