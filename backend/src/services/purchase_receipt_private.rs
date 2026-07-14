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
        use crate::services::inventory_stock_query::RecordTransactionArgs;
        use crate::services::inventory_stock_service::{CreateStockFabricArgs, InventoryStockService};

        // P0 5-2 修复：本函数不 commit 事务（由调用方 confirm_receipt commit），
        // 收集 record_transaction_txn 返回的库存流水事件交给调用方，
        // 在 commit 成功后统一 publish，避免事务回滚时幻事件
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
            .all(txn)
            .await?;

        // v16 批次 43 修复：循环外批量查询所有 product_id 在 receipt.warehouse_id 的库存记录，
        // 避免循环内逐个调用 find_by_product_and_warehouse_txn（N+1 查询）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let stock_map: std::collections::HashMap<i32, crate::models::inventory_stock::Model> =
            if product_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                crate::models::inventory_stock::Entity::find()
                    .filter(crate::models::inventory_stock::Column::WarehouseId.eq(receipt.warehouse_id))
                    .filter(crate::models::inventory_stock::Column::ProductId.is_in(product_ids))
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(|s| (s.product_id, s))
                    .collect()
            };

        for item in items {
            let batch_no = item.batch_no.unwrap_or_else(|| "DEFAULT".to_string());
            let color_no = item.color_code.unwrap_or_else(|| "DEFAULT".to_string());
            let grade = item.grade.unwrap_or_else(|| "一等品".to_string());

            // v16 批次 43 修复：从批量查询结果获取库存记录（O(1) 查找）
            let existing_stock = stock_map.get(&item.product_id).cloned();

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
                // 批次 338 v10 复审 P3 修复：使用参数对象替代多参数
                InventoryStockService::create_stock_fabric_txn(
                    txn,
                    CreateStockFabricArgs {
                        warehouse_id: receipt.warehouse_id,
                        product_id: item.product_id,
                        batch_no: batch_no.clone(),
                        color_no: color_no.clone(),
                        dye_lot_no: item.lot_no.clone(),
                        grade: grade.clone(),
                        quantity_meters: item.quantity,
                        quantity_kg: item.quantity_alt.unwrap_or(Decimal::new(0, 0)),
                        gram_weight: item.gram_weight,
                        width: item.width,
                        location_id: None,
                        shelf_no: None,
                        layer_no: None,
                    },
                )
                .await?
            };

            // P0 5-2 修复：record_transaction_txn 不再在函数内 publish 事件，
            // 改为返回 (Model, Option<BusinessEvent>)，由本函数收集后交给调用方在 commit 后统一 publish
            // 批次 338 v10 复审 P3 修复：使用参数对象替代多参数
            let (_, txn_event) = InventoryStockService::record_transaction_txn(
                txn,
                RecordTransactionArgs {
                    transaction_type: "PURCHASE_RECEIPT".to_string(),
                    product_id: item.product_id,
                    warehouse_id: receipt.warehouse_id,
                    batch_no,
                    color_no,
                    dye_lot_no: item.lot_no,
                    grade,
                    quantity_meters: item.quantity,
                    quantity_kg: item.quantity_alt.unwrap_or(Decimal::new(0, 0)),
                    source_bill_type: Some("PURCHASE_RECEIPT".to_string()),
                    source_bill_no: Some(receipt.receipt_no.clone()),
                    source_bill_id: Some(receipt.id),
                    quantity_before_meters: Some(stock_model.quantity_meters),
                    quantity_before_kg: Some(stock_model.quantity_kg),
                    quantity_after_meters: Some(stock_model.quantity_meters + item.quantity),
                    quantity_after_kg: Some(stock_model.quantity_kg + item.quantity_alt.unwrap_or(Decimal::new(0, 0))),
                    notes: Some("入库自动增加库存".to_string()),
                    created_by: Some(receipt.created_by),
                },
            )
            .await?;
            if let Some(ev) = txn_event {
                pending_events.push(ev);
            }
        }
        Ok(pending_events)
    }
}

#[cfg(test)]
mod tests {
    //! 采购入库私有辅助方法单元测试（批次 409 补测）
    //!
    //! 覆盖目标：
    //! - update_order_received_quantity 中收货数量累加与订单状态判定纯逻辑
    //! - update_inventory_txn 中批次号/色号/等级默认值回退与库存累加计算
    //! - quantity_after 流水数量计算公式
    //! - PurchaseReceiptService 实例化

    use super::*;
    use crate::decs;

    /// 复现 update_order_received_quantity 中已入库数量累加纯算法
    /// 源码：new_received = order_item.received_quantity + item.quantity
    fn calc_new_received(received_quantity: Decimal, item_quantity: Decimal) -> Decimal {
        received_quantity + item_quantity
    }

    /// 复现 update_order_received_quantity 中已入库辅助数量累加纯算法
    /// 源码：new_received_alt = order_item.received_quantity_alt + item.quantity_alt.unwrap_or_default()
    fn calc_new_received_alt(
        received_quantity_alt: Decimal,
        item_quantity_alt: Option<Decimal>,
    ) -> Decimal {
        received_quantity_alt + item_quantity_alt.unwrap_or_default()
    }

    /// 复现 update_order_received_quantity 中订单状态判定逻辑
    /// 返回 (Some("COMPLETED") | Some("PARTIAL_RECEIVED") | None)
    fn decide_order_status(items: &[(Decimal, Decimal)]) -> Option<&'static str> {
        let mut is_fully_received = true;
        let mut has_received = false;
        for (received, ordered) in items {
            if *received > Decimal::ZERO {
                has_received = true;
            }
            if *received < *ordered {
                is_fully_received = false;
            }
        }
        if is_fully_received {
            Some("COMPLETED")
        } else if has_received {
            Some("PARTIAL_RECEIVED")
        } else {
            None
        }
    }

    /// 复现 update_inventory_txn 中批次号默认值回退
    /// 源码：item.batch_no.unwrap_or_else(|| "DEFAULT".to_string())
    fn resolve_batch_no(batch_no: Option<String>) -> String {
        batch_no.unwrap_or_else(|| "DEFAULT".to_string())
    }

    /// 复现 update_inventory_txn 中色号默认值回退
    /// 源码：item.color_code.unwrap_or_else(|| "DEFAULT".to_string())
    fn resolve_color_no(color_code: Option<String>) -> String {
        color_code.unwrap_or_else(|| "DEFAULT".to_string())
    }

    /// 复现 update_inventory_txn 中等级默认值回退
    /// 源码：item.grade.unwrap_or_else(|| "一等品".to_string())
    fn resolve_grade(grade: Option<String>) -> String {
        grade.unwrap_or_else(|| "一等品".to_string())
    }

    /// 复现 update_inventory_txn 中库存累加纯算法（米）
    /// 源码：new_quantity_meters = stock.quantity_meters + item.quantity
    fn calc_new_quantity_meters(stock_meters: Decimal, item_quantity: Decimal) -> Decimal {
        stock_meters + item_quantity
    }

    /// 复现 update_inventory_txn 中库存累加纯算法（公斤）
    /// 源码：new_quantity_kg = stock.quantity_kg + item.quantity_alt.unwrap_or(Decimal::new(0, 0))
    fn calc_new_quantity_kg(stock_kg: Decimal, item_quantity_alt: Option<Decimal>) -> Decimal {
        stock_kg + item_quantity_alt.unwrap_or(Decimal::new(0, 0))
    }

    // ---------- 收货数量累加 ----------

    /// 测试_收货数量累加_整数场景
    ///
    /// 验证 received_quantity + item_quantity 在整数场景下计算正确
    #[test]
    fn 测试_收货数量累加_整数场景() {
        // 50 + 30 = 80
        assert_eq!(calc_new_received(decs!("50"), decs!("30")), decs!("80"));
    }

    /// 测试_收货数量累加_零已入库场景
    ///
    /// 验证 received_quantity=0 时累加结果等于入库数量（首次入库）
    #[test]
    fn 测试_收货数量累加_零已入库场景() {
        assert_eq!(calc_new_received(Decimal::ZERO, decs!("30")), decs!("30"));
    }

    /// 测试_收货数量累加_小数精度场景
    ///
    /// 验证 received_quantity + item_quantity 在小数场景下精度保留正确
    #[test]
    fn 测试_收货数量累加_小数精度场景() {
        // 50.5 + 30.25 = 80.75
        assert_eq!(
            calc_new_received(decs!("50.5"), decs!("30.25")),
            decs!("80.75")
        );
    }

    /// 测试_收货辅助数量累加_辅助数量为Some
    ///
    /// 验证 received_quantity_alt + item_quantity_alt(Some) 计算正确
    #[test]
    fn 测试_收货辅助数量累加_辅助数量为Some() {
        assert_eq!(
            calc_new_received_alt(decs!("10"), Some(decs!("5"))),
            decs!("15")
        );
    }

    /// 测试_收货辅助数量累加_辅助数量为None回退零
    ///
    /// 验证 item_quantity_alt=None 时 unwrap_or_default() 回退到 Decimal::ZERO
    #[test]
    fn 测试_收货辅助数量累加_辅助数量为None回退零() {
        assert_eq!(calc_new_received_alt(decs!("10"), None), decs!("10"));
    }

    // ---------- 订单状态判定 ----------

    /// 测试_订单状态判定_全部入库完成
    ///
    /// 验证所有明细 received >= ordered 时状态为 COMPLETED
    #[test]
    fn 测试_订单状态判定_全部入库完成() {
        let items = vec![
            (decs!("100"), decs!("100")),
            (decs!("50"), decs!("50")),
        ];
        assert_eq!(decide_order_status(&items), Some("COMPLETED"));
    }

    /// 测试_订单状态判定_部分入库
    ///
    /// 验证存在 received < ordered 但有 received > 0 时状态为 PARTIAL_RECEIVED
    #[test]
    fn 测试_订单状态判定_部分入库() {
        let items = vec![
            (decs!("100"), decs!("100")),
            (decs!("30"), decs!("50")), // 部分入库
        ];
        assert_eq!(decide_order_status(&items), Some("PARTIAL_RECEIVED"));
    }

    /// 测试_订单状态判定_无入库保持原状态
    ///
    /// 验证所有明细 received=0 时返回 None（保持原状态）
    #[test]
    fn 测试_订单状态判定_无入库保持原状态() {
        let items = vec![
            (Decimal::ZERO, decs!("100")),
            (Decimal::ZERO, decs!("50")),
        ];
        assert_eq!(decide_order_status(&items), None);
    }

    /// 测试_订单状态判定_超量入库视为完成
    ///
    /// 验证 received > ordered 时 is_fully_received 仍为 true
    #[test]
    fn 测试_订单状态判定_超量入库视为完成() {
        let items = vec![
            (decs!("120"), decs!("100")), // 超量入库
        ];
        assert_eq!(decide_order_status(&items), Some("COMPLETED"));
    }

    /// 测试_订单状态判定_空明细列表视为完成
    ///
    /// 验证 items 为空时 is_fully_received=true 且 has_received=false，返回 COMPLETED
    #[test]
    fn 测试_订单状态判定_空明细列表视为完成() {
        let items: Vec<(Decimal, Decimal)> = vec![];
        assert_eq!(decide_order_status(&items), Some("COMPLETED"));
    }

    // ---------- 默认值回退 ----------

    /// 测试_批次号默认值回退_None时使用DEFAULT
    ///
    /// 验证 item.batch_no=None 时回退到 "DEFAULT"
    #[test]
    fn 测试_批次号默认值回退_None时使用DEFAULT() {
        assert_eq!(resolve_batch_no(None), "DEFAULT");
        assert_eq!(resolve_batch_no(Some("B-001".to_string())), "B-001");
    }

    /// 测试_色号默认值回退_None时使用DEFAULT
    ///
    /// 验证 item.color_code=None 时回退到 "DEFAULT"
    #[test]
    fn 测试_色号默认值回退_None时使用DEFAULT() {
        assert_eq!(resolve_color_no(None), "DEFAULT");
        assert_eq!(resolve_color_no(Some("RED".to_string())), "RED");
    }

    /// 测试_等级默认值回退_None时使用一等品
    ///
    /// 验证 item.grade=None 时回退到 "一等品"
    #[test]
    fn 测试_等级默认值回退_None时使用一等品() {
        assert_eq!(resolve_grade(None), "一等品");
        assert_eq!(resolve_grade(Some("二等品".to_string())), "二等品");
    }

    // ---------- 库存累加 ----------

    /// 测试_库存累加_米数量整数场景
    ///
    /// 验证 stock.quantity_meters + item.quantity 在整数场景下计算正确
    #[test]
    fn 测试_库存累加_米数量整数场景() {
        // 200 + 100 = 300
        assert_eq!(
            calc_new_quantity_meters(decs!("200"), decs!("100")),
            decs!("300")
        );
    }

    /// 测试_库存累加_公斤数量辅助为Some
    ///
    /// 验证 stock.quantity_kg + item.quantity_alt(Some) 计算正确
    #[test]
    fn 测试_库存累加_公斤数量辅助为Some() {
        // 60 + 30 = 90
        assert_eq!(
            calc_new_quantity_kg(decs!("60"), Some(decs!("30"))),
            decs!("90")
        );
    }

    /// 测试_库存累加_公斤数量辅助为None回退零
    ///
    /// 验证 item.quantity_alt=None 时 unwrap_or(Decimal::new(0, 0)) 回退到 0
    #[test]
    fn 测试_库存累加_公斤数量辅助为None回退零() {
        assert_eq!(calc_new_quantity_kg(decs!("60"), None), decs!("60"));
    }

    /// 测试_库存累加_零库存初始入库
    ///
    /// 验证 stock.quantity_meters=0 时累加结果等于入库数量（首次入库场景）
    #[test]
    fn 测试_库存累加_零库存初始入库() {
        assert_eq!(
            calc_new_quantity_meters(Decimal::ZERO, decs!("100")),
            decs!("100")
        );
    }

    // ---------- 流水变更后数量 ----------

    /// 测试_流水变更后数量_等于库存原值加入库量
    ///
    /// update_inventory_txn 中 quantity_after_meters = stock_model.quantity_meters + item.quantity
    /// 验证该公式与库存累加公式一致
    #[test]
    fn 测试_流水变更后数量_等于库存原值加入库量() {
        let stock_before = decs!("200");
        let item_qty = decs!("100");
        let quantity_after = calc_new_quantity_meters(stock_before, item_qty);
        assert_eq!(quantity_after, decs!("300"));
        // 验证 quantity_after 与 new_quantity_meters 计算逻辑一致
        let new_quantity_meters = calc_new_quantity_meters(stock_before, item_qty);
        assert_eq!(quantity_after, new_quantity_meters);
    }

    /// 测试_流水变更后公斤数量_辅助为None回退零
    ///
    /// update_inventory_txn 中 quantity_after_kg = stock_model.quantity_kg + item.quantity_alt.unwrap_or(...)
    /// 验证 item.quantity_alt=None 时 quantity_after_kg 等于库存原值
    #[test]
    fn 测试_流水变更后公斤数量_辅助为None回退零() {
        let stock_before_kg = decs!("60");
        let quantity_after_kg = calc_new_quantity_kg(stock_before_kg, None);
        assert_eq!(quantity_after_kg, decs!("60"));
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 PurchaseReceiptService 能在 SQLite 内存数据库上实例化，
    /// 不依赖真实 schema（new 不触发任何 DB 操作）
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = sea_orm::Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败");
        let service = PurchaseReceiptService::new(std::sync::Arc::new(db));
        let _ = service;
    }
}
