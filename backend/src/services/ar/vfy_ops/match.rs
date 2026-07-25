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

/// 自动对账匹配策略解析结果
struct MatchStrategy {
    run_exact: bool,
    run_date_order: bool,
}

impl MatchStrategy {
    /// 解析 match_strategy 字段，非法值返回校验错误
    fn parse(match_strategy: Option<&str>) -> Result<Self, AppError> {
        let strategy = match_strategy.unwrap_or("all").to_lowercase();
        if !matches!(strategy.as_str(), "exact" | "date_order" | "all") {
            return Err(AppError::validation(format!(
                "无效的匹配策略: {}（支持 exact / date_order / all）",
                strategy
            )));
        }
        let run_exact = matches!(strategy.as_str(), "exact" | "date_order" | "all");
        let run_date_order = matches!(strategy.as_str(), "date_order" | "all");
        Ok(Self {
            run_exact,
            run_date_order,
        })
    }
}

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
        // 批次 158 v11：match_strategy 控制 exact / date_order / all 三种策略组合
        let strategy = MatchStrategy::parse(req.match_strategy.as_deref())?;

        let txn = (*self.db).begin().await?;

        // 1. 加载客户与批量预加载发票/收款（避免循环内 N+1 查询）
        let customers = Self::load_customers_for_match(&txn, req.customer_id).await?;
        let customer_ids: Vec<i32> = customers.iter().map(|c| c.id).collect();
        let all_invoices = Self::preload_invoices_for_match(&txn, &customer_ids, req.end_date).await?;
        let all_collections = Self::preload_collections_for_match(
            &txn,
            &customer_ids,
            req.start_date,
            req.end_date,
        )
        .await?;
        let invoices_by_customer = Self::group_invoice_refs_by_customer(&all_invoices);
        let collections_by_customer = Self::group_collections_by_customer(&all_collections);

        // 2. 逐客户匹配，收集明细 ActiveModel 待批量插入
        let mut results = Vec::new();
        let mut all_items_to_insert: Vec<crate::models::ar_reconciliation_item::ActiveModel> =
            Vec::new();

        for cust in &customers {
            let result = self
                .process_one_customer_match(
                    &txn,
                    cust,
                    &invoices_by_customer,
                    &collections_by_customer,
                    &req,
                    &strategy,
                    user_id,
                    &mut all_items_to_insert,
                )
                .await?;
            results.push(result);
        }

        // 3. 批量 INSERT 所有对账明细（v13 P1-3：N+1 重构）
        if !all_items_to_insert.is_empty() {
            crate::models::ar_reconciliation_item::Entity::insert_many(all_items_to_insert)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;
        Ok(results)
    }

    /// 加载参与匹配的客户列表
    async fn load_customers_for_match(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: Option<i32>,
    ) -> Result<Vec<customer::Model>, AppError> {
        Ok(if let Some(cid) = customer_id {
            vec![customer::Entity::find_by_id(cid)
                .one(txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", cid)))?]
        } else {
            // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载
            customer::Entity::find().limit(10_000).all(txn).await?
        })
    }

    /// 批量预加载所有客户的发票（InvoiceDate <= end_date 且非 CANCELLED）
    async fn preload_invoices_for_match(
        txn: &sea_orm::DatabaseTransaction,
        customer_ids: &[i32],
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<ar_invoice::Model>, AppError> {
        if customer_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.is_in(customer_ids.to_vec()))
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::InvoiceDate.lte(end_date))
            .all(txn)
            .await?)
    }

    /// 批量预加载所有客户的收款（[start, end] 内 CONFIRMED）
    async fn preload_collections_for_match(
        txn: &sea_orm::DatabaseTransaction,
        customer_ids: &[i32],
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<ar_collection::Model>, AppError> {
        if customer_ids.is_empty() {
            return Ok(Vec::new());
        }
        Ok(ar_collection::Entity::find()
            .filter(ar_collection::Column::CustomerId.is_in(customer_ids.to_vec()))
            .filter(ar_collection::Column::Status.eq(ar_status::COLLECTION_CONFIRMED))
            .filter(ar_collection::Column::CollectionDate.gte(start_date))
            .filter(ar_collection::Column::CollectionDate.lte(end_date))
            .all(txn)
            .await?)
    }

    /// 按客户 ID 分组发票引用（仅 id -> refs，不带客户名；与 aging.rs 同名不同签名故改名）
    fn group_invoice_refs_by_customer<'a>(
        invoices: &'a [ar_invoice::Model],
    ) -> std::collections::HashMap<i32, Vec<&'a ar_invoice::Model>> {
        let mut map: std::collections::HashMap<i32, Vec<&'a ar_invoice::Model>> =
            std::collections::HashMap::new();
        for inv in invoices {
            map.entry(inv.customer_id).or_default().push(inv);
        }
        map
    }

    /// 按客户 ID 分组收款引用
    fn group_collections_by_customer<'a>(
        collections: &'a [ar_collection::Model],
    ) -> std::collections::HashMap<i32, Vec<&'a ar_collection::Model>> {
        let mut map: std::collections::HashMap<i32, Vec<&'a ar_collection::Model>> =
            std::collections::HashMap::new();
        for c in collections {
            map.entry(c.customer_id).or_default().push(c);
        }
        map
    }

    /// 按对账期间分桶发票：期内存量 + 期初余额（InvoiceDate < start_date 的未付金额之和）
    fn partition_invoices_by_period<'a>(
        cust_invoices: &[&'a ar_invoice::Model],
        start_date: chrono::NaiveDate,
    ) -> (Vec<ar_invoice::Model>, Decimal) {
        let mut period_invoices = Vec::new();
        let mut opening = Decimal::ZERO;
        for inv in cust_invoices {
            if inv.invoice_date >= start_date {
                period_invoices.push((*inv).clone());
            } else {
                opening += inv.unpaid_amount;
            }
        }
        (period_invoices, opening)
    }

    /// 构建新对账单 ActiveModel
    fn build_new_reconciliation_active_model(
        req: &AutoMatchRequest,
        cust: &customer::Model,
        opening_balance: Decimal,
        total_invoices: Decimal,
        total_collections: Decimal,
        closing_balance: Decimal,
        reconciliation_no: &str,
        user_id: i32,
    ) -> ActiveModel {
        ActiveModel {
            id: Default::default(),
            reconciliation_no: Set(reconciliation_no.to_string()),
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
        }
    }

    /// 处理单客户的对账单创建 + 匹配策略执行，返回该客户的匹配结果
    async fn process_one_customer_match<'a, 'b>(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        cust: &customer::Model,
        invoices_by_customer: &std::collections::HashMap<i32, Vec<&'a ar_invoice::Model>>,
        collections_by_customer: &std::collections::HashMap<i32, Vec<&'b ar_collection::Model>>,
        req: &AutoMatchRequest,
        strategy: &MatchStrategy,
        user_id: i32,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> Result<AutoMatchResult, AppError> {
        let (invoices, opening_balance) = Self::partition_invoices_by_period(
            invoices_by_customer
                .get(&cust.id)
                .cloned()
                .unwrap_or_default()
                .as_slice(),
            req.start_date,
        );
        let collections: Vec<ar_collection::Model> = collections_by_customer
            .get(&cust.id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .collect();

        let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
        let total_collections: Decimal = collections.iter().map(|c| c.collection_amount).sum();

        // 批次 27 v7 P1 修复：单号生成移入 txn，避免断号/重复
        let reconciliation_no = generate_reconciliation_no(txn).await?;
        let closing_balance = opening_balance + total_invoices - total_collections;

        let reconciliation = Self::build_new_reconciliation_active_model(
            req,
            cust,
            opening_balance,
            total_invoices,
            total_collections,
            closing_balance,
            &reconciliation_no,
            user_id,
        );
        let rec_model = reconciliation.insert(txn).await?;

        let (mut matched_count, unmatched_invoices, mut unmatched_collections) =
            Self::run_exact_strategy(&invoices, &collections, strategy.run_exact, rec_model.id, all_items_to_insert);

        if strategy.run_date_order {
            matched_count += Self::run_date_order_match_strategy(
                unmatched_invoices,
                &mut unmatched_collections,
                all_items_to_insert,
                rec_model.id,
            );
        } else {
            Self::collect_unmatched_invoices(unmatched_invoices, all_items_to_insert, rec_model.id);
        }
        Self::collect_unmatched_collections(unmatched_collections, all_items_to_insert, rec_model.id);

        let unmatched_count = invoices.len() + collections.len() - matched_count * 2;
        Ok(Self::build_auto_match_result(
            rec_model.id,
            reconciliation_no,
            cust,
            total_invoices,
            total_collections,
            matched_count,
            unmatched_count,
        ))
    }

    /// 执行策略1精确匹配或跳过，返回 (匹配数, 未匹配发票, 未匹配收款)
    fn run_exact_strategy<'a, 'b>(
        invoices: &'a [ar_invoice::Model],
        collections: &'b [ar_collection::Model],
        run_exact: bool,
        rec_model_id: i32,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> (usize, Vec<&'a ar_invoice::Model>, Vec<&'b ar_collection::Model>) {
        let mut unmatched_collections: Vec<&'b ar_collection::Model> = collections.iter().collect();
        if run_exact {
            let (matched, unmatched_inv) = Self::run_exact_match_strategy(
                invoices,
                &mut unmatched_collections,
                all_items_to_insert,
                rec_model_id,
            );
            (matched, unmatched_inv, unmatched_collections)
        } else {
            let unmatched_invoices: Vec<&'a ar_invoice::Model> = invoices.iter().collect();
            (0, unmatched_invoices, unmatched_collections)
        }
    }

    /// 策略1：精确金额匹配，返回 (匹配数, 未匹配发票引用列表)
    fn run_exact_match_strategy<'a, 'b>(
        invoices: &'a [ar_invoice::Model],
        unmatched_collections: &mut Vec<&'b ar_collection::Model>,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
        rec_model_id: i32,
    ) -> (usize, Vec<&'a ar_invoice::Model>) {
        let mut matched_count = 0usize;
        let mut unmatched_invoices: Vec<&'a ar_invoice::Model> = Vec::new();

        for inv in invoices {
            let exact_match = unmatched_collections
                .iter()
                .position(|c| c.collection_amount == inv.invoice_amount);

            if let Some(idx) = exact_match {
                let coll = unmatched_collections.remove(idx);
                all_items_to_insert.push(Self::make_invoice_recon_item(
                    rec_model_id,
                    inv,
                    Some(inv.invoice_amount),
                    "MATCHED",
                    Some(coll.id),
                ));
                all_items_to_insert.push(Self::make_collection_recon_item(
                    rec_model_id,
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

        (matched_count, unmatched_invoices)
    }

    /// 策略2：日期顺序匹配（30 天窗口），返回新增匹配数
    fn run_date_order_match_strategy<'a, 'b>(
        unmatched_invoices: Vec<&'a ar_invoice::Model>,
        remaining_collections: &mut Vec<&'b ar_collection::Model>,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
        rec_model_id: i32,
    ) -> usize {
        let mut matched_count = 0usize;
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
                    rec_model_id,
                    inv,
                    Some(matched),
                    inv_status,
                    Some(coll.id),
                ));
                all_items_to_insert.push(Self::make_collection_recon_item(
                    rec_model_id,
                    coll,
                    Some(matched),
                    coll_status,
                    Some(inv.id),
                ));
                matched_count += 1;
            } else {
                all_items_to_insert.push(Self::make_invoice_recon_item(
                    rec_model_id,
                    inv,
                    None,
                    ar_status::MATCH_UNMATCHED,
                    None,
                ));
            }
        }
        matched_count
    }

    /// 将未匹配发票收集为 UNMATCHED 明细
    fn collect_unmatched_invoices<'a>(
        unmatched_invoices: Vec<&'a ar_invoice::Model>,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
        rec_model_id: i32,
    ) {
        for inv in unmatched_invoices {
            all_items_to_insert.push(Self::make_invoice_recon_item(
                rec_model_id,
                inv,
                None,
                ar_status::MATCH_UNMATCHED,
                None,
            ));
        }
    }

    /// 将未匹配收款收集为 UNMATCHED 明细
    fn collect_unmatched_collections<'b>(
        unmatched_collections: Vec<&'b ar_collection::Model>,
        all_items_to_insert: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
        rec_model_id: i32,
    ) {
        for coll in unmatched_collections {
            all_items_to_insert.push(Self::make_collection_recon_item(
                rec_model_id,
                coll,
                None,
                ar_status::MATCH_UNMATCHED,
                None,
            ));
        }
    }

    /// 构建单客户匹配结果
    fn build_auto_match_result(
        reconciliation_id: i32,
        reconciliation_no: String,
        cust: &customer::Model,
        total_invoices: Decimal,
        total_collections: Decimal,
        matched_count: usize,
        unmatched_count: usize,
    ) -> AutoMatchResult {
        AutoMatchResult {
            reconciliation_id,
            reconciliation_no,
            customer_id: cust.id,
            customer_name: cust.customer_name.clone(),
            total_invoices,
            total_collections,
            matched_count,
            unmatched_count,
            status: ar_status::RECONCILIATION_DRAFT.to_string(),
        }
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
