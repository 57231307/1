//! 采购收货服务（po/receipt）
//!
//! 包含采购订单的收货确认（含库存入库联动）、收货单号生成等。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{inventory_stock, product, purchase_order, purchase_order_item, status};
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::inventory_stock_query::RecordTransactionArgs;
use crate::services::inventory_stock_service::CreateStockFabricArgs;
use crate::services::po::CreateOrderItemRequest;
use crate::services::po::UpdateOrderItemRequest;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use super::order::PurchaseOrderService;

impl PurchaseOrderService {
    /// 标记采购订单为已收货（含库存入库联动）
    ///
    /// P0 3-6 修复（2026-07-01 八维度审计）：增加 receipt_id 参数做幂等校验。
    /// 原实现只接收 order_id，事件重投会导致重复入库。
    /// 现通过 receipt_id 查询入库单 receipt_status，若已 COMPLETED 则幂等返回。
    pub async fn receive_order(
        &self,
        order_id: i32,
        receipt_id: Option<i32>,
    ) -> Result<purchase_order::Model, AppError> {
        // 1. 开启事务保证数据一致性
        let txn = (*self.db).begin().await?;

        // P0 5-2 修复：收集 record_transaction_txn 返回的库存流水事件，
        // 在 commit 成功后统一 publish，避免事务回滚时幻事件
        let mut pending_events: Vec<BusinessEvent> = Vec::new();

        // P0 3-6 修复：幂等校验——若指定了 receipt_id 且入库单已 COMPLETED，直接返回当前订单
        // 防止事件重投或并发触发导致重复入库
        if let Some(rid) = receipt_id {
            use crate::models::purchase_receipt;
            let receipt = purchase_receipt::Entity::find_by_id(rid)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found(format!("入库单 {}", rid)))?;
            if receipt.receipt_status == status::purchase_receipt::COMPLETED {
                tracing::info!(
                    "入库单 {} 已 COMPLETED，跳过重复入库（幂等返回），订单 {}",
                    rid,
                    order_id
                );
                let order = purchase_order::Entity::find_by_id(order_id)
                    .one(&*self.db)
                    .await?
                    .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;
                txn.commit().await?;
                return Ok(order);
            }
        }

        let order = Self::lock_order_for_receive(&txn, order_id).await?;
        Self::validate_receive_status(&order)?;
        let order_items = Self::load_receive_order_items(&txn, order_id).await?;
        let product_map = Self::load_receive_product_map(&txn, &order_items).await?;
        let stock_map =
            Self::load_receive_stock_map(&txn, order.warehouse_id, &order_items).await?;
        for item in &order_items {
            if let Some(ev) =
                Self::process_receive_item(&txn, &order, item, &product_map, &stock_map).await?
            {
                pending_events.push(ev);
            }
        }
        let new_status = Self::determine_new_status(&txn, order_id).await?;
        let updated_order =
            Self::update_order_status_to_received(&txn, order, new_status).await?;
        Self::mark_receipt_completed(&txn, receipt_id).await?;
        txn.commit().await?;
        Self::publish_receive_events(pending_events);
        Ok(updated_order)
    }

    /// 加锁查询订单（串行化并发收货）
    async fn lock_order_for_receive(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))
    }

    /// 校验订单状态——只有已审批或部分收货的订单才能收货
    fn validate_receive_status(order: &purchase_order::Model) -> Result<(), AppError> {
        if order.order_status != status::purchase_order::APPROVED
            && order.order_status != status::purchase_order::PARTIAL_RECEIVED
        {
            return Err(AppError::business(format!(
                "订单状态不允许收货，当前状态：{}，需要状态：APPROVED 或 PARTIAL_RECEIVED",
                order.order_status
            )));
        }
        Ok(())
    }

    /// 加载订单明细
    async fn load_receive_order_items(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<Vec<purchase_order_item::Model>, AppError> {
        Ok(purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?)
    }

    /// 批量加载明细涉及的产品，避免循环内 N+1 查询
    async fn load_receive_product_map(
        txn: &sea_orm::DatabaseTransaction,
        order_items: &[purchase_order_item::Model],
    ) -> Result<std::collections::HashMap<i32, product::Model>, AppError> {
        let product_ids: Vec<i32> = order_items.iter().map(|item| item.product_id).collect();
        let products = if product_ids.is_empty() {
            Vec::new()
        } else {
            product::Entity::find()
                .filter(product::Column::Id.is_in(product_ids))
                .all(txn)
                .await?
        };
        Ok(products.into_iter().map(|p| (p.id, p)).collect())
    }

    /// 批量加载明细对应的库存记录（同一 warehouse_id），避免循环内 N+1 查询
    async fn load_receive_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        warehouse_id: i32,
        order_items: &[purchase_order_item::Model],
    ) -> Result<std::collections::HashMap<i32, inventory_stock::Model>, AppError> {
        let stock_product_ids: Vec<i32> =
            order_items.iter().map(|item| item.product_id).collect();
        let existing_stocks = if stock_product_ids.is_empty() {
            Vec::new()
        } else {
            inventory_stock::Entity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                .filter(inventory_stock::Column::ProductId.is_in(stock_product_ids))
                .all(txn)
                .await?
        };
        Ok(existing_stocks.into_iter().map(|s| (s.product_id, s)).collect())
    }

    /// 处理单个明细入库：更新/创建库存 + 记录流水 + 累加已收数量，返回流水事件
    async fn process_receive_item(
        txn: &sea_orm::DatabaseTransaction,
        order: &purchase_order::Model,
        item: &purchase_order_item::Model,
        product_map: &std::collections::HashMap<i32, product::Model>,
        stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
    ) -> Result<Option<BusinessEvent>, AppError> {
        let product = product_map
            .get(&item.product_id)
            .ok_or_else(|| AppError::not_found(format!("产品 ID {} 不存在", item.product_id)))?;
        let receive_qty_meters = item.quantity - item.received_quantity;
        let receive_qty_alt = item.quantity_alt - item.received_quantity_alt;
        if receive_qty_meters <= Decimal::ZERO {
            return Ok(None);
        }
        let existing_stock = stock_map.get(&item.product_id).cloned();
        let (before_meters, before_kg) = Self::upsert_stock_for_receive(
            txn, order, item, product, existing_stock, receive_qty_meters, receive_qty_alt,
        )
        .await?;
        let event = Self::record_receive_transaction(
            txn, order, item, receive_qty_meters, receive_qty_alt, before_meters, before_kg,
        )
        .await?;
        Self::update_received_quantity(txn, item, receive_qty_meters, receive_qty_alt).await?;
        Ok(event)
    }

    /// 更新现有库存或创建新库存记录，返回入库前数量
    async fn upsert_stock_for_receive(
        txn: &sea_orm::DatabaseTransaction,
        order: &purchase_order::Model,
        item: &purchase_order_item::Model,
        product: &product::Model,
        existing_stock: Option<inventory_stock::Model>,
        receive_qty_meters: Decimal,
        receive_qty_alt: Decimal,
    ) -> Result<(Decimal, Decimal), AppError> {
        match existing_stock {
            Some(stock) => {
                let new_meters = stock.quantity_meters + receive_qty_meters;
                let new_kg = stock.quantity_kg + receive_qty_alt;
                crate::services::inventory_stock_service::InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                    txn, stock.id, new_meters, new_kg, stock.version,
                )
                .await
                .map_err(|e| {
                    tracing::error!("更新库存失败: 库存ID={}, 错误: {}", stock.id, e);
                    AppError::internal(format!("更新库存失败: {}", e))
                })?;
                Ok((stock.quantity_meters, stock.quantity_kg))
            }
            None => {
                Self::create_stock_for_receive(
                    txn, order, item, product, receive_qty_meters, receive_qty_alt,
                )
                .await?;
                Ok((Decimal::ZERO, Decimal::ZERO))
            }
        }
    }

    /// 无现有库存时创建新库存记录（v14 批次 418：从明细获取真实缸号/色号/批号）
    async fn create_stock_for_receive(
        txn: &sea_orm::DatabaseTransaction,
        order: &purchase_order::Model,
        item: &purchase_order_item::Model,
        product: &product::Model,
        receive_qty_meters: Decimal,
        receive_qty_alt: Decimal,
    ) -> Result<(), AppError> {
        let _stock_model = crate::services::inventory_stock_service::InventoryStockService::create_stock_fabric_txn(
            txn,
            CreateStockFabricArgs {
                warehouse_id: order.warehouse_id,
                product_id: item.product_id,
                batch_no: item.batch_no.clone().unwrap_or_default(),
                color_no: item.color_code.clone().unwrap_or_default(),
                dye_lot_no: item.lot_no.clone(),
                grade: "A".to_string(),
                quantity_meters: receive_qty_meters,
                quantity_kg: receive_qty_alt,
                gram_weight: product.gram_weight,
                width: product.width,
                location_id: None,
                shelf_no: None,
                layer_no: None,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "创建库存记录失败: 产品ID={}, 仓库ID={}, 错误: {}",
                item.product_id, order.warehouse_id, e
            );
            AppError::internal(format!("创建库存记录失败: {}", e))
        })?;
        Ok(())
    }

    /// 记录库存流水（事务版本，返回事件由调用方 commit 后统一 publish）
    async fn record_receive_transaction(
        txn: &sea_orm::DatabaseTransaction,
        order: &purchase_order::Model,
        item: &purchase_order_item::Model,
        receive_qty_meters: Decimal,
        receive_qty_alt: Decimal,
        before_meters: Decimal,
        before_kg: Decimal,
    ) -> Result<Option<BusinessEvent>, AppError> {
        let (_, txn_event) = crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
            txn,
            RecordTransactionArgs {
                transaction_type: "PURCHASE_RECEIPT".to_string(),
                product_id: item.product_id,
                warehouse_id: order.warehouse_id,
                batch_no: item.batch_no.clone().unwrap_or_default(),
                color_no: item.color_code.clone().unwrap_or_default(),
                dye_lot_no: item.lot_no.clone(),
                grade: "A".to_string(),
                quantity_meters: receive_qty_meters,
                quantity_kg: receive_qty_alt,
                source_bill_type: Some("purchase_order".to_string()),
                source_bill_no: Some(order.order_no.clone()),
                source_bill_id: Some(order.id),
                quantity_before_meters: Some(before_meters),
                quantity_before_kg: Some(before_kg),
                quantity_after_meters: Some(before_meters + receive_qty_meters),
                quantity_after_kg: Some(before_kg + receive_qty_alt),
                notes: Some(format!("采购入库 - 订单 {}", order.order_no)),
                created_by: None,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!(
                "记录库存流水失败: 产品ID={}, 仓库ID={}, 错误: {}",
                item.product_id, order.warehouse_id, e
            );
            AppError::internal(format!("记录库存流水失败: {}", e))
        })?;
        Ok(txn_event)
    }

    /// 累加更新订单明细已入库数量
    async fn update_received_quantity(
        txn: &sea_orm::DatabaseTransaction,
        item: &purchase_order_item::Model,
        receive_qty_meters: Decimal,
        receive_qty_alt: Decimal,
    ) -> Result<(), AppError> {
        let mut item_active: purchase_order_item::ActiveModel = item.clone().into();
        item_active.received_quantity = Set(item.received_quantity + receive_qty_meters);
        item_active.received_quantity_alt = Set(item.received_quantity_alt + receive_qty_alt);
        item_active.updated_at = Set(Utc::now());
        purchase_order_item::Entity::update(item_active)
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 判断订单新状态（全部收货 COMPLETED / 部分收货 PARTIAL_RECEIVED）
    async fn determine_new_status(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<String, AppError> {
        let all_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;
        let is_fully_received = all_items.iter().all(|i| i.received_quantity >= i.quantity);
        Ok(if is_fully_received {
            status::purchase_order::COMPLETED.to_string()
        } else {
            status::purchase_order::PARTIAL_RECEIVED.to_string()
        })
    }

    /// 更新订单状态并记录审计日志
    /// 批次 94 P2-10：receive_order 由 PurchaseReceiptCompleted 事件触发，
    /// 该事件未携带 user_id，事件驱动场景下无用户上下文，暂保留 Some(0)。
    async fn update_order_status_to_received(
        txn: &sea_orm::DatabaseTransaction,
        order: purchase_order::Model,
        new_status: String,
    ) -> Result<purchase_order::Model, AppError> {
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(new_status);
        order_active.actual_delivery_date = Set(Some(now.date_naive()));
        order_active.updated_at = Set(now);
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await
    }

    /// P0 3-6 修复：入库成功后标记入库单为 COMPLETED，作为幂等键防止重复入库
    async fn mark_receipt_completed(
        txn: &sea_orm::DatabaseTransaction,
        receipt_id: Option<i32>,
    ) -> Result<(), AppError> {
        let Some(rid) = receipt_id else {
            return Ok(());
        };
        use crate::models::purchase_receipt;
        let now = chrono::Utc::now();
        // 使用结构体初始化器语法（避免 clippy::field_reassign_with_default）
        let receipt_active = purchase_receipt::ActiveModel {
            id: Set(rid),
            receipt_status: Set(status::purchase_receipt::COMPLETED.to_string()),
            updated_at: Set(now),
            ..Default::default()
        };
        purchase_receipt::Entity::update(receipt_active)
            .exec(txn)
            .await?;
        Ok(())
    }

    /// commit 成功后统一发布库存流水事件
    fn publish_receive_events(pending_events: Vec<BusinessEvent>) {
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }
    }

    // ===================================================================
    // 订单明细管理（与收货/入库密切相关的明细行操作）
    // 放置在 receipt 模块便于未来扩展按行收货、按行退货等业务
    // ===================================================================

    /// 添加订单明细
    pub async fn add_order_item(
        &self,
        order_id: i32,
        req: CreateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细写与总金额重算原子化。
        // 原实现明细 insert 与 calculate_order_total 非原子且均用 &*self.db，
        // 并发 add_order_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;
        let order = Self::lock_order_for_item(&txn, order_id).await?;
        Self::validate_order_for_item(&order, user_id)?;
        let item = Self::build_order_item_active(order_id, req, &txn).await?;
        // 事务内调用 _txn 变体，保证明细写与重算原子性；透传 user_id 用于审计日志
        self.calculate_order_total_txn(order_id, &txn, user_id).await?;
        txn.commit().await?;
        Ok(item)
    }

    /// 加锁查询订单（串行化并发明细操作）
    async fn lock_order_for_item(
        txn: &sea_orm::DatabaseTransaction,
        order_id: i32,
    ) -> Result<purchase_order::Model, AppError> {
        purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))
    }

    /// 校验订单状态与权限（仅 DRAFT 状态且创建人本人可添加明细）
    fn validate_order_for_item(
        order: &purchase_order::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许添加明细，当前状态：{}",
                order.order_status
            )));
        }
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能为自己创建的订单添加明细".to_string(),
            ));
        }
        Ok(())
    }

    /// 构建并插入订单明细行（金额计算 round_dp(2) 精度归一化）
    async fn build_order_item_active(
        order_id: i32,
        req: CreateOrderItemRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_order_item::Model, AppError> {
        // P3 维度 4 修复（批次 87）：金额计算补 round_dp(2) 精度归一化
        let quantity_ordered = req.quantity_ordered.unwrap_or(Decimal::ZERO);
        let unit_price = req.unit_price.unwrap_or(Decimal::ZERO);
        let amount = (quantity_ordered * unit_price).round_dp(2);
        let tax_percent = req.tax_rate.unwrap_or(Decimal::new(13, 2));
        let tax_amount = (amount * tax_percent / Decimal::new(100, 0)).round_dp(2);
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = (amount * discount_percent / Decimal::new(100, 0)).round_dp(2);
        let quantity_alt_ordered = req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);
        let item = purchase_order_item::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            line_no: Set(1),
            // material_id 缺失时拒绝创建收货行项，避免脏 product_id=0 记录
            product_id: Set(req
                .material_id
                .ok_or_else(|| AppError::validation("收货单缺少物料ID"))?),
            quantity: Set(quantity_ordered),
            quantity_alt: Set(quantity_alt_ordered),
            unit_price: Set(unit_price),
            unit_price_foreign: Set(unit_price),
            discount_percent: Set(discount_percent),
            tax_percent: Set(tax_percent),
            subtotal: Set(amount),
            tax_amount: Set(tax_amount),
            discount_amount: Set(discount_amount),
            total_amount: Set(amount + tax_amount - discount_amount),
            received_quantity: Set(Decimal::ZERO),
            received_quantity_alt: Set(Decimal::ZERO),
            notes: Set(req.notes),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // v14 批次 417：面料行业追溯字段（D-P1-6），使用 NotSet 让 DB 默认值处理
            color_code: sea_orm::ActiveValue::NotSet,
            lot_no: sea_orm::ActiveValue::NotSet,
            batch_no: sea_orm::ActiveValue::NotSet,
        }
        .insert(txn)
        .await?;
        Ok(item)
    }

    /// 更新订单明细
    pub async fn update_order_item(
        &self,
        item_id: i32,
        req: UpdateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细 update 与总金额重算原子化。
        // 原实现明细 update_with_audit 与 calculate_order_total 非原子且均用 &*self.db，
        // 并发 update_order_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;

        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单（加 lock_exclusive 串行化并发明细操作）
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", item.order_id)))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::business(format!(
                "订单状态不允许修改明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的订单明细".to_string(),
            ));
        }

        // 5. 更新明细（update_with_audit 传 &txn 纳入事务，保证原子性）
        let mut item_active: purchase_order_item::ActiveModel = item.into();

        if let Some(material_id) = req.material_id {
            item_active.product_id = Set(material_id);
        }
        if let Some(unit_price) = req.unit_price {
            item_active.unit_price = Set(unit_price);
        }
        if let Some(quantity) = req.quantity_ordered {
            item_active.quantity = Set(quantity);
        }
        if let Some(tax_rate) = req.tax_rate {
            item_active.tax_percent = Set(tax_rate);
        }
        if let Some(notes) = req.notes {
            item_active.notes = Set(Some(notes));
        }

        let item = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            item_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 6. 更新订单总金额（事务内调用 _txn 变体，保证明细写与重算原子性）
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.calculate_order_total_txn(order.id, &txn, user_id).await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 删除订单明细
    pub async fn delete_order_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细 delete 与总金额重算原子化。
        // 原实现明细 delete 与 calculate_order_total 非原子且均用 &*self.db，
        // 并发 delete_order_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;

        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单（加 lock_exclusive 串行化并发明细操作）
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", item.order_id)))?;

        // 3. 检查状态
        if order.order_status != "DRAFT" {
            return Err(AppError::business(format!(
                "订单状态不允许删除明细，当前状态：{}",
                order.order_status
            )));
        }

        // 4. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的订单明细".to_string(),
            ));
        }

        // 5. 删除明细
        purchase_order_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        // 6. 更新订单总金额（事务内调用 _txn 变体，保证明细写与重算原子性）
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.calculate_order_total_txn(order.id, &txn, user_id).await?;

        txn.commit().await?;

        Ok(())
    }

    /// 计算订单总金额（事务版本）
    ///
    /// 批次 19（2026-06-28）：新增 _txn 变体，接受外部事务参数，
    /// 供已有事务的调用方使用，保证明细写与总金额重算原子性。
    /// 内部 3 处 DB 句柄全部使用 txn，主表查询加 lock_exclusive 串行化并发重算，
    /// 防止两个并发重算基于过期明细快照导致丢失更新。
    pub async fn calculate_order_total_txn(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;

        // 2. 计算总和
        let mut total_amount = Decimal::new(0, 0);
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);

        for item in items {
            total_amount += item.total_amount;
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt;
        }

        // 3. 更新订单（加 lock_exclusive 串行化并发重算，防止丢失更新）
        let order = purchase_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            order_active,
            Some(user_id),
        )
        .await?;

        Ok(())
    }

    /// 计算订单总金额（便捷入口，内部自建事务）
    ///
    /// 批次 19（2026-06-28）：改为便捷入口，内部 begin + 调 _txn + commit。
    /// 已在事务内的调用方应直接调用 calculate_order_total_txn 以复用事务。
    pub async fn calculate_order_total(
        &self,
        order_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.calculate_order_total_txn(order_id, &txn, user_id).await?;
        txn.commit().await?;
        Ok(())
    }
}
