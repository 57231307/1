//! 应收账款-核销管理子模块（ar_ops/verification）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L753-1778 迁移。
//! 包含 23 个核销管理方法：
//! - list_verifications / get_verification / auto_verify / manual_verify（公开 API）
//! - cancel_verification（公开 API）/ get_unverified_invoices / get_unverified_payments（公开 API）
//! - load_auto_verify_data / process_customer_reconciliations / create_payment_reconciliation_record
//! - apply_matched_items / batch_update_invoice_states / match_payment_to_invoices
//! - make_invoice_verify_item / make_receipt_verify_item / update_invoice_state
//! - validate_verify_amount / lock_and_validate_invoice / lock_and_validate_payment
//! - check_payment_available_balance / create_reconciliation_record / create_reconciliation_items
//! - update_invoice_after_verify
//!
//! 业务规则：
//! - 核销基于 ar_reconciliation + ar_reconciliation_item 表实现
//! - ar_reconciliations 作为核销单主表（reconciliation_status = CLOSED/CANCELLED）
//! - ar_reconciliation_items 记录每笔核销明细（INVOICE/RECEIPT 类型）
//! - 自动核销按客户分组贪心匹配；手动核销单张发票 + 单张收款单

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

use super::json_helpers::{
    collection_to_json, invoice_to_json, reconciliation_item_to_json, reconciliation_to_json,
};
use super::types::{AutoVerifyData, ReconciliationItemContext, VerifyTotals};
use crate::services::ar_service::ArService;

impl ArService {
    /// 获取核销列表
    pub async fn list_verifications(
        &self,
        page: u64,
        page_size: u64,
        invoice_id: Option<i32>,
        payment_id: Option<i32>,
        status: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        // 若按 invoice_id/payment_id 过滤，需先查 ar_reconciliation_items 拿到 reconciliation_id 集合
        let mut query = ar_reconciliation::Entity::find();

        if let Some(s) = status {
            query = query.filter(ar_reconciliation::Column::ReconciliationStatus.eq(s));
        }

        if let Some(inv_id) = invoice_id {
            let rec_ids: Vec<i32> = ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("INVOICE"))
                .filter(ar_reconciliation_item::Column::DocumentId.eq(inv_id))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|i| i.reconciliation_id)
                .collect();
            if rec_ids.is_empty() {
                return Ok((vec![], 0));
            }
            query = query.filter(ar_reconciliation::Column::Id.is_in(rec_ids));
        }

        if let Some(pay_id) = payment_id {
            let rec_ids: Vec<i32> = ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.eq(pay_id))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|i| i.reconciliation_id)
                .collect();
            if rec_ids.is_empty() {
                return Ok((vec![], 0));
            }
            query = query.filter(ar_reconciliation::Column::Id.is_in(rec_ids));
        }

        // V15 P0-S01：行级数据权限过滤
        // ar_reconciliation 表无 department_id，Dept 退化为 Self，使用 created_by（Option<i32>）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                ar_reconciliation::Column::CreatedBy,
                ar_reconciliation::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }

        let total = query.clone().count(&*self.db).await? as i64;
        let items = query
            .order_by(ar_reconciliation::Column::ReconciliationDate, Order::Desc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        let list = items.into_iter().map(reconciliation_to_json).collect();
        Ok((list, total))
    }

    /// 获取核销详情（含明细行）
    pub async fn get_verification(
        &self,
        verification_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let reconciliation = ar_reconciliation::Entity::find_by_id(verification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {} 不存在", verification_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // ar_reconciliation 表无 department_id，Dept 退化为 Self；
        // ar_reconciliation.created_by 是 Option<i32>（可能为空，空时按"无主数据"处理）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, reconciliation.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问核销单 {}（数据范围限制）",
                    verification_id
                )));
            }
        }

        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ReconciliationId.eq(verification_id))
            .all(&*self.db)
            .await?;

        let mut result = reconciliation_to_json(reconciliation);
        if let Some(obj) = result.as_object_mut() {
            obj.insert(
                "items".to_string(),
                json!(items
                    .into_iter()
                    .map(reconciliation_item_to_json)
                    .collect::<Vec<_>>()),
            );
        }
        Ok(result)
    }

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

    /// 手动核销
    /// 指定一张发票 + 一张收款单 + 金额，创建核销记录
    pub async fn manual_verify(
        &self,
        invoice_id: i32,
        payment_id: i32,
        amount: Decimal,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_verify_amount(invoice_id, payment_id, amount, user_id)?;
        let txn = (*self.db).begin().await?;
        let invoice = self
            .lock_and_validate_invoice(invoice_id, amount, user_id, &txn)
            .await?;
        let payment = self
            .lock_and_validate_payment(&invoice, payment_id, user_id, &txn)
            .await?;
        self.check_payment_available_balance(payment_id, &payment, amount, user_id, &txn)
            .await?;
        let now = Utc::now();
        let reconciliation = self
            .create_reconciliation_record(&invoice, amount, user_id, now, &txn)
            .await?;
        let ctx = ReconciliationItemContext {
            reconciliation: &reconciliation,
            invoice: &invoice,
            payment: &payment,
            amount,
            remark,
            now,
        };
        self.create_reconciliation_items(ctx, &txn).await?;
        let (new_status, updated_invoice) = self
            .update_invoice_after_verify(&invoice, amount, user_id, now, &txn)
            .await?;
        txn.commit().await?;
        info!(
            "AR 手动核销成功：reconciliation_id={}, invoice={}, payment={}, amount={}, 新状态={}",
            reconciliation.id, invoice_id, payment_id, amount, new_status
        );
        Ok(json!({
            "id": reconciliation.id,
            "reconciliation_no": reconciliation.reconciliation_no,
            "invoice_id": invoice_id,
            "payment_id": payment_id,
            "amount": amount.to_string(),
            "status": crate::models::status::ar::RECONCILIATION_CLOSED,
            "verified_by": user_id,
            "verified_at": now,
            "invoice_status": new_status,
            "invoice_unpaid_amount": updated_invoice.unpaid_amount.to_string(),
        }))
    }

    /// 核销金额前置校验：金额>0 + 精度≤2 位小数
    fn validate_verify_amount(
        invoice_id: i32,
        payment_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 金额校验
        if amount <= Decimal::ZERO {
            // 批次 389 P2-2：金额校验失败记录 warn 日志，便于审计异常核销行为
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVALID_AMOUNT",
                invoice_id = invoice_id,
                payment_id = payment_id,
                amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：金额必须大于零"
            );
            return Err(AppError::validation("核销金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            // 批次 389 P2-2：精度校验失败记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVALID_PRECISION",
                invoice_id = invoice_id,
                payment_id = payment_id,
                amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：精度超过 2 位小数"
            );
            return Err(AppError::validation("核销金额精度不能超过 2 位小数"));
        }
        Ok(())
    }

    /// 锁定发票并校验：未取消 + 未收金额充足
    async fn lock_and_validate_invoice(
        &self,
        invoice_id: i32,
        amount: Decimal,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_invoice::Model, AppError> {
        // 锁定发票
        let invoice = ar_invoice::Entity::find_by_id(invoice_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应收单 {}", invoice_id)))?;
        if invoice.status == crate::models::status::common::STATUS_CANCELLED {
            // 批次 389 P2-2：应收单已取消拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INVOICE_CANCELLED",
                invoice_id = invoice_id,
                invoice_no = %invoice.invoice_no,
                operator = user_id,
                "AR 手动核销被拒：应收单已取消"
            );
            return Err(AppError::bad_request("应收单已取消，无法核销"));
        }
        if invoice.unpaid_amount < amount {
            // 批次 389 P2-2：未收金额不足拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INSUFFICIENT_UNPAID",
                invoice_id = invoice_id,
                invoice_no = %invoice.invoice_no,
                unpaid_amount = %invoice.unpaid_amount,
                verify_amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：未收金额不足"
            );
            return Err(AppError::business(format!(
                "应收单 {} 未收金额 {} 小于核销金额 {}",
                invoice.invoice_no, invoice.unpaid_amount, amount
            )));
        }
        Ok(invoice)
    }

    /// 锁定收款单并校验：已确认 + 客户与发票一致
    async fn lock_and_validate_payment(
        &self,
        invoice: &ar_invoice::Model,
        payment_id: i32,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_collection::Model, AppError> {
        // 锁定收款单
        let payment = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {}", payment_id)))?;
        if payment.status != crate::models::status::ar::COLLECTION_CONFIRMED {
            // 批次 389 P2-2：收款未确认拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_PAYMENT_NOT_CONFIRMED",
                payment_id = payment_id,
                payment_no = %payment.collection_no,
                status = %payment.status,
                operator = user_id,
                "AR 手动核销被拒：收款单未确认"
            );
            return Err(AppError::business(format!(
                "收款单 {} 状态为 {}，未确认不可核销",
                payment.collection_no, payment.status
            )));
        }
        if invoice.customer_id != payment.customer_id {
            // 批次 389 P2-2：客户不一致拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_CUSTOMER_MISMATCH",
                invoice_id = invoice.id,
                payment_id = payment_id,
                invoice_customer_id = invoice.customer_id,
                payment_customer_id = payment.customer_id,
                operator = user_id,
                "AR 手动核销被拒：发票客户与收款客户不一致"
            );
            return Err(AppError::business("发票客户与收款客户不一致，不可核销"));
        }
        Ok(payment)
    }

    /// 校验收款单可用余额：已核销金额 + 本次核销金额 ≤ 收款金额
    async fn check_payment_available_balance(
        &self,
        payment_id: i32,
        payment: &ar_collection::Model,
        amount: Decimal,
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 查询该收款单已核销金额
        let existing_verified: Decimal = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
            .all(txn)
            .await?
            .into_iter()
            .map(|i| i.amount.abs())
            .sum();
        let available = payment.collection_amount - existing_verified;
        if amount > available {
            // 批次 389 P2-2：余额不足拒绝核销记录 warn 日志
            warn!(
                target: "business_audit",
                event = "AR_VERIFY_INSUFFICIENT_BALANCE",
                payment_id = payment_id,
                payment_no = %payment.collection_no,
                available = %available,
                verify_amount = %amount,
                operator = user_id,
                "AR 手动核销被拒：收款单可用余额不足"
            );
            return Err(AppError::business(format!(
                "收款单 {} 可用余额 {} 小于核销金额 {}",
                payment.collection_no, available, amount
            )));
        }
        Ok(())
    }

    /// 创建核销单主记录（生成 VER 单号 + 初始化金额）
    async fn create_reconciliation_record(
        &self,
        invoice: &ar_invoice::Model,
        amount: Decimal,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<ar_reconciliation::Model, AppError> {
        // 创建核销单
        let verify_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            txn,
            "VER",
            ar_reconciliation::Entity,
            ar_reconciliation::Column::ReconciliationNo,
        )
        .await?;
        let today = now.date_naive();
        let reconciliation = ar_reconciliation::ActiveModel {
            reconciliation_no: Set(verify_no),
            reconciliation_date: Set(today),
            period_start: Set(today),
            period_end: Set(today),
            customer_id: Set(invoice.customer_id),
            customer_name: Set(invoice.customer_name.clone()),
            opening_balance: Set(Decimal::ZERO),
            total_invoices: Set(amount),
            total_collections: Set(amount),
            closing_balance: Set(Decimal::ZERO),
            reconciliation_status: Set(Some(
                crate::models::status::ar::RECONCILIATION_CLOSED.to_string(),
            )),
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

    /// 创建核销明细：INVOICE 明细 + RECEIPT 明细
    async fn create_reconciliation_items(
        &self,
        ctx: ReconciliationItemContext<'_>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // INVOICE 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(ctx.reconciliation.id),
            item_type: Set("INVOICE".to_string()),
            document_type: Set(Some("SALES_INVOICE".to_string())),
            document_id: Set(Some(ctx.invoice.id)),
            document_no: Set(Some(ctx.invoice.invoice_no.clone())),
            document_date: Set(Some(ctx.invoice.invoice_date)),
            amount: Set(ctx.amount),
            matched_amount: Set(Some(ctx.amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(ctx.remark.clone()),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        // RECEIPT 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(ctx.reconciliation.id),
            item_type: Set("RECEIPT".to_string()),
            document_type: Set(Some("AR_COLLECTION".to_string())),
            document_id: Set(Some(ctx.payment.id)),
            document_no: Set(Some(ctx.payment.collection_no.clone())),
            document_date: Set(Some(ctx.payment.collection_date)),
            amount: Set(-ctx.amount),
            matched_amount: Set(Some(ctx.amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(ctx.remark),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
        .insert(txn)
        .await?;
        Ok(())
    }

    /// 核销后更新发票状态（received_amount/unpaid_amount/status + 审计）
    async fn update_invoice_after_verify(
        &self,
        invoice: &ar_invoice::Model,
        amount: Decimal,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(String, ar_invoice::Model), AppError> {
        // 更新发票
        let mut inv_active: ar_invoice::ActiveModel = invoice.clone().into();
        let new_received = invoice.received_amount + amount;
        let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
        let new_status = if new_unpaid == Decimal::ZERO {
            crate::models::status::payment::PAYMENT_PAID.to_string()
        } else {
            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
        };
        inv_active.received_amount = Set(new_received);
        inv_active.unpaid_amount = Set(new_unpaid);
        inv_active.status = Set(new_status.clone());
        inv_active.updated_at = Set(now);
        let updated_invoice =
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_invoice::Entity,
                _,
                _,
            >(txn, "ar_invoice", inv_active, Some(user_id))
            .await?;
        Ok((new_status, updated_invoice))
    }

    /// 取消核销
    /// 状态门：COMPLETED → CANCELLED，恢复发票 received_amount/unpaid_amount/status
    pub async fn cancel_verification(
        &self,
        verification_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let reconciliation = ar_reconciliation::Entity::find_by_id(verification_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {} 不存在", verification_id)))?;

        if reconciliation.reconciliation_status.as_deref() != Some(crate::models::status::ar::RECONCILIATION_CLOSED) {
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_VERIFICATION_CANCEL_REJECTED",
                verification_id = verification_id,
                status = ?reconciliation.reconciliation_status,
                operator = user_id,
                "AR 核销取消被拒：状态非 closed"
            );
            return Err(AppError::bad_request(format!(
                "核销单状态为 {:?}，仅 closed 状态可取消",
                reconciliation.reconciliation_status
            )));
        }

        // 查询所有 INVOICE 明细，按 invoice_id 汇总应回滚金额
        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ReconciliationId.eq(verification_id))
            .filter(ar_reconciliation_item::Column::ItemType.eq("INVOICE"))
            .all(&txn)
            .await?;

        // 批量查询并锁定所有相关发票
        let inv_ids: Vec<i32> = items
            .iter()
            .filter_map(|i| i.document_id)
            .collect();
        let mut inv_map: std::collections::HashMap<i32, ar_invoice::Model> = if inv_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            ar_invoice::Entity::find()
                .filter(ar_invoice::Column::Id.is_in(inv_ids))
                .lock_exclusive()
                .all(&txn)
                .await?
                .into_iter()
                .map(|inv| (inv.id, inv))
                .collect()
        };

        let now = Utc::now();

        for item in &items {
            let inv_id = item.document_id.ok_or_else(|| {
                AppError::business("核销明细缺少 document_id".to_string())
            })?;
            let invoice = inv_map.get_mut(&inv_id).ok_or_else(|| {
                AppError::not_found(format!("应收单 {}", inv_id))
            })?;
            invoice.received_amount -= item.amount;
            invoice.unpaid_amount =
                (invoice.invoice_amount - invoice.received_amount).max(Decimal::ZERO);
            // 状态恢复
            if invoice.received_amount >= invoice.invoice_amount {
                invoice.status = crate::models::status::payment::PAYMENT_PAID.to_string();
            } else if invoice.received_amount > Decimal::ZERO {
                invoice.status = crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string();
            } else {
                invoice.status = crate::models::status::common::STATUS_APPROVED.to_string();
            }
            let inv_active: ar_invoice::ActiveModel = invoice.clone().into();
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_invoice::Entity,
                _,
                _,
            >(&txn, "ar_invoice", inv_active, Some(user_id))
            .await?;
        }

        // 更新核销单状态
        let mut rec_active: ar_reconciliation::ActiveModel = reconciliation.into();
        rec_active.reconciliation_status = Set(Some(crate::models::status::ar::RECONCILIATION_CANCELLED.to_string()));
        rec_active.confirmed_by = Set(None);
        rec_active.confirmed_at = Set(None);
        rec_active.updated_at = Set(now);
        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit::<
                ar_reconciliation::Entity,
                _,
                _,
            >(&txn, "ar_reconciliation", rec_active, Some(user_id))
            .await?;

        txn.commit().await?;

        info!("AR 核销取消成功：verification_id={}", verification_id);

        Ok(reconciliation_to_json(updated))
    }

    /// 获取未核销发票
    /// 支持 query.customer_id 过滤
    pub async fn get_unverified_invoices(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO));

        if let Some(cid) = query.get("customer_id").and_then(|v| v.as_i64()) {
            q = q.filter(ar_invoice::Column::CustomerId.eq(cid as i32));
        }

        let invoices = q
            .order_by(ar_invoice::Column::DueDate, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(json!(invoices
            .into_iter()
            .map(invoice_to_json)
            .collect::<Vec<_>>()))
    }

    /// 获取未核销收款
    /// 支持 query.customer_id 过滤
    pub async fn get_unverified_payments(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_collection::Entity::find()
            .filter(ar_collection::Column::Status.eq(crate::models::status::ar::COLLECTION_CONFIRMED));

        if let Some(cid) = query.get("customer_id").and_then(|v| v.as_i64()) {
            q = q.filter(ar_collection::Column::CustomerId.eq(cid as i32));
        }

        let payments = q
            .order_by(ar_collection::Column::CollectionDate, Order::Asc)
            .all(&*self.db)
            .await?;

        // 批量查询已有核销记录，过滤已完全核销的收款
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let verified_items = if payment_ids.is_empty() {
            Vec::new()
        } else {
            ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.is_in(payment_ids))
                .all(&*self.db)
                .await?
        };
        let mut verified_map: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for item in &verified_items {
            if let Some(doc_id) = item.document_id {
                *verified_map.entry(doc_id).or_insert(Decimal::ZERO) += item.amount.abs();
            }
        }

        let result: Vec<serde_json::Value> = payments
            .into_iter()
            .filter(|p| {
                let verified = verified_map.get(&p.id).copied().unwrap_or(Decimal::ZERO);
                verified < p.collection_amount
            })
            .map(collection_to_json)
            .collect();

        Ok(json!(result))
    }
}
