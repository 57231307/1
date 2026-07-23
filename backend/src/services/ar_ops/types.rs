//! 应收账款服务的内部类型定义（ar_ops/types）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L37-100 迁移。
//! - `CreateArPaymentParams`：外部 handler 引用，公开导出
//! - 其余 4 个 struct 仅 crate 内部使用，保持私有

use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::models::{ar_collection, ar_invoice, ar_reconciliation};

/// 创建收款参数对象
///
/// 批次 329 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告
#[derive(Debug)]
pub struct CreateArPaymentParams {
    /// 客户 ID
    pub customer_id: i32,
    /// 收款金额
    pub amount: Decimal,
    /// 收款方式
    pub payment_method: String,
    /// 收款日期
    pub payment_date: NaiveDate,
    /// 银行账号
    pub bank_account: Option<String>,
    /// 备注（ar_collections 表暂无 remark 列，schema 扩展后接入）
    pub remark: Option<String>,
    /// 关联发票 ID 列表
    pub invoice_ids: Option<Vec<i32>>,
}

/// 手动核销明细创建上下文：封装 reconciliation/invoice/payment 等参数
///
/// 批次 488 D08-1 拆分：引入参数对象消除 too_many_arguments 警告
pub(super) struct ReconciliationItemContext<'a> {
    pub reconciliation: &'a ar_reconciliation::Model,
    pub invoice: &'a ar_invoice::Model,
    pub payment: &'a ar_collection::Model,
    pub amount: Decimal,
    pub remark: Option<String>,
    pub now: chrono::DateTime<chrono::Utc>,
}

/// 自动核销数据集：封装发票/收款/已核销汇总
///
/// 批次 488 D08-1 拆分：用于 load_auto_verify_data → process_customer_reconciliations 间传递
pub(super) struct AutoVerifyData {
    pub invoices: Vec<ar_invoice::Model>,
    pub payments: Vec<ar_collection::Model>,
    pub verified_map: std::collections::HashMap<i32, Decimal>,
}

/// 自动核销累计：核销笔数 + 核销金额
///
/// 批次 488 D08-1 拆分：在 helper 间传递累计结果
pub(super) struct VerifyTotals {
    pub count: i64,
    pub amount: Decimal,
}

/// 收款单构建上下文：封装构造 ar_collection::ActiveModel 所需字段
///
/// 批次 488 D08-1 拆分：引入参数对象消除 too_many_arguments 警告
pub(super) struct CollectionBuildContext {
    pub collection_no: String,
    pub payment_date: NaiveDate,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub amount: Decimal,
    pub payment_method: String,
    pub bank_account: Option<String>,
    pub user_id: i32,
    pub now: chrono::DateTime<chrono::Utc>,
}
