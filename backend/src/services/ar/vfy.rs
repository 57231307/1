//! 应收对账 - 核销服务（ar/vfy）
//!
//! 包含高级对账算法：
//! - `auto_match`         自动对账：精确金额 + 日期顺序 + 客户汇总三种策略
//! - `get_aging_report`   账龄分桶分析（5 档：当期 / 1-30 / 31-60 / 61-90 / 90+）
//! - `generate_reconciliation` 自动生成对账单（含明细行）
//! - `customer_confirm` / `customer_dispute` 带状态校验的客户操作
//!
//! 拆分自原 `ar_reconciliation_service.rs` 的 `// 增强功能` 段。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use tracing::info;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::models::ar_reconciliation::{ActiveModel, Entity as ReconciliationEntity};
use crate::models::customer;
use crate::utils::error::AppError;

use super::{
    generate_reconciliation_no, AgingBucket, AgingReport, ArReconciliationService,
    AutoMatchRequest, AutoMatchResult, CustomerAgingSummary, GenerateReconciliationRequest,
};

impl ArReconciliationService {
    /// 自动对账 - 按客户批量匹配发票和收款
    ///
    /// 匹配策略：
    /// 1. 精确匹配：金额完全相等的发票和收款
    /// 2. 日期匹配：同一客户在对账期间内的发票和收款按时间顺序配对
    /// 3. 客户汇总：按客户汇总应收和实收，生成对账单
    pub async fn auto_match(
        &self,
        req: AutoMatchRequest,
        user_id: i32,
    ) -> Result<Vec<AutoMatchResult>, AppError> {
        let txn = (*self.db).begin().await?;

        let customers = if let Some(cid) = req.customer_id {
            vec![customer::Entity::find_by_id(cid)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", cid)))?]
        } else {
            customer::Entity::find().all(&txn).await?
        };

        let mut results = Vec::new();

        for cust in customers {
            let invoices = ar_invoice::Entity::find()
                .filter(ar_invoice::Column::CustomerId.eq(cust.id))
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .filter(ar_invoice::Column::InvoiceDate.gte(req.start_date))
                .filter(ar_invoice::Column::InvoiceDate.lte(req.end_date))
                .all(&txn)
                .await?;

            let collections = ar_collection::Entity::find()
                .filter(ar_collection::Column::CustomerId.eq(cust.id))
                .filter(ar_collection::Column::Status.eq("CONFIRMED"))
                .filter(ar_collection::Column::CollectionDate.gte(req.start_date))
                .filter(ar_collection::Column::CollectionDate.lte(req.end_date))
                .all(&txn)
                .await?;

            let opening_balance: Decimal = ar_invoice::Entity::find()
                .filter(ar_invoice::Column::CustomerId.eq(cust.id))
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .filter(ar_invoice::Column::InvoiceDate.lt(req.start_date))
                .all(&txn)
                .await?
                .iter()
                .map(|inv| inv.unpaid_amount)
                .sum();

            let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
            let total_collections: Decimal = collections.iter().map(|c| c.collection_amount).sum();

            // 批次 27 v7 P1 修复：事务边界泄漏，单号生成移入 txn，避免断号/重复
            let reconciliation_no = generate_reconciliation_no(&txn).await?;
            let closing_balance = opening_balance + total_invoices - total_collections;

            let reconciliation = ActiveModel {
                id: Set(0),
                reconciliation_no: Set(reconciliation_no.clone()),
                reconciliation_date: Set(Utc::now().date_naive()),
                period_start: Set(req.start_date),
                period_end: Set(req.end_date),
                customer_id: Set(cust.id),
                customer_name: Set(Some(cust.customer_name.clone())),
                opening_balance: Set(opening_balance),
                total_invoices: Set(total_invoices),
                total_collections: Set(total_collections),
                closing_balance: Set(closing_balance),
                reconciliation_status: Set(Some("draft".to_string())),
                confirmed_by_customer: Set(None),
                dispute_reason: Set(None),
                confirmed_by: Set(None),
                confirmed_at: Set(None),
                created_by: Set(Some(user_id)),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };

            let rec_model = reconciliation.insert(&txn).await?;

            let mut matched_count = 0usize;
            let mut unmatched_invoices: Vec<&ar_invoice::Model> = Vec::new();
            let mut unmatched_collections: Vec<&ar_collection::Model> =
                collections.iter().collect();

            // 策略1: 精确金额匹配
            for inv in &invoices {
                let exact_match = unmatched_collections
                    .iter()
                    .position(|c| c.collection_amount == inv.invoice_amount);

                if let Some(idx) = exact_match {
                    let coll = unmatched_collections.remove(idx);

                    // 创建发票明细
                    let inv_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Set(0),
                        reconciliation_id: Set(rec_model.id),
                        item_type: Set("INVOICE".to_string()),
                        document_type: Set(Some("SALES_INVOICE".to_string())),
                        document_id: Set(Some(inv.id)),
                        document_no: Set(Some(inv.invoice_no.clone())),
                        document_date: Set(Some(inv.invoice_date)),
                        amount: Set(inv.invoice_amount),
                        matched_amount: Set(Some(inv.invoice_amount)),
                        match_status: Set("MATCHED".to_string()),
                        matched_item_id: Set(Some(coll.id)),
                        remarks: Set(None),
                        created_at: Set(Utc::now()),
                        updated_at: Set(Utc::now()),
                    };
                    inv_item.insert(&txn).await?;

                    // 创建收款明细
                    let col_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Set(0),
                        reconciliation_id: Set(rec_model.id),
                        item_type: Set("RECEIPT".to_string()),
                        document_type: Set(Some("COLLECTION".to_string())),
                        document_id: Set(Some(coll.id)),
                        document_no: Set(Some(coll.collection_no.clone())),
                        document_date: Set(Some(coll.collection_date)),
                        amount: Set(-coll.collection_amount),
                        matched_amount: Set(Some(coll.collection_amount)),
                        match_status: Set("MATCHED".to_string()),
                        matched_item_id: Set(Some(inv.id)),
                        remarks: Set(None),
                        created_at: Set(Utc::now()),
                        updated_at: Set(Utc::now()),
                    };
                    col_item.insert(&txn).await?;

                    matched_count += 1;
                } else {
                    unmatched_invoices.push(inv);
                }
            }

            // 策略2: 日期顺序匹配（剩余未精确匹配的）
            let mut remaining_collections = unmatched_collections.clone();
            for inv in unmatched_invoices {
                let date_match = remaining_collections.iter().position(|c| {
                    let date_diff = (c.collection_date - inv.invoice_date).num_days().abs();
                    date_diff <= 30
                });

                if let Some(idx) = date_match {
                    let coll = remaining_collections.remove(idx);
                    let matched = std::cmp::min(inv.invoice_amount, coll.collection_amount);

                    let inv_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Set(0),
                        reconciliation_id: Set(rec_model.id),
                        item_type: Set("INVOICE".to_string()),
                        document_type: Set(Some("SALES_INVOICE".to_string())),
                        document_id: Set(Some(inv.id)),
                        document_no: Set(Some(inv.invoice_no.clone())),
                        document_date: Set(Some(inv.invoice_date)),
                        amount: Set(inv.invoice_amount),
                        matched_amount: Set(Some(matched)),
                        match_status: Set(if matched == inv.invoice_amount {
                            "MATCHED".to_string()
                        } else {
                            "PARTIAL".to_string()
                        }),
                        matched_item_id: Set(Some(coll.id)),
                        remarks: Set(None),
                        created_at: Set(Utc::now()),
                        updated_at: Set(Utc::now()),
                    };
                    inv_item.insert(&txn).await?;

                    let col_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Set(0),
                        reconciliation_id: Set(rec_model.id),
                        item_type: Set("RECEIPT".to_string()),
                        document_type: Set(Some("COLLECTION".to_string())),
                        document_id: Set(Some(coll.id)),
                        document_no: Set(Some(coll.collection_no.clone())),
                        document_date: Set(Some(coll.collection_date)),
                        amount: Set(-coll.collection_amount),
                        matched_amount: Set(Some(matched)),
                        match_status: Set(if matched == coll.collection_amount {
                            "MATCHED".to_string()
                        } else {
                            "PARTIAL".to_string()
                        }),
                        matched_item_id: Set(Some(inv.id)),
                        remarks: Set(None),
                        created_at: Set(Utc::now()),
                        updated_at: Set(Utc::now()),
                    };
                    col_item.insert(&txn).await?;

                    matched_count += 1;
                } else {
                    // 未匹配发票
                    let inv_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Set(0),
                        reconciliation_id: Set(rec_model.id),
                        item_type: Set("INVOICE".to_string()),
                        document_type: Set(Some("SALES_INVOICE".to_string())),
                        document_id: Set(Some(inv.id)),
                        document_no: Set(Some(inv.invoice_no.clone())),
                        document_date: Set(Some(inv.invoice_date)),
                        amount: Set(inv.invoice_amount),
                        matched_amount: Set(None),
                        match_status: Set("UNMATCHED".to_string()),
                        matched_item_id: Set(None),
                        remarks: Set(None),
                        created_at: Set(Utc::now()),
                        updated_at: Set(Utc::now()),
                    };
                    inv_item.insert(&txn).await?;
                }
            }

            // 剩余未匹配收款
            for coll in remaining_collections {
                let col_item = crate::models::ar_reconciliation_item::ActiveModel {
                    id: Set(0),
                    reconciliation_id: Set(rec_model.id),
                    item_type: Set("RECEIPT".to_string()),
                    document_type: Set(Some("COLLECTION".to_string())),
                    document_id: Set(Some(coll.id)),
                    document_no: Set(Some(coll.collection_no.clone())),
                    document_date: Set(Some(coll.collection_date)),
                    amount: Set(-coll.collection_amount),
                    matched_amount: Set(None),
                    match_status: Set("UNMATCHED".to_string()),
                    matched_item_id: Set(None),
                    remarks: Set(None),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                };
                col_item.insert(&txn).await?;
            }

            let unmatched_count = invoices.len() + collections.len() - matched_count * 2;

            results.push(AutoMatchResult {
                reconciliation_id: rec_model.id,
                reconciliation_no,
                customer_id: cust.id,
                customer_name: cust.customer_name.clone(),
                total_invoices,
                total_collections,
                matched_count,
                unmatched_count,
                status: "draft".to_string(),
            });
        }

        txn.commit().await?;
        Ok(results)
    }

    /// 计算账龄分析报告
    ///
    /// 分桶规则：
    /// - 当期（未逾期）：due_date >= 今天
    /// - 1-30天：今天 - due_date 在 1~30 天
    /// - 31-60天：今天 - due_date 在 31~60 天
    /// - 61-90天：今天 - due_date 在 61~90 天
    /// - 90天以上：今天 - due_date > 90 天
    pub async fn get_aging_report(
        &self,
        customer_id: Option<i32>,
    ) -> Result<AgingReport, AppError> {
        let today = Utc::now().date_naive();

        let mut invoice_query = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO));

        if let Some(cid) = customer_id {
            invoice_query = invoice_query.filter(ar_invoice::Column::CustomerId.eq(cid));
        }

        let invoices = invoice_query.all(&*self.db).await?;

        let mut customer_map: std::collections::HashMap<i32, (String, Vec<&ar_invoice::Model>)> =
            std::collections::HashMap::new();

        for inv in &invoices {
            let entry = customer_map
                .entry(inv.customer_id)
                .or_insert_with(|| (inv.customer_name.clone().unwrap_or_default(), Vec::new()));
            entry.1.push(inv);
        }

        let mut customer_summaries = Vec::new();
        let mut overall_buckets = vec![
            AgingBucket {
                label: "当期".to_string(),
                min_days: 0,
                max_days: Some(0),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "1-30天".to_string(),
                min_days: 1,
                max_days: Some(30),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "31-60天".to_string(),
                min_days: 31,
                max_days: Some(60),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "61-90天".to_string(),
                min_days: 61,
                max_days: Some(90),
                amount: Decimal::ZERO,
                count: 0,
            },
            AgingBucket {
                label: "90天以上".to_string(),
                min_days: 91,
                max_days: None,
                amount: Decimal::ZERO,
                count: 0,
            },
        ];

        let mut total_receivable = Decimal::ZERO;

        for (cust_id, (cust_name, cust_invoices)) in &customer_map {
            let mut buckets = vec![
                AgingBucket {
                    label: "当期".to_string(),
                    min_days: 0,
                    max_days: Some(0),
                    amount: Decimal::ZERO,
                    count: 0,
                },
                AgingBucket {
                    label: "1-30天".to_string(),
                    min_days: 1,
                    max_days: Some(30),
                    amount: Decimal::ZERO,
                    count: 0,
                },
                AgingBucket {
                    label: "31-60天".to_string(),
                    min_days: 31,
                    max_days: Some(60),
                    amount: Decimal::ZERO,
                    count: 0,
                },
                AgingBucket {
                    label: "61-90天".to_string(),
                    min_days: 61,
                    max_days: Some(90),
                    amount: Decimal::ZERO,
                    count: 0,
                },
                AgingBucket {
                    label: "90天以上".to_string(),
                    min_days: 91,
                    max_days: None,
                    amount: Decimal::ZERO,
                    count: 0,
                },
            ];

            let mut cust_total = Decimal::ZERO;

            for inv in cust_invoices {
                let overdue_days = (today - inv.due_date).num_days();
                let amount = inv.unpaid_amount;
                cust_total += amount;

                let bucket_idx = if overdue_days <= 0 {
                    0
                } else if overdue_days <= 30 {
                    1
                } else if overdue_days <= 60 {
                    2
                } else if overdue_days <= 90 {
                    3
                } else {
                    4
                };

                buckets[bucket_idx].amount += amount;
                buckets[bucket_idx].count += 1;
                overall_buckets[bucket_idx].amount += amount;
                overall_buckets[bucket_idx].count += 1;
            }

            total_receivable += cust_total;

            customer_summaries.push(CustomerAgingSummary {
                customer_id: *cust_id,
                customer_name: cust_name.clone(),
                total_amount: cust_total,
                buckets,
            });
        }

        customer_summaries.sort_by_key(|b| std::cmp::Reverse(b.total_amount));

        Ok(AgingReport {
            analysis_date: today,
            total_receivable,
            customer_summaries,
            overall_buckets,
        })
    }

    /// 为指定客户自动生成对账单（从发票/收款汇总）
    pub async fn generate_reconciliation(
        &self,
        req: GenerateReconciliationRequest,
        user_id: i32,
    ) -> Result<crate::models::ar_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let cust = customer::Entity::find_by_id(req.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", req.customer_id)))?;

        // 批次 27 v7 P1 修复：事务边界泄漏，单号生成移入 txn，避免断号/重复
        let reconciliation_no = generate_reconciliation_no(&txn).await?;

        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.eq(req.customer_id))
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::InvoiceDate.gte(req.start_date))
            .filter(ar_invoice::Column::InvoiceDate.lte(req.end_date))
            .all(&txn)
            .await?;

        let collections = ar_collection::Entity::find()
            .filter(ar_collection::Column::CustomerId.eq(req.customer_id))
            .filter(ar_collection::Column::Status.eq("CONFIRMED"))
            .filter(ar_collection::Column::CollectionDate.gte(req.start_date))
            .filter(ar_collection::Column::CollectionDate.lte(req.end_date))
            .all(&txn)
            .await?;

        let opening_balance: Decimal = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.eq(req.customer_id))
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::InvoiceDate.lt(req.start_date))
            .all(&txn)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum();

        let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
        let total_collections: Decimal = collections.iter().map(|c| c.collection_amount).sum();
        let closing_balance = opening_balance + total_invoices - total_collections;

        let reconciliation = ActiveModel {
            id: Set(0),
            reconciliation_no: Set(reconciliation_no),
            reconciliation_date: Set(Utc::now().date_naive()),
            period_start: Set(req.start_date),
            period_end: Set(req.end_date),
            customer_id: Set(req.customer_id),
            customer_name: Set(Some(cust.customer_name.clone())),
            opening_balance: Set(opening_balance),
            total_invoices: Set(total_invoices),
            total_collections: Set(total_collections),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set(Some("draft".to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let rec_model = reconciliation.insert(&txn).await?;

        // 创建发票明细行
        for inv in &invoices {
            let item = crate::models::ar_reconciliation_item::ActiveModel {
                id: Set(0),
                reconciliation_id: Set(rec_model.id),
                item_type: Set("INVOICE".to_string()),
                document_type: Set(Some("SALES_INVOICE".to_string())),
                document_id: Set(Some(inv.id)),
                document_no: Set(Some(inv.invoice_no.clone())),
                document_date: Set(Some(inv.invoice_date)),
                amount: Set(inv.invoice_amount),
                matched_amount: Set(None),
                match_status: Set("UNMATCHED".to_string()),
                matched_item_id: Set(None),
                remarks: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            item.insert(&txn).await?;
        }

        // 创建收款明细行
        for coll in &collections {
            let item = crate::models::ar_reconciliation_item::ActiveModel {
                id: Set(0),
                reconciliation_id: Set(rec_model.id),
                item_type: Set("RECEIPT".to_string()),
                document_type: Set(Some("COLLECTION".to_string())),
                document_id: Set(Some(coll.id)),
                document_no: Set(Some(coll.collection_no.clone())),
                document_date: Set(Some(coll.collection_date)),
                amount: Set(-coll.collection_amount),
                matched_amount: Set(None),
                match_status: Set("UNMATCHED".to_string()),
                matched_item_id: Set(None),
                remarks: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            item.insert(&txn).await?;
        }

        info!(
            "自动生成对账单成功：客户={}, 发票={}笔, 收款={}笔",
            cust.customer_name,
            invoices.len(),
            collections.len()
        );

        txn.commit().await?;
        Ok(rec_model)
    }

    /// 客户确认对账单（带状态校验）
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现完全无 txn 无 lock，两并发 customer_confirm 同时通过状态门后基于过期状态写入，
    /// 导致 confirmed_by/confirmed_at 被覆盖、审计日志完全丢失（未走 update_with_audit）。
    pub async fn customer_confirm(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<crate::models::ar_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or("draft");
        if status == "confirmed" {
            return Err(AppError::business("对账单已确认，不可重复确认".to_string()));
        }
        if status == "disputed" {
            return Err(AppError::business(
                "对账单存在争议，请先解决争议后再确认".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("confirmed".to_string()));
        active_model.confirmed_by_customer = Set(Some(true));
        active_model.confirmed_by = Set(Some(user_id));
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("客户确认对账单成功：id={}", id);
        Ok(updated)
    }

    /// 客户提出争议（带状态校验）
    ///
    /// 批次 27 v7 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
    /// 原实现完全无 txn 无 lock，两并发 customer_dispute 同时通过状态门后基于过期状态写入，
    /// 导致 dispute_reason 被覆盖、审计日志完全丢失（未走 update_with_audit）。
    pub async fn customer_dispute(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<crate::models::ar_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or("draft");
        if status == "confirmed" {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }
        if status == "closed" {
            return Err(AppError::business("对账单已关闭，不可提出争议".to_string()));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("disputed".to_string()));
        active_model.dispute_reason = Set(Some(reason.clone()));
        active_model.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("客户对账单提出争议：id={}, reason={}", id, reason);
        Ok(updated)
    }
}
