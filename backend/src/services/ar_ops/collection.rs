//! 应收账款-收款管理子模块（ar_ops/collection）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L112-751 迁移。
//! 包含 17 个收款管理方法：
//! - list_payments / get_payment / create_payment（公开 API）
//! - validate_payment_amount / check_payment_period_locked / load_customer_for_payment
//! - generate_collection_no / build_collection_active_model / build_and_insert_collection
//! - link_invoices_to_payment / allocate_payment_to_invoice / publish_payment_events
//! - update_payment / confirm_payment（公开 API）
//! - cancel_collection（公开 API）/ validate_cancel_collection / build_cancelled_active
//!
//! 业务规则：
//! - 收款基于 ar_collection 表
//! - 状态机：pending → confirmed（不可逆）/ pending → cancelled
//! - 金额校验 round_dp(2) 限制货币精度
//! - 期间锁定检查通过 AccountingPeriodService::check_date_locked_txn
//! - 所有写操作在事务内执行，状态变更加 lock_exclusive 串行化

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
// 批次 389 P2-2：补充 warn/error 日志宏，关键操作失败场景补审计日志
use tracing::{info, warn};

use crate::models::{ar_collection, ar_invoice, ar_reconciliation_item};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

use super::json_helpers::collection_to_json;
use super::types::{CollectionBuildContext, CreateArPaymentParams};
use crate::services::ar_service::ArService;

impl ArService {
    /// 获取收款列表
    /// 基于 ar_collection 表分页查询，支持状态/客户/收款单号过滤
    pub async fn list_payments(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        customer_id: Option<i32>,
        payment_no: Option<String>,
        data_scope: Option<&DataScopeContext>,
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

        // V15 P0-S01：行级数据权限过滤
        // ar_collection 表无 department_id，Dept 退化为 Self，使用 created_by（i32 必填）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                ar_collection::Column::CreatedBy,
                ar_collection::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
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
    pub async fn get_payment(
        &self,
        payment_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let payment = ar_collection::Entity::find_by_id(payment_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", payment_id)))?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // ar_collection 表无 department_id，Dept 退化为 Self；
        // ar_collection.created_by 是 i32（必填）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(payment.created_by), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问收款单 {}（数据范围限制）",
                    payment_id
                )));
            }
        }
        Ok(collection_to_json(payment))
    }

    /// 创建收款：金额校验→期间锁定→客户校验→单号生成→插入收款→关联发票→事件发布
    /// 批次 488 D08-1 拆分：主函数仅做协调，细节逻辑提取到 helper
    pub async fn create_payment(
        &self,
        params: CreateArPaymentParams,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_payment_amount(params.customer_id, params.amount)?;

        let txn = (*self.db).begin().await?;

        // 期间锁定检查（事务内，避免 TOCTOU）
        self.check_payment_period_locked(&txn, params.payment_date)
            .await?;

        let customer = Self::load_customer_for_payment(&txn, params.customer_id).await?;

        let collection_no = Self::generate_collection_no(&txn).await?;

        let now = Utc::now();
        let collection_model = Self::build_and_insert_collection(
            &txn,
            collection_no,
            &params,
            customer.customer_name,
            user_id,
            now,
        )
        .await?;

        // 关联多张发票：累加 received_amount、扣减 unpaid_amount、按需更新状态
        let linked_invoices = Self::link_invoices_to_payment(
            &txn,
            params.invoice_ids,
            params.amount,
            params.customer_id,
            user_id,
            now,
        )
        .await?;

        txn.commit().await?;

        Self::publish_payment_events(
            &collection_model,
            &linked_invoices,
            params.amount,
            user_id,
            params.payment_date,
        );

        Ok(collection_to_json(collection_model))
    }

    /// 金额校验：必须大于零且精度不超过 2 位小数
    fn validate_payment_amount(customer_id: i32, amount: Decimal) -> Result<(), AppError> {
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
        Ok(())
    }

    /// 期间锁定检查（事务内，避免 TOCTOU）
    async fn check_payment_period_locked(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        payment_date: NaiveDate,
    ) -> Result<(), AppError> {
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc
            .check_date_locked_txn(txn, payment_date)
            .await
            .map_err(|e| AppError::business(e.to_string()))
    }

    /// 客户存在性校验 + 名称查询（事务内）
    async fn load_customer_for_payment(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
    ) -> Result<crate::models::customer::Model, AppError> {
        crate::models::customer::Entity::find_by_id(customer_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))
    }

    /// 单号生成（事务内，advisory_xact_lock 串行化）
    async fn generate_collection_no(
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<String, AppError> {
        crate::utils::number_generator::DocumentNumberGenerator::generate_no(
            txn,
            "COL",
            ar_collection::Entity,
            ar_collection::Column::CollectionNo,
        )
        .await
    }

    /// 构造收款单 ActiveModel（纯函数，无 IO）
    fn build_collection_active_model(ctx: CollectionBuildContext) -> ar_collection::ActiveModel {
        ar_collection::ActiveModel {
            collection_no: Set(ctx.collection_no),
            collection_date: Set(ctx.payment_date),
            customer_id: Set(ctx.customer_id),
            customer_name: Set(ctx.customer_name),
            collection_amount: Set(ctx.amount),
            collection_method: Set(Some(ctx.payment_method)),
            bank_account: Set(ctx.bank_account),
            status: Set(crate::models::status::ar::COLLECTION_PENDING.to_string()),
            created_by: Set(ctx.user_id),
            created_at: Set(ctx.now),
            updated_at: Set(ctx.now),
            ..Default::default()
        }
    }

    /// 构造收款单上下文并插入：复用 CollectionBuildContext + build_collection_active_model
    async fn build_and_insert_collection(
        txn: &sea_orm::DatabaseTransaction,
        collection_no: String,
        params: &CreateArPaymentParams,
        customer_name: String,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<ar_collection::Model, AppError> {
        let ctx = CollectionBuildContext {
            collection_no,
            payment_date: params.payment_date,
            customer_id: params.customer_id,
            customer_name: Some(customer_name),
            amount: params.amount,
            payment_method: params.payment_method.clone(),
            bank_account: params.bank_account.clone(),
            user_id,
            now,
        };
        Ok(Self::build_collection_active_model(ctx).insert(txn).await?)
    }

    /// 关联多张发票：累加 received_amount、扣减 unpaid_amount、按需更新状态
    async fn link_invoices_to_payment(
        txn: &sea_orm::DatabaseTransaction,
        invoice_ids: Option<Vec<i32>>,
        amount: Decimal,
        customer_id: i32,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<i32>, AppError> {
        let mut linked_invoices: Vec<i32> = Vec::new();
        if let Some(inv_ids) = invoice_ids {
            // 批量查询所有发票并加锁，避免循环内 N+1
            let invoice_map: std::collections::HashMap<i32, ar_invoice::Model> = if inv_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                ar_invoice::Entity::find()
                    .filter(ar_invoice::Column::Id.is_in(inv_ids.clone()))
                    .lock_exclusive()
                    .all(txn)
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
                let linked = Self::allocate_payment_to_invoice(
                    invoice,
                    &mut remaining,
                    customer_id,
                    user_id,
                    now,
                    txn,
                )
                .await?;
                if linked {
                    linked_invoices.push(inv_id);
                }
            }
        }
        Ok(linked_invoices)
    }

    /// 单张发票的扣减分配：校验 + 计算 allocate + 更新 received/unpaid/status
    async fn allocate_payment_to_invoice(
        invoice: &ar_invoice::Model,
        remaining: &mut Decimal,
        customer_id: i32,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<bool, AppError> {
        if invoice.status == crate::models::status::common::STATUS_CANCELLED {
            return Err(AppError::bad_request(format!(
                "应收单 {} 已取消，无法关联收款",
                invoice.id
            )));
        }
        if invoice.customer_id != customer_id {
            return Err(AppError::bad_request(format!(
                "应收单 {} 客户与收款客户不一致",
                invoice.id
            )));
        }
        let allocate = (*remaining).min(invoice.unpaid_amount);
        if allocate <= Decimal::ZERO {
            return Ok(false);
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
        >(txn, "ar_invoice", active, Some(user_id))
        .await?;

        *remaining -= allocate;
        Ok(true)
    }

    /// 事件发布（commit 后，避免事件处理器回写导致事务膨胀）
    fn publish_payment_events(
        collection: &ar_collection::Model,
        linked_invoices: &[i32],
        amount: Decimal,
        user_id: i32,
        payment_date: NaiveDate,
    ) {
        info!(
            "AR 收款创建成功：collection_no={}, customer_id={}, amount={}, 关联发票数={}",
            collection.collection_no,
            collection.customer_id,
            amount,
            linked_invoices.len()
        );
        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        for inv_id in linked_invoices {
            EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
                collection_id: collection.id,
                invoice_id: Some(*inv_id),
                amount,
                user_id,
            });
        }
        let period = format!("{:04}-{:02}", payment_date.year(), payment_date.month());
        EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source: format!("ar_collection_completed:{}", collection.collection_no),
        });
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

        // 状态门 + 核销引用校验
        Self::validate_cancel_collection(&txn, &collection, payment_id, user_id).await?;

        let active = Self::build_cancelled_active(collection);

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

    /// 校验收款单取消条件：状态必须为 pending，且未被核销单引用
    async fn validate_cancel_collection(
        txn: &sea_orm::DatabaseTransaction,
        collection: &ar_collection::Model,
        payment_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        if collection.status != crate::models::status::ar::COLLECTION_PENDING {
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

        let referenced_count = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
            .filter(ar_reconciliation_item::Column::DocumentId.eq(payment_id))
            .count(txn)
            .await?;
        if referenced_count > 0 {
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
        Ok(())
    }

    /// 构建取消状态的 ActiveModel（status=cancelled，清空 confirmed_by/at）
    fn build_cancelled_active(collection: ar_collection::Model) -> ar_collection::ActiveModel {
        let now = Utc::now();
        let mut active: ar_collection::ActiveModel = collection.into();
        active.status = Set(crate::models::status::ar::COLLECTION_CANCELLED.to_string());
        active.confirmed_by = Set(None);
        active.confirmed_at = Set(None);
        active.updated_at = Set(now);
        active
    }
}
