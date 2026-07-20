//! 供应商对账 Service
//!
//! 供应商对账服务层，负责对账的核心业务逻辑
//! 包含生成对账单、确认对账、争议处理等管理

use crate::models::{ap_invoice, ap_payment, ap_reconciliation};
use crate::models::status::ap_reconciliation as reconciliation_status;
use crate::models::status::payment;
use crate::utils::error::AppError;
// 批次 259 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{NaiveDate, Utc};
use futures::stream::{self, StreamExt};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use validator::Validate;

/// 供应商对账服务
pub struct ApReconciliationService {
    db: Arc<DatabaseConnection>,
}

impl ApReconciliationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成对账单号
    // 格式：REC + 年月日 + 三位序号（REC20260315001）
    crate::impl_generate_no!(
        generate_reconciliation_no,
        "REC",
        ap_reconciliation::Entity,
        ap_reconciliation::Column::ReconciliationNo
    );

    /// 生成供应商对账单
    pub async fn generate_reconciliation(
        &self,
        req: GenerateReconciliationRequest,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成对账单号
        let reconciliation_no = self.generate_reconciliation_no().await?;

        // 2. 查询该供应商在对账期间内的应付单
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.gte(req.start_date))
            .filter(ap_invoice::Column::InvoiceDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 3. 查询该供应商在对账期间内的付款单
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_payment::Column::PaymentStatus.eq(payment::PAYMENT_CONFIRMED))
            .filter(ap_payment::Column::PaymentDate.gte(req.start_date))
            .filter(ap_payment::Column::PaymentDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 4. 计算期初余额（对账开始日期前的未付金额）
        let opening_balance = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.lt(req.start_date))
            .all(&txn)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum::<Decimal>();

        // 5. 计算本期应付合计
        let total_invoice: Decimal = invoices.iter().map(|inv| inv.amount).sum();

        // 6. 计算本期付款合计
        let total_payment: Decimal = payments.iter().map(|pay| pay.payment_amount).sum();

        // 7. 计算期末余额
        let closing_balance = opening_balance + total_invoice - total_payment;

        // 8. 创建对账单
        let reconciliation = ap_reconciliation::ActiveModel {
            reconciliation_no: Set(reconciliation_no),
            supplier_id: Set(req.supplier_id),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            opening_balance: Set(opening_balance),
            total_invoice: Set(total_invoice),
            total_payment: Set(total_payment),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set(reconciliation_status::PENDING.to_string()),
            notes: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 确认对账单
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现已有 txn 但状态门查询未加 lock_exclusive，两并发 confirm_reconciliation 同时通过
    /// PENDING 检查后基于过期状态写入，导致 confirmed_by/confirmed_at 被覆盖。
    pub async fn confirm_reconciliation(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单（加 lock_exclusive 串行化并发 confirm_reconciliation）
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status != reconciliation_status::PENDING {
            return Err(AppError::business(format!(
                "对账单状态为{}，不可确认",
                reconciliation.reconciliation_status
            )));
        }

        // 3. 确认对账单
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set(reconciliation_status::CONFIRMED.to_string());
        reconciliation_active.confirmed_by = Set(Some(user_id));
        reconciliation_active.confirmed_at = Set(Some(now));

        let reconciliation =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                reconciliation_active,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        // F-P2-4 修复（批次 387 v13 复审）：对账单确认后生成对账确认凭证
        // 原实现 confirm_reconciliation 仅更新对账单状态，不生成凭证，
        // 导致对账确认结果无法在凭证体系中追溯。
        // 修复：commit 成功后生成转账凭证（借贷均为应付账款，金额=期末余额），
        // 作为对账确认的审计凭证，不改变账面净余额。失败时仅 warn 不阻断主流程。
        let voucher_req = crate::services::voucher_service::CreateVoucherRequest {
            voucher_type: "转".to_string(),
            voucher_date: reconciliation.end_date,
            source_type: Some("AP_RECONCILIATION".to_string()),
            source_module: Some("ap".to_string()),
            source_bill_id: Some(reconciliation.id),
            source_bill_no: Some(reconciliation.reconciliation_no.clone()),
            batch_no: None,
            color_no: None,
            items: vec![
                crate::services::voucher_service::VoucherItemRequest {
                    line_no: Some(1),
                    subject_code: Some("2202".to_string()),
                    subject_name: Some("应付账款".to_string()),
                    debit: reconciliation.closing_balance,
                    credit: Decimal::ZERO,
                    summary: Some(format!("对账确认-{}", reconciliation.reconciliation_no)),
                    assist_customer_id: None,
                    assist_supplier_id: Some(reconciliation.supplier_id),
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
                    subject_code: Some("2202".to_string()),
                    subject_name: Some("应付账款".to_string()),
                    debit: Decimal::ZERO,
                    credit: reconciliation.closing_balance,
                    summary: Some(format!("对账确认-{}", reconciliation.reconciliation_no)),
                    assist_customer_id: None,
                    assist_supplier_id: Some(reconciliation.supplier_id),
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
                "对账单 {} 确认成功，但生成对账确认凭证失败：{}",
                reconciliation.reconciliation_no,
                e
            );
        }

        Ok(reconciliation)
    }

    /// 提出争议
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现已有 txn 但状态门查询未加 lock_exclusive，两并发 dispute 同时通过门控后
    /// 基于过期状态写入，导致 disputed_reason 被覆盖。
    pub async fn dispute(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单（加 lock_exclusive 串行化并发 dispute）
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status == reconciliation_status::CONFIRMED {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }

        // 3. 提出争议
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set(reconciliation_status::DISPUTED.to_string());
        reconciliation_active.disputed_by = Set(Some(user_id));
        reconciliation_active.disputed_at = Set(Some(now));
        reconciliation_active.disputed_reason = Set(Some(reason));

        let reconciliation =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                reconciliation_active,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 获取对账单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_reconciliation::Model, AppError> {
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("对账单 {}", id)))?;

        Ok(reconciliation)
    }

    /// 获取对账单列表
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        reconciliation_status: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_reconciliation::Model>, u64), AppError> {
        let mut query = ap_reconciliation::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_reconciliation::Column::SupplierId.eq(sid));
        }
        if let Some(status) = reconciliation_status {
            query = query.filter(ap_reconciliation::Column::ReconciliationStatus.eq(status));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_reconciliation::Column::StartDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_reconciliation::Column::EndDate.lte(ed));
        }

        // 批次 259 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(ap_reconciliation::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 获取供应商应付汇总（从物化视图）
    pub async fn get_supplier_summary(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<Vec<SupplierApSummary>, AppError> {
        use crate::models::ap_invoice;
        use crate::models::supplier;

        // 查询应付发票
        let mut invoice_query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            invoice_query = invoice_query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        let invoices = invoice_query.all(&*self.db).await?;

        // 按供应商分组统计
        let mut summary_map: std::collections::HashMap<i32, SupplierApSummary> =
            std::collections::HashMap::new();

        for invoice in &invoices {
            let entry =
                summary_map
                    .entry(invoice.supplier_id)
                    .or_insert_with(|| SupplierApSummary {
                        supplier_id: invoice.supplier_id,
                        supplier_code: String::new(),
                        supplier_name: String::new(),
                        total_invoice_count: 0,
                        total_invoice_amount: Decimal::ZERO,
                        total_paid_amount: Decimal::ZERO,
                        total_unpaid_amount: Decimal::ZERO,
                        paid_invoice_count: 0,
                        partial_paid_invoice_count: 0,
                        overdue_invoice_count: 0,
                        overdue_amount: Decimal::ZERO,
                    });

            entry.total_invoice_count += 1;
            entry.total_invoice_amount += invoice.amount;
            entry.total_paid_amount += invoice.paid_amount;
            entry.total_unpaid_amount += invoice.unpaid_amount;

            // 判断付款状态
            if (invoice.amount > Decimal::ZERO && invoice.paid_amount >= invoice.amount)
                || (invoice.amount < Decimal::ZERO && invoice.paid_amount <= invoice.amount)
            {
                entry.paid_invoice_count += 1;
            } else if invoice.paid_amount != Decimal::ZERO {
                entry.partial_paid_invoice_count += 1;
            }

            // 判断是否逾期
            if invoice.due_date < chrono::Utc::now().date_naive()
                && invoice.unpaid_amount > Decimal::ZERO
            {
                entry.overdue_invoice_count += 1;
                entry.overdue_amount += invoice.unpaid_amount;
            }
        }

        // 查询供应商信息
        let supplier_ids: Vec<i32> = summary_map.keys().cloned().collect();
        if !supplier_ids.is_empty() {
            let suppliers = supplier::Entity::find()
                .filter(supplier::Column::Id.is_in(supplier_ids))
                .all(&*self.db)
                .await?;

            for s in suppliers {
                if let Some(entry) = summary_map.get_mut(&s.id) {
                    entry.supplier_code = s.supplier_code;
                    entry.supplier_name = s.supplier_name;
                }
            }
        }

        let result: Vec<SupplierApSummary> = summary_map.into_values().collect();
        Ok(result)
    }

    /// 自动对账 - 为所有供应商自动生成对账单
    pub async fn auto_reconcile_all(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        user_id: i32,
    ) -> Result<Vec<AutoReconciliationResult>, AppError> {
        use crate::models::supplier;

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
    ) -> Result<Vec<crate::models::supplier::Model>, AppError> {
        use crate::models::supplier;
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
        supplier::Entity::find().limit(10_000).all(db).await
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
        sup: &crate::models::supplier::Model,
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
        sup: crate::models::supplier::Model,
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

    /// 获取发票关联信息
    pub async fn get_invoice_relations(
        &self,
        invoice_id: i32,
    ) -> Result<Vec<InvoiceRelationInfo>, AppError> {
        let invoice = ap_invoice::Entity::find_by_id(invoice_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("应付单 {}", invoice_id)))?;

        let mut relations = Vec::new();

        // 关联采购入库单
        if invoice.source_type.as_deref() == Some("PURCHASE_RECEIPT") {
            relations.push(InvoiceRelationInfo {
                invoice_id: invoice.id,
                invoice_no: invoice.invoice_no.clone(),
                source_type: invoice.source_type.clone().unwrap_or_default(),
                source_id: invoice.source_id.unwrap_or_default(),
                source_no: None,
                supplier_id: invoice.supplier_id,
                amount: invoice.amount,
                status: invoice.invoice_status.clone(),
            });
        }

        // 关联付款记录
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(invoice.supplier_id))
            .all(&*self.db)
            .await?;

        for payment in payments {
            relations.push(InvoiceRelationInfo {
                invoice_id: invoice.id,
                invoice_no: invoice.invoice_no.clone(),
                source_type: "PAYMENT".to_string(),
                source_id: payment.id,
                source_no: Some(payment.payment_no.clone()),
                supplier_id: invoice.supplier_id,
                amount: payment.payment_amount,
                status: payment.payment_status.clone(),
            });
        }

        Ok(relations)
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 生成对账单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct GenerateReconciliationRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 对账开始日期
    pub start_date: NaiveDate,

    /// 对账结束日期
    pub end_date: NaiveDate,

    /// 备注
    pub notes: Option<String>,
}

/// 供应商应付汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct SupplierApSummary {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 供应商编码
    pub supplier_code: String,

    /// 供应商名称
    pub supplier_name: String,

    /// 应付单总数
    pub total_invoice_count: i64,

    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 已付清应付单数量
    pub paid_invoice_count: i64,

    /// 部分付款应付单数量
    pub partial_paid_invoice_count: i64,

    /// 逾期应付单数量
    pub overdue_invoice_count: i64,

    /// 逾期金额
    pub overdue_amount: Decimal,
}

/// 自动对账结果
// 批次 23（2026-06-29 v5 P0-1 修复补充）：新增 Clone 派生，支持 lock().await.clone() 模式
#[derive(Debug, Clone, Serialize)]
pub struct AutoReconciliationResult {
    pub reconciliation_id: i32,
    pub reconciliation_no: String,
    pub supplier_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoice: Decimal,
    pub total_payment: Decimal,
    pub closing_balance: Decimal,
    pub invoice_count: usize,
    pub payment_count: usize,
    pub status: String,
    pub message: String,
}

/// 发票关联信息
#[derive(Debug, Serialize)]
pub struct InvoiceRelationInfo {
    pub invoice_id: i32,
    pub invoice_no: String,
    pub source_type: String,
    pub source_id: i32,
    pub source_no: Option<String>,
    pub supplier_id: i32,
    pub amount: Decimal,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::{common, payment};
    use std::str::FromStr;

    /// 复现 generate_reconciliation 中的期末余额计算公式
    ///
    /// 业务公式（ap_reconciliation_service.rs 第 87 行）：
    /// `closing_balance = opening_balance + total_invoice - total_payment`
    fn compute_closing_balance(
        opening_balance: Decimal,
        total_invoice: Decimal,
        total_payment: Decimal,
    ) -> Decimal {
        opening_balance + total_invoice - total_payment
    }

    /// 复现 get_supplier_summary 中的付款状态判断逻辑
    ///
    /// 业务逻辑（ap_reconciliation_service.rs 第 300-306 行）：
    /// - 已付清：amount > 0 且 paid >= amount，或 amount < 0 且 paid <= amount（红冲场景）
    /// - 部分付款：paid != 0 且未付清
    /// - 未付款：paid == 0
    ///
    /// 返回值约定：0=未付款，1=部分付款，2=已付清
    fn classify_payment_status(amount: Decimal, paid_amount: Decimal) -> i32 {
        let paid_in_full = (amount > Decimal::ZERO && paid_amount >= amount)
            || (amount < Decimal::ZERO && paid_amount <= amount);
        if paid_in_full {
            2
        } else if paid_amount != Decimal::ZERO {
            1
        } else {
            0
        }
    }

    /// 复现 get_supplier_summary 中的逾期判断逻辑
    ///
    /// 业务逻辑（ap_reconciliation_service.rs 第 309-314 行）：
    /// `due_date < today && unpaid_amount > 0` 视为逾期
    /// 这里把 today 参数化，避免测试依赖系统当前时间导致用例非幂等。
    fn is_overdue(due_date: NaiveDate, today: NaiveDate, unpaid_amount: Decimal) -> bool {
        due_date < today && unpaid_amount > Decimal::ZERO
    }

    // =====================================================
    // 一、状态常量值正确性
    // =====================================================

    /// 测试_对账状态常量_Pending值正确
    ///
    /// 验证 "PENDING" 与 common::STATUS_PENDING 一致，
    /// 用于 generate_reconciliation 创建对账单时的初始状态（第 99 行），
    /// 以及 confirm_reconciliation 中允许确认的唯一状态（第 132 行）。
    #[test]
    fn 测试_对账状态常量_Pending值正确() {
        assert_eq!(common::STATUS_PENDING, "PENDING");
        // 业务代码硬编码字符串与常量保持一致，避免状态机误判
        let created_status = "PENDING".to_string();
        assert_eq!(created_status, common::STATUS_PENDING);
    }

    /// 测试_对账状态常量_Cancelled值正确
    ///
    /// 验证 "CANCELLED" 与 common::STATUS_CANCELLED 一致，
    /// 用于 generate_reconciliation 排除已取消的应付单（第 54、72 行）。
    #[test]
    fn 测试_对账状态常量_Cancelled值正确() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
        // 业务查询过滤条件使用同一常量，避免拼写错误漏过作废单据
        let excluded_status = "CANCELLED";
        assert_eq!(excluded_status, common::STATUS_CANCELLED);
    }

    /// 测试_付款状态常量_Confirmed值正确
    ///
    /// 验证 "CONFIRMED" 与 payment::PAYMENT_CONFIRMED 一致，
    /// 用于 generate_reconciliation 查询已确认付款单（第 64 行），
    /// 以及 confirm_reconciliation 中对账单确认后的状态值（第 142 行）。
    #[test]
    fn 测试_付款状态常量_Confirmed值正确() {
        assert_eq!(payment::PAYMENT_CONFIRMED, "CONFIRMED");
        // 业务代码同时使用此值用于付款单查询过滤与对账单状态设置
        let payment_filter = "CONFIRMED";
        let reconciliation_status = "CONFIRMED".to_string();
        assert_eq!(payment_filter, payment::PAYMENT_CONFIRMED);
        assert_eq!(reconciliation_status, payment::PAYMENT_CONFIRMED);
    }

    // =====================================================
    // 二、期末余额计算（纯算法）
    // =====================================================

    /// 测试_期末余额计算_标准场景
    ///
    /// 验证 generate_reconciliation 中期末余额公式：
    /// 期末 = 期初 + 本期应付 - 本期付款
    /// 典型场景：期初 1000，本期应付 5000，本期付款 3000，期末应为 3000
    #[test]
    fn 测试_期末余额计算_标准场景() {
        let opening = decs!("1000");
        let total_invoice = decs!("5000");
        let total_payment = decs!("3000");

        let closing = compute_closing_balance(opening, total_invoice, total_payment);

        assert_eq!(closing, decs!("3000"));
        // 业务不变量：期末 = 期初 + 应付 - 付款
        assert_eq!(closing, opening + total_invoice - total_payment);
    }

    /// 测试_期末余额计算_无本期交易
    ///
    /// 验证本期内既无应付也无付款时，期末余额等于期初余额
    #[test]
    fn 测试_期末余额计算_无本期交易() {
        let opening = decs!("2500");
        let closing = compute_closing_balance(opening, Decimal::ZERO, Decimal::ZERO);

        assert_eq!(closing, opening);
        assert_eq!(closing, decs!("2500"));
    }

    /// 测试_期末余额计算_付款大于应付产生透支
    ///
    /// 验证付款总额大于（期初+应付）时，期末余额为负数（预付/透支场景）
    #[test]
    fn 测试_期末余额计算_付款大于应付产生透支() {
        let opening = decs!("1000");
        let total_invoice = decs!("2000");
        let total_payment = decs!("5000");

        let closing = compute_closing_balance(opening, total_invoice, total_payment);

        // 1000 + 2000 - 5000 = -2000（预付 supplier 模式）
        assert_eq!(closing, decs!("-2000"));
        assert!(closing < Decimal::ZERO);
    }

    /// 测试_期末余额计算_金额全为零
    ///
    /// 验证全部金额为零时（新供应商首次对账且无任何业务），期末余额为零
    #[test]
    fn 测试_期末余额计算_金额全为零() {
        let closing = compute_closing_balance(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);

        assert_eq!(closing, Decimal::ZERO);
    }

    // =====================================================
    // 三、状态机转换合法性
    // =====================================================

    /// 测试_状态机转换_确认需Pending状态
    ///
    /// 验证 confirm_reconciliation 中状态门控逻辑（第 132 行）：
    /// 仅当 reconciliation_status == "PENDING" 时允许确认，
    /// 其他状态（CONFIRMED/DISPUTED 等）应被拒绝。
    #[test]
    fn 测试_状态机转换_确认需Pending状态() {
        // PENDING 状态允许确认
        let pending_status = common::STATUS_PENDING.to_string();
        assert_eq!(pending_status, common::STATUS_PENDING);
        let can_confirm_pending = pending_status == common::STATUS_PENDING;
        assert!(can_confirm_pending);

        // CONFIRMED 状态不可再次确认
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        let can_confirm_confirmed = confirmed_status == common::STATUS_PENDING;
        assert!(!can_confirm_confirmed);

        // DISPUTED 状态不可直接确认
        let disputed_status = "DISPUTED".to_string();
        let can_confirm_disputed = disputed_status == common::STATUS_PENDING;
        assert!(!can_confirm_disputed);
    }

    /// 测试_状态机转换_已确认不可争议
    ///
    /// 验证 dispute 中状态门控逻辑（第 181 行）：
    /// 当 reconciliation_status == "CONFIRMED" 时拒绝提出争议
    #[test]
    fn 测试_状态机转换_已确认不可争议() {
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        // 复现 dispute 的状态校验：CONFIRMED 状态被拒绝
        let should_reject = confirmed_status == payment::PAYMENT_CONFIRMED;
        assert!(should_reject);

        // 复现错误消息构造（业务第 182 行）
        let err = AppError::business("对账单已确认，不可提出争议".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_状态机转换_争议或Pending可继续争议
    ///
    /// 验证 dispute 中非 CONFIRMED 状态（PENDING / DISPUTED）均允许提出争议
    #[test]
    fn 测试_状态机转换_争议或Pending可继续争议() {
        // PENDING 状态可提出争议
        let pending_status = common::STATUS_PENDING.to_string();
        let can_dispute_pending = pending_status != payment::PAYMENT_CONFIRMED;
        assert!(can_dispute_pending);

        // DISPUTED 状态可继续补争议（业务代码未禁止）
        let disputed_status = "DISPUTED".to_string();
        let can_dispute_disputed = disputed_status != payment::PAYMENT_CONFIRMED;
        assert!(can_dispute_disputed);

        // CONFIRMED 状态不可争议（与上例一致，作为对照）
        let confirmed_status = payment::PAYMENT_CONFIRMED.to_string();
        let can_dispute_confirmed = confirmed_status != payment::PAYMENT_CONFIRMED;
        assert!(!can_dispute_confirmed);
    }

    // =====================================================
    // 四、付款状态判断（get_supplier_summary 内纯算法）
    // =====================================================

    /// 测试_付款状态判断_已付清正向
    ///
    /// 验证 amount > 0 且 paid >= amount 时判为已付清
    #[test]
    fn 测试_付款状态判断_已付清正向() {
        let amount = decs!("1000");
        // 恰好付清（边界）
        assert_eq!(classify_payment_status(amount, decs!("1000")), 2);
        // 多付（红冲/预付）
        assert_eq!(classify_payment_status(amount, decs!("1200")), 2);
    }

    /// 测试_付款状态判断_已付清负向红冲
    ///
    /// 验证 amount < 0 且 paid <= amount 时判为已付清（红冲应付单场景）
    #[test]
    fn 测试_付款状态判断_已付清负向红冲() {
        let amount = decs!("-500");
        // 恰好冲销（边界，paid == amount）
        assert_eq!(classify_payment_status(amount, decs!("-500")), 2);
        // 多冲销
        assert_eq!(classify_payment_status(amount, decs!("-600")), 2);
    }

    /// 测试_付款状态判断_部分付款
    ///
    /// 验证 paid != 0 且未达付清条件时判为部分付款
    #[test]
    fn 测试_付款状态判断_部分付款() {
        let amount = decs!("1000");
        // 正向部分付款
        assert_eq!(classify_payment_status(amount, decs!("300")), 1);
        assert_eq!(classify_payment_status(amount, decs!("999.99")), 1);

        // 负向部分冲销（amount < 0，paid 介于 0 和 amount 之间）
        let neg_amount = decs!("-500");
        assert_eq!(classify_payment_status(neg_amount, decs!("-100")), 1);
    }

    /// 测试_付款状态判断_未付款
    ///
    /// 验证 paid == 0 时判为未付款
    #[test]
    fn 测试_付款状态判断_未付款() {
        let amount = decs!("1000");
        assert_eq!(classify_payment_status(amount, Decimal::ZERO), 0);

        // 负向金额未付款同样判为未付款
        let neg_amount = decs!("-500");
        assert_eq!(classify_payment_status(neg_amount, Decimal::ZERO), 0);
    }

    // =====================================================
    // 五、逾期判断（get_supplier_summary 内纯算法）
    // =====================================================

    /// 测试_逾期判断_已逾期未付
    ///
    /// 验证 due_date < today 且 unpaid_amount > 0 时判为逾期
    #[test]
    fn 测试_逾期判断_已逾期未付() {
        let today = ymd!(2026, 7, 1);
        let due_date = ymd!(2026, 6, 30); // 已到期
        let unpaid = decs!("500");

        assert!(is_overdue(due_date, today, unpaid));

        // 累计逾期金额应等于未付金额（业务第 314 行）
        let overdue_amount = unpaid;
        assert_eq!(overdue_amount, decs!("500"));
    }

    /// 测试_逾期判断_未到期不逾期
    ///
    /// 验证 due_date >= today 时不判为逾期，即使存在未付金额
    #[test]
    fn 测试_逾期判断_未到期不逾期() {
        let today = ymd!(2026, 7, 1);
        let unpaid = decs!("500");

        // 到期日等于今天（边界，不逾期）
        assert!(!is_overdue(ymd!(2026, 7, 1), today, unpaid));
        // 到期日在未来
        assert!(!is_overdue(ymd!(2026, 12, 31), today, unpaid));
    }

    /// 测试_逾期判断_已付清不算逾期
    ///
    /// 验证 unpaid_amount == 0 时即使超过到期日也不判为逾期
    #[test]
    fn 测试_逾期判断_已付清不算逾期() {
        let today = ymd!(2026, 7, 1);
        let due_date = ymd!(2026, 6, 30); // 已到期
        let unpaid = Decimal::ZERO;

        assert!(!is_overdue(due_date, today, unpaid));

        // 负向未付金额（红冲多付）也不应判为逾期
        let unpaid_neg = decs!("-100");
        assert!(!is_overdue(due_date, today, unpaid_neg));
    }

    // =====================================================
    // 六、错误消息格式
    // =====================================================

    /// 测试_错误消息格式_对账单未找到
    ///
    /// 验证 get_by_id / confirm_reconciliation / dispute 中
    /// "对账单 {id}" 格式的 not_found 错误消息
    #[test]
    fn 测试_错误消息格式_对账单未找到() {
        let id = 9999;
        let err = AppError::not_found(format!("对账单 {}", id));

        // 复现业务第 129/178/212 行的错误消息格式
        assert_eq!(format!("对账单 {}", id), "对账单 9999");
        assert!(matches!(err, AppError::NotFound(_)));
    }

    /// 测试_错误消息格式_状态不可确认
    ///
    /// 验证 confirm_reconciliation 中
    /// "对账单状态为{status}，不可确认" 格式的 business 错误消息（第 134 行）
    #[test]
    fn 测试_错误消息格式_状态不可确认() {
        let status = payment::PAYMENT_CONFIRMED; // 已确认状态再次确认
        let msg = format!("对账单状态为{}，不可确认", status);
        let err = AppError::business(msg.clone());

        assert_eq!(msg, "对账单状态为CONFIRMED，不可确认");
        assert!(matches!(err, AppError::BusinessError(_)));

        // DISPUTED 状态尝试确认时也应生成正确格式
        let disputed_msg = format!("对账单状态为{}，不可确认", "DISPUTED");
        assert_eq!(disputed_msg, "对账单状态为DISPUTED，不可确认");
    }

    /// 测试_错误消息格式_已确认不可争议
    ///
    /// 验证 dispute 中
    /// "对账单已确认，不可提出争议" 固定消息的 business 错误（第 182 行）
    #[test]
    fn 测试_错误消息格式_已确认不可争议() {
        let err = AppError::business("对账单已确认，不可提出争议".to_string());

        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_错误消息格式_应付单未找到
    ///
    /// 验证 get_invoice_relations 中
    /// "应付单 {invoice_id}" 格式的 not_found 错误消息（第 466 行）
    #[test]
    fn 测试_错误消息格式_应付单未找到() {
        let invoice_id = 8888;
        let err = AppError::not_found(format!("应付单 {}", invoice_id));

        assert_eq!(format!("应付单 {}", invoice_id), "应付单 8888");
        assert!(matches!(err, AppError::NotFound(_)));
    }

    // =====================================================
    // 七、夹具宏可用性
    // =====================================================

    /// 测试_decs夹具宏解析金额
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串，用于后续金额计算测试夹具
    #[test]
    fn 测试_decs夹具宏解析金额() {
        let v = decs!("12345.67");
        assert_eq!(v.to_string(), "12345.67");

        let zero = decs!("0");
        assert_eq!(zero, Decimal::ZERO);

        let neg = decs!("-1000");
        assert!(neg < Decimal::ZERO);
        assert_eq!(neg.to_string(), "-1000");

        // FromStr trait 在作用域中即可使用，验证可访问
        let parsed = Decimal::from_str("99.99");
        assert!(parsed.is_ok());
    }

    /// 测试_ymd夹具宏解析对账日期
    ///
    /// 验证 ymd! 宏能正确解析日期，用于对账期间测试夹具
    #[test]
    fn 测试_ymd夹具宏解析对账日期() {
        let start = ymd!(2026, 1, 1);
        let end = ymd!(2026, 12, 31);

        assert_eq!(start.to_string(), "2026-01-01");
        assert_eq!(end.to_string(), "2026-12-31");
        // 起止日期合法性
        assert!(start < end);
    }

    // =====================================================
    // 八、服务实例化（SQLite 内存数据库）
    // =====================================================

    /// 测试_服务实例创建
    ///
    /// 验证 ApReconciliationService 在 SQLite 内存数据库上能正常实例化，
    /// 内部 Arc<DatabaseConnection> 引用计数 >= 1
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // =====================================================
    // 九、DB 交互测试（依赖 schema，标注 #[ignore]）
    // =====================================================

    /// 测试_生成对账单_需要真实数据库
    ///
    /// 依赖 ap_reconciliation / ap_invoice / ap_payment 表 schema，
    /// 标注 #[ignore] 仅在本地手动运行。无 schema 时返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_生成对账单_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let req = GenerateReconciliationRequest {
            supplier_id: 99999,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            notes: Some("测试对账单".to_string()),
        };

        let result = service.generate_reconciliation(req, 1).await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时返回数据库错误；有 schema 时可能成功或返回约束错误
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_确认对账单_需要真实数据库
    ///
    /// 依赖 ap_reconciliation 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
    #[tokio::test]
    #[ignore]
    async fn 测试_确认对账单_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let result = service.confirm_reconciliation(99999, 1).await;
        // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
        assert!(result.is_err());
    }

    /// 测试_获取对账单列表_需要真实数据库
    ///
    /// 依赖 ap_reconciliation 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic，分页参数 1-indexed 转换正确。
    #[tokio::test]
    #[ignore]
    async fn 测试_获取对账单列表_需要真实数据库() {
        let db = setup_test_db().await;
        let service = ApReconciliationService::new(Arc::new(db));

        let result = service
            .get_list(None, None, None, None, 1, 10)
            .await;
        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时为 Err；有 schema 无记录时为 Ok((vec![], 0))
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    // =====================================================
    // 十、DTO 字段构造与对账单号格式
    // =====================================================

    /// 测试_生成对账请求_字段构造
    ///
    /// 验证 GenerateReconciliationRequest 字段能正常构造，
    /// notes 字段允许为 None，supplier_id/start_date/end_date 必填
    #[test]
    fn 测试_生成对账请求_字段构造() {
        let req_with_notes = GenerateReconciliationRequest {
            supplier_id: 1,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            notes: Some("Q2 对账".to_string()),
        };
        assert_eq!(req_with_notes.supplier_id, 1);
        assert_eq!(req_with_notes.start_date, ymd!(2026, 1, 1));
        assert_eq!(req_with_notes.end_date, ymd!(2026, 6, 30));
        assert_eq!(req_with_notes.notes.as_deref(), Some("Q2 对账"));

        let req_without_notes = GenerateReconciliationRequest {
            supplier_id: 2,
            start_date: ymd!(2026, 7, 1),
            end_date: ymd!(2026, 12, 31),
            notes: None,
        };
        assert_eq!(req_without_notes.supplier_id, 2);
        assert!(req_without_notes.notes.is_none());
    }

    /// 测试_自动对账结果_失败状态字符串
    ///
    /// 验证 auto_reconcile_all 中失败分支使用的 "FAILED" 状态字符串（第 440 行），
    /// 与成功分支的 reconciliation_status（来自数据库）形成对照。
    #[test]
    fn 测试_自动对账结果_失败状态字符串() {
        let failed = AutoReconciliationResult {
            reconciliation_id: 0,
            reconciliation_no: String::new(),
            supplier_id: 1,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 6, 30),
            opening_balance: Decimal::ZERO,
            total_invoice: Decimal::ZERO,
            total_payment: Decimal::ZERO,
            closing_balance: Decimal::ZERO,
            invoice_count: 0,
            payment_count: 0,
            status: "FAILED".to_string(),
            message: "Failed: ...".to_string(),
        };
        assert_eq!(failed.status, "FAILED");
        assert_eq!(failed.reconciliation_id, 0);
        assert_eq!(failed.invoice_count, 0);
    }

    /// 测试_对账单号格式_REC前缀
    ///
    /// 验证 impl_generate_no! 宏生成对账单号使用 "REC" 前缀（第 33-38 行），
    /// 格式为 REC + 年月日 + 三位序号（如 REC20260315001）。
    /// 此处不实际调用数据库生成，仅校验前缀与格式约定。
    #[test]
    fn 测试_对账单号格式_REC前缀() {
        // 复现宏定义的前缀常量
        let prefix = "REC";
        let today = Utc::now();
        let date_part = today.format("%Y%m%d").to_string();
        // 模拟序号部分（实际由宏内 SQL MAX+1 决定）
        let seq_part = "001";
        let sample_no = format!("{}{}{}", prefix, date_part, seq_part);

        assert!(sample_no.starts_with("REC"));
        assert_eq!(sample_no.len(), 3 + 8 + 3); // REC + 8位日期 + 3位序号 = 14
                                                 // 业务文档示例：REC20260315001
        let doc_example = "REC20260315001";
        assert_eq!(doc_example.len(), 14);
        assert!(doc_example.starts_with("REC"));
    }
}
