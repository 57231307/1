//! 应收对账服务模块（ar = accounts receivable）
//!
//! 由原 `services/ar_reconciliation_service.rs`（1121 行）按业务子领域拆分而来。
//! 子模块：
//! - `recon` 对账单主流程（CRUD / 状态机：draft → sent → confirmed/disputed → closed）
//! - `vfy`   核销：自动对账算法、账龄分桶、自动生成对账单、客户确认/争议
//! - `inv`   发票 PDF 导出（含明细拼装与 ExportService 协作）
//! - `pay`   付款：占位模块，实际收款业务由 `services/ar_collection_service.rs` 提供
//!
//! 兼容说明：原 `crate::services::ar::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::ar::*;` 重新导出以保持向后兼容。

#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, Order};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::models::ar_reconciliation::{
    ActiveModel as ReconciliationActiveModel, Entity as ReconciliationEntity,
    Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item::{
    Entity as ReconciliationItemEntity, Model as ReconciliationItemModel,
};
use crate::models::customer;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

pub mod inv;
pub mod pay;
pub mod recon;
pub mod vfy;

// =====================================================
// 共享 DTO（与原 ar_reconciliation_service.rs 保持一致）
// =====================================================

/// 创建对账单请求
#[derive(Debug, Clone)]
pub struct CreateReconciliationRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub notes: Option<String>,
}

/// 更新对账单请求
#[derive(Debug, Clone)]
pub struct UpdateReconciliationRequest {
    pub opening_balance: Option<Decimal>,
    pub total_invoices: Option<Decimal>,
    pub total_collections: Option<Decimal>,
    pub notes: Option<String>,
}

/// 对账单查询参数
#[derive(Debug, Clone)]
pub struct ReconciliationQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 自动对账请求
#[derive(Debug, Clone, Deserialize)]
pub struct AutoMatchRequest {
    pub customer_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub match_strategy: Option<String>,
}

/// 自动对账结果
#[derive(Debug, Serialize)]
pub struct AutoMatchResult {
    pub reconciliation_id: i32,
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: String,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub matched_count: usize,
    pub unmatched_count: usize,
    pub status: String,
}

/// 账龄分桶
#[derive(Debug, Serialize, Clone)]
pub struct AgingBucket {
    pub label: String,
    pub min_days: i64,
    pub max_days: Option<i64>,
    pub amount: Decimal,
    pub count: usize,
}

/// 客户账龄汇总
#[derive(Debug, Serialize)]
pub struct CustomerAgingSummary {
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: Decimal,
    pub buckets: Vec<AgingBucket>,
}

/// 账龄报告
#[derive(Debug, Serialize)]
pub struct AgingReport {
    pub analysis_date: NaiveDate,
    pub total_receivable: Decimal,
    pub customer_summaries: Vec<CustomerAgingSummary>,
    pub overall_buckets: Vec<AgingBucket>,
}

/// 对账明细行
#[derive(Debug, Serialize)]
pub struct ReconciliationDetail {
    pub id: i32,
    pub reconciliation_id: i32,
    pub item_type: String,
    pub document_type: Option<String>,
    pub document_id: Option<i32>,
    pub document_no: Option<String>,
    pub document_date: Option<NaiveDate>,
    pub amount: Decimal,
    pub matched_amount: Option<Decimal>,
    pub match_status: String,
    pub matched_item_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 对账单详情（含明细）
#[derive(Debug, Serialize)]
pub struct ReconciliationWithDetails {
    pub reconciliation: ReconciliationModel,
    pub details: Vec<ReconciliationDetail>,
}

/// 自动生成对账单请求
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReconciliationRequest {
    pub customer_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub notes: Option<String>,
}

// =====================================================
// 共享 Service 结构体（子模块均通过 impl ArReconciliationService 扩展）
// =====================================================

/// 应收对账 Service
pub struct ArReconciliationService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ArReconciliationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// =====================================================
// 共享内部辅助（供 vfy.rs 自动对账使用）
// =====================================================

/// 为对账单生成对账单号（共用）
pub(crate) async fn generate_reconciliation_no(
    db: &DatabaseConnection,
) -> Result<String, AppError> {
    DocumentNumberGenerator::generate_no(
        db,
        "RC",
        ReconciliationEntity,
        crate::models::ar_reconciliation::Column::ReconciliationNo,
    )
    .await
}

/// 重新导出共用 Order 枚举给子模块（避免重复导入）
pub(crate) use sea_orm::Order as SharedOrder;

/// 抑制未使用导入警告
#[allow(dead_code)]
fn _unused() {
    let _: Option<ReconciliationItemModel> = None;
    let _: Option<ReconciliationActiveModel> = None;
    let _: Option<ReconciliationItemEntity> = None;
    let _: Option<customer::Entity> = None;
    let _: Option<ar_invoice::Entity> = None;
    let _: Option<ar_collection::Entity> = None;
    let _: fn() -> _ = || info!("");
    let _: Order = Order::Asc;
}
