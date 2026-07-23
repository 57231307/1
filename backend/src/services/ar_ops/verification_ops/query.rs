//! 应收账款-核销查询（ar_ops/verification_ops/query）
//!
//! D10 拆分自原 `ar_ops/verification.rs`，包含 4 个查询类公开 API：
//! - `list_verifications`     核销列表（支持 invoice_id / payment_id / status 过滤 + 行级数据权限）
//! - `get_verification`       核销详情（含明细行 + IDOR 防护）
//! - `get_unverified_invoices` 未核销发票列表（支持 customer_id 过滤）
//! - `get_unverified_payments` 未核销收款列表（支持 customer_id 过滤 + 已核销金额排除）
//!
//! 业务规则：
//! - list/get 支持 V15 P0-S01 行级数据权限（ar_reconciliation 表无 department_id，Dept 退化为 Self）
//! - get_unverified_payments 批量查询已有核销记录，过滤已完全核销的收款

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde_json::json;

use crate::models::{ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;

use super::super::json_helpers::{
    collection_to_json, invoice_to_json, reconciliation_item_to_json, reconciliation_to_json,
};
use crate::services::ar_service::ArService;

impl ArService {
    /// 获取核销列表
    pub async fn list_verifications(
        &self,
        page: u64,
        page_size: u64,
        invoice_id: Option<i32>,
        payment_id: Option<i32>,
        status: Option<String>,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<(Vec<serde_json::Value>, i64), AppError> {
        // 若按 invoice_id/payment_id 过滤，需先查 ar_reconciliation_items 拿到 reconciliation_id 集合
        let mut query = ar_reconciliation::Entity::find();

        if let Some(s) = status {
            query = query.filter(ar_reconciliation::Column::ReconciliationStatus.eq(s));
        }

        if let Some(inv_id) = invoice_id {
            let rec_ids: Vec<i32> = ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("INVOICE"))
                .filter(ar_reconciliation_item::Column::DocumentId.eq(inv_id))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|i| i.reconciliation_id)
                .collect();
            if rec_ids.is_empty() {
                return Ok((vec![], 0));
            }
            query = query.filter(ar_reconciliation::Column::Id.is_in(rec_ids));
        }

        if let Some(pay_id) = payment_id {
            let rec_ids: Vec<i32> = ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.eq(pay_id))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|i| i.reconciliation_id)
                .collect();
            if rec_ids.is_empty() {
                return Ok((vec![], 0));
            }
            query = query.filter(ar_reconciliation::Column::Id.is_in(rec_ids));
        }

        // V15 P0-S01：行级数据权限过滤
        // ar_reconciliation 表无 department_id，Dept 退化为 Self，使用 created_by（Option<i32>）。
        if let Some(ctx) = data_scope {
            query = apply_data_scope(
                query,
                ctx,
                ar_reconciliation::Column::CreatedBy,
                ar_reconciliation::Column::CreatedBy, // 无 department_id，Dept 退化为 Self，复用 created_by
            );
        }

        let total = query.clone().count(&*self.db).await? as i64;
        let items = query
            .order_by(ar_reconciliation::Column::ReconciliationDate, Order::Desc)
            .offset(page.saturating_sub(1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;

        let list = items.into_iter().map(reconciliation_to_json).collect();
        Ok((list, total))
    }

    /// 获取核销详情（含明细行）
    pub async fn get_verification(
        &self,
        verification_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let reconciliation = ar_reconciliation::Entity::find_by_id(verification_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("核销单 {} 不存在", verification_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // ar_reconciliation 表无 department_id，Dept 退化为 Self；
        // ar_reconciliation.created_by 是 Option<i32>（可能为空，空时按"无主数据"处理）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, reconciliation.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问核销单 {}（数据范围限制）",
                    verification_id
                )));
            }
        }

        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ReconciliationId.eq(verification_id))
            .all(&*self.db)
            .await?;

        let mut result = reconciliation_to_json(reconciliation);
        if let Some(obj) = result.as_object_mut() {
            obj.insert(
                "items".to_string(),
                json!(items
                    .into_iter()
                    .map(reconciliation_item_to_json)
                    .collect::<Vec<_>>()),
            );
        }
        Ok(result)
    }

    /// 获取未核销发票
    /// 支持 query.customer_id 过滤
    pub async fn get_unverified_invoices(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::Status.ne(crate::models::status::common::STATUS_CANCELLED))
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO));

        if let Some(cid) = query.get("customer_id").and_then(|v| v.as_i64()) {
            q = q.filter(ar_invoice::Column::CustomerId.eq(cid as i32));
        }

        let invoices = q
            .order_by(ar_invoice::Column::DueDate, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(json!(invoices
            .into_iter()
            .map(invoice_to_json)
            .collect::<Vec<_>>()))
    }

    /// 获取未核销收款
    /// 支持 query.customer_id 过滤
    pub async fn get_unverified_payments(
        &self,
        query: serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        let mut q = ar_collection::Entity::find()
            .filter(ar_collection::Column::Status.eq(crate::models::status::ar::COLLECTION_CONFIRMED));

        if let Some(cid) = query.get("customer_id").and_then(|v| v.as_i64()) {
            q = q.filter(ar_collection::Column::CustomerId.eq(cid as i32));
        }

        let payments = q
            .order_by(ar_collection::Column::CollectionDate, Order::Asc)
            .all(&*self.db)
            .await?;

        // 批量查询已有核销记录，过滤已完全核销的收款
        let payment_ids: Vec<i32> = payments.iter().map(|p| p.id).collect();
        let verified_items = if payment_ids.is_empty() {
            Vec::new()
        } else {
            ar_reconciliation_item::Entity::find()
                .filter(ar_reconciliation_item::Column::ItemType.eq("RECEIPT"))
                .filter(ar_reconciliation_item::Column::DocumentId.is_in(payment_ids))
                .all(&*self.db)
                .await?
        };
        let mut verified_map: std::collections::HashMap<i32, Decimal> =
            std::collections::HashMap::new();
        for item in &verified_items {
            if let Some(doc_id) = item.document_id {
                *verified_map.entry(doc_id).or_insert(Decimal::ZERO) += item.amount.abs();
            }
        }

        let result: Vec<serde_json::Value> = payments
            .into_iter()
            .filter(|p| {
                let verified = verified_map.get(&p.id).copied().unwrap_or(Decimal::ZERO);
                verified < p.collection_amount
            })
            .map(collection_to_json)
            .collect();

        Ok(json!(result))
    }
}
