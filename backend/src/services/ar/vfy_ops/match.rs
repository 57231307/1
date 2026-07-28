//! 应收对账 - 核销自动匹配：按客户批量匹配发票和收款（精确/日期顺序/汇总三策略）

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
    /// 自动对账：按客户批量匹配发票和收款（精确/日期顺序/汇总三策略）
    pub async fn auto_match(
        &self,
        req: AutoMatchRequest,
        user_id: i32,
    ) -> Result<Vec<AutoMatchResult>, AppError> {
        let (run_exact, run_date_order) = Self::parse_match_strategy(&req)?;
        let txn = (*self.db).begin().await?;
        let customers = Self::load_match_customers(&txn, req.customer_id).await?;
        let customer_ids: Vec<i32> = customers.iter().map(|c| c.id).collect();
        let mut invoices_by_customer =
            Self::group_invoices_by_customer_for_match(&txn, &customer_ids, req.end_date).await?;
        let mut collections_by_customer =
            Self::group_collections_by_customer(&txn, &customer_ids, req.start_date, req.end_date)
                .await?;

        let mut results = Vec::new();
        // v13 P1-3：N+1 重构，收集所有明细 ActiveModel，循环结束后批量 INSERT
        let mut all_items_to_insert: Vec<crate::models::ar_reconciliation_item::ActiveModel> =
            Vec::new();

        for cust in customers {
            let cust_invoices = invoices_by_customer.remove(&cust.id).unwrap_or_default();
            let cust_collections = collections_by_customer.remove(&cust.id).unwrap_or_default();
            let result = Self::process_customer_match(
                &txn,
                &req,
                user_id,
                run_exact,
                run_date_order,
                cust,
                cust_invoices,
                cust_collections,
                &mut all_items_to_insert,
            )
            .await?;
            results.push(result);
        }

        Self::batch_insert_recon_items(&txn, all_items_to_insert).await?;
        txn.commit().await?;
        Ok(results)
    }

    /// 批量插入对账明细（空集合跳过，避免无效 SQL）
    async fn batch_insert_recon_items(
        txn: &sea_orm::DatabaseTransaction,
        items: Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }
        crate::models::ar_reconciliation_item::Entity::insert_many(items)
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 解析并校验匹配策略，返回 (run_exact, run_date_order) 开关
    fn parse_match_strategy(req: &AutoMatchRequest) -> Result<(bool, bool), AppError> {
        // match_strategy 控制：exact=仅策略1 / date_order=策略1+2 / all=全策略（默认）
        let strategy = req
            .match_strategy
            .as_deref()
            .unwrap_or("all")
            .to_lowercase();
        if !matches!(strategy.as_str(), "exact" | "date_order" | "all") {
            return Err(AppError::validation(format!(
                "无效的匹配策略: {}（支持 exact / date_order / all）",
                strategy
            )));
        }
        let run_exact = matches!(strategy.as_str(), "exact" | "date_order" | "all");
        let run_date_order = matches!(strategy.as_str(), "date_order" | "all");
        Ok((run_exact, run_date_order))
    }

    /// 加载参与匹配的客户列表（指定 ID 或全量 LIMIT 兜底）
    async fn load_match_customers(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: Option<i32>,
    ) -> Result<Vec<customer::Model>, AppError> {
        if let Some(cid) = customer_id {
            Ok(vec![customer::Entity::find_by_id(cid)
                .one(txn)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("客户 {} 不存在", cid))
                })?])
        } else {
            // P3 维度 6 修复（批次 87）：LIMIT 兜底防止全表加载
            Ok(customer::Entity::find().limit(10_000).all(txn).await?)
        }
    }

    /// 批量预加载发票并按 customer_id 分组（InvoiceDate <= end_date 且非 CANCELLED）
    /// 注：aging.rs 中已存在同名 `group_invoices_by_customer`（不同签名，针对账龄分桶），；同一 impl 块不允许重复定义同名方法，故此函数加 `_for_match` 后缀以区分。
    async fn group_invoices_by_customer_for_match(
        txn: &sea_orm::DatabaseTransaction,
        customer_ids: &[i32],
        end_date: chrono::NaiveDate,
    ) -> Result<std::collections::HashMap<i32, Vec<ar_invoice::Model>>, AppError> {
        let all_invoices = if customer_ids.is_empty() {
            Vec::new()
        } else {
            ar_invoice::Entity::find()
                .filter(ar_invoice::Column::CustomerId.is_in(customer_ids.to_vec()))
                .filter(ar_invoice::Column::Status.ne("CANCELLED"))
                .filter(ar_invoice::Column::InvoiceDate.lte(end_date))
                .all(txn)
                .await?
        };
        let mut map: std::collections::HashMap<i32, Vec<ar_invoice::Model>> =
            std::collections::HashMap::new();
        for inv in all_invoices {
            map.entry(inv.customer_id).or_default().push(inv);
        }
        Ok(map)
    }

    /// 批量预加载收款并按 customer_id 分组（[start,end] 内 CONFIRMED）
    async fn group_collections_by_customer(
        txn: &sea_orm::DatabaseTransaction,
        customer_ids: &[i32],
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<std::collections::HashMap<i32, Vec<ar_collection::Model>>, AppError> {
        let all_collections = if customer_ids.is_empty() {
            Vec::new()
        } else {
            ar_collection::Entity::find()
                .filter(ar_collection::Column::CustomerId.is_in(customer_ids.to_vec()))
                .filter(ar_collection::Column::Status.eq(ar_status::COLLECTION_CONFIRMED))
                .filter(ar_collection::Column::CollectionDate.gte(start_date))
                .filter(ar_collection::Column::CollectionDate.lte(end_date))
                .all(txn)
                .await?
        };
        let mut map: std::collections::HashMap<i32, Vec<ar_collection::Model>> =
            std::collections::HashMap::new();
        for c in all_collections {
            map.entry(c.customer_id).or_default().push(c);
        }
        Ok(map)
    }

    /// 单客户对账匹配主流程：建对账单 → 精确匹配 → 日期顺序匹配 → 汇总结果
    async fn process_customer_match(
        txn: &sea_orm::DatabaseTransaction,
        req: &AutoMatchRequest,
        user_id: i32,
        run_exact: bool,
        run_date_order: bool,
        cust: customer::Model,
        cust_invoices: Vec<ar_invoice::Model>,
        cust_collections: Vec<ar_collection::Model>,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> Result<AutoMatchResult, AppError> {
        let (invoices, opening_balance) =
            Self::bucket_period_and_opening(cust_invoices, req.start_date);
        let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
        let total_collections: Decimal = cust_collections.iter().map(|c| c.collection_amount).sum();
        // 批次 27 v7 P1：单号生成在 txn 内，避免断号/重复
        let reconciliation_no = generate_reconciliation_no(txn).await?;
        let closing_balance = opening_balance + total_invoices - total_collections;
        let reconciliation = Self::build_match_reconciliation_model(
            req,
            user_id,
            &cust,
            opening_balance,
            total_invoices,
            total_collections,
            closing_balance,
            reconciliation_no.clone(),
        );
        let rec_model = reconciliation.insert(txn).await?;
        let matched_count = Self::execute_match_strategies(
            &invoices,
            &cust_collections,
            rec_model.id,
            run_exact,
            run_date_order,
            all_items,
        );
        Ok(Self::build_auto_match_result(
            rec_model.id,
            reconciliation_no,
            &cust,
            total_invoices,
            total_collections,
            matched_count,
            invoices.len(),
            cust_collections.len(),
        ))
    }

    /// 构建 auto_match 单客户结果（未匹配数 = 发票+收款 - 命中*2）
    fn build_auto_match_result(
        reconciliation_id: i32,
        reconciliation_no: String,
        cust: &customer::Model,
        total_invoices: Decimal,
        total_collections: Decimal,
        matched_count: usize,
        invoice_count: usize,
        collection_count: usize,
    ) -> AutoMatchResult {
        let unmatched_count = invoice_count + collection_count - matched_count * 2;
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

    /// 执行匹配策略（精确 → 日期顺序）并收集未匹配项，返回匹配命中数
    fn execute_match_strategies(
        invoices: &[ar_invoice::Model],
        cust_collections: &[ar_collection::Model],
        rec_id: i32,
        run_exact: bool,
        run_date_order: bool,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> usize {
        let mut matched_count = 0usize;
        let mut unmatched_collections: Vec<&ar_collection::Model> =
            cust_collections.iter().collect();
        let unmatched_invoices: Vec<&ar_invoice::Model> = if run_exact {
            let (matched, unmatched) =
                Self::run_exact_match_pass(invoices, &mut unmatched_collections, rec_id, all_items);
            matched_count += matched;
            unmatched
        } else {
            invoices.iter().collect()
        };

        if run_date_order {
            let mut remaining = unmatched_collections.clone();
            matched_count += Self::run_date_order_match_pass(
                &unmatched_invoices,
                &mut remaining,
                rec_id,
                all_items,
            );
            Self::push_unmatched_collections(rec_id, remaining, all_items);
        } else {
            Self::push_all_unmatched(
                rec_id,
                unmatched_invoices,
                &unmatched_collections,
                all_items,
            );
        }
        matched_count
    }

    /// 日期顺序匹配后剩余收款标记为 UNMATCHED
    fn push_unmatched_collections(
        rec_id: i32,
        collections: Vec<&ar_collection::Model>,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) {
        for coll in collections {
            all_items.push(Self::make_collection_recon_item(
                rec_id,
                coll,
                None,
                ar_status::MATCH_UNMATCHED,
                None,
            ));
        }
    }

    /// 全部未匹配项（发票+收款）统一标记为 UNMATCHED
    fn push_all_unmatched(
        rec_id: i32,
        unmatched_invoices: Vec<&ar_invoice::Model>,
        unmatched_collections: &[&ar_collection::Model],
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) {
        for inv in unmatched_invoices {
            all_items.push(Self::make_invoice_recon_item(
                rec_id,
                inv,
                None,
                ar_status::MATCH_UNMATCHED,
                None,
            ));
        }
        for coll in unmatched_collections {
            all_items.push(Self::make_collection_recon_item(
                rec_id,
                *coll,
                None,
                ar_status::MATCH_UNMATCHED,
                None,
            ));
        }
    }

    /// 按起始日期分桶：期内发票进 period_invoices，期初未付金额求和为 opening
    fn bucket_period_and_opening(
        cust_invoices: Vec<ar_invoice::Model>,
        start_date: chrono::NaiveDate,
    ) -> (Vec<ar_invoice::Model>, Decimal) {
        let mut period_invoices = Vec::new();
        let mut opening = Decimal::ZERO;
        for inv in cust_invoices {
            if inv.invoice_date >= start_date {
                period_invoices.push(inv);
            } else {
                opening += inv.unpaid_amount;
            }
        }
        (period_invoices, opening)
    }

    /// 构建 auto_match 对账单 ActiveModel（notes 为 None，区别于 reconciliation.rs 同名方法）
    fn build_match_reconciliation_model(
        req: &AutoMatchRequest,
        user_id: i32,
        cust: &customer::Model,
        opening_balance: Decimal,
        total_invoices: Decimal,
        total_collections: Decimal,
        closing_balance: Decimal,
        reconciliation_no: String,
    ) -> ActiveModel {
        ActiveModel {
            id: Default::default(),
            reconciliation_no: Set(reconciliation_no),
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

    /// 策略1 精确金额匹配：金额相等的发票与收款配对，返回 (命中数, 未匹配发票引用)
    fn run_exact_match_pass<'a>(
        invoices: &'a [ar_invoice::Model],
        unmatched_collections: &mut Vec<&ar_collection::Model>,
        rec_id: i32,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> (usize, Vec<&'a ar_invoice::Model>) {
        let mut matched_count = 0;
        let mut unmatched_invoices = Vec::new();
        for inv in invoices {
            let exact_match = unmatched_collections
                .iter()
                .position(|c| c.collection_amount == inv.invoice_amount);

            if let Some(idx) = exact_match {
                let coll = unmatched_collections.remove(idx);
                all_items.push(Self::make_invoice_recon_item(
                    rec_id,
                    inv,
                    Some(inv.invoice_amount),
                    "MATCHED",
                    Some(coll.id),
                ));
                all_items.push(Self::make_collection_recon_item(
                    rec_id,
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

    /// 策略2 日期顺序匹配：30 天内日期最近的发票与收款配对，返回命中数
    fn run_date_order_match_pass(
        unmatched_invoices: &[&ar_invoice::Model],
        remaining_collections: &mut Vec<&ar_collection::Model>,
        rec_id: i32,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) -> usize {
        let mut matched_count = 0;
        for inv in unmatched_invoices {
            let date_match = remaining_collections.iter().position(|c| {
                let date_diff = (c.collection_date - inv.invoice_date).num_days().abs();
                date_diff <= 30
            });

            if let Some(idx) = date_match {
                let coll = remaining_collections.remove(idx);
                let matched = std::cmp::min(inv.invoice_amount, coll.collection_amount);
                Self::push_date_order_matched_items(rec_id, inv, coll, matched, all_items);
                matched_count += 1;
            } else {
                all_items.push(Self::make_invoice_recon_item(
                    rec_id,
                    inv,
                    None,
                    ar_status::MATCH_UNMATCHED,
                    None,
                ));
            }
        }
        matched_count
    }

    /// 日期顺序匹配命中后，构建发票+收款两条对账明细并推入 all_items
    fn push_date_order_matched_items(
        rec_id: i32,
        inv: &ar_invoice::Model,
        coll: &ar_collection::Model,
        matched: Decimal,
        all_items: &mut Vec<crate::models::ar_reconciliation_item::ActiveModel>,
    ) {
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
        all_items.push(Self::make_invoice_recon_item(
            rec_id,
            inv,
            Some(matched),
            inv_status,
            Some(coll.id),
        ));
        all_items.push(Self::make_collection_recon_item(
            rec_id,
            coll,
            Some(matched),
            coll_status,
            Some(inv.id),
        ));
    }

    // ===== auto_match 明细创建辅助（D12 圈复杂度优化，抽取重复构造） =====

    /// 创建发票对账明细 ActiveModel（未插入），统一三种匹配场景的构造
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

    /// 创建收款对账明细 ActiveModel（未插入），统一三种匹配场景的构造
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
