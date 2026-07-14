//! 付款 Service
//!
//! 付款服务层，负责付款执行的核心业务逻辑
//! 包含付款单创建、确认、付款计划等管理

// 批次 100 P3-A 修复（v5 复审）：状态字符串常量化，引用 crate::models::status

use crate::models::{ap_invoice, ap_payment, ap_payment_request, ap_payment_request_item};
use crate::utils::error::AppError;
// 批次 259 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 付款服务
pub struct ApPaymentService {
    db: Arc<DatabaseConnection>,
}

/// 付款单列表查询参数（service 层，page/page_size 已解析为非 Option）
#[derive(Debug, Clone)]
pub struct ApPaymentListQuery {
    pub supplier_id: Option<i32>,
    pub payment_status: Option<String>,
    pub payment_method: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: u64,
    pub page_size: u64,
}

impl ApPaymentService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成付款单号
    // 格式：PAY + 年月日 + 三位序号（PAY20260315001）
    crate::impl_generate_no!(
        generate_payment_no,
        "PAY",
        ap_payment::Entity,
        ap_payment::Column::PaymentNo
    );

    /// 创建付款单（从审批通过的付款申请）
    pub async fn create(
        &self,
        req: CreateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成付款单号
        let payment_no = self.generate_payment_no().await?;

        // 2. 检查付款申请是否存在且已审批
        let request = ap_payment_request::Entity::find_by_id(req.request_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 {}", req.request_id)))?;

        if request.approval_status != crate::models::status::common::STATUS_APPROVED {
            return Err(AppError::business(format!(
                "付款申请状态为{}，未审批通过不可创建付款单",
                request.approval_status
            )));
        }

        // 3. 检查是否已创建过付款单
        let exists = ap_payment::Entity::find()
            .filter(ap_payment::Column::RequestId.eq(Some(req.request_id)))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::business("该付款申请已创建过付款单".to_string()));
        }

        // 4. 创建付款单
        let payment = ap_payment::ActiveModel {
            payment_no: Set(payment_no),
            payment_date: Set(req.payment_date),
            supplier_id: Set(request.supplier_id),
            request_id: Set(Some(req.request_id)),
            payment_method: Set(request.payment_method.clone()),
            payment_amount: Set(request.request_amount),
            payment_status: Set(crate::models::status::payment::PAYMENT_REGISTERED.to_string()),
            currency: Set(request.currency.clone()),
            exchange_rate: Set(request.exchange_rate),
            bank_name: Set(request.bank_name.clone()),
            bank_account: Set(request.bank_account.clone()),
            notes: Set(req.notes.or(request.notes)),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(payment)
    }

    /// 更新付款单（仅已登记状态）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款单
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款单 {}", id)))?;

        // 2. 检查状态（仅已登记可修改）
        if payment.payment_status != crate::models::status::payment::PAYMENT_REGISTERED {
            return Err(AppError::business(format!(
                "付款单状态为{}，不可修改",
                payment.payment_status
            )));
        }

        // 3. 更新付款单
        let mut payment_active: ap_payment::ActiveModel = payment.into();

        if let Some(payment_date) = req.payment_date {
            payment_active.payment_date = Set(payment_date);
        }
        if let Some(payment_method) = req.payment_method {
            payment_active.payment_method = Set(payment_method);
        }
        if let Some(bank_name) = req.bank_name {
            payment_active.bank_name = Set(Some(bank_name));
        }
        if let Some(bank_account) = req.bank_account {
            payment_active.bank_account = Set(Some(bank_account));
        }
        if let Some(transaction_no) = req.transaction_no {
            payment_active.transaction_no = Set(Some(transaction_no));
        }
        if let Some(notes) = req.notes {
            payment_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            payment_active.attachment_urls = Set(Some(attachment_urls));
        }

        payment_active.updated_by = Set(Some(user_id));

        let payment = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            payment_active,
            // P1 1-1 修复：Some(0) 改 Some(user_id)，审计日志记录真实操作人
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(payment)
    }

    /// 确认付款（执行支付）
    ///
    /// 批次 16（2026-06-28）：付款单状态门查询加 lock_exclusive，
    /// 防止并发 confirm 同一付款单导致 ap_invoice paid_amount 重复累加（资金双重支付风险）。
    /// 原状态门无锁，两并发 confirm 均通过 REGISTERED 检查，第二个 confirm 在 invoice lock 后
    /// 读取已更新的 paid_amount 再次累加，导致应付单已付金额翻倍。
    pub async fn confirm(&self, id: i32, user_id: i32) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款单（加 lock_exclusive 串行化并发 confirm）
        let payment = ap_payment::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款单 ID: {}", id)))?;

        // 2. 检查状态
        if payment.payment_status != crate::models::status::payment::PAYMENT_REGISTERED {
            return Err(AppError::business(format!(
                "付款单状态为{}，不可确认",
                payment.payment_status
            )));
        }

        // 3. 检查必要字段
        if payment
            .transaction_no
            .as_deref()
            .is_none_or(|t| t.is_empty())
        {
            return Err(AppError::business(
                "付款单必须填写交易流水号才能确认".to_string(),
            ));
        }

        // 4. 确认付款
        let now = chrono::Utc::now();
        let mut payment_active: ap_payment::ActiveModel = payment.into();
        payment_active.payment_status = Set(crate::models::status::payment::PAYMENT_CONFIRMED.to_string());
        payment_active.confirmed_by = Set(Some(user_id));
        payment_active.confirmed_at = Set(Some(now));
        payment_active.updated_at = Set(now);

        let payment = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            payment_active,
            // P1 1-1 修复：Some(0) 改 Some(user_id)，审计日志记录真实操作人
            Some(user_id),
        )
        .await?;

        // P0 5-1 修复：收集本次付款完全结清的应付单（invoice_id, 分摊已付金额），
        // 在 commit 成功后补发 PaymentCompleted 事件，触发 event_bus 监听器自动标记 PAID
        let mut fully_paid_invoices: Vec<(i32, Decimal)> = Vec::new();

        // 5. 更新关联的应付单已付金额
        if let Some(request_id) = payment.request_id {
            // 查询付款申请明细
            let items = ap_payment_request_item::Entity::find()
                .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
                .all(&txn)
                .await?;

            // 计算每个应付单应分摊的付款金额（按申请金额比例）
            let total_apply_amount: Decimal = items.iter().map(|item| item.apply_amount).sum();

            // P1 3-12/5-6 修复（批次 63）：分摊总额为 0 时提前报错
            // 原实现 total_apply_amount==0 时 unwrap_or_default 返回 0，paid_amount=0 但状态改 PARTIAL_PAID，
            // 导致应付单状态与金额不一致（无付款却标记部分付款）。
            if total_apply_amount <= Decimal::ZERO && !items.is_empty() {
                return Err(AppError::business(
                    "付款申请明细分摊总额必须大于 0，请检查申请明细的 apply_amount",
                ));
            }

            // v16 批次 44 修复：循环外批量查询并锁定所有关联的应付单，避免循环内逐个 lock_exclusive（N+1）
            let invoice_ids: Vec<i32> = items.iter().map(|item| item.invoice_id).collect();
            let mut invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
                if total_apply_amount <= Decimal::ZERO || invoice_ids.is_empty() {
                    std::collections::HashMap::new()
                } else {
                    use sea_orm::QuerySelect;
                    ap_invoice::Entity::find()
                        .filter(ap_invoice::Column::Id.is_in(invoice_ids))
                        .lock_exclusive()
                        .all(&txn)
                        .await?
                        .into_iter()
                        .map(|inv| (inv.id, inv))
                        .collect()
                };

            for item in items {
                if total_apply_amount > Decimal::ZERO {
                    let ratio = item
                        .apply_amount
                        .checked_div(total_apply_amount)
                        .unwrap_or_default();
                    let paid_amount = payment
                        .payment_amount
                        .checked_mul(ratio)
                        .unwrap_or_default();

                    // v16 批次 44 修复：从批量查询结果获取应付单（O(1) 查找）
                    if let Some(mut inv) = invoice_map.remove(&item.invoice_id) {
                        inv.paid_amount = inv
                            .paid_amount
                            .checked_add(paid_amount)
                            .unwrap_or(inv.paid_amount);
                        inv.unpaid_amount = inv
                            .amount
                            .checked_sub(inv.paid_amount)
                            .unwrap_or(inv.amount);

                        // 更新应付状态
                        let became_fully_paid = inv.unpaid_amount <= Decimal::ZERO;
                        inv.invoice_status = if became_fully_paid {
                            crate::models::status::payment::PAYMENT_PAID.to_string()
                        } else {
                            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
                        };

                        let invoice_active: ap_invoice::ActiveModel = inv.into();
                        crate::services::audit_log_service::AuditLogService::update_with_audit(
                            &txn,
                            "auto_audit",
                            invoice_active,
                            // P1 1-1 修复：Some(0) 改 Some(user_id)，审计日志记录真实操作人
                            Some(user_id),
                        )
                        .await?;

                        // P0 5-1 修复：记录已完全结清的应付单，commit 后补发 PaymentCompleted 事件
                        if became_fully_paid {
                            fully_paid_invoices.push((item.invoice_id, paid_amount));
                        }
                    }
                }
            }
        }

        txn.commit().await?;

        // F-P0-5 修复（批次 381 v13 复审）：确认付款后生成付款凭证
        // 借：应付账款（挂供应商辅助核算）/ 贷：银行存款/库存现金
        // 失败时仅 warn 不阻断主流程（与采购入库容错模式一致）
        let payment_amount = payment.payment_amount;
        let (credit_code, credit_name) = match payment.payment_method.as_str() {
            "CASH" => ("1001", "库存现金"),
            _ => ("1002", "银行存款"),
        };
        let payment_no_clone = payment.payment_no.clone();
        let payment_date = payment.payment_date;
        let payment_id_val = payment.id;
        let supplier_id = payment.supplier_id;
        let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "付".to_string(),
            voucher_date: payment_date,
            source_type: Some("AP_PAYMENT".to_string()),
            source_module: Some("ap".to_string()),
            source_bill_id: Some(payment_id_val),
            source_bill_no: Some(payment_no_clone.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(1),
                    subject_code: Some("2202".to_string()),
                    subject_name: Some("应付账款".to_string()),
                    debit: payment_amount,
                    credit: Decimal::ZERO,
                    summary: Some(format!("付款确认-{}", payment_no_clone)),
                    assist_customer_id: None,
                    assist_supplier_id: Some(supplier_id),
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
                    subject_code: Some(credit_code.to_string()),
                    subject_name: Some(credit_name.to_string()),
                    debit: Decimal::ZERO,
                    credit: payment_amount,
                    summary: Some(format!("付款确认-{}", payment_no_clone)),
                    assist_customer_id: None,
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
                "付款单 {} 确认成功，但生成付款凭证失败：{}",
                payment_no_clone,
                e
            );
        }

        // P0 5-1 修复：commit 成功后补发 PaymentCompleted 事件，
        // 触发 event_bus 监听器（event_bus.rs）自动将关联 AP 发票标记为 PAID
        // 批次 97 P1-2 修复：透传付款操作人 user_id，供 mark_as_paid 审计日志使用
        for (invoice_id, paid_amount) in fully_paid_invoices {
            crate::services::event_bus::EVENT_BUS.publish(
                crate::services::event_bus::BusinessEvent::PaymentCompleted {
                    payment_id: payment.id,
                    invoice_id,
                    amount: paid_amount,
                    user_id,
                },
            );
        }

        // 6. 预算核销（非阻断）
        // 通过付款申请找到关联的应付单，再通过应付单的来源找到采购入库单的部门信息
        // P2 5-22 修复：移除原 _request 死查询（结果未使用），串行查询链由 4 次降为 3 次
        if let Some(request_id) = payment.request_id {
            // 查询付款申请明细，获取关联的应付单（原实现先查 request 主表但结果 _request 未使用，已移除）
            if let Ok(items) = ap_payment_request_item::Entity::find()
                .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
                .all(&*self.db)
                .await
            {
                // 获取第一个应付单的部门信息
                if let Some(first_item) = items.first() {
                    if let Ok(Some(invoice)) =
                        ap_invoice::Entity::find_by_id(first_item.invoice_id)
                            .one(&*self.db)
                            .await
                    {
                        // 从应付单的 source_type 和 source_id 找到采购入库单的部门
                        let department_id =
                            if invoice.source_type.as_deref() == Some("PURCHASE_RECEIPT") {
                                if let Some(receipt_id) = invoice.source_id {
                                    if let Ok(Some(receipt)) =
                                        crate::models::purchase_receipt::Entity::find_by_id(
                                            receipt_id,
                                        )
                                        .one(&*self.db)
                                        .await
                                    {
                                        receipt.department_id.unwrap_or(1)
                                    } else {
                                        1
                                    }
                                } else {
                                    1
                                }
                            } else {
                                1
                            };

                        // 查找预算方案
                        let budget_service = crate::services::budget_management_service::BudgetManagementService::new(self.db.clone());
                        match budget_service
                            .get_available_plan_by_department(department_id)
                            .await
                        {
                            Ok(Some(plan)) => {
                                // 核销预算
                                match budget_service
                                    .write_off_budget(
                                        department_id,
                                        plan.id,
                                        payment.payment_amount,
                                        "ap_payment".to_string(),
                                        payment.id,
                                        user_id,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        tracing::info!(
                                            "付款单 {} 预算核销成功，部门ID={}, 方案ID={}, 金额={}",
                                            payment.payment_no, department_id, plan.id, payment.payment_amount
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "付款单 {} 预算核销失败：{}",
                                            payment.payment_no,
                                            e
                                        );
                                    }
                                }
                            }
                            Ok(None) => {
                                tracing::warn!(
                                    "付款单 {} 未找到部门 {} 的预算方案，跳过预算核销",
                                    payment.payment_no,
                                    department_id
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "付款单 {} 查询预算方案失败：{}，跳过预算核销",
                                    payment.payment_no,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        // 触发财务指标更新事件
        let now_date = chrono::Utc::now().date_naive();
        let period = format!("{:04}-{:02}", now_date.year(), now_date.month());
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source: format!("payment_completed:{}", payment.payment_no),
            },
        );

        Ok(payment)
    }

    /// 获取付款单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_payment::Model, AppError> {
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款单 {}", id)))?;

        Ok(payment)
    }

    /// 获取付款单列表
    pub async fn get_list(
        &self,
        params: ApPaymentListQuery,
    ) -> Result<(Vec<ap_payment::Model>, u64), AppError> {
        let mut query = ap_payment::Entity::find();

        // 筛选条件
        if let Some(sid) = params.supplier_id {
            query = query.filter(ap_payment::Column::SupplierId.eq(sid));
        }
        if let Some(status) = params.payment_status {
            query = query.filter(ap_payment::Column::PaymentStatus.eq(status));
        }
        if let Some(method) = params.payment_method {
            query = query.filter(ap_payment::Column::PaymentMethod.eq(method));
        }
        if let Some(sd) = params.start_date {
            query = query.filter(ap_payment::Column::PaymentDate.gte(sd));
        }
        if let Some(ed) = params.end_date {
            query = query.filter(ap_payment::Column::PaymentDate.lte(ed));
        }

        // 批次 259 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(ap_payment::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, params.page_size);

        let (items, total) = paginate_with_total(paginator, params.page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 获取付款计划（按供应商和日期）
    pub async fn get_payment_schedule(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PaymentScheduleItem>, AppError> {
        let mut query = ap_payment_request::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_payment_request::Column::SupplierId.eq(sid));
        }

        // 查询已审批的付款申请
        let requests = query
            .filter(ap_payment_request::Column::ApprovalStatus.eq(crate::models::status::common::STATUS_APPROVED))
            .filter(ap_payment_request::Column::ExpectedPaymentDate.between(start_date, end_date))
            .order_by(ap_payment_request::Column::ExpectedPaymentDate, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut schedule_map: std::collections::BTreeMap<NaiveDate, PaymentScheduleItem> =
            std::collections::BTreeMap::new();

        for request in requests {
            let date = request
                .expected_payment_date
                .unwrap_or(request.request_date);

            let entry = schedule_map
                .entry(date)
                .or_insert_with(|| PaymentScheduleItem {
                    payment_date: date,
                    total_amount: Decimal::ZERO,
                    payment_count: 0,
                });

            entry.total_amount += request.request_amount;
            entry.payment_count += 1;
        }

        Ok(schedule_map.into_values().collect())
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建付款单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateApPaymentRequest {
    /// 付款申请 ID
    pub request_id: i32,

    /// 付款日期
    pub payment_date: NaiveDate,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表（付款凭证）
    pub attachment_urls: Option<Vec<String>>,
}

/// 更新付款单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateApPaymentRequest {
    /// 付款日期
    pub payment_date: Option<NaiveDate>,

    /// 付款方式
    pub payment_method: Option<String>,

    /// 付款银行
    pub bank_name: Option<String>,

    /// 付款账号
    pub bank_account: Option<String>,

    /// 交易流水号
    pub transaction_no: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}

/// 付款计划项
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentScheduleItem {
    /// 付款日期
    pub payment_date: NaiveDate,

    /// 总金额
    pub total_amount: Decimal,

    /// 付款单数量
    pub payment_count: i64,
}
