//! 应收账款服务（批次 96 P0-1 修复：真实实现）
//!
//! 替换原占位实现，基于 ar_invoice / ar_collection / ar_reconciliation /
//! ar_reconciliation_item 模型实现真实数据库读写。
//!
//! 设计要点：
//! - 收款管理基于 ar_collection 表
//! - 核销管理基于 ar_reconciliation + ar_reconciliation_item 表
//! - 报表管理基于 ar_invoice + ar_collection 聚合查询
//! - 所有写操作在事务内执行，状态变更加 lock_exclusive 串行化
//! - 所有更新通过 update_with_audit 记录审计日志
//! - 金额校验 round_dp(2) 限制货币精度
//! - 期间锁定检查通过 AccountingPeriodService::check_date_locked_txn

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::json;
use std::sync::Arc;
use tracing::info;

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
use crate::utils::error::AppError;

// 批次 102 v6 P3-1 修复：状态字符串常量化，引用 crate::models::status
// - ar_collection.status（小写）→ ar::COLLECTION_*
// - ar_reconciliation.reconciliation_status（大写）→ ar::RECONCILIATION_*
// - ar_reconciliation_item.match_status（大写 MATCHED）→ ar::MATCH_MATCHED
// - ar_invoice.status（大写）→ 复用 common::STATUS_* / payment::PAYMENT_*（与 ar_invoice_service.rs 一致）

/// 应收账款服务
pub struct ArService {
    db: Arc<DatabaseConnection>,
}

impl ArService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ========== 收款管理 ==========

    /// 获取收款列表
    /// 基于 ar_collection 表分页查询，支持状态/客户/收款单号过滤
    pub async fn list_payments(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        customer_id: Option<i32>,
        payment_no: Option<String>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        let mut query = ar_collection::Entity::find();

        if let Some(s) = status {
            query = query.filter(ar_collection::Column::Status.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(ar_collection::Column::CustomerId.eq(cid));
        }
        if let Some(no) = payment_no {
            query = query.filter(ar_collection::Column::CollectionNo.eq(no));
        }

        let total = query.clone().count(&*self.db).await? as i64;
        let items = query
            .order_by(ar_collection::Column::CollectionDate, Order::Desc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        let list = items.into_iter().map(collection_to_json).collect();
        Ok((list, total))
    }

    /// 获取收款详情
    pub async fn get_payment(&self, payment_id: i32) -> Result<serde_json::Value, AppError> {
        let payment = ar_collection::Entity::find_by_id(payment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        Ok(collection_to_json(payment))
    }

    /// 创建收款
    /// 在事务内：期间锁定检查 → 客户存在性校验 → 单号生成 → 插入收款单 →
    /// 关联多张发票（更新 received_amount/unpaid_amount/status）→ 事件发布
    #[allow(clippy::too_many_arguments)]
    pub async fn create_payment(
        &self,
        customer_id: i32,
        amount: Decimal,
        payment_method: String,
        payment_date: NaiveDate,
        bank_account: Option<String>,
        // 批次 96 CI 修复：ar_collections 表无 remark 列，备注暂不持久化（schema 扩展后接入）
        _remark: Option<String>,
        invoice_ids: Option<Vec<i32>>,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 金额校验
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("收款金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            return Err(AppError::validation("收款金额精度不能超过 2 位小数"));
        }

        let txn = (*self.db).begin().await?;

        // 期间锁定检查（事务内，避免 TOCTOU）
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc
            .check_date_locked_txn(&txn, payment_date)
            .await
            .map_err(|e| AppError::business(e.to_string()))?;

        // 客户存在性校验 + 名称查询
        let customer = crate::models::customer::Entity::find_by_id(customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;
        let customer_name = customer.customer_name;

        // 单号生成（事务内，advisory_xact_lock 串行化）
        let collection_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            &txn,
            "COL",
            ar_collection::Entity,
            ar_collection::Column::CollectionNo,
        )
        .await?;

        // 插入收款单
        let now = Utc::now();
        let collection = ar_collection::ActiveModel {
            collection_no: Set(collection_no.clone()),
            collection_date: Set(payment_date),
            customer_id: Set(customer_id),
            customer_name: Set(Some(customer_name)),
            collection_amount: Set(amount),
            collection_method: Set(Some(payment_method)),
            bank_account: Set(bank_account),
            status: Set(crate::models::status::ar::COLLECTION_PENDING.to_string()),
            created_by: Set(user_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let collection_model = collection.insert(&txn).await?;

        // 关联多张发票：累加 received_amount、扣减 unpaid_amount、按需更新状态
        let mut linked_invoices: Vec<i32> = Vec::new();
        if let Some(inv_ids) = invoice_ids {
            // 批量查询所有发票并加锁，避免循环内 N+1
            let invoice_map: std::collections::HashMap<i32, ar_invoice::Model> = if inv_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                ar_invoice::Entity::find()
                    .filter(ar_invoice::Column::Id.is_in(inv_ids.clone()))
                    .lock_exclusive()
                    .all(&txn)
                    .await?
                    .into_iter()
                    .map(|inv| (inv.id, inv))
                    .collect()
            };

            // 按比例分摊收款金额到各发票
            // 简化策略：按发票顺序扣减，每张发票扣减 min(剩余收款, 发票未收金额)
            let mut remaining = amount;
            for inv_id in inv_ids {
                if remaining <= Decimal::ZERO {
                    break;
                }
                let invoice = invoice_map.get(&inv_id).ok_or_else(|| {
                    AppError::not_found(format!("应收单 {} 不存在", inv_id))
                })?;
                if invoice.status == crate::models::status::common::STATUS_CANCELLED {
                    return Err(AppError::bad_request(format!(
                        "应收单 {} 已取消，无法关联收款",
                        inv_id
                    )));
                }
                if invoice.customer_id != customer_id {
                    return Err(AppError::bad_request(format!(
                        "应收单 {} 客户与收款客户不一致",
                        inv_id
                    )));
                }
                let allocate = remaining.min(invoice.unpaid_amount);
                if allocate <= Decimal::ZERO {
                    continue;
                }
                let new_received = invoice.received_amount + allocate;
                let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
                let new_status = if new_unpaid == Decimal::ZERO {
                    crate::models::status::payment::PAYMENT_PAID.to_string()
                } else if new_received > Decimal::ZERO {
                    crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
                } else {
                    invoice.status.clone()
                };

                let mut active: ar_invoice::ActiveModel = invoice.clone().into();
                active.received_amount = Set(new_received);
                active.unpaid_amount = Set(new_unpaid);
                active.status = Set(new_status);
                active.updated_at = Set(now);
                crate::services::audit_log_service::AuditLogService::update_with_audit::<
                    ar_invoice::Entity,
                    _,
                    _,
                >(&txn, "ar_invoice", active, Some(user_id))
                .await?;

                remaining -= allocate;
                linked_invoices.push(inv_id);
            }
        }

        txn.commit().await?;

        info!(
            "AR 收款创建成功：collection_no={}, customer_id={}, amount={}, 关联发票数={}",
            collection_no,
            customer_id,
            amount,
            linked_invoices.len()
        );

        // 事件发布（commit 后，避免事件处理器回写导致事务膨胀）
        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        for inv_id in &linked_invoices {
            EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
                collection_id: collection_model.id,
                invoice_id: Some(*inv_id),
                amount,
                user_id,
            });
        }
        let period = format!("{:04}-{:02}", payment_date.year(), payment_date.month());
        EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source: format!("ar_collection_completed:{}", collection_no),
        });

        Ok(collection_to_json(collection_model))
    }

    /// 更新收款
    /// 仅 pending 状态可修改；金额变更需同步调整关联发票（简化：仅允许修改备注/银行账号/收款方式）
    pub async fn update_payment(
        &self,
        payment_id: i32,
        payload: serde_json::Value,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;

        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            return Err(AppError::bad_request(
                "非 pending 状态的收款单不可修改",
            ));
        }

        let mut active: ar_collection::ActiveModel = collection.into();

        if let Some(method) = payload.get("payment_method").and_then(|v| v.as_str()) {
            active.collection_method = Set(Some(method.to_string()));
        }
        if let Some(bank) = payload.get("bank_account").and_then(|v| v.as_str()) {
            active.bank_account = Set(Some(bank.to_string()));
        }
        // 收款单无 remark 字段，备注通过 check_no 字段承载（避免 schema 变更）
        // 若未来添加 remark 列，此处需切换
        if let Some(remark) = payload.get("remark").and_then(|v| v.as_str()) {
            active.check_no = Set(Some(remark.to_string()));
        }
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(&txn, "ar_collection", active, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(collection_to_json(updated))
    }

    /// 确认收款
    /// 状态门：pending → confirmed，lock_exclusive 串行化并发
    pub async fn confirm_payment(
        &self,
        payment_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;

        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            return Err(AppError::bad_request(format!(
                "收款单状态为 {}，仅 pending 状态可确认",
                collection.status
            )));
        }

        let mut active: ar_collection::ActiveModel = collection.into();
        active.status = Set(crate::models::status::ar::COLLECTION_CONFIRMED.to_string());
        active.confirmed_by = Set(Some(user_id));
        active.confirmed_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(&txn, "ar_collection", active, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(collection_to_json(updated))
    }

    /// 取消收款单（批次 158 v11 真实接入 COLLECTION_CANCELLED 常量）
    ///
    /// 业务规则：
    /// - 仅 `pending` 状态的收款单可直接取消；`confirmed` 状态需先取消关联核销单后再操作
    /// - 取消前校验该收款未被任何核销单引用（ar_reconciliation_item.item_type = 'RECEIPT'），
    ///   避免取消后核销明细悬空
    /// - 状态置为 `cancelled`，清空 confirmed_by/confirmed_at（pending 状态本应为空，防御性清理）
    /// - 通过 update_with_audit 记录审计日志
    pub async fn cancel_collection(
        &self,
        payment_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let txn = (*self.db).begin().await?;

        let collection = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;

        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
            return Err(AppError::bad_request(format!(
                "收款单状态为 {}，仅 pending 状态可直接取消；confirmed 状态请先取消关联核销单",
                collection.status
            )));
        }

        // 校验：收款未被任何核销单引用（pending 状态通常未被核销，防御性检查）
        let referenced_count = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
            .count(&txn)
            .await?;
        if referenced_count > 0 {
            return Err(AppError::bad_request(format!(
                "收款单 {} 已被 {} 笔核销单引用，请先取消关联核销单",
                payment_id, referenced_count
            )));
        }

        let now = Utc::now();
        let mut active: ar_collection::ActiveModel = collection.into();
        active.status = Set(crate::models::status::ar::COLLECTION_CANCELLED.to_string());
        active.confirmed_by = Set(None);
        active.confirmed_at = Set(None);
        active.updated_at = Set(now);

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit::<
            ar_collection::Entity,
            _,
            _,
        >(&txn, "ar_collection", active, Some(user_id))
        .await?;

        txn.commit().await?;

        info!(
            "AR 收款单取消成功：payment_id={}, operator={}",
            payment_id, user_id
        );

        Ok(collection_to_json(updated))
    }

    // ========== 核销管理 ==========
    //
    // 核销基于 ar_reconciliation + ar_reconciliation_item 表实现：
    // - ar_reconciliations 作为核销单主表（reconciliation_status = COMPLETED/CANCELLED）
    // - ar_reconciliation_items 记录每笔核销明细（INVOICE/RECEIPT 类型）

    /// 获取核销列表
    pub async fn list_verifications(
        &self,
        page: u64,
        page_size: u64,
        invoice_id: Option<i32>,
        payment_id: Option<i32>,
        status: Option<String>,
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
    ) -> Result<serde_json::Value, AppError> {
        let reconciliation = ar_reconciliation::Entity::find_by_id(verification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {} 不存在", verification_id)))?;

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

        // 查询所有未核销发票（按客户 + 到期日）
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .order_by(ar_invoice::Column::CustomerId, Order::Asc)
            .order_by(ar_invoice::Column::DueDate, Order::Asc)
            .all(&txn)
            .await?;

        // 查询所有已确认未核销收款（按客户 + 日期）
        let payments = ar_collection::Entity::find()
            .filter(ar_collection::Column::Status.eq(crate::models::status::ar::COLLECTION_CONFIRMED))
            .order_by(ar_collection::Column::CustomerId, Order::Asc)
            .order_by(ar_collection::Column::CollectionDate, Order::Asc)
            .all(&txn)
            .await?;

        // 批量查询已有核销记录，按 payment_id 汇总已核销金额
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let existing_items: Vec<ar_reconciliation_item::Model> = if payment_ids.is_empty() {
            Vec::new()
        } else {
            ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.is_in(payment_ids))
                .all(&txn)
                .await?
        };
        let mut verified_map: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for item in &existing_items {
            if let Some(doc_id) = item.document_id {
                *verified_map.entry(doc_id).or_insert(Decimal::ZERO) += item.amount.abs();
            }
        }

        // 按客户分组匹配
        let mut invoice_by_customer: std::collections::HashMap<i32, Vec<&ar_invoice::Model>> =
            std::collections::HashMap::new();
        for inv in &invoices {
            invoice_by_customer
                .entry(inv.customer_id)
                .or_default()
                .push(inv);
        }

        let mut total_verified_count: i64 = 0;
        let mut total_verified_amount = Decimal::ZERO;

        for (customer_id, cust_invoices) in invoice_by_customer.iter() {
            // 该客户已确认未核销收款
            let cust_payments: Vec<&ar_collection::Model> = payments
                .iter()
                .filter(|p| p.customer_id == *customer_id)
                .collect();

            let mut invoice_remaining: std::collections::HashMap<i32, Decimal> = cust_invoices
                .iter()
                .map(|inv| (inv.id, inv.unpaid_amount))
                .collect();

            for payment in cust_payments {
                let already_verified = verified_map.get(&payment.id).copied().unwrap_or(Decimal::ZERO);
                let mut remaining = payment.collection_amount - already_verified;
                if remaining <= Decimal::ZERO {
                    continue;
                }

                // 批量创建该收款匹配的明细（先收集，统一 insert）
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

                if matched_items.is_empty() {
                    continue;
                }

                // 创建核销单
                let verify_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
                    &txn,
                    "VER",
                    ar_reconciliation::Entity,
                    ar_reconciliation::Column::ReconciliationNo,
                )
                .await?;
                let now = Utc::now();
                let today = now.date_naive();

                let reconciliation = ar_reconciliation::ActiveModel {
                    reconciliation_no: Set(verify_no),
                    reconciliation_date: Set(today),
                    period_start: Set(today),
                    period_end: Set(today),
                    customer_id: Set(*customer_id),
                    customer_name: Set(cust_invoices.first().and_then(|i| i.customer_name.clone())),
                    opening_balance: Set(Decimal::ZERO),
                    total_invoices: Set(matched_items.iter().map(|(_, a)| *a).sum()),
                    total_collections: Set(matched_items.iter().map(|(_, a)| *a).sum()),
                    closing_balance: Set(Decimal::ZERO),
                    reconciliation_status: Set(Some(crate::models::status::ar::RECONCILIATION_CLOSED.to_string())),
                    confirmed_by: Set(Some(user_id)),
                    confirmed_at: Set(Some(now)),
                    created_by: Set(Some(user_id)),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;

                // 批量查询并锁定所有相关发票（v13 P1-3：N+1 重构，明细批量 INSERT + 发票批量 UPDATE）
                let inv_ids: Vec<i32> = matched_items.iter().map(|(id, _)| *id).collect();
                let mut inv_map: std::collections::HashMap<i32, ar_invoice::Model> =
                    ar_invoice::Entity::find()
                        .filter(ar_invoice::Column::Id.is_in(inv_ids))
                        .lock_exclusive()
                        .all(&txn)
                        .await?
                        .into_iter()
                        .map(|inv| (inv.id, inv))
                        .collect();

                // 收集所有明细 ActiveModel，循环结束后批量 INSERT
                let mut items_to_insert: Vec<ar_reconciliation_item::ActiveModel> = Vec::new();
                // 记录本收款涉及的发票 ID，用于内层循环结束后批量 UPDATE
                let mut touched_invoice_ids: Vec<i32> = Vec::new();

                for (inv_id, verify_amount) in matched_items {
                    let inv = inv_map.get(&inv_id).ok_or_else(|| {
                        AppError::not_found(format!("应收单 {}", inv_id))
                    })?;

                    // INVOICE 明细（正金额，收集不立即 INSERT）
                    items_to_insert.push(ar_reconciliation_item::ActiveModel {
                        reconciliation_id: Set(reconciliation.id),
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
                    });

                    // RECEIPT 明细（负金额，按惯例收款为负，收集不立即 INSERT）
                    items_to_insert.push(ar_reconciliation_item::ActiveModel {
                        reconciliation_id: Set(reconciliation.id),
                        item_type: Set("RECEIPT".to_string()),
                        document_type: Set(Some("AR_COLLECTION".to_string())),
                        document_id: Set(Some(payment.id)),
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
                    });

                    // 内存中累计发票状态变更（不立即 UPDATE）
                    let invoice = inv_map.get_mut(&inv_id).ok_or_else(|| {
                        AppError::not_found(format!("应收单 {}", inv_id))
                    })?;
                    invoice.received_amount += verify_amount;
                    invoice.unpaid_amount =
                        (invoice.invoice_amount - invoice.received_amount).max(Decimal::ZERO);
                    if invoice.unpaid_amount == Decimal::ZERO {
                        invoice.status = crate::models::status::payment::PAYMENT_PAID.to_string();
                    } else {
                        invoice.status = crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string();
                    }
                    touched_invoice_ids.push(inv_id);

                    total_verified_count += 1;
                    total_verified_amount += verify_amount;
                }

                // 批量 INSERT 所有明细（INVOICE + RECEIPT），替代逐条 INSERT
                if !items_to_insert.is_empty() {
                    ar_reconciliation_item::Entity::insert_many(items_to_insert)
                        .exec(&txn)
                        .await?;
                }

                // 批量 UPDATE 本收款涉及的发票（去重后逐个 update_with_audit）
                // 同一发票在同一收款内可能被多次匹配，内存中已累计最终状态
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
                        >(&txn, "ar_invoice", inv_active, Some(user_id))
                        .await?;
                    }
                }
            }
        }

        txn.commit().await?;

        info!(
            "AR 自动核销完成：核销笔数={}, 核销金额={}",
            total_verified_count, total_verified_amount
        );

        Ok(json!({
            "verified_count": total_verified_count,
            "verified_amount": total_verified_amount.to_string(),
        }))
    }

    /// 手动核销
    /// 指定一张发票 + 一张收款单 + 金额，创建核销记录
    #[allow(clippy::too_many_arguments)]
    pub async fn manual_verify(
        &self,
        invoice_id: i32,
        payment_id: i32,
        amount: Decimal,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 金额校验
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("核销金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            return Err(AppError::validation("核销金额精度不能超过 2 位小数"));
        }

        let txn = (*self.db).begin().await?;

        // 锁定发票和收款单
        let invoice = ar_invoice::Entity::find_by_id(invoice_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应收单 {}", invoice_id)))?;

        if invoice.status == crate::models::status::common::STATUS_CANCELLED {
            return Err(AppError::bad_request("应收单已取消，无法核销"));
        }
        if invoice.unpaid_amount < amount {
            return Err(AppError::business(format!(
                "应收单 {} 未收金额 {} 小于核销金额 {}",
                invoice.invoice_no, invoice.unpaid_amount, amount
            )));
        }

        let payment = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {}", payment_id)))?;

        if payment.status != crate::models::status::ar::COLLECTION_CONFIRMED {
            return Err(AppError::business(format!(
                "收款单 {} 状态为 {}，未确认不可核销",
                payment.collection_no, payment.status
            )));
        }
        if invoice.customer_id != payment.customer_id {
            return Err(AppError::business("发票客户与收款客户不一致，不可核销"));
        }

        // 查询该收款单已核销金额
        let existing_verified: Decimal = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
            .all(&txn)
            .await?
            .into_iter()
            .map(|i| i.amount.abs())
            .sum();
        let available = payment.collection_amount - existing_verified;
        if amount > available {
            return Err(AppError::business(format!(
                "收款单 {} 可用余额 {} 小于核销金额 {}",
                payment.collection_no, available, amount
            )));
        }

        // 创建核销单
        let verify_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            &txn,
            "VER",
            ar_reconciliation::Entity,
            ar_reconciliation::Column::ReconciliationNo,
        )
        .await?;
        let now = Utc::now();
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
            reconciliation_status: Set(Some(crate::models::status::ar::RECONCILIATION_CLOSED.to_string())),
            confirmed_by: Set(Some(user_id)),
            confirmed_at: Set(Some(now)),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // INVOICE 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(reconciliation.id),
            item_type: Set("INVOICE".to_string()),
            document_type: Set(Some("SALES_INVOICE".to_string())),
            document_id: Set(Some(invoice_id)),
            document_no: Set(Some(invoice.invoice_no.clone())),
            document_date: Set(Some(invoice.invoice_date)),
            amount: Set(amount),
            matched_amount: Set(Some(amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(remark.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // RECEIPT 明细
        ar_reconciliation_item::ActiveModel {
            reconciliation_id: Set(reconciliation.id),
            item_type: Set("RECEIPT".to_string()),
            document_type: Set(Some("AR_COLLECTION".to_string())),
            document_id: Set(Some(payment_id)),
            document_no: Set(Some(payment.collection_no.clone())),
            document_date: Set(Some(payment.collection_date)),
            amount: Set(-amount),
            matched_amount: Set(Some(amount)),
            match_status: Set(crate::models::status::ar::MATCH_MATCHED.to_string()),
            matched_item_id: Set(None),
            remarks: Set(remark),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

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
            >(&txn, "ar_invoice", inv_active, Some(user_id))
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

    // ========== 报表管理 ==========

    /// 获取统计报表
    pub async fn get_statistics_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED));

        if let Some(cid) = customer_id {
            q = q.filter(ar_invoice::Column::CustomerId.eq(cid));
        }
        if let Some(sd) = start_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = end_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.lte(ed));
        }

        let invoices = q.all(&*self.db).await?;

        let total_invoices = invoices.len() as i64;
        let total_amount: Decimal = invoices.iter().map(|i| i.invoice_amount).sum();
        let paid_amount: Decimal = invoices.iter().map(|i| i.received_amount).sum();
        let unpaid_amount: Decimal = invoices.iter().map(|i| i.unpaid_amount).sum();

        let today = Utc::now().date_naive();
        let overdue: Vec<&ar_invoice::Model> = invoices
            .iter()
            .filter(|i| i.due_date < today && i.unpaid_amount > Decimal::ZERO)
            .collect();
        let overdue_count = overdue.len() as i64;
        let overdue_amount: Decimal = overdue.iter().map(|i| i.unpaid_amount).sum();

        let collection_rate = if total_amount > Decimal::ZERO {
            (paid_amount / total_amount)
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        Ok(json!({
            "total_invoices": total_invoices,
            "total_amount": total_amount.to_string(),
            "paid_amount": paid_amount.to_string(),
            "unpaid_amount": unpaid_amount.to_string(),
            "overdue_count": overdue_count,
            "overdue_amount": overdue_amount.to_string(),
            "collection_rate": collection_rate,
        }))
    }

    /// 获取日报表
    /// 按 invoice_date 聚合每日发票金额、已收、未收
    pub async fn get_daily_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED));

        if let Some(cid) = customer_id {
            q = q.filter(ar_invoice::Column::CustomerId.eq(cid));
        }
        if let Some(sd) = start_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = end_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.lte(ed));
        }

        let invoices = q.all(&*self.db).await?;

        let mut daily_map: std::collections::HashMap<NaiveDate, DailyAgg> =
            std::collections::HashMap::new();
        for inv in invoices {
            let agg = daily_map.entry(inv.invoice_date).or_default();
            agg.invoice_count += 1;
            agg.invoice_amount += inv.invoice_amount;
            agg.paid_amount += inv.received_amount;
            agg.unpaid_amount += inv.unpaid_amount;
        }

        let mut result: Vec<serde_json::Value> = daily_map
            .into_iter()
            .map(|(date, agg)| {
                json!({
                    "date": date.to_string(),
                    "invoice_count": agg.invoice_count,
                    "invoice_amount": agg.invoice_amount.to_string(),
                    "paid_amount": agg.paid_amount.to_string(),
                    "unpaid_amount": agg.unpaid_amount.to_string(),
                })
            })
            .collect();
        result.sort_by(|a, b| {
            a["date"]
                .as_str()
                .unwrap_or("")
                .cmp(b["date"].as_str().unwrap_or(""))
        });

        Ok(json!(result))
    }

    /// 获取月报表
    pub async fn get_monthly_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED));

        if let Some(cid) = customer_id {
            q = q.filter(ar_invoice::Column::CustomerId.eq(cid));
        }
        if let Some(sd) = start_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = end_date {
            q = q.filter(ar_invoice::Column::InvoiceDate.lte(ed));
        }

        let invoices = q.all(&*self.db).await?;

        let mut monthly_map: std::collections::HashMap<String, MonthlyAgg> =
            std::collections::HashMap::new();
        for inv in invoices {
            let key = format!("{:04}-{:02}", inv.invoice_date.year(), inv.invoice_date.month());
            let agg = monthly_map.entry(key).or_default();
            agg.invoice_count += 1;
            agg.invoice_amount += inv.invoice_amount;
            agg.paid_amount += inv.received_amount;
            agg.unpaid_amount += inv.unpaid_amount;
        }

        let mut result: Vec<serde_json::Value> = monthly_map
            .into_iter()
            .map(|(month, agg)| {
                json!({
                    "month": month,
                    "invoice_count": agg.invoice_count,
                    "invoice_amount": agg.invoice_amount.to_string(),
                    "paid_amount": agg.paid_amount.to_string(),
                    "unpaid_amount": agg.unpaid_amount.to_string(),
                })
            })
            .collect();
        result.sort_by(|a, b| {
            a["month"]
                .as_str()
                .unwrap_or("")
                .cmp(b["month"].as_str().unwrap_or(""))
        });

        Ok(json!(result))
    }

    /// 获取账龄报表（v14 P0-2 修复：SQL 层聚合，避免全表数据加载到应用层）
    /// 按 due_date 计算 0-30/31-60/61-90/90+ 分桶，数据库层完成 SUM/COUNT 聚合
    pub async fn get_aging_report(
        &self,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // v14 P0-2 修复：使用 SQL CASE WHEN + SUM + COUNT 在数据库层完成分桶聚合
        // 避免全表数据加载到应用层导致内存溢出风险（原实现 .all() 加载全部发票到内存）
        // 规则 12 合规：customer_id 使用参数化绑定，禁止字符串拼接
        let today = Utc::now().date_naive();

        let (sql, params): (&str, Vec<sea_orm::Value>) = if let Some(cid) = customer_id {
            (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoice
                WHERE status <> $2
                  AND unpaid_amount > 0
                  AND customer_id = $3
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                    cid.into(),
                ],
            )
        } else {
            (
                r#"
                SELECT
                    COALESCE(SUM(CASE WHEN due_date >= $1 THEN unpaid_amount ELSE 0 END), 0) AS not_due,
                    COALESCE(SUM(CASE WHEN due_date < $1 AND (CURRENT_DATE - due_date) <= 30 THEN unpaid_amount ELSE 0 END), 0) AS bucket_0_30,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 31 AND 60 THEN unpaid_amount ELSE 0 END), 0) AS bucket_31_60,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) BETWEEN 61 AND 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_61_90,
                    COALESCE(SUM(CASE WHEN (CURRENT_DATE - due_date) > 90 THEN unpaid_amount ELSE 0 END), 0) AS bucket_90_plus,
                    COUNT(*) AS invoice_count
                FROM ar_invoice
                WHERE status <> $2
                  AND unpaid_amount > 0
                "#,
                vec![
                    today.into(),
                    crate::models::status::common::STATUS_CANCELLED.into(),
                ],
            )
        };

        let result: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("账龄报表聚合查询失败: {}", e)))?;

        let row = result
            .ok_or_else(|| AppError::internal("账龄报表聚合查询无结果".to_string()))?;

        // 按索引读取聚合字段（与 purchase_delivery_calculator.rs 一致的项目风格）
        let not_due: Decimal = row.try_get_by_index::<Decimal>(0).unwrap_or(Decimal::ZERO);
        let bucket_0_30: Decimal = row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);
        let bucket_31_60: Decimal = row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let bucket_61_90: Decimal = row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
        let bucket_90_plus: Decimal = row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
        let invoice_count: i64 = row.try_get_by_index::<i64>(5).unwrap_or(0);

        let total_overdue =
            bucket_0_30 + bucket_31_60 + bucket_61_90 + bucket_90_plus;

        Ok(json!({
            "not_due": not_due.to_string(),
            "bucket_0_30": bucket_0_30.to_string(),
            "bucket_31_60": bucket_31_60.to_string(),
            "bucket_61_90": bucket_61_90.to_string(),
            "bucket_90_plus": bucket_90_plus.to_string(),
            "total_overdue": total_overdue.to_string(),
            "invoice_count": invoice_count,
        }))
    }
}

// =====================================================
// JSON 序列化辅助函数
// =====================================================

/// 收款单 → JSON
fn collection_to_json(c: ar_collection::Model) -> serde_json::Value {
    json!({
        "id": c.id,
        "payment_no": c.collection_no,
        "collection_no": c.collection_no,
        "payment_date": c.collection_date.to_string(),
        "collection_date": c.collection_date.to_string(),
        "customer_id": c.customer_id,
        "customer_name": c.customer_name,
        "amount": c.collection_amount.to_string(),
        "collection_amount": c.collection_amount.to_string(),
        "payment_method": c.collection_method,
        "collection_method": c.collection_method,
        "bank_account": c.bank_account,
        "status": c.status,
        "confirmed_by": c.confirmed_by,
        "confirmed_at": c.confirmed_at,
        "created_by": c.created_by,
        "created_at": c.created_at,
        "updated_at": c.updated_at,
    })
}

/// 发票 → JSON
fn invoice_to_json(i: ar_invoice::Model) -> serde_json::Value {
    json!({
        "id": i.id,
        "invoice_no": i.invoice_no,
        "invoice_date": i.invoice_date.to_string(),
        "due_date": i.due_date.to_string(),
        "customer_id": i.customer_id,
        "customer_name": i.customer_name,
        "customer_code": i.customer_code,
        "invoice_amount": i.invoice_amount.to_string(),
        "received_amount": i.received_amount.to_string(),
        "unpaid_amount": i.unpaid_amount.to_string(),
        "status": i.status,
        "approval_status": i.approval_status,
        "batch_no": i.batch_no,
        "color_no": i.color_no,
        "sales_order_no": i.sales_order_no,
    })
}

/// 对账单 → JSON
fn reconciliation_to_json(r: ar_reconciliation::Model) -> serde_json::Value {
    json!({
        "id": r.id,
        "verification_no": r.reconciliation_no,
        "reconciliation_no": r.reconciliation_no,
        "verification_date": r.reconciliation_date.to_string(),
        "reconciliation_date": r.reconciliation_date.to_string(),
        "period_start": r.period_start.to_string(),
        "period_end": r.period_end.to_string(),
        "customer_id": r.customer_id,
        "customer_name": r.customer_name,
        "opening_balance": r.opening_balance.to_string(),
        "total_invoices": r.total_invoices.to_string(),
        "total_collections": r.total_collections.to_string(),
        "closing_balance": r.closing_balance.to_string(),
        "status": r.reconciliation_status,
        "reconciliation_status": r.reconciliation_status,
        "confirmed_by": r.confirmed_by,
        "confirmed_at": r.confirmed_at,
        "created_by": r.created_by,
        "created_at": r.created_at,
    })
}

/// 对账明细 → JSON
fn reconciliation_item_to_json(i: ar_reconciliation_item::Model) -> serde_json::Value {
    json!({
        "id": i.id,
        "reconciliation_id": i.reconciliation_id,
        "item_type": i.item_type,
        "document_type": i.document_type,
        "document_id": i.document_id,
        "document_no": i.document_no,
        "document_date": i.document_date.map(|d| d.to_string()),
        "amount": i.amount.to_string(),
        "matched_amount": i.matched_amount.map(|a| a.to_string()),
        "match_status": i.match_status,
        "remarks": i.remarks,
        "created_at": i.created_at,
    })
}

// =====================================================
// 内部聚合辅助结构
// =====================================================

#[derive(Default)]
struct DailyAgg {
    invoice_count: i64,
    invoice_amount: Decimal,
    paid_amount: Decimal,
    unpaid_amount: Decimal,
}

#[derive(Default)]
struct MonthlyAgg {
    invoice_count: i64,
    invoice_amount: Decimal,
    paid_amount: Decimal,
    unpaid_amount: Decimal,
}
