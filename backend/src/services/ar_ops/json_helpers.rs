//! Model → JSON 序列化辅助函数（ar_ops/json_helpers）
//!
//! 批次 488 D10-1 拆分：从原 `ar_service.rs` L2180-2270 迁移。
//! 4 个自由函数：collection_to_json / invoice_to_json / reconciliation_to_json / reconciliation_item_to_json

use serde_json::json;

use crate::models::{
    ar_collection, ar_invoice, ar_reconciliation, ar_reconciliation_item,
};

/// 收款单 → JSON
pub(super) fn collection_to_json(c: ar_collection::Model) -> serde_json::Value {
    json!({
        "id": c.id,
        "payment_no": c.collection_no,
        "collection_no": c.collection_no,
        "payment_date": c.collection_date.to_string(),
        "collection_date": c.collection_date.to_string(),
        "customer_id": c.customer_id,
        "customer_name": c.customer_name,
        "amount": c.collection_amount.to_string(),
        "collection_amount": c.collection_amount.to_string(),
        "payment_method": c.collection_method,
        "collection_method": c.collection_method,
        "bank_account": c.bank_account,
        "status": c.status,
        "confirmed_by": c.confirmed_by,
        "confirmed_at": c.confirmed_at,
        "created_by": c.created_by,
        "created_at": c.created_at,
        "updated_at": c.updated_at,
    })
}

/// 发票 → JSON
pub(super) fn invoice_to_json(i: ar_invoice::Model) -> serde_json::Value {
    json!({
        "id": i.id,
        "invoice_no": i.invoice_no,
        "invoice_date": i.invoice_date.to_string(),
        "due_date": i.due_date.to_string(),
        "customer_id": i.customer_id,
        "customer_name": i.customer_name,
        "customer_code": i.customer_code,
        "invoice_amount": i.invoice_amount.to_string(),
        "received_amount": i.received_amount.to_string(),
        "unpaid_amount": i.unpaid_amount.to_string(),
        "status": i.status,
        "approval_status": i.approval_status,
        "batch_no": i.batch_no,
        "color_no": i.color_no,
        "sales_order_no": i.sales_order_no,
    })
}

/// 对账单 → JSON
pub(super) fn reconciliation_to_json(r: ar_reconciliation::Model) -> serde_json::Value {
    json!({
        "id": r.id,
        "verification_no": r.reconciliation_no,
        "reconciliation_no": r.reconciliation_no,
        "verification_date": r.reconciliation_date.to_string(),
        "reconciliation_date": r.reconciliation_date.to_string(),
        "period_start": r.period_start.to_string(),
        "period_end": r.period_end.to_string(),
        "customer_id": r.customer_id,
        "customer_name": r.customer_name,
        "opening_balance": r.opening_balance.to_string(),
        "total_invoices": r.total_invoices.to_string(),
        "total_collections": r.total_collections.to_string(),
        "closing_balance": r.closing_balance.to_string(),
        "status": r.reconciliation_status,
        "reconciliation_status": r.reconciliation_status,
        "confirmed_by": r.confirmed_by,
        "confirmed_at": r.confirmed_at,
        "created_by": r.created_by,
        "created_at": r.created_at,
    })
}

/// 对账明细 → JSON
pub(super) fn reconciliation_item_to_json(i: ar_reconciliation_item::Model) -> serde_json::Value {
    json!({
        "id": i.id,
        "reconciliation_id": i.reconciliation_id,
        "item_type": i.item_type,
        "document_type": i.document_type,
        "document_id": i.document_id,
        "document_no": i.document_no,
        "document_date": i.document_date.map(|d| d.to_string()),
        "amount": i.amount.to_string(),
        "matched_amount": i.matched_amount.map(|a| a.to_string()),
        "match_status": i.match_status,
        "remarks": i.remarks,
        "created_at": i.created_at,
    })
}
