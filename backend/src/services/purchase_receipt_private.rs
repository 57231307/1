//! 采购入库服务内部辅助方法（私有：订单数量更新 + 库存事务更新）
//!
//! 拆分自 purchase_receipt_service.rs：原 2 个私有 fn 独立成文件，
//! 与公开方法分离便于测试和维护。

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::models::{purchase_receipt, purchase_receipt_item};
use crate::services::event_bus::BusinessEvent;
use crate::utils::error::AppError;

use super::purchase_receipt_service::PurchaseReceiptService;

impl PurchaseReceiptService {
    pub async fn update_order_received_quantity(
        &self,
        order_id: i32,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 1. 获取入库单明细
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(txn)
            .await?;

        // v11 批次 38 修复：批量查询本入库单关联的所有订单明细，避免循环内逐个 find_by_id（N+1 查询）
        let order_item_ids: Vec<i32> = items
            .iter()
            .filter_map(|i| i.order_item_id)
            .collect();
        let mut order_item_map: std::collections::HashMap<i32, crate::models::purchase_order_item::Model> =
            if order_item_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                crate::models::purchase_order_item::Entity::find()
                    .filter(crate::models::purchase_order_item::Column::Id.is_in(order_item_ids))
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(|oi| (oi.id, oi))
                    .collect()
            };

        // 2. 更新每个订单明细的已入库数量
        for item in items {
            if let Some(order_item_id) = item.order_item_id {
                let order_item = order_item_map
                    .remove(&order_item_id)
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
                // update_with_audit 需逐条执行以生成审计日志
                // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    txn,
                    "auto_audit",
                    active_order_item,
                    Some(user_id),
                )
                .await?;
            }
        }

        // 3. 更新采购订单状态（重新查询最新订单明细，因为上方 update 已修改 received_quantity）
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
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            active_order,
            Some(user_id),
        )
        .await?;

        Ok(())
    }

    pub async fn update_inventory_txn(
        &self,
        receipt: &purchase_receipt::Model,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<BusinessEvent>, AppError> {
        // 不 commit 事务（由调用方 commit），收集库存流水事件交调用方 publish
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
            .all(txn)
            .await?;

        let stock_map = Self::fetch_stock_map(txn, &items, receipt.warehouse_id).await?;

        for item in items {
            let existing = stock_map.get(&item.product_id);
            let stock_model = Self::upsert_stock_for_item(txn, &item, existing, receipt).await?;
            if let Some(ev) =
                Self::record_receipt_transaction(txn, &item, &stock_model, receipt).await?
            {
                pending_events.push(ev);
            }
        }
        Ok(pending_events)
    }

    /// 批量查询入库明细关联的库存记录（避免 N+1 查询）
    async fn fetch_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        items: &[purchase_receipt_item::Model],
        warehouse_id: i32,
    ) -> Result<std::collections::HashMap<i32, crate::models::inventory_stock::Model>, AppError>
    {
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        if product_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let map = crate::models::inventory_stock::Entity::find()
            .filter(crate::models::inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .filter(crate::models::inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?
            .into_iter()
            .map(|s| (s.product_id, s))
            .collect();
        Ok(map)
    }

    /// 更新或创建库存记录（存在则加库存，不存在则新建）
    async fn upsert_stock_for_item(
        txn: &sea_orm::DatabaseTransaction,
        item: &purchase_receipt_item::Model,
        existing_stock: Option<&crate::models::inventory_stock::Model>,
        receipt: &purchase_receipt::Model,
    ) -> Result<crate::models::inventory_stock::Model, AppError> {
        use crate::services::inventory_stock_service::{
            CreateStockFabricArgs, InventoryStockService,
        };
        if let Some(stock) = existing_stock {
            let new_meters = stock.quantity_meters + item.quantity;
            let new_kg = stock.quantity_kg + item.quantity_alt.unwrap_or(Decimal::ZERO);
            InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                txn,
                stock.id,
                new_meters,
                new_kg,
                stock.version,
            )
            .await?;
            Ok(stock.clone())
        } else {
            let batch_no = item.batch_no.clone().unwrap_or_default();
            let color_no = item.color_code.clone().unwrap_or_default();
            let grade = item.grade.clone().unwrap_or_else(|| "一等品".to_string());
            let stock = InventoryStockService::create_stock_fabric_txn(
                txn,
                CreateStockFabricArgs {
                    warehouse_id: receipt.warehouse_id,
                    product_id: item.product_id,
                    batch_no,
                    color_no,
                    dye_lot_no: item.lot_no.clone(),
                    grade,
                    quantity_meters: item.quantity,
                    quantity_kg: item.quantity_alt.unwrap_or(Decimal::ZERO),
                    gram_weight: item.gram_weight,
                    width: item.width,
                    location_id: None,
                    shelf_no: None,
                    layer_no: None,
                },
            )
            .await?;
            Ok(stock)
        }
    }

    /// 记录库存流水并返回事件（由调用方在 commit 后 publish）
    async fn record_receipt_transaction(
        txn: &sea_orm::DatabaseTransaction,
        item: &purchase_receipt_item::Model,
        stock_model: &crate::models::inventory_stock::Model,
        receipt: &purchase_receipt::Model,
    ) -> Result<Option<BusinessEvent>, AppError> {
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::InventoryStockService;
        let batch_no = item.batch_no.clone().unwrap_or_default();
        let color_no = item.color_code.clone().unwrap_or_default();
        let grade = item.grade.clone().unwrap_or_else(|| "一等品".to_string());
        let (_, txn_event) = InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PURCHASE_RECEIPT".to_string(),
                product_id: item.product_id,
                warehouse_id: receipt.warehouse_id,
                batch_no,
                color_no,
                dye_lot_no: item.lot_no.clone(),
                grade,
                quantity_meters: item.quantity,
                quantity_kg: item.quantity_alt.unwrap_or(Decimal::ZERO),
                source_bill_type: Some("PURCHASE_RECEIPT".to_string()),
                source_bill_no: Some(receipt.receipt_no.clone()),
                source_bill_id: Some(receipt.id),
                quantity_before_meters: Some(stock_model.quantity_meters),
                quantity_before_kg: Some(stock_model.quantity_kg),
                quantity_after_meters: Some(stock_model.quantity_meters + item.quantity),
                quantity_after_kg: Some(
                    stock_model.quantity_kg + item.quantity_alt.unwrap_or(Decimal::ZERO),
                ),
                notes: Some("入库自动增加库存".to_string()),
                created_by: Some(receipt.created_by),
            },
        )
        .await?;
        Ok(txn_event)
    }
}
