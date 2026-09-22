//! 采购入库服务内部辅助方法（私有：订单数量更新 + 库存事务更新）
//!
//! 拆分自 purchase_receipt_service.rs：原 2 个私有 fn 独立成文件，
//! 与公开方法分离便于测试和维护。

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::models::status::purchase_inventory::inventory_stock_grade;
use crate::models::{purchase_receipt, purchase_receipt_item};
use crate::services::event_bus::BusinessEvent;
use crate::utils::error::AppError;

use super::purchase_receipt_service::PurchaseReceiptService;

/// 库存维度键：产品 + 批次 + 色号 + 缸号 + 等级，与库存行落库字段同口径
type StockDimKey = (i32, String, String, String, String);

/// 单行入库前后的库存快照 (入库前, 入库后)
type StockSnapshots = (
    crate::models::inventory_stock::Model,
    crate::models::inventory_stock::Model,
);

impl PurchaseReceiptService {
    /// 获取入库单明细并批量构建订单明细映射（避免 N+1 查询）
    async fn fetch_receipt_items_with_order_map(
        txn: &sea_orm::DatabaseTransaction,
        receipt_id: i32,
    ) -> Result<
        (
            Vec<purchase_receipt_item::Model>,
            std::collections::HashMap<i32, crate::models::purchase_order_item::Model>,
        ),
        AppError,
    > {
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(txn)
            .await?;
        let order_item_ids: Vec<i32> = items.iter().filter_map(|i| i.order_item_id).collect();
        let order_item_map = if order_item_ids.is_empty() {
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
        Ok((items, order_item_map))
    }

    /// 按订单明细行汇总后更新已入库数量（含审计日志）
    ///
    /// 必须先聚合再写：一个订单明细行通常对应多条入库明细（面料按缸号/批次/匹号分行入库），
    /// 原实现逐条 `map.remove(order_item_id)`，同一订单行的第二条入库明细就查不到映射，
    /// 确认入库直接报「订单明细不存在」（CI 1-4 的 NOT_FOUND 之因），
    /// 而且同一条订单行被写两次也会互相覆盖。
    async fn update_order_items_received_quantity(
        txn: &sea_orm::DatabaseTransaction,
        items: Vec<purchase_receipt_item::Model>,
        order_item_map: std::collections::HashMap<i32, crate::models::purchase_order_item::Model>,
        user_id: i32,
    ) -> Result<(), AppError> {
        // BTreeMap 按订单明细 ID 升序，写入顺序稳定便于审计比对
        let mut sums: std::collections::BTreeMap<i32, (Decimal, Decimal)> =
            std::collections::BTreeMap::new();
        for item in &items {
            let Some(order_item_id) = item.order_item_id else {
                continue;
            };
            if !order_item_map.contains_key(&order_item_id) {
                return Err(AppError::not_found(format!(
                    "入库单第 {} 行关联的采购订单明细 {} 不存在（订单明细可能已被删除）",
                    item.line_no, order_item_id
                )));
            }
            let entry = sums
                .entry(order_item_id)
                .or_insert((Decimal::ZERO, Decimal::ZERO));
            entry.0 += item.quantity;
            entry.1 += item.quantity_alt.unwrap_or(Decimal::ZERO);
        }
        let mut order_item_map = order_item_map;
        for (order_item_id, (quantity, quantity_alt)) in sums {
            let order_item = order_item_map
                .remove(&order_item_id)
                .ok_or_else(|| AppError::not_found(format!("订单明细 {}", order_item_id)))?;
            let new_received = order_item.received_quantity + quantity;
            let new_received_alt = order_item.received_quantity_alt + quantity_alt;
            let mut active_order_item: crate::models::purchase_order_item::ActiveModel =
                order_item.into();
            active_order_item.received_quantity = sea_orm::ActiveValue::Set(new_received);
            active_order_item.received_quantity_alt = sea_orm::ActiveValue::Set(new_received_alt);
            active_order_item.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                txn,
                "auto_audit",
                active_order_item,
                Some(user_id),
            )
            .await?;
        }
        Ok(())
    }

    /// 根据订单明细已入库数量判定新状态（None 表示无需更新）
    async fn determine_order_receipt_status(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<Option<&'static str>, AppError> {
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
        let new_status = if is_fully_received {
            "COMPLETED"
        } else if has_received {
            "PARTIAL_RECEIVED"
        } else {
            return Ok(None);
        };
        Ok(Some(new_status))
    }

    /// 更新采购订单状态并写审计日志
    async fn save_order_status_update(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
        new_status: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
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
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 更新采购订单的已入库数量与状态（事务内调用）
    pub async fn update_order_received_quantity(
        &self,
        order_id: i32,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        let (items, order_item_map) =
            Self::fetch_receipt_items_with_order_map(txn, receipt_id).await?;
        Self::update_order_items_received_quantity(txn, items, order_item_map, user_id).await?;
        if let Some(new_status) = Self::determine_order_receipt_status(txn, order_id).await? {
            Self::save_order_status_update(txn, order_id, new_status, user_id).await?;
        }
        Ok(())
    }

    /// 入库明细行的批次维度必须真实存在：确认入库前逐行校验，缺批次即整单拒绝
    /// （不落库存、不改进度），不允许把缺失批次落成库存行的 `DEFAULT ''` 空串。
    /// 返回去除首尾空白的批次号。
    fn require_receipt_batch(
        item: &purchase_receipt_item::Model,
        receipt: &purchase_receipt::Model,
    ) -> Result<String, AppError> {
        let batch = item.batch_no.as_deref().map(str::trim).unwrap_or("");
        if batch.is_empty() {
            return Err(AppError::business(format!(
                "入库单 {} 第 {} 行（产品 {}）缺少批次号，四维不全，拒绝确认入库",
                receipt.receipt_no, item.line_no, item.product_id
            )));
        }
        Ok(batch.to_string())
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

        // 整单 fail-closed：任一行批次缺失即在建库前拒绝，事务不落任何库存行。
        // 色号/缸号的「染色布必填」口径待白坯布共享判定落地后在此追加（见
        // `upsert_stock_for_item` 内 TODO），本域不自行按色号名称判定白色。
        for item in &items {
            Self::require_receipt_batch(item, receipt)?;
        }

        let mut stock_map = Self::fetch_stock_map(txn, &items, receipt.warehouse_id).await?;

        for item in items {
            let key = Self::receipt_item_stock_key(&item);
            let existing = stock_map.get(&key).cloned();
            // 流水的期初/期末分别取本行入库前后，同产品不同缸号的行不共用库存行
            let (stock_before, stock_after) =
                Self::upsert_stock_for_item(txn, &item, existing.as_ref(), receipt).await?;
            stock_map.insert(key, stock_after);
            if let Some(ev) =
                Self::record_receipt_transaction(txn, &item, &stock_before, receipt).await?
            {
                pending_events.push(ev);
            }
        }
        Ok(pending_events)
    }

    /// 入库明细行的库存维度键：与创建库存行时写入的字段口径一致
    /// （batch_no/color_no 空值落库为空串，grade 缺省为一等品，dye_lot_no 保持 Option）
    fn receipt_item_stock_key(item: &purchase_receipt_item::Model) -> StockDimKey {
        (
            item.product_id,
            // 与 upsert_stock_for_item 落库口径一致：批次以去除首尾空白后的值定位库存行
            item.batch_no
                .as_deref()
                .map(str::trim)
                .map(str::to_string)
                .unwrap_or_default(),
            item.color_code.clone().unwrap_or_default(),
            item.lot_no.clone().unwrap_or_default(),
            item.grade
                .clone()
                .unwrap_or_else(|| inventory_stock_grade::FIRST.to_string()),
        )
    }

    /// 库存行的库存维度键，需与 `receipt_item_stock_key` 同口径
    fn stock_row_key(stock: &crate::models::inventory_stock::Model) -> StockDimKey {
        (
            stock.product_id,
            stock.batch_no.clone(),
            stock.color_no.clone(),
            stock.dye_lot_no.clone().unwrap_or_default(),
            stock.grade.clone(),
        )
    }

    /// 批量查询入库明细涉及的库存行，按库存维度（产品+批次+色号+缸号+等级）建索引
    ///
    /// 只按产品索引会把不同缸号的行错误合并到同一条库存上（同批货重复累加、
    /// 另一缸号的行被覆盖），这里按落库维度建键，保证「四维库存」逐行对得上。
    async fn fetch_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        items: &[purchase_receipt_item::Model],
        warehouse_id: i32,
    ) -> Result<
        std::collections::HashMap<StockDimKey, crate::models::inventory_stock::Model>,
        AppError,
    > {
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
            .map(|s| {
                let key = Self::stock_row_key(&s);
                (key, s)
            })
            .collect();
        Ok(map)
    }

    /// 更新或创建库存记录，返回 (本行入库前快照, 入库后库存行)
    ///
    /// 入库后行带数据库回写的最新数量与版本号，供同一维度的后续明细行继续累加，
    /// 否则第二行起会拿陈旧版本做乐观锁校验触发「并发冲突」并按陈旧基数覆盖数量。
    async fn upsert_stock_for_item(
        txn: &sea_orm::DatabaseTransaction,
        item: &purchase_receipt_item::Model,
        existing_stock: Option<&crate::models::inventory_stock::Model>,
        receipt: &purchase_receipt::Model,
    ) -> Result<StockSnapshots, AppError> {
        use crate::services::inventory_stock_service::{
            CreateStockFabricArgs, InventoryStockService,
        };
        if let Some(stock) = existing_stock {
            let new_meters = stock.quantity_meters + item.quantity;
            let new_kg = stock.quantity_kg + item.quantity_alt.unwrap_or(Decimal::ZERO);
            let after = InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                txn,
                stock.id,
                new_meters,
                new_kg,
                stock.version,
            )
            .await?;
            Ok((stock.clone(), after))
        } else {
            // 批次在建库前已逐行校验非空（update_inventory_txn），如实落库不再 unwrap 兜底成空串；
            // 色号：白坯布合法为空（落 ''），染色布是否必填待白坯布共享判定落地后强制（见下 TODO）。
            let batch_no = Self::require_receipt_batch(item, receipt)?;
            // TODO(共享白坯布判定)：色号非空⇒染色布⇒缸号(lot_no)/批次必填的口径应改调
            //   采购/库存统一的白坯布判定函数（doto iter31 第 3/5 条，本域外同事落地，尚未存在）。
            //   落地前保持色号/缸号原样落库，不在此按色号名称嗅探白色。
            let color_no = item.color_code.clone().unwrap_or_default();
            let grade = item
                .grade
                .clone()
                .unwrap_or_else(|| inventory_stock_grade::FIRST.to_string());
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
            let mut before = stock.clone();
            before.quantity_meters = Decimal::ZERO;
            before.quantity_kg = Decimal::ZERO;
            Ok((before, stock))
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
        // 流水维度与库存行同口径：批次如实写入（建库前已校验非空），不再 unwrap 成空串
        let batch_no = Self::require_receipt_batch(item, receipt)?;
        let color_no = item.color_code.clone().unwrap_or_default();
        let grade = item
            .grade
            .clone()
            .unwrap_or_else(|| inventory_stock_grade::FIRST.to_string());
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
