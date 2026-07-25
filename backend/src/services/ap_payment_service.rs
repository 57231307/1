//! 付款 Service
//!
//! 付款服务层，负责付款执行的核心业务逻辑
//! 包含付款单创建、确认、付款计划等管理

// 批次 100 P3-A 修复（v5 复审）：状态字符串常量化，引用 crate::models::status

use crate::models::{ap_invoice, ap_payment, ap_payment_request, ap_payment_request_item};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
// 批次 259 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
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
    /// 状态门加 lock_exclusive 防并发 confirm 导致 paid_amount 双扣（批次 16）
    pub async fn confirm(&self, id: i32, user_id: i32) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款单（加 lock_exclusive 串行化并发 confirm）
        let payment = Self::query_payment_for_confirm(&txn, id).await?;

        // 2-3. 校验状态与交易流水号
        Self::validate_payment_can_confirm(&payment)?;

        // 4. 标记付款单为已确认并写审计日志
        let payment = Self::mark_payment_confirmed(&txn, payment, user_id).await?;

        // 5. 更新关联应付单已付金额，收集已完全结清的发票（P0 5-1 修复）
        let fully_paid_invoices =
            Self::update_ap_invoices_paid_amount(&txn, &payment, user_id).await?;

        txn.commit().await?;

        // 6. 生成付款凭证（非阻断，失败仅 warn，与采购入库容错模式一致）
        self.generate_payment_voucher(&payment, user_id).await;

        // 7. 发布 PaymentCompleted 事件，触发 AP 发票自动标记 PAID（P0 5-1 修复）
        Self::publish_payment_completed_events(payment.id, fully_paid_invoices, user_id);

        // 8. 预算核销（非阻断，P2 5-22 修复：移除原 _request 死查询）
        self.write_off_payment_budget(&payment, user_id).await;

        // 9. 触发财务指标更新事件
        Self::publish_financial_indicator_event(&payment.payment_no);

        Ok(payment)
    }

    /// 查询付款单并加排他锁
    async fn query_payment_for_confirm(
        txn: &DatabaseTransaction,
        id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        ap_payment::Entity::find_by_id(id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款单 ID: {}", id)))
    }

    /// 校验付款单可确认（状态必须为 REGISTERED + 必须有交易流水号）
    fn validate_payment_can_confirm(payment: &ap_payment::Model) -> Result<(), AppError> {
        if payment.payment_status != crate::models::status::payment::PAYMENT_REGISTERED {
            return Err(AppError::business(format!(
                "付款单状态为{}，不可确认",
                payment.payment_status
            )));
        }
        if payment
            .transaction_no
            .as_deref()
            .is_none_or(|t| t.is_empty())
        {
            return Err(AppError::business(
                "付款单必须填写交易流水号才能确认".to_string(),
            ));
        }
        Ok(())
    }

    /// 标记付款单为已确认并写审计日志
    async fn mark_payment_confirmed(
        txn: &DatabaseTransaction,
        payment: ap_payment::Model,
        user_id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        let now = chrono::Utc::now();
        let mut payment_active: ap_payment::ActiveModel = payment.into();
        payment_active.payment_status =
            Set(crate::models::status::payment::PAYMENT_CONFIRMED.to_string());
        payment_active.confirmed_by = Set(Some(user_id));
        payment_active.confirmed_at = Set(Some(now));
        payment_active.updated_at = Set(now);
        // P1 1-1 修复：Some(0) 改 Some(user_id)，审计日志记录真实操作人
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            payment_active,
            Some(user_id),
        )
        .await
    }

    /// 查询付款申请明细并校验分摊总额（P1 3-12/5-6 修复：总额为 0 报错）
    async fn query_payment_request_items(
        txn: &DatabaseTransaction,
        request_id: i32,
    ) -> Result<(Vec<ap_payment_request_item::Model>, Decimal), AppError> {
        let items = ap_payment_request_item::Entity::find()
            .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
            .all(txn)
            .await?;
        let total_apply_amount: Decimal = items.iter().map(|item| item.apply_amount).sum();
        if total_apply_amount <= Decimal::ZERO && !items.is_empty() {
            return Err(AppError::business(
                "付款申请明细分摊总额必须大于 0，请检查申请明细的 apply_amount",
            ));
        }
        Ok((items, total_apply_amount))
    }

    /// 批量查询并锁定关联应付单（v16 批次 44 修复：避免循环内逐个 lock_exclusive）
    async fn lock_invoices_batch(
        txn: &DatabaseTransaction,
        invoice_ids: &[i32],
    ) -> Result<std::collections::HashMap<i32, ap_invoice::Model>, AppError> {
        if invoice_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        Ok(ap_invoice::Entity::find()
            .filter(ap_invoice::Column::Id.is_in(invoice_ids.to_vec()))
            .lock_exclusive()
            .all(txn)
            .await?
            .into_iter()
            .map(|inv| (inv.id, inv))
            .collect())
    }

    /// 分摊付款金额到单个应付单并更新状态，返回是否完全结清
    async fn apply_payment_to_invoice(
        txn: &DatabaseTransaction,
        mut inv: ap_invoice::Model,
        paid_amount: Decimal,
        user_id: i32,
    ) -> Result<bool, AppError> {
        inv.paid_amount = inv
            .paid_amount
            .checked_add(paid_amount)
            .unwrap_or(inv.paid_amount);
        inv.unpaid_amount = inv.amount.checked_sub(inv.paid_amount).unwrap_or(inv.amount);
        let became_fully_paid = inv.unpaid_amount <= Decimal::ZERO;
        inv.invoice_status = if became_fully_paid {
            crate::models::status::payment::PAYMENT_PAID.to_string()
        } else {
            crate::models::status::payment::PAYMENT_PARTIAL_PAID.to_string()
        };
        let invoice_active: ap_invoice::ActiveModel = inv.into();
        // P1 1-1 修复：Some(0) 改 Some(user_id)，审计日志记录真实操作人
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            invoice_active,
            Some(user_id),
        )
        .await?;
        Ok(became_fully_paid)
    }

    /// 更新关联应付单已付金额，返回已完全结清的 (invoice_id, paid_amount) 列表
    async fn update_ap_invoices_paid_amount(
        txn: &DatabaseTransaction,
        payment: &ap_payment::Model,
        user_id: i32,
    ) -> Result<Vec<(i32, Decimal)>, AppError> {
        let mut fully_paid_invoices: Vec<(i32, Decimal)> = Vec::new();
        let Some(request_id) = payment.request_id else {
            return Ok(fully_paid_invoices);
        };
        let (items, total_apply_amount) =
            Self::query_payment_request_items(txn, request_id).await?;
        if total_apply_amount <= Decimal::ZERO {
            return Ok(fully_paid_invoices);
        }
        let invoice_ids: Vec<i32> = items.iter().map(|item| item.invoice_id).collect();
        let mut invoice_map = Self::lock_invoices_batch(txn, &invoice_ids).await?;
        for item in items {
            let ratio = item
                .apply_amount
                .checked_div(total_apply_amount)
                .unwrap_or_default();
            let paid_amount = payment
                .payment_amount
                .checked_mul(ratio)
                .unwrap_or_default();
            if let Some(inv) = invoice_map.remove(&item.invoice_id) {
                let became_fully_paid =
                    Self::apply_payment_to_invoice(txn, inv, paid_amount, user_id).await?;
                if became_fully_paid {
                    fully_paid_invoices.push((item.invoice_id, paid_amount));
                }
            }
        }
        Ok(fully_paid_invoices)
    }

    /// 构建付款凭证借方分录（应付账款，挂供应商辅助核算）
    fn build_voucher_debit_item(
        payment_amount: Decimal,
        payment_no: &str,
        supplier_id: i32,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(1),
            subject_code: Some("2202".to_string()),
            subject_name: Some("应付账款".to_string()),
            debit: payment_amount,
            credit: Decimal::ZERO,
            summary: Some(format!("付款确认-{}", payment_no)),
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
        }
    }

    /// 构建付款凭证贷方分录（银行存款或库存现金）
    fn build_voucher_credit_item(
        payment_amount: Decimal,
        payment_no: &str,
        credit_code: &str,
        credit_name: &str,
    ) -> crate::services::voucher_service::VoucherItemRequest {
        crate::services::voucher_service::VoucherItemRequest {
            line_no: Some(2),
            subject_code: Some(credit_code.to_string()),
            subject_name: Some(credit_name.to_string()),
            debit: Decimal::ZERO,
            credit: payment_amount,
            summary: Some(format!("付款确认-{}", payment_no)),
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
        }
    }

    /// 构建付款凭证请求（借应付账款 / 贷银行存款或库存现金）
    fn build_payment_voucher_request(
        payment: &ap_payment::Model,
    ) -> crate::services::voucher_service::CreateVoucherRequest {
        let (credit_code, credit_name) = match payment.payment_method.as_str() {
            "CASH" => ("1001", "库存现金"),
            _ => ("1002", "银行存款"),
        };
        let payment_no = payment.payment_no.clone();
        crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "付".to_string(),
            voucher_date: payment.payment_date,
            source_type: Some("AP_PAYMENT".to_string()),
            source_module: Some("ap".to_string()),
            source_bill_id: Some(payment.id),
            source_bill_no: Some(payment_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                Self::build_voucher_debit_item(
                    payment.payment_amount,
                    &payment_no,
                    payment.supplier_id,
                ),
                Self::build_voucher_credit_item(
                    payment.payment_amount,
                    &payment_no,
                    credit_code,
                    credit_name,
                ),
            ],
        }
    }

    /// 生成付款凭证（非阻断，失败仅 warn）
    async fn generate_payment_voucher(&self, payment: &ap_payment::Model, user_id: i32) {
        let voucher_req = Self::build_payment_voucher_request(payment);
        let voucher_service = crate::services::voucher_service::VoucherService::new(self.db.clone());
        if let Err(e) = voucher_service.create_and_post(voucher_req, user_id).await {
            tracing::warn!(
                "付款单 {} 确认成功，但生成付款凭证失败：{}",
                payment.payment_no,
                e
            );
        }
    }

    /// 发布 PaymentCompleted 事件（触发 AP 发票自动标记 PAID）
    fn publish_payment_completed_events(
        payment_id: i32,
        fully_paid_invoices: Vec<(i32, Decimal)>,
        user_id: i32,
    ) {
        for (invoice_id, paid_amount) in fully_paid_invoices {
            crate::services::event_bus::EVENT_BUS.publish(
                crate::services::event_bus::BusinessEvent::PaymentCompleted {
                    payment_id,
                    invoice_id,
                    amount: paid_amount,
                    user_id,
                },
            );
        }
    }

    /// 通过付款申请查找关联应付单的部门 ID（来源 PURCHASE_RECEIPT）
    async fn lookup_department_for_payment(&self, request_id: i32) -> Option<i32> {
        let items = ap_payment_request_item::Entity::find()
            .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
            .all(&*self.db)
            .await
            .ok()?;
        let first_item = items.first()?;
        let invoice = ap_invoice::Entity::find_by_id(first_item.invoice_id)
            .one(&*self.db)
            .await
            .ok()??;
        if invoice.source_type.as_deref() != Some("PURCHASE_RECEIPT") {
            return Some(1);
        }
        let Some(receipt_id) = invoice.source_id else {
            return Some(1);
        };
        let receipt = crate::models::purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await
            .ok()
            .flatten();
        Some(receipt.map(|r| r.department_id.unwrap_or(1)).unwrap_or(1))
    }

    /// 执行预算核销（非阻断，失败仅 warn）
    async fn write_off_payment_budget(&self, payment: &ap_payment::Model, user_id: i32) {
        let Some(request_id) = payment.request_id else {
            return;
        };
        let Some(department_id) = self.lookup_department_for_payment(request_id).await else {
            return;
        };
        let budget_service =
            crate::services::budget_management_service::BudgetManagementService::new(
                self.db.clone(),
            );
        match budget_service
            .get_available_plan_by_department(department_id)
            .await
        {
            Ok(Some(plan)) => {
                if let Err(e) = budget_service
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
                    tracing::warn!("付款单 {} 预算核销失败：{}", payment.payment_no, e);
                } else {
                    tracing::info!(
                        "付款单 {} 预算核销成功，部门ID={}, 方案ID={}, 金额={}",
                        payment.payment_no, department_id, plan.id, payment.payment_amount
                    );
                }
            }
            Ok(None) => tracing::warn!(
                "付款单 {} 未找到部门 {} 的预算方案，跳过预算核销",
                payment.payment_no,
                department_id
            ),
            Err(e) => tracing::warn!(
                "付款单 {} 查询预算方案失败：{}，跳过预算核销",
                payment.payment_no,
                e
            ),
        }
    }

    /// 发布 FinancialIndicatorUpdate 事件（触发财务指标重算）
    fn publish_financial_indicator_event(payment_no: &str) {
        let now_date = chrono::Utc::now().date_naive();
        let period = format!("{:04}-{:02}", now_date.year(), now_date.month());
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::FinancialIndicatorUpdate {
                period,
                trigger_source: format!("payment_completed:{}", payment_no),
            },
        );
    }

    /// 获取付款单详情
    pub async fn get_by_id(
        &self,
        id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<ap_payment::Model, AppError> {
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款单 {}", id)))?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // ap_payment 表无 department_id，Dept 退化为 Self；
        // ap_payment.created_by 是 i32（必填）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(payment.created_by), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问付款单 {}（数据范围限制）", id
                )));
            }
        }
        Ok(payment)
    }

    /// 获取付款单列表
    pub async fn get_list(
        &self,
        params: ApPaymentListQuery,
        data_scope: Option<&DataScopeContext>,
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

        // V15 P0-S01：行级数据权限过滤
        // ap_payment 表无 department_id，Dept 退化为 Self；
        // ap_payment.created_by 是 i32（必填）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                ap_payment::Column::CreatedBy,
                ap_payment::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
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
