//! 供应商对账 Service
//!
//! 供应商对账服务层，负责对账的核心业务逻辑
//! 包含生成对账单、确认对账、争议处理等管理

use crate::models::{ap_invoice, ap_payment, ap_reconciliation};
use crate::utils::error::AppError;
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
            .filter(ap_payment::Column::PaymentStatus.eq("CONFIRMED"))
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
            reconciliation_status: Set("PENDING".to_string()),
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
        if reconciliation.reconciliation_status != "PENDING" {
            return Err(AppError::business(format!(
                "对账单状态为{}，不可确认",
                reconciliation.reconciliation_status
            )));
        }

        // 3. 确认对账单
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set("CONFIRMED".to_string());
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
        if reconciliation.reconciliation_status == "CONFIRMED" {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }

        // 3. 提出争议
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set("DISPUTED".to_string());
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

        // 分页
        let paginator = query
            .order_by(ap_reconciliation::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // v18 批次 48 修复：调用方传 1-indexed page，SeaORM fetch_page 是 0-indexed，需 saturating_sub(1)
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

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

        let suppliers = supplier::Entity::find().all(&*self.db).await?;
        let supplier_ids: Vec<i32> = suppliers.iter().map(|s| s.id).collect();

        // v12 批次 39 修复：批量预加载所有供应商的发票数和付款数，避免 for_each_concurrent 内逐个 count（N+1，2N 次查询）
        let invoice_counts = if supplier_ids.is_empty() {
            std::collections::HashMap::<i32, usize>::new()
        } else {
            ap_invoice::Entity::find()
                .filter(ap_invoice::Column::SupplierId.is_in(supplier_ids.clone()))
                .filter(ap_invoice::Column::InvoiceDate.gte(start_date))
                .filter(ap_invoice::Column::InvoiceDate.lte(end_date))
                .all(&*self.db)
                .await?
                .into_iter()
                .fold(std::collections::HashMap::<i32, usize>::new(), |mut acc, inv| {
                    *acc.entry(inv.supplier_id).or_default() += 1;
                    acc
                })
        };
        let payment_counts = if supplier_ids.is_empty() {
            std::collections::HashMap::<i32, usize>::new()
        } else {
            ap_payment::Entity::find()
                .filter(ap_payment::Column::SupplierId.is_in(supplier_ids))
                .filter(ap_payment::Column::PaymentDate.gte(start_date))
                .filter(ap_payment::Column::PaymentDate.lte(end_date))
                .all(&*self.db)
                .await?
                .into_iter()
                .fold(std::collections::HashMap::<i32, usize>::new(), |mut acc, pay| {
                    *acc.entry(pay.supplier_id).or_default() += 1;
                    acc
                })
        };

        let results = Arc::new(Mutex::new(Vec::new()));

        stream::iter(suppliers)
            .for_each_concurrent(10, |sup| {
                let results = results.clone();
                let db = self.db.clone();
                // v12 批次 39 修复：克隆预加载的计数 map 供并发任务读取（避免在闭包内查询 DB）
                let invoice_counts = invoice_counts.clone();
                let payment_counts = payment_counts.clone();
                async move {
                    let service = ApReconciliationService::new(db.clone());
                    let req = GenerateReconciliationRequest {
                        supplier_id: sup.id,
                        start_date,
                        end_date,
                        notes: Some(format!(
                            "Auto-generated reconciliation for {}",
                            sup.supplier_name
                        )),
                    };

                    let result = match service.generate_reconciliation(req, user_id).await {
                        Ok(rec) => {
                            // v12 批次 39 修复：从预加载的计数 map 中取，避免循环内逐个 count（N+1）
                            let invoice_count = invoice_counts.get(&sup.id).copied().unwrap_or(0);
                            let payment_count = payment_counts.get(&sup.id).copied().unwrap_or(0);

                            AutoReconciliationResult {
                                reconciliation_id: rec.id,
                                reconciliation_no: rec.reconciliation_no,
                                supplier_id: sup.id,
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
                        Err(e) => AutoReconciliationResult {
                            reconciliation_id: 0,
                            reconciliation_no: String::new(),
                            supplier_id: sup.id,
                            start_date,
                            end_date,
                            opening_balance: Decimal::ZERO,
                            total_invoice: Decimal::ZERO,
                            total_payment: Decimal::ZERO,
                            closing_balance: Decimal::ZERO,
                            invoice_count: 0,
                            payment_count: 0,
                            status: "FAILED".to_string(),
                            message: format!("Failed: {}", e),
                        },
                    };

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
