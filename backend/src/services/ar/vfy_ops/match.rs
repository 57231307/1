//! 应收对账 - 核销自动匹配（ar/vfy_ops/match）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs` 的 `auto_match` 方法及其明细创建辅助函数。
//! 职责：按客户批量匹配发票和收款，支持三种策略（精确金额 / 日期顺序 / 客户汇总）。
//! 本模块扩展 `ArReconciliationService` 的 `auto_match` 公开方法与
//! `make_invoice_recon_item` / `make_collection_recon_item` 私有辅助。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::models::ar_reconciliation::ActiveModel;
use crate::models::customer;
use crate::models::status::ar as ar_status;
use crate::utils::error::AppError;

use super::super::{
    generate_reconciliation_no, ArReconciliationService, AutoMatchRequest, AutoMatchResult,
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
                .filter(ar_collection::Column::Status.eq(ar_status::COLLECTION_CONFIRMED))
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
        // v13 P1-3：N+1 重构，收集所有明细 ActiveModel，循环结束后批量 INSERT
        let mut all_items_to_insert: Vec<crate::models::ar_reconciliation_item::ActiveModel> =
            Vec::new();

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
                reconciliation_status: Set(Some(ar_status::RECONCILIATION_DRAFT.to_string())),
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

                        // 创建发票明细（收集不立即 INSERT）
                        all_items_to_insert.push(Self::make_invoice_recon_item(
                            rec_model.id,
                            inv,
                            Some(inv.invoice_amount),
                            "MATCHED",
                            Some(coll.id),
                        ));

                        // 创建收款明细（收集不立即 INSERT）
                        all_items_to_insert.push(Self::make_collection_recon_item(
                            rec_model.id,
                            coll,
                            Some(coll.collection_amount),
                            "MATCHED",
                            Some(inv.id),
                        ));

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
                        let inv_status = if matched == inv.invoice_amount {
                            "MATCHED"
                        } else {
                            "PARTIAL"
                        };
                        let coll_status = if matched == coll.collection_amount {
                            "MATCHED"
                        } else {
                            "PARTIAL"
                        };

                        all_items_to_insert.push(Self::make_invoice_recon_item(
                            rec_model.id,
                            inv,
                            Some(matched),
                            inv_status,
                            Some(coll.id),
                        ));

                        all_items_to_insert.push(Self::make_collection_recon_item(
                            rec_model.id,
                            coll,
                            Some(matched),
                            coll_status,
                            Some(inv.id),
                        ));

                        matched_count += 1;
                    } else {
                        // 未匹配发票（收集不立即 INSERT）
                        all_items_to_insert.push(Self::make_invoice_recon_item(
                            rec_model.id,
                            inv,
                            None,
                            ar_status::MATCH_UNMATCHED,
                            None,
                        ));
                    }
                }

                // 剩余未匹配收款（收集不立即 INSERT）
                for coll in remaining_collections {
                    all_items_to_insert.push(Self::make_collection_recon_item(
                        rec_model.id,
                        coll,
                        None,
                        ar_status::MATCH_UNMATCHED,
                        None,
                    ));
                }
            } else {
                // 跳过日期顺序匹配：所有未精确匹配的发票与收款直接收集为 UNMATCHED
                for inv in unmatched_invoices {
                    all_items_to_insert.push(Self::make_invoice_recon_item(
                        rec_model.id,
                        inv,
                        None,
                        ar_status::MATCH_UNMATCHED,
                        None,
                    ));
                }
                for coll in &unmatched_collections {
                    all_items_to_insert.push(Self::make_collection_recon_item(
                        rec_model.id,
                        *coll,
                        None,
                        ar_status::MATCH_UNMATCHED,
                        None,
                    ));
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
                status: ar_status::RECONCILIATION_DRAFT.to_string(),
            });
        }

        // 批量 INSERT 所有对账明细，替代逐条 INSERT（v13 P1-3：N+1 重构）
        if !all_items_to_insert.is_empty() {
            crate::models::ar_reconciliation_item::Entity::insert_many(all_items_to_insert)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;
        Ok(results)
    }

    // ===== auto_match 明细创建辅助函数（D12 圈复杂度优化） =====
    // 抽取自 auto_match 内 8 处重复的 ActiveModel 构造代码，消除冗余并降低圈复杂度
    // 调用方负责传入正确的 matched_amount / match_status / matched_item_id

    /// 创建发票对账明细 ActiveModel（未插入）
    ///
    /// 统一 auto_match 三种场景的发票明细创建：
    /// - 精确匹配命中：matched_amount=Some(inv.invoice_amount), status=MATCHED
    /// - 日期顺序匹配命中：matched_amount=Some(matched), status=MATCHED/PARTIAL
    /// - 未匹配：matched_amount=None, status=UNMATCHED
    fn make_invoice_recon_item(
        reconciliation_id: i32,
        inv: &ar_invoice::Model,
        matched_amount: Option<Decimal>,
        match_status: &str,
        matched_item_id: Option<i32>,
    ) -> crate::models::ar_reconciliation_item::ActiveModel {
        crate::models::ar_reconciliation_item::ActiveModel {
            id: Default::default(),
            reconciliation_id: Set(reconciliation_id),
            item_type: Set("INVOICE".to_string()),
            document_type: Set(Some("SALES_INVOICE".to_string())),
            document_id: Set(Some(inv.id)),
            document_no: Set(Some(inv.invoice_no.clone())),
            document_date: Set(Some(inv.invoice_date)),
            amount: Set(inv.invoice_amount),
            matched_amount: Set(matched_amount),
            match_status: Set(match_status.to_string()),
            matched_item_id: Set(matched_item_id),
            remarks: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
    }

    /// 创建收款对账明细 ActiveModel（未插入）
    ///
    /// 统一 auto_match 三种场景的收款明细创建：
    /// - 精确匹配命中：matched_amount=Some(coll.collection_amount), status=MATCHED
    /// - 日期顺序匹配命中：matched_amount=Some(matched), status=MATCHED/PARTIAL
    /// - 未匹配：matched_amount=None, status=UNMATCHED
    fn make_collection_recon_item(
        reconciliation_id: i32,
        coll: &ar_collection::Model,
        matched_amount: Option<Decimal>,
        match_status: &str,
        matched_item_id: Option<i32>,
    ) -> crate::models::ar_reconciliation_item::ActiveModel {
        crate::models::ar_reconciliation_item::ActiveModel {
            id: Default::default(),
            reconciliation_id: Set(reconciliation_id),
            item_type: Set("RECEIPT".to_string()),
            document_type: Set(Some("COLLECTION".to_string())),
            document_id: Set(Some(coll.id)),
            document_no: Set(Some(coll.collection_no.clone())),
            document_date: Set(Some(coll.collection_date)),
            amount: Set(-coll.collection_amount),
            matched_amount: Set(matched_amount),
            match_status: Set(match_status.to_string()),
            matched_item_id: Set(matched_item_id),
            remarks: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }
    }
}
