//! 应收对账 Service
//!
//! 提供客户应收对账单的生成、发送、确认和争议处理
//! 增强功能：自动对账、账龄分桶、对账单自动生成、客户确认/争议处理

#![allow(dead_code)]

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item::{
    Entity as ReconciliationItemEntity, Model as ReconciliationItemModel,
};
use crate::models::customer;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

/// 创建对账单请求
#[derive(Debug, Clone)]
pub struct CreateReconciliationRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub notes: Option<String>,
}

/// 更新对账单请求
#[derive(Debug, Clone)]
pub struct UpdateReconciliationRequest {
    pub opening_balance: Option<Decimal>,
    pub total_invoices: Option<Decimal>,
    pub total_collections: Option<Decimal>,
    pub notes: Option<String>,
}

/// 对账单查询参数
#[derive(Debug, Clone)]
pub struct ReconciliationQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 自动对账请求
#[derive(Debug, Clone, Deserialize)]
pub struct AutoMatchRequest {
    pub customer_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub match_strategy: Option<String>,
}

/// 自动对账结果
#[derive(Debug, Serialize)]
pub struct AutoMatchResult {
    pub reconciliation_id: i32,
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: String,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub matched_count: usize,
    pub unmatched_count: usize,
    pub status: String,
}

/// 账龄分桶
#[derive(Debug, Serialize, Clone)]
pub struct AgingBucket {
    pub label: String,
    pub min_days: i64,
    pub max_days: Option<i64>,
    pub amount: Decimal,
    pub count: usize,
}

/// 客户账龄汇总
#[derive(Debug, Serialize)]
pub struct CustomerAgingSummary {
    pub customer_id: i32,
    pub customer_name: String,
    pub total_amount: Decimal,
    pub buckets: Vec<AgingBucket>,
}

/// 账龄报告
#[derive(Debug, Serialize)]
pub struct AgingReport {
    pub analysis_date: NaiveDate,
    pub total_receivable: Decimal,
    pub customer_summaries: Vec<CustomerAgingSummary>,
    pub overall_buckets: Vec<AgingBucket>,
}

/// 对账明细行
#[derive(Debug, Serialize)]
pub struct ReconciliationDetail {
    pub id: i32,
    pub reconciliation_id: i32,
    pub item_type: String,
    pub document_type: Option<String>,
    pub document_id: Option<i32>,
    pub document_no: Option<String>,
    pub document_date: Option<NaiveDate>,
    pub amount: Decimal,
    pub matched_amount: Option<Decimal>,
    pub match_status: String,
    pub matched_item_id: Option<i32>,
    pub remarks: Option<String>,
}

/// 对账单详情（含明细）
#[derive(Debug, Serialize)]
pub struct ReconciliationWithDetails {
    pub reconciliation: ReconciliationModel,
    pub details: Vec<ReconciliationDetail>,
}

/// 自动生成对账单请求
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReconciliationRequest {
    pub customer_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub notes: Option<String>,
}

/// 应收对账 Service
pub struct ArReconciliationService {
    db: Arc<DatabaseConnection>,
}

impl ArReconciliationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建对账单
    pub async fn create(
        &self,
        req: CreateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let closing_balance = req.opening_balance + req.total_invoices - req.total_collections;

        let active_model = ActiveModel {
            id: Set(0),
            reconciliation_no: Set(req.reconciliation_no),
            reconciliation_date: Set(Utc::now().date_naive()),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            customer_id: Set(req.customer_id),
            customer_name: Set(req.customer_name),
            opening_balance: Set(req.opening_balance),
            total_invoices: Set(req.total_invoices),
            total_collections: Set(req.total_collections),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set(Some("draft".to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 根据ID获取对账单
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReconciliationModel>, AppError> {
        let model = ReconciliationEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 获取对账单列表
    pub async fn list(
        &self,
        query: ReconciliationQuery,
    ) -> Result<(Vec<ReconciliationModel>, u64), AppError> {
        let mut select = ReconciliationEntity::find();

        if let Some(status) = query.status {
            select = select
                .filter(crate::models::ar_reconciliation::Column::ReconciliationStatus.eq(status));
        }

        if let Some(customer_id) = query.customer_id {
            select =
                select.filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id));
        }

        let total = select.clone().count(&*self.db).await?;

        let paginator = select
            .order_by_desc(crate::models::ar_reconciliation::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator.fetch_page(query.page - 1).await?;

        Ok((models, total))
    }

    /// 更新对账单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(opening_balance) = req.opening_balance {
            active_model.opening_balance = Set(opening_balance);
        }
        if let Some(total_invoices) = req.total_invoices {
            active_model.total_invoices = Set(total_invoices);
        }
        if let Some(total_collections) = req.total_collections {
            active_model.total_collections = Set(total_collections);
        }

        let opening = *active_model.opening_balance.as_ref();
        let invoices = *active_model.total_invoices.as_ref();
        let collections = *active_model.total_collections.as_ref();
        active_model.closing_balance = Set(opening + invoices - collections);

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除对账单
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 只有草稿状态的对账单可以删除
        if model.reconciliation_status.as_deref() != Some("draft") {
            return Err(AppError::business(
                "只有草稿状态的对账单可以删除".to_string(),
            ));
        }

        ReconciliationEntity::delete_by_id(id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        if model.reconciliation_status.as_deref() != Some("draft") {
            return Err(AppError::business(
                "只有草稿状态的对账单可以发送".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("sent".to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 客户确认对账单
    pub async fn confirm(
        &self,
        id: i32,
        confirmed_by: Option<i32>,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("confirmed".to_string()));
        active_model.confirmed_by_customer = Set(Some(true));
        active_model.confirmed_by = Set(confirmed_by);
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 客户提出争议
    pub async fn dispute(&self, id: i32, reason: String) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("disputed".to_string()));
        active_model.dispute_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 关闭对账单
    pub async fn close(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or("draft");
        if status != "confirmed" && status != "disputed" {
            return Err(AppError::business(
                "只有已确认或有争议的对账单可以关闭".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("closed".to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 更新对账单状态（通用）
    pub async fn update_status(
        &self,
        id: i32,
        status: &str,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    // ================================================================
    // 增强功能：自动对账算法
    // ================================================================

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

            let reconciliation_no = DocumentNumberGenerator::generate_no(
                &*self.db,
                "RC",
                ReconciliationEntity,
                crate::models::ar_reconciliation::Column::ReconciliationNo,
            )
            .await?;

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

    // ================================================================
    // 增强功能：账龄分桶计算
    // ================================================================

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

    // ================================================================
    // 增强功能：对账单自动生成
    // ================================================================

    /// 为指定客户自动生成对账单（从发票/收款汇总）
    pub async fn generate_reconciliation(
        &self,
        req: GenerateReconciliationRequest,
        user_id: i32,
    ) -> Result<ReconciliationModel, AppError> {
        let txn = (*self.db).begin().await?;

        let cust = customer::Entity::find_by_id(req.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", req.customer_id)))?;

        let reconciliation_no = DocumentNumberGenerator::generate_no(
            &*self.db,
            "RC",
            ReconciliationEntity,
            crate::models::ar_reconciliation::Column::ReconciliationNo,
        )
        .await?;

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

    // ================================================================
    // 增强功能：对账明细查询
    // ================================================================

    /// 获取对账单及其明细
    pub async fn get_with_details(&self, id: i32) -> Result<ReconciliationWithDetails, AppError> {
        let reconciliation = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let items = crate::models::ar_reconciliation_item::Entity::find()
            .filter(crate::models::ar_reconciliation_item::Column::ReconciliationId.eq(id))
            .order_by(
                crate::models::ar_reconciliation_item::Column::CreatedAt,
                Order::Asc,
            )
            .all(&*self.db)
            .await?;

        let details: Vec<ReconciliationDetail> = items
            .into_iter()
            .map(|item| ReconciliationDetail {
                id: item.id,
                reconciliation_id: item.reconciliation_id,
                item_type: item.item_type,
                document_type: item.document_type,
                document_id: item.document_id,
                document_no: item.document_no,
                document_date: item.document_date,
                amount: item.amount,
                matched_amount: item.matched_amount,
                match_status: item.match_status,
                matched_item_id: item.matched_item_id,
                remarks: item.remarks,
            })
            .collect();

        Ok(ReconciliationWithDetails {
            reconciliation,
            details,
        })
    }

    // ================================================================
    // 增强功能：客户确认/争议处理流程
    // ================================================================

    /// 客户确认对账单（带状态校验）
    pub async fn customer_confirm(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
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

        let updated = active_model.update(&*self.db).await?;

        info!("客户确认对账单成功：id={}", id);
        Ok(updated)
    }

    /// 客户提出争议（带状态校验）
    pub async fn customer_dispute(
        &self,
        id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
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

        let updated = active_model.update(&*self.db).await?;

        info!("客户对账单提出争议：id={}, reason={}", id, reason);
        Ok(updated)
    }

    /// 导出对账单PDF
    pub async fn export_pdf(&self, id: i32) -> Result<Vec<u8>, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 获取对账明细
        let items = ReconciliationItemEntity::find()
            .filter(crate::models::ar_reconciliation_item::Column::ReconciliationId.eq(id))
            .all(&*self.db)
            .await?;

        // 生成PDF内容
        let pdf_content = self.generate_reconciliation_pdf(&model, &items)?;

        Ok(pdf_content)
    }

    /// 生成对账单PDF
    fn generate_reconciliation_pdf(
        &self,
        reconciliation: &ReconciliationModel,
        items: &[ReconciliationItemModel],
    ) -> Result<Vec<u8>, AppError> {
        use crate::services::export_service::{ExportService, ReconciliationPdfItem};

        // 构建明细项
        let pdf_items: Vec<ReconciliationPdfItem> = items
            .iter()
            .map(|item| ReconciliationPdfItem {
                item_type: item.item_type.clone(),
                document_no: item.document_no.as_deref().unwrap_or("").to_string(),
                amount: item.amount.to_string(),
                date: item
                    .document_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default(),
            })
            .collect();

        // 获取客户名称
        let customer_name = format!("客户#{}", reconciliation.customer_id);

        // 生成PDF
        ExportService::generate_reconciliation_pdf(
            &reconciliation.reconciliation_no,
            &customer_name,
            &reconciliation.period_start.format("%Y-%m-%d").to_string(),
            &reconciliation.period_end.format("%Y-%m-%d").to_string(),
            reconciliation
                .reconciliation_status
                .as_deref()
                .unwrap_or("draft"),
            pdf_items,
            &reconciliation.closing_balance.to_string(),
        )
    }
}
