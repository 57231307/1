//! 应收账款-自动核销（ar_ops/verification_ops/auto）
//!
//! D10 拆分自原 `ar_ops/verification.rs`，包含自动核销流程：
//! - `auto_verify`（公开 API）按客户分组贪心匹配未核销发票与已确认收款
//!
//! 内部辅助（9 个，私有）：
//! - `load_auto_verify_data`           加载未核销发票 + 已确认收款 + 已核销汇总
//! - `process_customer_reconciliations` 处理单客户所有收款的核销
//! - `create_payment_reconciliation_record` 创建核销单主记录（VER 单号）
//! - `apply_matched_items`             批量插入核销明细 + 更新发票状态（v13 P1-3 N+1 重构）
//! - `batch_update_invoice_states`     批量 UPDATE 发票状态（去重 + 审计）
//! - `match_payment_to_invoices`       收款匹配发票（贪心，D12 重构提取）
//! - `make_invoice_verify_item`        构造 INVOICE 核销明细（D12 重构提取）
//! - `make_receipt_verify_item`        构造 RECEIPT 核销明细（D12 重构提取）
//! - `update_invoice_state`            累加核销金额并更新状态（D12 重构提取）

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use serde_json::json;
use tracing::info;

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
use crate::utils::error::AppError;

use super::super::types::{AutoVerifyData, VerifyTotals};
use crate::services::ar_service::ArService;

impl ArService {
    /// 自动核销
    /// 策略：按客户分组，未核销发票（unpaid_amount > 0）按到期日升序，
    /// 已确认收款（status = confirmed）按日期升序，贪心匹配。
    pub async fn auto_verify(&self, user_id: i32) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;
        let data = self.load_auto_verify_data(&txn).await?;

        // 按客户分组匹配
        let mut invoice_by_customer: std::collections::HashMap<i32, Vec<&ar_invoice::Model>> =
            std::collections::HashMap::new();
        for inv in &data.invoices {
            invoice_by_customer
                .entry(inv.customer_id)
                .or_default()
                .push(inv);
        }

        let mut totals = VerifyTotals {
            count: 0,
            amount: Decimal::ZERO,
        };
        for (customer_id, cust_invoices) in invoice_by_customer.iter() {
            self.process_customer_reconciliations(
                *customer_id,
                cust_invoices,
                &data,
                user_id,
                &txn,
                &mut totals,
            )
            .await?;
        }

        txn.commit().await?;
        info!(
            "AR 自动核销完成：核销笔数={}, 核销金额={}",
            totals.count, totals.amount
        );

        Ok(json!({
            "verified_count": totals.count,
            "verified_amount": totals.amount.to_string(),
        }))
    }

    /// 加载自动核销数据：未核销发票 + 已确认收款 + 已核销汇总
    async fn load_auto_verify_data(
        &self,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<AutoVerifyData, AppError> {
        // 查询所有未核销发票（按客户 + 到期日）
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .order_by(ar_invoice::Column::CustomerId, Order::Asc)
            .order_by(ar_invoice::Column::DueDate, Order::Asc)
            .all(txn)
            .await?;
        // 查询所有已确认未核销收款（按客户 + 日期）
        let payments = ar_collection::Entity::find()
            .filter(ar_collection::Column::Status.eq(crate::models::status::ar::COLLECTION_CONFIRMED))
            .order_by(ar_collection::Column::CustomerId, Order::Asc)
            .order_by(ar_collection::Column::CollectionDate, Order::Asc)
            .all(txn)
            .await?;
        // 批量查询已有核销记录，按 payment_id 汇总已核销金额
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let existing_items: Vec<ar_reconciliation_item::Model> = if payment_ids.is_empty() {
            Vec::new()
        } else {
            ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.is_in(payment_ids))
                .all(txn)
                .await?
        };
        let mut verified_map: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for item in &existing_items {
            if let Some(doc_id) = item.document_id {
                *verified_map.entry(doc_id).or_insert(Decimal::ZERO) += item.amount.abs();
            }
        }
        Ok(AutoVerifyData {
            invoices,
            payments,
            verified_map,
        })
    }

    /// 处理单客户所有收款的核销：遍历收款，匹配发票，创建核销单
    async fn process_customer_reconciliations(
        &self,
        customer_id: i32,
        cust_invoices: &[&ar_invoice::Model],
        data: &AutoVerifyData,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        totals: &mut VerifyTotals,
    ) -> Result<(), AppError> {
        // 该客户已确认未核销收款
        let cust_payments: Vec<&ar_collection::Model> = data
            .payments
            .iter()
            .filter(|p| p.customer_id == customer_id)
            .collect();

        let mut invoice_remaining: std::collections::HashMap<i32, Decimal> = cust_invoices
            .iter()
            .map(|inv| (inv.id, inv.unpaid_amount))
            .collect();

        for payment in cust_payments {
            let already_verified = data
                .verified_map
                .get(&payment.id)
                .copied()
                .unwrap_or(Decimal::ZERO);
            // D12 重构：匹配逻辑提取到 match_payment_to_invoices，消除内层循环+3 分支
            let matched_items = Self::match_payment_to_invoices(
                payment,
                cust_invoices,
                &mut invoice_remaining,
                already_verified,
            );
            if matched_items.is_empty() {
                continue;
            }
            // 创建核销单
            let customer_name = cust_invoices
                .first()
                .and_then(|i| i.customer_name.clone());
            let reconciliation = self
                .create_payment_reconciliation_record(
                    customer_id,
                    customer_name,
                    &matched_items,
                    user_id,
                    txn,
                )
                .await?;
            // 批量插入明细 + 更新发票（v13 P1-3：N+1 重构）
            self.apply_matched_items(
                &reconciliation,
                payment,
                matched_items,
                user_id,
                txn,
                totals,
            )
            .await?;
        }
        Ok(())
    }

    /// 创建收款核销单主记录（生成 VER 单号 + 初始化金额）
    async fn create_payment_reconciliation_record(
        &self,
        customer_id: i32,
        customer_name: Option<String>,
        matched_items: &[(i32, Decimal)],
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_reconciliation::Model, AppError> {
        let verify_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            txn,
            "VER",
            ar_reconciliation::Entity,
            ar_reconciliation::Column::ReconciliationNo,
        )
        .await?;
        let now = Utc::now();
        let today = now.date_naive();
        let total_amount: Decimal = matched_items.iter().map(|(_, a)| *a).sum();
        let reconciliation = ar_reconciliation::ActiveModel {
            reconciliation_no: Set(verify_no),
            reconciliation_date: Set(today),
            period_start: Set(today),
            period_end: Set(today),
            customer_id: Set(customer_id),
            customer_name: Set(customer_name),
            opening_balance: Set(Decimal::ZERO),
            total_invoices: Set(total_amount),
            total_collections: Set(total_amount),
            closing_balance: Set(Decimal::ZERO),
            reconciliation_status: Set(Some(crate::models::status::ar::RECONCILIATION_CLOSED.to_string())),
            confirmed_by: Set(Some(user_id)),
            confirmed_at: Set(Some(now)),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(reconciliation)
    }

    /// 批量插入核销明细并更新发票状态（v13 P1-3：N+1 重构，明细批量 INSERT + 发票批量 UPDATE）
    async fn apply_matched_items(
        &self,
        reconciliation: &ar_reconciliation::Model,
        payment: &ar_collection::Model,
        matched_items: Vec<(i32, Decimal)>,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        totals: &mut VerifyTotals,
    ) -> Result<(), AppError> {
        let now = reconciliation.created_at;
        // 批量查询并锁定所有相关发票
        let inv_ids: Vec<i32> = matched_items.iter().map(|(id, _)| *id).collect();
        let mut inv_map: std::collections::HashMap<i32, ar_invoice::Model> =
            ar_invoice::Entity::find()
                .filter(ar_invoice::Column::Id.is_in(inv_ids))
                .lock_exclusive()
                .all(txn)
                .await?
                .into_iter()
                .map(|inv| (inv.id, inv))
                .collect();
        // 收集所有明细 ActiveModel，循环结束后批量 INSERT
        let mut items_to_insert: Vec<ar_reconciliation_item::ActiveModel> = Vec::new();
        // 记录本收款涉及的发票 ID，用于内层循环结束后批量 UPDATE
        let mut touched_invoice_ids: Vec<i32> = Vec::new();
        for (inv_id, verify_amount) in matched_items {
            let inv = inv_map
                .get(&inv_id)
                .ok_or_else(|| AppError::not_found(format!("应收单 {}", inv_id)))?;
            // D12 重构：明细构造提取到 make_invoice_verify_item / make_receipt_verify_item
            items_to_insert.push(Self::make_invoice_verify_item(
                reconciliation.id,
                inv_id,
                inv,
                verify_amount,
                now,
            ));
            items_to_insert.push(Self::make_receipt_verify_item(
                reconciliation.id,
                payment.id,
                payment,
                verify_amount,
                now,
            ));
            // D12 重构：发票状态更新提取到 update_invoice_state（消除 if-else 三元分支）
            let invoice = inv_map
                .get_mut(&inv_id)
                .ok_or_else(|| AppError::not_found(format!("应收单 {}", inv_id)))?;
            Self::update_invoice_state(invoice, verify_amount);
            touched_invoice_ids.push(inv_id);
            totals.count += 1;
            totals.amount += verify_amount;
        }
        // 批量 INSERT 所有明细（INVOICE + RECEIPT），替代逐条 INSERT
        if !items_to_insert.is_empty() {
            ar_reconciliation_item::Entity::insert_many(items_to_insert)
                .exec(txn)
                .await?;
        }
        // 批量 UPDATE 本收款涉及的发票（同一发票可能多次匹配，内存中已累计最终状态）
        self.batch_update_invoice_states(inv_map, touched_invoice_ids, user_id, txn)
            .await
    }

    /// 批量 UPDATE 发票状态：去重后逐个 update_with_audit
    async fn batch_update_invoice_states(
        &self,
        mut inv_map: std::collections::HashMap<i32, ar_invoice::Model>,
        touched_invoice_ids: Vec<i32>,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let mut seen: std::collections::HashSet<i32> = std::collections::HashSet::new();
        for inv_id in touched_invoice_ids {
            if !seen.insert(inv_id) {
                continue; // 跳过已处理的发票
            }
            if let Some(invoice) = inv_map.remove(&inv_id) {
                let inv_active: ar_invoice::ActiveModel = invoice.into();
                crate::services::audit_log_service::AuditLogService::update_with_audit::<
                    ar_invoice::Entity,
                    _,
                    _,
                >(txn, "ar_invoice", inv_active, Some(user_id))
                .await?;
            }
        }
        Ok(())
    }

    /// 收款匹配发票（贪心，返回 (inv_id, verify_amount) 列表，并更新 invoice_remaining）
    fn match_payment_to_invoices<'a>(
        payment: &ar_collection::Model,
        cust_invoices: &[&'a ar_invoice::Model],
        invoice_remaining: &mut std::collections::HashMap<i32, Decimal>,
        already_verified: Decimal,
    ) -> Vec<(i32, Decimal)> {
        let mut remaining = payment.collection_amount - already_verified;
        if remaining <= Decimal::ZERO {
            return Vec::new();
        }
        let mut matched_items: Vec<(i32, Decimal)> = Vec::new();
        for inv in cust_invoices {
            if remaining <= Decimal::ZERO {
                break;
            }
            let unpaid = invoice_remaining
                .get(&inv.id)
                .copied()
                .unwrap_or(Decimal::ZERO);
            if unpaid <= Decimal::ZERO {
                continue;
            }
            let verify_amount = remaining.min(unpaid);
            matched_items.push((inv.id, verify_amount));
            remaining -= verify_amount;
            invoice_remaining.insert(inv.id, unpaid - verify_amount);
        }
        matched_items
    }

    /// 构造 INVOICE 核销明细（正金额）
    fn make_invoice_verify_item(
        reconciliation_id: i32,
        inv_id: i32,
        inv: &ar_invoice::Model,
        verify_amount: Decimal,
        now: chrono::DateTime<Utc>,
    ) -> ar_reconciliation_item::ActiveModel {
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(reconciliation_id),
            item_type: Set("INVOICE".to_string()),
            document_type: Set(Some("SALES_INVOICE".to_string())),
            document_id: Set(Some(inv_id)),
            document_no: Set(Some(inv.invoice_no.clone())),
            document_date: Set(Some(inv.invoice_date)),
            amount: Set(verify_amount),
            matched_amount: Set(Some(verify_amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
    }

    /// 构造 RECEIPT 核销明细（负金额，按惯例收款为负）
    fn make_receipt_verify_item(
        reconciliation_id: i32,
        payment_id: i32,
        payment: &ar_collection::Model,
        verify_amount: Decimal,
        now: chrono::DateTime<Utc>,
    ) -> ar_reconciliation_item::ActiveModel {
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(reconciliation_id),
            item_type: Set("RECEIPT".to_string()),
            document_type: Set(Some("AR_COLLECTION".to_string())),
            document_id: Set(Some(payment_id)),
            document_no: Set(Some(payment.collection_no.clone())),
            document_date: Set(Some(payment.collection_date)),
            amount: Set(-verify_amount),
            matched_amount: Set(Some(verify_amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
    }

    /// 累加发票核销金额并更新状态（PAID / PARTIAL_PAID）
    fn update_invoice_state(invoice: &mut ar_invoice::Model, verify_amount: Decimal) {
        invoice.received_amount += verify_amount;
        invoice.unpaid_amount = (invoice.invoice_amount - invoice.received_amount).max(Decimal::ZERO);
        invoice.status = if invoice.unpaid_amount == Decimal::ZERO {
            crate::models::status::payment::PAYMENT_PAID.to_string()
        } else {
            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
        };
    }
}
