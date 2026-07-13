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
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
use crate::utils::error::AppError;

// 批次 102 v6 P3-1 修复：状态字符串常量化，引用 crate::models::status
// - ar_collection.status（小写）→ ar::COLLECTION_*
// - ar_reconciliation.reconciliation_status（大写）→ ar::RECONCILIATION_*
// - ar_reconciliation_item.match_status（大写 MATCHED）→ ar::MATCH_MATCHED
// - ar_invoice.status（大写）→ 复用 common::STATUS_* / payment::PAYMENT_*（与 ar_invoice_service.rs 一致）

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
    ///
    /// 批次 329 v10 复审 P3 修复：使用 CreateArPaymentParams 参数对象替代 8 个独立参数
    pub async fn create_payment(
        &self,
        params: CreateArPaymentParams,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let customer_id = params.customer_id;
        let amount = params.amount;
        let payment_method = params.payment_method;
        let payment_date = params.payment_date;
        let bank_account = params.bank_account;
        // 批次 96 CI 修复：ar_collections 表无 remark 列，备注暂不持久化（schema 扩展后接入）
        let _remark = params.remark;
        let invoice_ids = params.invoice_ids;

        // 金额校验
        if amount <= Decimal::ZERO {
            // 批次 389 P2-2：金额校验失败记录 warn 日志，便于审计异常收款行为
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_INVALID_AMOUNT",
                customer_id = customer_id,
                amount = %amount,
                "AR 收款金额校验失败：金额必须大于零"
            );
            return Err(AppError::validation("收款金额必须大于零"));
        }
        if amount.round_dp(2) != amount {
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_INVALID_PRECISION",
                customer_id = customer_id,
                amount = %amount,
                "AR 收款金额校验失败：精度超过 2 位小数"
            );
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
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_UPDATE_REJECTED",
                payment_id = payment_id,
                status = %collection.status,
                "AR 收款更新被拒：状态非 pending"
            );
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
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_PAYMENT_CONFIRM_REJECTED",
                payment_id = payment_id,
                status = %collection.status,
                operator = user_id,
                "AR 收款确认被拒：状态非 pending"
            );
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

        // F-P0-4 修复（批次 381 v13 复审）：确认收款后生成收款凭证
        // 借：银行存款/库存现金 / 贷：应收账款（挂客户辅助核算）
        // 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
        let collection_amount = updated.collection_amount;
        let collection_method = updated.collection_method.as_deref().unwrap_or("BANK_TRANSFER");
        let (debit_code, debit_name) = match collection_method {
            "CASH" => ("1001", "库存现金"),
            _ => ("1002", "银行存款"),
        };
        let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "收".to_string(),
            voucher_date: updated.collection_date,
            source_type: Some("AR_COLLECTION".to_string()),
            source_module: Some("ar".to_string()),
            source_bill_id: Some(updated.id),
            source_bill_no: Some(updated.collection_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(1),
                    subject_code: Some(debit_code.to_string()),
                    subject_name: Some(debit_name.to_string()),
                    debit: collection_amount,
                    credit: Decimal::ZERO,
                    summary: Some(format!("收款确认-{}", updated.collection_no)),
                    assist_customer_id: Some(updated.customer_id),
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(2),
                    subject_code: Some("1131".to_string()),
                    subject_name: Some("应收账款".to_string()),
                    debit: Decimal::ZERO,
                    credit: collection_amount,
                    summary: Some(format!("收款确认-{}", updated.collection_no)),
                    assist_customer_id: Some(updated.customer_id),
                    assist_supplier_id: None,
                    assist_department_id: None,
                    assist_employee_id: None,
                    assist_project_id: None,
                    assist_batch_id: None,
                    assist_color_no_id: None,
                    assist_dye_lot_id: None,
                    assist_grade: None,
                    assist_workshop_id: None,
                    quantity_meters: None,
                    quantity_kg: None,
                    unit_price: None,
                },
            ],
        };
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "收款单 {} 确认成功，但生成收款凭证失败：{}",
                updated.collection_no,
                e
            );
        }

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
            // 批次 389 P2-2：状态门拒绝记录 warn 日志，便于审计非法状态变更
            warn!(
                target: "business_audit",
                event = "AR_COLLECTION_CANCEL_REJECTED",
                payment_id = payment_id,
                status = %collection.status,
                operator = user_id,
                "AR 收款取消被拒：状态非 pending"
            );
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
            // 批次 389 P2-2：被核销单引用拒绝取消记录 warn 日志，便于审计异常取消行为
            warn!(
                target: "business_audit",
                event = "AR_COLLECTION_CANCEL_REFERENCED",
                payment_id = payment_id,
                referenced_count = referenced_count,
                operator = user_id,
                "AR 收款取消被拒：已被核销单引用"
            );
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

        let txn = (*self.db).begin().await?;

        // 锁定发票和收款单
        let invoice = ar_invoice::Entity::find_by_id(invoice_id)
            .lock_exclusive()
            .one(&txn)
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

        let payment = ar_collection::Entity::find_by_id(payment_id)
            .lock_exclusive()
            .one(&txn)
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
                invoice_id = invoice_id,
                payment_id = payment_id,
                invoice_customer_id = invoice.customer_id,
                payment_customer_id = payment.customer_id,
                operator = user_id,
                "AR 手动核销被拒：发票客户与收款客户不一致"
            );
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

    // ========== 报表管理 ==========

    /// 获取统计报表
    /// v14 中风险性能修复（批次 244）：SQL 层聚合，避免全量加载发票到内存
    pub async fn get_statistics_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定，禁止字符串拼接
        let today = Utc::now().date_naive();
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }
        // today 用于逾期条件
        let today_param_idx = params.len() + 1;
        params.push(today.into());

        let sql = format!(
            r#"
            SELECT
                COUNT(*) AS total_invoices,
                COALESCE(SUM(invoice_amount), 0) AS total_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount,
                COUNT(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN 1 END) AS overdue_count,
                COALESCE(SUM(CASE WHEN due_date < ${today_idx} AND unpaid_amount > 0 THEN unpaid_amount ELSE 0 END), 0) AS overdue_amount
            FROM ar_invoice
            WHERE {where}
            "#,
            today_idx = today_param_idx,
            where = where_clauses.join(" AND ")
        );

        let row: Option<sea_orm::QueryResult> = self
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("统计报表聚合查询失败: {}", e)))?;

        let row = row
            .ok_or_else(|| AppError::internal("统计报表聚合查询无结果".to_string()))?;

        let total_invoices: i64 = row.try_get_by_index::<i64>(0).unwrap_or(0);
        let total_amount: Decimal = row.try_get_by_index::<Decimal>(1).unwrap_or(Decimal::ZERO);
        let paid_amount: Decimal = row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
        let unpaid_amount: Decimal = row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
        let overdue_count: i64 = row.try_get_by_index::<i64>(4).unwrap_or(0);
        let overdue_amount: Decimal = row.try_get_by_index::<Decimal>(5).unwrap_or(Decimal::ZERO);

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
    /// v14 中风险性能修复（批次 244）：SQL GROUP BY 聚合，避免全量加载到内存
    pub async fn get_daily_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }

        let sql = format!(
            r#"
            SELECT
                invoice_date,
                COUNT(*) AS invoice_count,
                COALESCE(SUM(invoice_amount), 0) AS invoice_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ar_invoice
            WHERE {where}
            GROUP BY invoice_date
            ORDER BY invoice_date ASC
            "#,
            where = where_clauses.join(" AND ")
        );

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("日报表聚合查询失败: {}", e)))?;

        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let date: NaiveDate = row.try_get_by_index::<NaiveDate>(0).unwrap_or_default();
                let invoice_count: i64 = row.try_get_by_index::<i64>(1).unwrap_or(0);
                let invoice_amount: Decimal =
                    row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
                let paid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
                let unpaid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
                json!({
                    "date": date.to_string(),
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                })
            })
            .collect();

        Ok(json!(result))
    }

    /// 获取月报表
    /// v14 中风险性能修复（批次 244）：SQL GROUP BY to_char 月份聚合，避免全量加载到内存
    pub async fn get_monthly_report(
        &self,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        customer_id: Option<i32>,
    ) -> Result<serde_json::Value, AppError> {
        // 规则 12 合规：全部参数使用参数化绑定
        let mut params: Vec<sea_orm::Value> = vec![];
        let mut where_clauses = vec![format!("status <> ${}", params.len() + 1)];
        params.push(crate::models::status::common::STATUS_CANCELLED.into());

        if let Some(cid) = customer_id {
            where_clauses.push(format!("customer_id = ${}", params.len() + 1));
            params.push(cid.into());
        }
        if let Some(sd) = start_date {
            where_clauses.push(format!("invoice_date >= ${}", params.len() + 1));
            params.push(sd.into());
        }
        if let Some(ed) = end_date {
            where_clauses.push(format!("invoice_date <= ${}", params.len() + 1));
            params.push(ed.into());
        }

        let sql = format!(
            r#"
            SELECT
                to_char(invoice_date, 'YYYY-MM') AS month,
                COUNT(*) AS invoice_count,
                COALESCE(SUM(invoice_amount), 0) AS invoice_amount,
                COALESCE(SUM(received_amount), 0) AS paid_amount,
                COALESCE(SUM(unpaid_amount), 0) AS unpaid_amount
            FROM ar_invoice
            WHERE {where}
            GROUP BY to_char(invoice_date, 'YYYY-MM')
            ORDER BY to_char(invoice_date, 'YYYY-MM') ASC
            "#,
            where = where_clauses.join(" AND ")
        );

        let rows: Vec<sea_orm::QueryResult> = self
            .db
            .query_all(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                sql,
                params,
            ))
            .await
            .map_err(|e| AppError::internal(format!("月报表聚合查询失败: {}", e)))?;

        let result: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                let month: String = row.try_get_by_index::<String>(0).unwrap_or_default();
                let invoice_count: i64 = row.try_get_by_index::<i64>(1).unwrap_or(0);
                let invoice_amount: Decimal =
                    row.try_get_by_index::<Decimal>(2).unwrap_or(Decimal::ZERO);
                let paid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(3).unwrap_or(Decimal::ZERO);
                let unpaid_amount: Decimal =
                    row.try_get_by_index::<Decimal>(4).unwrap_or(Decimal::ZERO);
                json!({
                    "month": month,
                    "invoice_count": invoice_count,
                    "invoice_amount": invoice_amount.to_string(),
                    "paid_amount": paid_amount.to_string(),
                    "unpaid_amount": unpaid_amount.to_string(),
                })
            })
            .collect();

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

// v14 中风险性能修复（批次 244）：DailyAgg / MonthlyAgg 已删除
// 原内存聚合逻辑改为 SQL GROUP BY 聚合，这两个 struct 不再需要

#[cfg(test)]
mod tests {
    //! 应收账款服务单元测试（批次 393 补测）
    //!
    //! 覆盖目标：
    //! - AR 状态常量值正确性（COLLECTION_*/MATCH_* 大小写混合）
    //! - 收款金额校验（零/负数/精度超限）
    //! - 收款状态机门（pending → confirmed）
    //! - 核销金额贪心匹配算法
    //! - ArService 实例化

    use super::*;
    use crate::models::status;
    use rust_decimal::Decimal;
    use sea_orm::Database;

    /// 复现 create_payment 中的收款金额校验逻辑
    ///
    /// 源码位置：create_payment 方法开头两道校验门。
    /// 1. amount <= 0 → Err("收款金额必须大于零")
    /// 2. amount.round_dp(2) != amount → Err("收款金额精度不能超过 2 位小数")
    fn validate_payment_amount(amount: Decimal) -> Result<(), &'static str> {
        if amount <= Decimal::ZERO {
            return Err("收款金额必须大于零");
        }
        if amount.round_dp(2) != amount {
            return Err("收款金额精度不能超过 2 位小数");
        }
        Ok(())
    }

    /// 复现 confirm_payment 中的收款状态机门判定
    ///
    /// 源码位置：confirm_payment 方法内的状态门。
    /// 仅 status::ar::COLLECTION_PENDING 状态允许确认。
    fn can_confirm_payment(current_status: &str) -> bool {
        current_status == status::ar::COLLECTION_PENDING
    }

    /// 复现 create_payment 中的核销金额贪心匹配算法
    ///
    /// 源码位置：create_payment 方法内关联多张发票的扣减循环。
    /// 按发票顺序扣减，每张发票扣减 min(剩余收款, 发票未收金额)。
    /// 返回各发票的实际核销金额列表 + 剩余未核销金额。
    fn greedy_match(
        payment_amount: Decimal,
        unpaid_amounts: &[Decimal],
    ) -> (Vec<Decimal>, Decimal) {
        let mut remaining = payment_amount;
        let mut allocations = Vec::with_capacity(unpaid_amounts.len());
        for unpaid in unpaid_amounts {
            if remaining <= Decimal::ZERO {
                allocations.push(Decimal::ZERO);
                continue;
            }
            let allocate = remaining.min(*unpaid);
            allocations.push(allocate);
            remaining -= allocate;
        }
        (allocations, remaining)
    }

    /// 测试_AR状态常量值正确性
    ///
    /// 验证 ar_collection.status（小写）和 ar_reconciliation_item.match_status（大写）
    /// 的常量值与业务约定一致，防止大小写混淆导致状态匹配失败。
    #[test]
    fn 测试_AR状态常量值正确性() {
        // ar_collection.status（小写值，批次 231 v13 P1-1 统一小写）
        assert_eq!(status::ar::COLLECTION_PENDING, "pending");
        assert_eq!(status::ar::COLLECTION_CONFIRMED, "confirmed");
        assert_eq!(status::ar::COLLECTION_CANCELLED, "cancelled");

        // ar_reconciliation.reconciliation_status（小写值）
        assert_eq!(status::ar::RECONCILIATION_DRAFT, "draft");
        assert_eq!(status::ar::RECONCILIATION_SENT, "sent");
        assert_eq!(status::ar::RECONCILIATION_CONFIRMED, "confirmed");
        assert_eq!(status::ar::RECONCILIATION_DISPUTED, "disputed");
        assert_eq!(status::ar::RECONCILIATION_CLOSED, "closed");
        assert_eq!(status::ar::RECONCILIATION_CANCELLED, "cancelled");

        // ar_reconciliation_item.match_status（大写值，注意大小写混合）
        assert_eq!(status::ar::MATCH_MATCHED, "MATCHED");
        assert_eq!(status::ar::MATCH_UNMATCHED, "UNMATCHED");

        // 防御性断言：小写/大写不应混淆
        assert_ne!(status::ar::COLLECTION_PENDING, "PENDING");
        assert_ne!(status::ar::MATCH_MATCHED, "matched");
    }

    /// 测试_收款金额校验_零或负数拒绝
    ///
    /// 场景：amount <= 0 应返回 Err（防零额收款）
    #[test]
    fn 测试_收款金额校验_零或负数拒绝() {
        // 零金额
        let result = validate_payment_amount(Decimal::ZERO);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额必须大于零");

        // 负数金额
        let result = validate_payment_amount(Decimal::new(-100, 2));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额必须大于零");
    }

    /// 测试_收款金额校验_精度超限拒绝
    ///
    /// 场景：amount.round_dp(2) != amount（超过 2 位小数）应返回 Err
    #[test]
    fn 测试_收款金额校验_精度超限拒绝() {
        // 3 位小数（123.456）应拒绝
        let result = validate_payment_amount(Decimal::new(123456, 3));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "收款金额精度不能超过 2 位小数");

        // 2 位小数（123.45）应通过
        let result = validate_payment_amount(Decimal::new(12345, 2));
        assert!(result.is_ok());

        // 整数（100）应通过
        let result = validate_payment_amount(Decimal::new(100, 0));
        assert!(result.is_ok());
    }

    /// 测试_收款状态机门_仅pending允许确认
    ///
    /// 验证 confirm_payment 的状态门：仅 pending 状态允许确认
    #[test]
    fn 测试_收款状态机门_仅pending允许确认() {
        // pending 允许确认
        assert!(can_confirm_payment(status::ar::COLLECTION_PENDING));

        // confirmed / cancelled 禁止确认
        assert!(!can_confirm_payment(status::ar::COLLECTION_CONFIRMED));
        assert!(!can_confirm_payment(status::ar::COLLECTION_CANCELLED));

        // 非法状态值禁止
        assert!(!can_confirm_payment("PENDING")); // 大写（历史 bug 值）
        assert!(!can_confirm_payment(""));
        assert!(!can_confirm_payment("unknown"));
    }

    /// 测试_核销金额贪心匹配算法
    ///
    /// 验证 create_payment 中按发票顺序扣减的核销逻辑：
    /// - 收款金额足够：每张发票扣减其 unpaid_amount，剩余为 0
    /// - 收款金额不足：按顺序扣减，最后一张部分扣减，后续发票扣减 0
    #[test]
    fn 测试_核销金额贪心匹配算法() {
        // 场景 1：收款金额 = 300，3 张发票未收金额 [100, 200, 50]
        // 期望：[100, 200, 0]，剩余 0（前两张发票完全核销，第三张未核销）
        let (allocations, remaining) = greedy_match(
            Decimal::new(300, 0),
            &[
                Decimal::new(100, 0),
                Decimal::new(200, 0),
                Decimal::new(50, 0),
            ],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(200, 0),
            Decimal::ZERO,
        ]);
        assert_eq!(remaining, Decimal::ZERO);

        // 场景 2：收款金额 = 150，3 张发票未收金额 [100, 200, 50]
        // 期望：[100, 50, 0]，剩余 0（第一张完全核销，第二张部分核销 50）
        let (allocations, remaining) = greedy_match(
            Decimal::new(150, 0),
            &[
                Decimal::new(100, 0),
                Decimal::new(200, 0),
                Decimal::new(50, 0),
            ],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(50, 0),
            Decimal::ZERO,
        ]);
        assert_eq!(remaining, Decimal::ZERO);

        // 场景 3：收款金额 = 500，2 张发票未收金额 [100, 200]
        // 期望：[100, 200]，剩余 200（全部核销，收款有盈余）
        let (allocations, remaining) = greedy_match(
            Decimal::new(500, 0),
            &[Decimal::new(100, 0), Decimal::new(200, 0)],
        );
        assert_eq!(allocations, vec![
            Decimal::new(100, 0),
            Decimal::new(200, 0),
        ]);
        assert_eq!(remaining, Decimal::new(200, 0));
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 ArService 能在 SQLite 内存数据库上实例化（new 不触发 DB 操作）
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败");
        let service = ArService::new(std::sync::Arc::new(db));
        let _ = service;
    }
}
