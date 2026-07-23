//! 应收对账 - 自动生成对账单（ar/vfy_ops/reconciliation）
//!
//! 批次 490 D10-4b 拆分自原 `ar/vfy.rs` 的 `generate_reconciliation` 方法及其辅助函数。
//! 职责：为指定客户在给定期间内拉取发票/收款/期初余额，汇总后生成对账单（含明细行）。
//! 本模块扩展 `ArReconciliationService` 的 `generate_reconciliation` 公开方法与
//! `fetch_invoices_for_reconciliation` / `fetch_collections_for_reconciliation` /
//! `fetch_opening_balance` / `build_reconciliation_active_model` /
//! `insert_invoice_items` / `insert_collection_items` 私有辅助。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use tracing::info;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::models::ar_reconciliation::ActiveModel;
use crate::models::customer;
use crate::models::status::ar as ar_status;
use crate::utils::error::AppError;

use super::super::{
    generate_reconciliation_no, ArReconciliationService, GenerateReconciliationRequest,
};

impl ArReconciliationService {
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

        let invoices = Self::fetch_invoices_for_reconciliation(
            &txn, req.customer_id, req.start_date, req.end_date,
        )
        .await?;

        let collections = Self::fetch_collections_for_reconciliation(
            &txn, req.customer_id, req.start_date, req.end_date,
        )
        .await?;

        let opening_balance =
            Self::fetch_opening_balance(&txn, req.customer_id, req.start_date).await?;

        let total_invoices: Decimal = invoices.iter().map(|inv| inv.invoice_amount).sum();
        let total_collections: Decimal = collections.iter().map(|c| c.collection_amount).sum();
        let closing_balance = opening_balance + total_invoices - total_collections;

        let reconciliation = Self::build_reconciliation_active_model(
            reconciliation_no,
            &req,
            &cust,
            opening_balance,
            total_invoices,
            total_collections,
            closing_balance,
            user_id,
        );

        let rec_model = reconciliation.insert(&txn).await?;

        Self::insert_invoice_items(&txn, rec_model.id, &invoices).await?;
        Self::insert_collection_items(&txn, rec_model.id, &collections).await?;

        info!(
            "自动生成对账单成功：客户={}, 发票={}笔, 收款={}笔",
            cust.customer_name,
            invoices.len(),
            collections.len()
        );

        txn.commit().await?;
        Ok(rec_model)
    }

    async fn fetch_invoices_for_reconciliation(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<ar_invoice::Model>, AppError> {
        Ok(ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.eq(customer_id))
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::InvoiceDate.gte(start_date))
            .filter(ar_invoice::Column::InvoiceDate.lte(end_date))
            .all(txn)
            .await?)
    }

    async fn fetch_collections_for_reconciliation(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<Vec<ar_collection::Model>, AppError> {
        Ok(ar_collection::Entity::find()
            .filter(ar_collection::Column::CustomerId.eq(customer_id))
            .filter(ar_collection::Column::Status.eq(ar_status::COLLECTION_CONFIRMED))
            .filter(ar_collection::Column::CollectionDate.gte(start_date))
            .filter(ar_collection::Column::CollectionDate.lte(end_date))
            .all(txn)
            .await?)
    }

    async fn fetch_opening_balance(
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
        start_date: chrono::NaiveDate,
    ) -> Result<Decimal, AppError> {
        let prev_invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::CustomerId.eq(customer_id))
            .filter(ar_invoice::Column::Status.ne("CANCELLED"))
            .filter(ar_invoice::Column::InvoiceDate.lt(start_date))
            .all(txn)
            .await?;
        Ok(prev_invoices.iter().map(|inv| inv.unpaid_amount).sum())
    }

    fn build_reconciliation_active_model(
        reconciliation_no: String,
        req: &GenerateReconciliationRequest,
        cust: &customer::Model,
        opening_balance: Decimal,
        total_invoices: Decimal,
        total_collections: Decimal,
        closing_balance: Decimal,
        user_id: i32,
    ) -> ActiveModel {
        ActiveModel {
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
            reconciliation_status: Set(Some(ar_status::RECONCILIATION_DRAFT.to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // 批次 109 P1-1：接入 notes 持久化（原 DTO 有字段但未写入 DB）
            notes: Set(req.notes.clone()),
        }
    }

    async fn insert_invoice_items(
        txn: &sea_orm::DatabaseTransaction,
        rec_id: i32,
        invoices: &[ar_invoice::Model],
    ) -> Result<(), AppError> {
        for inv in invoices {
            let item = crate::models::ar_reconciliation_item::ActiveModel {
                id: Default::default(),
                reconciliation_id: Set(rec_id),
                item_type: Set("INVOICE".to_string()),
                document_type: Set(Some("SALES_INVOICE".to_string())),
                document_id: Set(Some(inv.id)),
                document_no: Set(Some(inv.invoice_no.clone())),
                document_date: Set(Some(inv.invoice_date)),
                amount: Set(inv.invoice_amount),
                matched_amount: Set(None),
                match_status: Set(ar_status::MATCH_UNMATCHED.to_string()),
                matched_item_id: Set(None),
                remarks: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            item.insert(txn).await?;
        }
        Ok(())
    }

    async fn insert_collection_items(
        txn: &sea_orm::DatabaseTransaction,
        rec_id: i32,
        collections: &[ar_collection::Model],
    ) -> Result<(), AppError> {
        for coll in collections {
            let item = crate::models::ar_reconciliation_item::ActiveModel {
                id: Default::default(),
                reconciliation_id: Set(rec_id),
                item_type: Set("RECEIPT".to_string()),
                document_type: Set(Some("COLLECTION".to_string())),
                document_id: Set(Some(coll.id)),
                document_no: Set(Some(coll.collection_no.clone())),
                document_date: Set(Some(coll.collection_date)),
                amount: Set(-coll.collection_amount),
                matched_amount: Set(None),
                match_status: Set(ar_status::MATCH_UNMATCHED.to_string()),
                matched_item_id: Set(None),
                remarks: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            item.insert(txn).await?;
        }
        Ok(())
    }
}
