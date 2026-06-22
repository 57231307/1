//! 采购入库服务内部辅助方法（私有：订单数量更新 + 库存事务更新）
//!
//! 拆分自 purchase_receipt_service.rs：原 2 个私有 fn 独立成文件，
//! 与公开方法分离便于测试和维护。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::Set;
use std::sync::Arc;

use crate::models::{purchase_receipt, purchase_receipt_item};
use crate::utils::error::AppError;

use super::purchase_receipt_service::PurchaseReceiptService;

impl PurchaseReceiptService {
    async fn update_order_received_quantity(
        &self,
        order_id: i32,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 1. 获取入库单明细
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(txn)
            .await?;

        // 2. 更新每个订单明细的已入库数量
        for item in items {
            if let Some(order_item_id) = item.order_item_id {
                let order_item =
                    crate::models::purchase_order_item::Entity::find_by_id(order_item_id)
                        .one(txn)
                        .await?
                        .ok_or_else(|| {
                            AppError::not_found(format!("订单明细 {}", order_item_id))
                        })?;

                // 累加已入库数量
                let new_received = order_item.received_quantity + item.quantity;
                let new_received_alt =
                    order_item.received_quantity_alt + item.quantity_alt.unwrap_or_default();

                let mut active_order_item: crate::models::purchase_order_item::ActiveModel =
                    order_item.into();
                active_order_item.received_quantity = sea_orm::ActiveValue::Set(new_received);
                active_order_item.received_quantity_alt =
                    sea_orm::ActiveValue::Set(new_received_alt);
                active_order_item.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    active_order_item,
                    Some(0),
                )
                .await?;
            }
        }

        // 3. 更新采购订单状态
        let all_order_items = crate::models::purchase_order_item::Entity::find()
            .filter(crate::models::purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;

        let mut is_fully_received = true;
        let mut has_received = false;

        for oi in &all_order_items {
            if oi.received_quantity > Decimal::ZERO {
                has_received = true;
            }
            if oi.received_quantity < oi.quantity {
                is_fully_received = false;
            }
        }

        // 根据入库情况设置状态
        let new_status = if is_fully_received {
            "COMPLETED"
        } else if has_received {
            "PARTIAL_RECEIVED"
        } else {
            // 没有入库数量，保持原状态
            return Ok(());
        };

        let order = crate::models::purchase_order::Entity::find_by_id(order_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        let mut active_order: crate::models::purchase_order::ActiveModel = order.into();
        active_order.order_status = Set(new_status.to_string());
        active_order.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_order,
            Some(0),
        )
        .await?;

        Ok(())
    }

    async fn update_inventory_txn(
        &self,
        receipt: &purchase_receipt::Model,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        use crate::services::inventory_stock_service::InventoryStockService;

        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
            .all(txn)
            .await?;

        for item in items {
            let batch_no = item.batch_no.unwrap_or_else(|| "DEFAULT".to_string());
            let color_no = item.color_code.unwrap_or_else(|| "DEFAULT".to_string());
            let grade = item.grade.unwrap_or_else(|| "一等品".to_string());

            let existing_stock = InventoryStockService::find_by_product_and_warehouse_txn(
                txn,
                item.product_id,
                receipt.warehouse_id,
            )
            .await?;

            let stock_model = if let Some(stock) = existing_stock {
                let new_quantity_meters = stock.quantity_meters + item.quantity;
                let new_quantity_kg =
                    stock.quantity_kg + item.quantity_alt.unwrap_or(Decimal::new(0, 0));

                InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                    txn,
                    stock.id,
                    new_quantity_meters,
                    new_quantity_kg,
                    stock.version,
                )
                .await?;

                stock
            } else {
                InventoryStockService::create_stock_fabric_txn(
                    txn,
                    receipt.warehouse_id,
                    item.product_id,
                    batch_no.clone(),
                    color_no.clone(),
                    item.lot_no.clone(),
                    grade.clone(),
                    item.quantity,
                    item.quantity_alt.unwrap_or(Decimal::new(0, 0)),
                    item.gram_weight,
                    item.width,
                    None,
                    None,
                    None,
                )
                .await?
            };

            InventoryStockService::record_transaction_txn(
                txn,
                "PURCHASE_RECEIPT".to_string(),
                item.product_id,
                receipt.warehouse_id,
                batch_no,
                color_no,
                item.lot_no,
                grade,
                item.quantity,
                item.quantity_alt.unwrap_or(Decimal::new(0, 0)),
                Some("PURCHASE_RECEIPT".to_string()),
                Some(receipt.receipt_no.clone()),
                Some(receipt.id),
                Some(stock_model.quantity_meters),
                Some(stock_model.quantity_kg),
                Some(stock_model.quantity_meters + item.quantity),
                Some(stock_model.quantity_kg + item.quantity_alt.unwrap_or(Decimal::new(0, 0))),
                Some("入库自动增加库存".to_string()),
                Some(receipt.created_by),
            )
            .await?;
        }
        Ok(())
    }
}
