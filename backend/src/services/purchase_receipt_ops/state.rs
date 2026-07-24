//! 采购入库-状态流转子模块（purchase_receipt_ops/state）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 1 个状态流转方法 + 2 个 helper：
//! - `confirm_receipt`：确认入库单（DRAFT → CONFIRMED），触发库存入库 + 事件发布 + 自动生成应付账单
//! - `lock_and_validate_receipt_txn`：锁定入库单并校验状态（私有 helper）
//! - `publish_events_and_generate_ap`：commit 后发布事件并自动生成应付账单（私有 helper）
//!
//! 跨模块调用：
//! - `confirm_receipt` 调用 `purchase_receipt_private` 中的 update_order_received_quantity / update_inventory_txn（已 `pub`，跨 impl 块可访问）
//! - `confirm_receipt` 调用 facade 的纯函数 `build_confirmed_receipt_active_model`（`pub(crate)`）

use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, TransactionTrait};

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::event_bus::EVENT_BUS;
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// 确认采购入库单
    ///
    /// 批次 16（2026-06-28）：入库单状态门查询加 lock_exclusive，
    /// 防止并发 confirm_receipt 同一入库单导致重复入库 + 重复生成应付账单 + 重复累加采购单已收数量。
    /// 原状态门无锁，两并发 confirm 均通过 DRAFT 检查，第二个 confirm 重复执行库存入库与
    /// order_item received_quantity 累加，commit 后还会重复触发 auto_generate_from_receipt 生成应付账单。
    pub async fn confirm_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 锁定并校验入库单（DRAFT + 明细数 > 0），串行化并发 confirm
        let receipt = self.lock_and_validate_receipt_txn(receipt_id, &txn).await?;

        // 关联采购单时更新已收数量
        if let Some(order_id) = receipt.order_id {
            self.update_order_received_quantity(order_id, receipt_id, &txn, user_id)
                .await?;
        }

        // 更新状态为 CONFIRMED 并写入审计
        let receipt_active = Self::build_confirmed_receipt_active_model(receipt, user_id);
        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            Some(user_id),
        )
        .await?;

        // 事务内更新库存，收集待发布事件
        let pending_events = self.update_inventory_txn(&receipt, &txn).await?;

        txn.commit().await?;

        // commit 后发布事件并自动生成应付账单
        self.publish_events_and_generate_ap(&receipt, pending_events, user_id)
            .await;

        Ok(receipt)
    }

    /// 锁定入库单并校验状态为 DRAFT 且明细数 > 0
    async fn lock_and_validate_receipt_txn(
        &self,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_receipt::Model, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许确认，当前状态：{}",
                receipt.receipt_status
            )));
        }

        let item_count = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .count(txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("入库单至少需要一行明细".to_string()));
        }

        Ok(receipt)
    }

    /// commit 后发布库存事件并自动生成应付账单（失败仅告警不阻塞）
    async fn publish_events_and_generate_ap(
        &self,
        receipt: &purchase_receipt::Model,
        pending_events: Vec<crate::services::event_bus::BusinessEvent>,
        user_id: i32,
    ) {
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

        let ap_service =
            crate::services::ap_invoice_service::ApInvoiceService::new(self.db.clone());
        if let Err(e) = ap_service
            .auto_generate_from_receipt(receipt.id, user_id)
            .await
        {
            tracing::warn!(
                "⚠ 入库单 {} 已确认成功，但自动生成应付账单失败，需人工补生成应付单：{}",
                receipt.receipt_no,
                e
            );
        } else {
            tracing::info!("成功自动生成应付账单 (入库单 {})", receipt.receipt_no);
        }
    }
}
