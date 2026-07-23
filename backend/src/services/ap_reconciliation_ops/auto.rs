//! 供应商自动对账 impl 子模块（ap_reconciliation_ops/auto）
//!
//! D10-5 拆分：从原 `ap_reconciliation_service.rs` 迁移。
//! 包含 ApReconciliationService 的 8 个方法（1 公开 + 7 静态辅助）：
//! - auto_reconcile_all（为所有供应商自动生成对账单，并发处理）
//! - fetch_all_suppliers / fetch_invoice_counts_by_supplier / fetch_payment_counts_by_supplier
//!   （批量预加载，避免 for_each_concurrent 内 N+1 查询）
//! - build_auto_reconciliation_request / build_success_reconciliation_result /
//!   build_failure_reconciliation_result / process_supplier_reconciliation（构造与结果装配）

use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::{ap_invoice, ap_payment, ap_reconciliation};
use crate::models::supplier;
use crate::services::ap_reconciliation_ops::types::{AutoReconciliationResult, GenerateReconciliationRequest};
use crate::services::ap_reconciliation_service::ApReconciliationService;
use crate::utils::error::AppError;

impl ApReconciliationService {
    /// 自动对账 - 为所有供应商自动生成对账单
    pub async fn auto_reconcile_all(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        user_id: i32,
    ) -> Result<Vec<AutoReconciliationResult>, AppError> {
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
        let suppliers = Self::fetch_all_suppliers(&*self.db).await?;
        let supplier_ids: Vec<i32> = suppliers.iter().map(|s| s.id).collect();

        // v12 批次 39 修复：批量预加载所有供应商的发票数和付款数，避免 for_each_concurrent 内逐个 count（N+1，2N 次查询）
        let invoice_counts = Self::fetch_invoice_counts_by_supplier(
            &*self.db,
            &supplier_ids,
            start_date,
            end_date,
        )
        .await?;
        let payment_counts = Self::fetch_payment_counts_by_supplier(
            &*self.db,
            &supplier_ids,
            start_date,
            end_date,
        )
        .await?;

        let results = Arc::new(Mutex::new(Vec::new()));

        stream::iter(suppliers)
            .for_each_concurrent(10, |sup| {
                let results = results.clone();
                let db = self.db.clone();
                // v12 批次 39 修复：克隆预加载的计数 map 供并发任务读取（避免在闭包内查询 DB）
                let invoice_counts = invoice_counts.clone();
                let payment_counts = payment_counts.clone();
                async move {
                    let result = Self::process_supplier_reconciliation(
                        db,
                        sup,
                        user_id,
                        start_date,
                        end_date,
                        invoice_counts,
                        payment_counts,
                    )
                    .await;
                    let mut results_guard = results.lock().await;
                    results_guard.push(result);
                }
            })
            .await;

        // 批次 23（2026-06-29 v5 P0-1）：避免 Arc::try_unwrap().unwrap() 在 future 被取消时 panic
        // 原 try_unwrap 依赖"所有 clone 已 drop"的隐含契约，future 取消时 strong_count > 1 导致 panic。
        // 改为 lock().await.clone() 模式，安全且无 panic 风险（auto_reconcile 是低频批处理，clone 成本可接受）。
        let results = results.lock().await.clone();
        Ok(results)
    }

    async fn fetch_all_suppliers(
        db: &DatabaseConnection,
    ) -> Result<Vec<supplier::Model>, AppError> {
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
        supplier::Entity::find()
            .limit(10_000)
            .all(db)
            .await
            .map_err(AppError::from)
    }

    async fn fetch_invoice_counts_by_supplier(
        db: &DatabaseConnection,
        supplier_ids: &[i32],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<std::collections::HashMap<i32, usize>, AppError> {
        if supplier_ids.is_empty() {
            return Ok(std::collections::HashMap::<i32, usize>::new());
        }
        Ok(ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.is_in(supplier_ids.to_vec()))
            .filter(ap_invoice::Column::InvoiceDate.gte(start_date))
            .filter(ap_invoice::Column::InvoiceDate.lte(end_date))
            .all(db)
            .await?
            .into_iter()
            .fold(std::collections::HashMap::<i32, usize>::new(), |mut acc, inv| {
                *acc.entry(inv.supplier_id).or_default() += 1;
                acc
            }))
    }

    async fn fetch_payment_counts_by_supplier(
        db: &DatabaseConnection,
        supplier_ids: &[i32],
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<std::collections::HashMap<i32, usize>, AppError> {
        if supplier_ids.is_empty() {
            return Ok(std::collections::HashMap::<i32, usize>::new());
        }
        Ok(ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.is_in(supplier_ids.to_vec()))
            .filter(ap_payment::Column::PaymentDate.gte(start_date))
            .filter(ap_payment::Column::PaymentDate.lte(end_date))
            .all(db)
            .await?
            .into_iter()
            .fold(std::collections::HashMap::<i32, usize>::new(), |mut acc, pay| {
                *acc.entry(pay.supplier_id).or_default() += 1;
                acc
            }))
    }

    fn build_auto_reconciliation_request(
        sup: &supplier::Model,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> GenerateReconciliationRequest {
        GenerateReconciliationRequest {
            supplier_id: sup.id,
            start_date,
            end_date,
            notes: Some(format!("Auto-generated reconciliation for {}", sup.supplier_name)),
        }
    }

    fn build_success_reconciliation_result(
        rec: ap_reconciliation::Model,
        supplier_id: i32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        invoice_count: usize,
        payment_count: usize,
    ) -> AutoReconciliationResult {
        AutoReconciliationResult {
            reconciliation_id: rec.id,
            reconciliation_no: rec.reconciliation_no,
            supplier_id,
            start_date,
            end_date,
            opening_balance: rec.opening_balance,
            total_invoice: rec.total_invoice,
            total_payment: rec.total_payment,
            closing_balance: rec.closing_balance,
            invoice_count,
            payment_count,
            status: rec.reconciliation_status,
            message: "Auto reconciliation successful".to_string(),
        }
    }

    fn build_failure_reconciliation_result(
        supplier_id: i32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        error: AppError,
    ) -> AutoReconciliationResult {
        AutoReconciliationResult {
            reconciliation_id: 0,
            reconciliation_no: String::new(),
            supplier_id,
            start_date,
            end_date,
            opening_balance: Decimal::ZERO,
            total_invoice: Decimal::ZERO,
            total_payment: Decimal::ZERO,
            closing_balance: Decimal::ZERO,
            invoice_count: 0,
            payment_count: 0,
            status: "FAILED".to_string(),
            message: format!("Failed: {}", error),
        }
    }

    async fn process_supplier_reconciliation(
        db: Arc<DatabaseConnection>,
        sup: supplier::Model,
        user_id: i32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        invoice_counts: std::collections::HashMap<i32, usize>,
        payment_counts: std::collections::HashMap<i32, usize>,
    ) -> AutoReconciliationResult {
        let supplier_id = sup.id;
        // v12 批次 39 修复：从预加载的计数 map 中取，避免循环内逐个 count（N+1）
        let invoice_count = invoice_counts.get(&sup.id).copied().unwrap_or(0);
        let payment_count = payment_counts.get(&sup.id).copied().unwrap_or(0);

        let service = ApReconciliationService::new(db);
        let req = Self::build_auto_reconciliation_request(&sup, start_date, end_date);

        match service.generate_reconciliation(req, user_id).await {
            Ok(rec) => Self::build_success_reconciliation_result(
                rec,
                supplier_id,
                start_date,
                end_date,
                invoice_count,
                payment_count,
            ),
            Err(e) => Self::build_failure_reconciliation_result(
                supplier_id,
                start_date,
                end_date,
                e,
            ),
        }
    }
}
