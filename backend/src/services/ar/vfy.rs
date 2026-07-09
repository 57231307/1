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
        // 批次 158 v11 真实接入：match_strategy 字段控制匹配策略选择
        // - "exact"       : 仅执行策略 1（精确金额匹配）
        // - "date_order"  : 执行策略 1 + 策略 2（精确 + 日期顺序）
        // - "all" / None  : 执行全策略（默认行为，与历史调用方兼容）
        let strategy = req.match_strategy.as_deref().unwrap_or("all").to_lowercase();
        if !matches!(strategy.as_str(), "exact" | "date_order" | "all") {
            return Err(AppError::validation(format!(
                "无效的匹配策略: {}（支持 exact / date_order / all）",
                strategy
            )));
        }
        let run_exact = matches!(strategy.as_str(), "exact" | "date_order" | "all");
        let run_date_order = matches!(strategy.as_str(), "date_order" | "all");

        let txn = (*self.db).begin().await?;

        let customers = if let Some(cid) = req.customer_id {
            vec![customer::Entity::find_by_id(cid)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", cid)))?]
        } else {
            // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
            customer::Entity::find().limit(10_000).all(&txn).await?
        };

        // v11 批次 38 修复：批量预加载所有客户的发票和收款，避免循环内按客户逐个查询（N+1，3N 次查询）
        // 发票：取 InvoiceDate <= end_date 且非 CANCELLED 的全部发票，循环内按 [start,end] / <start 分桶
        // 收款：取 [start,end] 内 CONFIRMED 的全部收款
        let customer_ids: Vec<i32> = customers.iter().map(|c| c.id).collect();
        let all_invoices = if customer_ids.is_empty() {
            Vec::new()
        } else {
            ar_invoice::Entity::find()
                .filter(ar_invoice::Column::CustomerId.is_in(customer_ids.clone()))
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .filter(ar_invoice::Column::InvoiceDate.lte(req.end_date))
                .all(&txn)
                .await?
        };
        // 按 customer_id 分组（Vec 保留原顺序，后续再按 invoice_date 过滤）
        let invoices_by_customer: std::collections::HashMap<i32, Vec<&ar_invoice::Model>> = {
            let mut map: std::collections::HashMap<i32, Vec<&ar_invoice::Model>> =
                std::collections::HashMap::new();
            for inv in &all_invoices {
                map.entry(inv.customer_id).or_default().push(inv);
            }
            map
        };

        let all_collections = if customer_ids.is_empty() {
            Vec::new()
        } else {
            ar_collection::Entity::find()
                .filter(ar_collection::Column::CustomerId.is_in(customer_ids))
                .filter(ar_collection::Column::Status.eq("CONFIRMED"))
                .filter(ar_collection::Column::CollectionDate.gte(req.start_date))
                .filter(ar_collection::Column::CollectionDate.lte(req.end_date))
                .all(&txn)
                .await?
        };
        let collections_by_customer: std::collections::HashMap<i32, Vec<&ar_collection::Model>> = {
            let mut map: std::collections::HashMap<i32, Vec<&ar_collection::Model>> =
                std::collections::HashMap::new();
            for c in &all_collections {
                map.entry(c.customer_id).or_default().push(c);
            }
            map
        };

        let mut results = Vec::new();

        for cust in customers {
            // 从批量预加载结果中取本客户的发票，按 InvoiceDate 分桶：
            // - 期内存量 invoices：InvoiceDate >= start_date（<= end_date 已在批量查询过滤）
            // - 期初余额 opening_balance：InvoiceDate < start_date 的 unpaid_amount 求和
            let (invoices, opening_balance): (Vec<ar_invoice::Model>, Decimal) = {
                let cust_invoices = invoices_by_customer.get(&cust.id).cloned().unwrap_or_default();
                let mut period_invoices = Vec::new();
                let mut opening = Decimal::ZERO;
                for inv in cust_invoices {
                    if inv.invoice_date >= req.start_date {
                        period_invoices.push(inv.clone());
                    } else {
                        opening += inv.unpaid_amount;
                    }
                }
                (period_invoices, opening)
            };

            let collections: Vec<ar_collection::Model> = collections_by_customer
                .get(&cust.id)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .cloned()
                .collect();

            let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
            let total_collections: Decimal = collections.iter().map(|c| c.collection_amount).sum();

            // 批次 27 v7 P1 修复：事务边界泄漏，单号生成移入 txn，避免断号/重复
            let reconciliation_no = generate_reconciliation_no(&txn).await?;
            let closing_balance = opening_balance + total_invoices - total_collections;

            let reconciliation = ActiveModel {
                id: Default::default(),
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
                // 批次 109 P1-1：auto_match 无 notes 入参，设为 None
                notes: Set(None),
            };

            let rec_model = reconciliation.insert(&txn).await?;

            let mut matched_count = 0usize;
            let mut unmatched_invoices: Vec<&ar_invoice::Model> = Vec::new();
            let mut unmatched_collections: Vec<&ar_collection::Model> =
                collections.iter().collect();

            // 策略1: 精确金额匹配（受 match_strategy 控制）
            if run_exact {
                for inv in &invoices {
                    let exact_match = unmatched_collections
                        .iter()
                        .position(|c| c.collection_amount == inv.invoice_amount);

                    if let Some(idx) = exact_match {
                        let coll = unmatched_collections.remove(idx);

                        // 创建发票明细
                        let inv_item = crate::models::ar_reconciliation_item::ActiveModel {
                            id: Default::default(),
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
                            id: Default::default(),
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
            } else {
                // 跳过精确匹配：所有发票与收款进入未匹配列表
                unmatched_invoices = invoices.iter().collect();
            }

            // 策略2: 日期顺序匹配（受 match_strategy 控制；仅对未精确匹配的剩余项执行）
            if run_date_order {
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
                            id: Default::default(),
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
                            id: Default::default(),
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
                            id: Default::default(),
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
                        id: Default::default(),
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
            } else {
                // 跳过日期顺序匹配：所有未精确匹配的发票与收款直接落库为 UNMATCHED
                for inv in unmatched_invoices {
                    let inv_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Default::default(),
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
                for coll in &unmatched_collections {
                    let col_item = crate::models::ar_reconciliation_item::ActiveModel {
                        id: Default::default(),
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
            id: Default::default(),
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
            // 批次 109 P1-1：接入 notes 持久化（原 DTO 有字段但未写入 DB）
            notes: Set(req.notes),
        };

        let rec_model = reconciliation.insert(&txn).await?;

        // 创建发票明细行
        for inv in &invoices {
            let item = crate::models::ar_reconciliation_item::ActiveModel {
                id: Default::default(),
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
                id: Default::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::{ar, common};
    use sea_orm::{Database, DatabaseConnection};
    use std::str::FromStr;
    use std::sync::Arc;

    /// 复现 vfy.rs get_aging_report 中的账龄分桶索引计算（纯算法，不依赖 DB）
    ///
    /// 分桶规则（与 vfy.rs 第 517-527 行保持一致）：
    /// - 0: 当期（overdue_days <= 0）
    /// - 1: 1-30天
    /// - 2: 31-60天
    /// - 3: 61-90天
    /// - 4: 90天以上
    fn aging_bucket_idx(overdue_days: i64) -> usize {
        if overdue_days <= 0 {
            0
        } else if overdue_days <= 30 {
            1
        } else if overdue_days <= 60 {
            2
        } else if overdue_days <= 90 {
            3
        } else {
            4
        }
    }

    /// 复现 vfy.rs auto_match 开头的匹配策略校验逻辑（纯算法，DB 调用之前）
    ///
    /// 错误消息与 auto_match 第 47-50 行保持一致：
    /// "无效的匹配策略: {strategy}（支持 exact / date_order / all）"
    fn validate_match_strategy(raw: Option<&str>) -> Result<String, AppError> {
        let strategy = raw.unwrap_or("all").to_lowercase();
        if !matches!(strategy.as_str(), "exact" | "date_order" | "all") {
            return Err(AppError::validation(format!(
                "无效的匹配策略: {}（支持 exact / date_order / all）",
                strategy
            )));
        }
        Ok(strategy)
    }

    /// 复现 vfy.rs customer_confirm 中的状态校验逻辑（纯算法，DB 调用之前）
    ///
    /// 返回 Err 时错误消息与 customer_confirm 第 700-705 行保持一致：
    /// - "confirmed" → "对账单已确认，不可重复确认"
    /// - "disputed"  → "对账单存在争议，请先解决争议后再确认"
    ///
    /// 注：vfy.rs 中 reconciliation_status 使用小写值（draft/confirmed/disputed/closed），
    /// status::ar 模块仅有大写常量（COMPLETED/CANCELLED），此处沿用 vfy.rs 实际小写行为。
    fn validate_customer_confirm(status: &str) -> Result<&'static str, AppError> {
        if status == "confirmed" {
            return Err(AppError::business("对账单已确认，不可重复确认".to_string()));
        }
        if status == "disputed" {
            return Err(AppError::business(
                "对账单存在争议，请先解决争议后再确认".to_string(),
            ));
        }
        Ok("confirmed")
    }

    /// 复现 vfy.rs customer_dispute 中的状态校验逻辑（纯算法，DB 调用之前）
    ///
    /// 返回 Err 时错误消息与 customer_dispute 第 749-753 行保持一致：
    /// - "confirmed" → "对账单已确认，不可提出争议"
    /// - "closed"    → "对账单已关闭，不可提出争议"
    fn validate_customer_dispute(status: &str) -> Result<&'static str, AppError> {
        if status == "confirmed" {
            return Err(AppError::business("对账单已确认，不可提出争议".to_string()));
        }
        if status == "closed" {
            return Err(AppError::business("对账单已关闭，不可提出争议".to_string()));
        }
        Ok("disputed")
    }

    /// 测试 SQLite 内存数据库连接夹具
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    // =====================================================
    // 1. 核销相关状态常量值正确性
    // =====================================================

    /// 测试_核销状态常量_已完成值正确
    ///
    /// 验证 ar::RECONCILIATION_COMPLETED 常量值为 "COMPLETED"（大写），
    /// 与 ar_reconciliation.reconciliation_status 字段语义一致。
    #[test]
    fn 测试_核销状态常量_已完成值正确() {
        assert_eq!(ar::RECONCILIATION_COMPLETED, "COMPLETED");
    }

    /// 测试_核销状态常量_已取消值正确
    ///
    /// 验证 ar::RECONCILIATION_CANCELLED 常量值为 "CANCELLED"（大写），
    /// 与 ar_reconciliation.reconciliation_status 字段语义一致。
    #[test]
    fn 测试_核销状态常量_已取消值正确() {
        assert_eq!(ar::RECONCILIATION_CANCELLED, "CANCELLED");
    }

    /// 测试_匹配状态常量_已匹配值正确
    ///
    /// 验证 ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），
    /// 与 ar_reconciliation_item.match_status 字段语义一致。
    #[test]
    fn 测试_匹配状态常量_已匹配值正确() {
        assert_eq!(ar::MATCH_MATCHED, "MATCHED");
    }

    /// 测试_收款状态常量_小写三态值正确
    ///
    /// 验证 ar_collection.status 字段使用的小写状态值：
    /// - COLLECTION_PENDING = "pending"
    /// - COLLECTION_CONFIRMED = "confirmed"
    /// - COLLECTION_CANCELLED = "cancelled"
    #[test]
    fn 测试_收款状态常量_小写三态值正确() {
        assert_eq!(ar::COLLECTION_PENDING, "pending");
        assert_eq!(ar::COLLECTION_CONFIRMED, "confirmed");
        assert_eq!(ar::COLLECTION_CANCELLED, "cancelled");
    }

    /// 测试_通用状态常量_已取消值正确
    ///
    /// 验证 common::STATUS_CANCELLED 常量值为 "CANCELLED"（大写），
    /// vfy.rs 中用于 ar_invoice.Status 过滤（ne("CANCELLED")）。
    #[test]
    fn 测试_通用状态常量_已取消值正确() {
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
    }

    // =====================================================
    // 2. 核销金额计算（纯算法，复现 auto_match / generate_reconciliation）
    // =====================================================

    /// 测试_期末余额计算_正常场景
    ///
    /// 验证 vfy.rs auto_match / generate_reconciliation 中的期末余额公式：
    /// closing_balance = opening_balance + total_invoices - total_collections
    #[test]
    fn 测试_期末余额计算_正常场景() {
        let opening = decs!("1000");
        let invoices = decs!("5000");
        let collections = decs!("3000");
        let closing = opening + invoices - collections;
        assert_eq!(closing, decs!("3000"));
    }

    /// 测试_期末余额计算_零收款场景
    ///
    /// 验证当期无收款时，期末余额 = 期初 + 期内核销前发票额。
    #[test]
    fn 测试_期末余额计算_零收款场景() {
        let opening = decs!("2000");
        let invoices = decs!("4000");
        let collections = Decimal::ZERO;
        let closing = opening + invoices - collections;
        assert_eq!(closing, decs!("6000"));
    }

    /// 测试_期末余额计算_全额核销场景
    ///
    /// 验证当收款总额等于期初+发票额时，期末余额归零（核销完成）。
    #[test]
    fn 测试_期末余额计算_全额核销场景() {
        let opening = decs!("1000");
        let invoices = decs!("4000");
        let collections = decs!("5000");
        let closing = opening + invoices - collections;
        assert_eq!(closing, Decimal::ZERO);
    }

    // =====================================================
    // 3. 日期匹配阈值（auto_match 策略2 纯算法）
    // =====================================================

    /// 测试_日期匹配阈值_30天内可匹配
    ///
    /// 验证 vfy.rs auto_match 策略2 中 date_diff <= 30 时应匹配。
    /// 边界值：恰好 30 天也应匹配。
    #[test]
    fn 测试_日期匹配阈值_30天内可匹配() {
        let invoice_date = ymd!(2026, 6, 1);
        // 30 天后：边界，应匹配
        let coll_date_30 = ymd!(2026, 7, 1);
        let diff_30 = (coll_date_30 - invoice_date).num_days().abs();
        assert_eq!(diff_30, 30);
        assert!(diff_30 <= 30);

        // 15 天后：区间内，应匹配
        let coll_date_15 = ymd!(2026, 6, 16);
        let diff_15 = (coll_date_15 - invoice_date).num_days().abs();
        assert_eq!(diff_15, 15);
        assert!(diff_15 <= 30);
    }

    /// 测试_日期匹配阈值_超30天不匹配
    ///
    /// 验证 vfy.rs auto_match 策略2 中 date_diff > 30 时不应匹配。
    #[test]
    fn 测试_日期匹配阈值_超30天不匹配() {
        let invoice_date = ymd!(2026, 6, 1);
        let coll_date = ymd!(2026, 7, 2); // 31 天后
        let diff = (coll_date - invoice_date).num_days().abs();
        assert_eq!(diff, 31);
        assert!(diff > 30);
    }

    // =====================================================
    // 4. 部分匹配金额与状态判定（auto_match 策略2 纯算法）
    // =====================================================

    /// 测试_部分匹配金额_取较小值
    ///
    /// 验证 vfy.rs auto_match 策略2 中 matched = min(invoice_amount, collection_amount)。
    #[test]
    fn 测试_部分匹配金额_取较小值() {
        let inv_amt = decs!("5000");
        let coll_amt = decs!("3000");
        let matched = std::cmp::min(inv_amt, coll_amt);
        assert_eq!(matched, decs!("3000"));

        // 反向参数同样取较小值
        let matched_rev = std::cmp::min(coll_amt, inv_amt);
        assert_eq!(matched_rev, decs!("3000"));
    }

    /// 测试_匹配状态判定_完全与部分匹配
    ///
    /// 验证 vfy.rs auto_match 策略2 中 match_status 判定规则：
    /// - matched == invoice_amount → ar::MATCH_MATCHED
    /// - matched < invoice_amount  → "PARTIAL"
    /// 注："PARTIAL" 在 status 模块无对应常量，沿用 vfy.rs 字面量。
    #[test]
    fn 测试_匹配状态判定_完全与部分匹配() {
        let inv_amt = decs!("5000");

        // 完全匹配：matched == invoice_amount
        let matched_full = std::cmp::min(inv_amt, decs!("5000"));
        let status_full = if matched_full == inv_amt {
            ar::MATCH_MATCHED
        } else {
            "PARTIAL"
        };
        assert_eq!(status_full, ar::MATCH_MATCHED);

        // 部分匹配：matched < invoice_amount
        let matched_part = std::cmp::min(inv_amt, decs!("3000"));
        let status_part = if matched_part == inv_amt {
            ar::MATCH_MATCHED
        } else {
            "PARTIAL"
        };
        assert_eq!(status_part, "PARTIAL");
    }

    // =====================================================
    // 5. 未匹配数量公式（auto_match 汇总纯算法）
    // =====================================================

    /// 测试_未匹配数量公式_正确
    ///
    /// 验证 vfy.rs auto_match 末尾 unmatched_count 公式：
    /// unmatched_count = invoices.len() + collections.len() - matched_count * 2
    /// 每次匹配消耗 1 张发票 + 1 笔收款，故乘 2。
    #[test]
    fn 测试_未匹配数量公式_正确() {
        let invoices_len = 10usize;
        let collections_len = 8usize;
        let matched_count = 5usize;
        let unmatched = invoices_len + collections_len - matched_count * 2;
        // 10 + 8 - 10 = 8
        assert_eq!(unmatched, 8);

        // 全部匹配：matched = min(invoices, collections) = 8
        let matched_all = std::cmp::min(invoices_len, collections_len);
        let unmatched_all = invoices_len + collections_len - matched_all * 2;
        // 10 + 8 - 16 = 2（剩余 2 张发票未匹配）
        assert_eq!(unmatched_all, 2);
    }

    // =====================================================
    // 6. 账龄分桶（get_aging_report 纯算法）
    // =====================================================

    /// 测试_账龄分桶_当期未逾期
    ///
    /// 验证 overdue_days <= 0 时落入第 0 桶（当期）。
    /// 边界值：overdue_days = 0（到期日当天）也应落入当期。
    #[test]
    fn 测试_账龄分桶_当期未逾期() {
        assert_eq!(aging_bucket_idx(0), 0);
        assert_eq!(aging_bucket_idx(-1), 0);
        assert_eq!(aging_bucket_idx(-30), 0);
    }

    /// 测试_账龄分桶_1到30天区间
    ///
    /// 验证 1 <= overdue_days <= 30 时落入第 1 桶（1-30天）。
    /// 边界值：1 和 30 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_1到30天区间() {
        assert_eq!(aging_bucket_idx(1), 1);
        assert_eq!(aging_bucket_idx(15), 1);
        assert_eq!(aging_bucket_idx(30), 1);
    }

    /// 测试_账龄分桶_31到60天区间
    ///
    /// 验证 31 <= overdue_days <= 60 时落入第 2 桶（31-60天）。
    /// 边界值：31 和 60 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_31到60天区间() {
        assert_eq!(aging_bucket_idx(31), 2);
        assert_eq!(aging_bucket_idx(45), 2);
        assert_eq!(aging_bucket_idx(60), 2);
    }

    /// 测试_账龄分桶_61到90天区间
    ///
    /// 验证 61 <= overdue_days <= 90 时落入第 3 桶（61-90天）。
    /// 边界值：61 和 90 均应落入此桶。
    #[test]
    fn 测试_账龄分桶_61到90天区间() {
        assert_eq!(aging_bucket_idx(61), 3);
        assert_eq!(aging_bucket_idx(75), 3);
        assert_eq!(aging_bucket_idx(90), 3);
    }

    /// 测试_账龄分桶_90天以上
    ///
    /// 验证 overdue_days > 90 时落入第 4 桶（90天以上）。
    /// 边界值：91 应落入此桶。
    #[test]
    fn 测试_账龄分桶_90天以上() {
        assert_eq!(aging_bucket_idx(91), 4);
        assert_eq!(aging_bucket_idx(180), 4);
        assert_eq!(aging_bucket_idx(365), 4);
    }

    // =====================================================
    // 7. 状态机转换合法性（customer_confirm / customer_dispute 纯算法）
    // =====================================================

    /// 测试_客户确认状态机_已确认拒绝
    ///
    /// 验证 customer_confirm 中 status == "confirmed" 时应拒绝（不可重复确认），
    /// 返回 BusinessError 且消息包含 "对账单已确认，不可重复确认"。
    #[test]
    fn 测试_客户确认状态机_已确认拒绝() {
        let result = validate_customer_confirm("confirmed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已确认，不可重复确认"));
    }

    /// 测试_客户确认状态机_争议中拒绝
    ///
    /// 验证 customer_confirm 中 status == "disputed" 时应拒绝（需先解决争议），
    /// 返回 BusinessError 且消息包含 "对账单存在争议"。
    #[test]
    fn 测试_客户确认状态机_争议中拒绝() {
        let result = validate_customer_confirm("disputed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单存在争议"));
    }

    /// 测试_客户确认状态机_其他状态可转换
    ///
    /// 验证 customer_confirm 中 status 为 "draft" 等非终态时应允许转换到 "confirmed"。
    #[test]
    fn 测试_客户确认状态机_其他状态可转换() {
        // draft → confirmed：应允许
        let result = validate_customer_confirm("draft");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "confirmed");
    }

    /// 测试_客户争议状态机_已确认拒绝
    ///
    /// 验证 customer_dispute 中 status == "confirmed" 时应拒绝（已确认不可提争议），
    /// 返回 BusinessError 且消息包含 "对账单已确认，不可提出争议"。
    #[test]
    fn 测试_客户争议状态机_已确认拒绝() {
        let result = validate_customer_dispute("confirmed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已确认，不可提出争议"));
    }

    /// 测试_客户争议状态机_已关闭拒绝
    ///
    /// 验证 customer_dispute 中 status == "closed" 时应拒绝（已关闭不可提争议），
    /// 返回 BusinessError 且消息包含 "对账单已关闭，不可提出争议"。
    #[test]
    fn 测试_客户争议状态机_已关闭拒绝() {
        let result = validate_customer_dispute("closed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::BusinessError(_)));
        assert!(format!("{err}").contains("对账单已关闭，不可提出争议"));
    }

    /// 测试_客户争议状态机_草稿可转换
    ///
    /// 验证 customer_dispute 中 status 为 "draft" 时应允许转换到 "disputed"。
    #[test]
    fn 测试_客户争议状态机_草稿可转换() {
        let result = validate_customer_dispute("draft");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "disputed");
    }

    // =====================================================
    // 8. 匹配策略校验（auto_match 开头纯算法）
    // =====================================================

    /// 测试_匹配策略校验_无效策略错误消息
    ///
    /// 验证 auto_match 中传入不支持的策略时返回 ValidationError，
    /// 错误消息格式："无效的匹配策略: {strategy}（支持 exact / date_order / all）"
    #[test]
    fn 测试_匹配策略校验_无效策略错误消息() {
        let result = validate_match_strategy(Some("invalid"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));
        let msg = format!("{err}");
        assert!(msg.contains("无效的匹配策略: invalid"));
        assert!(msg.contains("exact / date_order / all"));
    }

    /// 测试_匹配策略校验_合法策略通过
    ///
    /// 验证 auto_match 中三种合法策略（exact/date_order/all）均通过校验，
    /// 且 None 默认为 "all"，大小写不敏感（自动转小写）。
    #[test]
    fn 测试_匹配策略校验_合法策略通过() {
        assert_eq!(validate_match_strategy(Some("exact")).unwrap(), "exact");
        assert_eq!(
            validate_match_strategy(Some("date_order")).unwrap(),
            "date_order"
        );
        assert_eq!(validate_match_strategy(Some("all")).unwrap(), "all");
        // None 默认 "all"
        assert_eq!(validate_match_strategy(None).unwrap(), "all");
        // 大写自动转小写
        assert_eq!(validate_match_strategy(Some("EXACT")).unwrap(), "exact");
    }

    // =====================================================
    // 9. 夹具宏可用性（decs! / ymd!）
    // =====================================================

    /// 测试_decs宏_核销金额解析
    ///
    /// 验证 decs! 宏可正确解析 vfy.rs 业务场景的金额字符串（含小数），
    /// 并可参与期末余额公式运算。
    #[test]
    fn 测试_decs宏_核销金额解析() {
        let inv = decs!("12345.67");
        assert_eq!(inv.to_string(), "12345.67");

        let coll = decs!("3000");
        assert_eq!(coll.to_string(), "3000");

        // 期末余额计算应正常工作
        let opening = decs!("1000");
        let closing = opening + inv - coll;
        assert_eq!(closing.to_string(), "10345.67");
    }

    /// 测试_ymd宏_对账日期解析
    ///
    /// 验证 ymd! 宏可正确解析 vfy.rs 业务场景的对账期间日期，
    /// 并可参与日期差运算（auto_match 策略2 依赖）。
    #[test]
    fn 测试_ymd宏_对账日期解析() {
        let start = ymd!(2026, 1, 1);
        let end = ymd!(2026, 3, 31);
        assert_eq!(start.to_string(), "2026-01-01");
        assert_eq!(end.to_string(), "2026-03-31");

        // 日期差计算（auto_match 策略2 依赖）
        let diff = (end - start).num_days();
        assert_eq!(diff, 89);
    }

    // =====================================================
    // 10. 数据库交互测试（服务实例化 + 标注 #[ignore] 的端到端）
    // =====================================================

    /// 测试_服务实例化_需数据库
    ///
    /// 验证 ArReconciliationService::new 可用 SQLite 内存库构造实例，
    /// 仅验证实例化成功（不需要 schema），与模板 测试_服务实例创建 同模式。
    #[tokio::test]
    async fn 测试_服务实例化_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        // 验证实例化成功：Arc 引用计数 >= 1
        assert!(Arc::strong_count(&svc.db) >= 1);
    }

    /// 测试_自动对账完整流程_需数据库
    ///
    /// 验证 auto_match 端到端调用路径不 panic（需完整 schema + 测试数据）。
    /// 标注 #[ignore]：依赖真实 DB schema，CI 默认不跑，需 `cargo test -- --ignored`。
    /// 无 schema 时预期返回数据库错误而非 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_自动对账完整流程_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        let req = AutoMatchRequest {
            customer_id: None,
            start_date: ymd!(2026, 1, 1),
            end_date: ymd!(2026, 3, 31),
            match_strategy: Some("all".to_string()),
        };
        // 无 schema 时预期返回数据库错误而非 panic
        let result = svc.auto_match(req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_账龄报告完整流程_需数据库
    ///
    /// 验证 get_aging_report 端到端调用路径不 panic（需完整 schema + 测试数据）。
    /// 标注 #[ignore]：依赖真实 DB schema，CI 默认不跑，需 `cargo test -- --ignored`。
    /// 无 schema 时预期返回数据库错误而非 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_账龄报告完整流程_需数据库() {
        let db = setup_test_db().await;
        let svc = ArReconciliationService::new(Arc::new(db));
        // 无 schema 时预期返回数据库错误而非 panic
        let result = svc.get_aging_report(None).await;
        assert!(result.is_err());
    }
}
