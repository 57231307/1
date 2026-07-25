//! 库存调拨批次服务（inv/batch）
//!
//! 包含调拨单明细行的增删改查（list/add/update/delete item），
//! 以及发出/接收（ship_transfer / receive_transfer）时的批次处理：
//! - ship_transfer:  扣减源仓库库存（含乐观锁）+ 记录 TRANSFER_OUT 流水
//! - receive_transfer: 增加目标仓库库存 + 记录 TRANSFER_IN 流水（自动建档）
//!
//! 原 `inventory_transfer_service.rs` 拆分而来。

use sea_orm::sea_query::{BinOper, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Order, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};

use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transaction;
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
use crate::utils::error::AppError;

use super::{
    InventoryTransferDetail, InventoryTransferItemDetail, InventoryTransferItemRequest,
    InventoryTransferService,
};

/// 仓库 → 产品库存映射（避免在函数签名中重复书写长类型）
type StockMap = std::collections::HashMap<i32, inventory_stock::Model>;

impl InventoryTransferService {
    /// 发出库存调拨
    pub async fn ship_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        let txn = (*self.db).begin().await?;
        let mut pending_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();
        let transfer = Self::lock_transfer_for_ship(&txn, transfer_id).await?;
        let items = Self::load_transfer_items(&txn, transfer_id).await?;
        let stock_map =
            Self::load_source_stock_map(&txn, transfer.from_warehouse_id, &items).await?;
        for item in items {
            Self::process_ship_item(&txn, &transfer, item, &stock_map, &mut pending_events)
                .await?;
        }
        Self::update_transfer_to_shipped(&txn, transfer).await?;
        txn.commit().await?;
        Self::publish_pending_events(
            pending_events,
            transfer_id,
            "调拨出库完成，已发布 InventoryTransactionCreated 事件触发财务凭证生成",
        );
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 接收库存调拨
    pub async fn receive_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        let txn = (*self.db).begin().await?;
        let mut pending_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();
        let transfer = Self::lock_transfer_for_receive(&txn, transfer_id).await?;
        let items = Self::load_transfer_items(&txn, transfer_id).await?;
        let (target_map, source_map) =
            Self::load_receive_stock_maps(&txn, &transfer, &items).await?;
        for item in items {
            Self::process_receive_item(
                &txn,
                &transfer,
                item,
                &target_map,
                &source_map,
                &mut pending_events,
            )
            .await?;
        }
        Self::update_transfer_to_completed(&txn, transfer).await?;
        txn.commit().await?;
        Self::publish_pending_events(
            pending_events,
            transfer_id,
            "调拨入库完成，已发布 InventoryTransactionCreated 事件触发财务凭证生成",
        );
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 加载调拨明细项
    async fn load_transfer_items(
        txn: &sea_orm::DatabaseTransaction,
        transfer_id: i32,
    ) -> Result<Vec<inventory_transfer_item::Model>, AppError> {
        Ok(InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(txn)
            .await?)
    }

    /// 锁定调拨单并校验状态为 approved（发出流程）
    async fn lock_transfer_for_ship(
        txn: &sea_orm::DatabaseTransaction,
        transfer_id: i32,
    ) -> Result<inventory_transfer::Model, AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;
        if transfer.status != "approved" {
            return Err(AppError::business(
                "只有已审核状态的调拨单可以发出".to_string(),
            ));
        }
        Ok(transfer)
    }

    /// 锁定调拨单并校验状态为 shipped（接收流程）
    async fn lock_transfer_for_receive(
        txn: &sea_orm::DatabaseTransaction,
        transfer_id: i32,
    ) -> Result<inventory_transfer::Model, AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;
        if transfer.status != "shipped" {
            return Err(AppError::business(
                "只有已发出状态的调拨单可以接收".to_string(),
            ));
        }
        Ok(transfer)
    }

    /// 批量加载源仓库库存映射（避免 N+1）
    async fn load_source_stock_map(
        txn: &sea_orm::DatabaseTransaction,
        warehouse_id: i32,
        items: &[inventory_transfer_item::Model],
    ) -> Result<StockMap, AppError> {
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        Ok(stocks.into_iter().map(|s| (s.product_id, s)).collect())
    }

    /// 批量加载目标仓库和源仓库库存映射（接收流程）
    async fn load_receive_stock_maps(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        items: &[inventory_transfer_item::Model],
    ) -> Result<(StockMap, StockMap), AppError> {
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let target_stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids.clone()))
            .all(txn)
            .await?;
        let target_map: StockMap =
            target_stocks.into_iter().map(|s| (s.product_id, s)).collect();
        let source_stocks = if product_ids.is_empty() {
            Vec::new()
        } else {
            InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                .filter(inventory_stock::Column::ProductId.is_in(product_ids))
                .all(txn)
                .await?
        };
        let source_map = source_stocks.into_iter().map(|s| (s.product_id, s)).collect();
        Ok((target_map, source_map))
    }

    /// 处理单个明细行的发出：扣减库存、写流水、收集事件、更新明细
    async fn process_ship_item(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: inventory_transfer_item::Model,
        stock_map: &StockMap,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
    ) -> Result<(), AppError> {
        let stock = stock_map.get(&item.product_id).cloned().ok_or_else(|| {
            tracing::error!("产品 {} 在源仓库无库存记录", item.product_id);
            AppError::business(format!("产品 {} 在源仓库无库存记录", item.product_id))
        })?;
        if stock.quantity_on_hand < item.quantity {
            tracing::error!("产品 {} 库存不足", item.product_id);
            return Err(AppError::business(format!(
                "产品 {} 库存不足",
                item.product_id
            )));
        }
        let (new_meters, new_kg) =
            Self::decrement_source_stock(txn, &stock, item.quantity).await?;
        let inserted = Self::insert_transfer_out_transaction(
            txn,
            transfer,
            &item,
            &stock,
            new_meters,
            new_kg,
        )
        .await?;
        pending_events.push(Self::make_transaction_event(&inserted));
        Self::update_shipped_item(txn, item).await?;
        Ok(())
    }

    /// 乐观锁扣减源仓库库存，返回 (new_meters, new_kg)
    async fn decrement_source_stock(
        txn: &sea_orm::DatabaseTransaction,
        stock: &inventory_stock::Model,
        quantity: rust_decimal::Decimal,
    ) -> Result<(rust_decimal::Decimal, rust_decimal::Decimal), AppError> {
        let new_meters = stock.quantity_meters - quantity;
        let new_kg = if stock.quantity_meters > rust_decimal::Decimal::ZERO {
            (stock.quantity_kg - (stock.quantity_kg * quantity / stock.quantity_meters))
                .round_dp(4)
        } else {
            stock.quantity_kg
        };
        let result = inventory_stock::Entity::update_many()
            .col_expr(
                inventory_stock::Column::QuantityOnHand,
                Expr::col(inventory_stock::Column::QuantityOnHand)
                    .binary(BinOper::Sub, Expr::val(quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                Expr::col(inventory_stock::Column::QuantityAvailable)
                    .binary(BinOper::Sub, Expr::val(quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityMeters,
                Expr::val(new_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityKg,
                Expr::val(new_kg).into(),
            )
            .col_expr(
                inventory_stock::Column::Version,
                Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::Version.eq(stock.version))
            .exec(txn)
            .await?;
        if result.rows_affected == 0 {
            tracing::error!("产品 {} 并发冲突", stock.product_id);
            return Err(AppError::business(format!(
                "产品 {} 库存记录已被其他用户修改，请重试",
                stock.product_id
            )));
        }
        Ok((new_meters, new_kg))
    }

    /// 写入 TRANSFER_OUT 库存流水
    async fn insert_transfer_out_transaction(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: &inventory_transfer_item::Model,
        stock: &inventory_stock::Model,
        new_meters: rust_decimal::Decimal,
        new_kg: rust_decimal::Decimal,
    ) -> Result<inventory_transaction::Model, AppError> {
        let transaction = inventory_transaction::ActiveModel {
            id: sea_orm::ActiveValue::Set(0),
            transaction_type: sea_orm::ActiveValue::Set("TRANSFER_OUT".to_string()),
            product_id: sea_orm::ActiveValue::Set(item.product_id),
            warehouse_id: sea_orm::ActiveValue::Set(transfer.from_warehouse_id),
            batch_no: sea_orm::ActiveValue::Set(stock.batch_no.clone()),
            color_no: sea_orm::ActiveValue::Set(stock.color_no.clone()),
            dye_lot_no: sea_orm::ActiveValue::Set(stock.dye_lot_no.clone()),
            grade: sea_orm::ActiveValue::Set(stock.grade.clone()),
            quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
            quantity_kg: sea_orm::ActiveValue::Set(stock.quantity_kg - new_kg),
            source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
            source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(transfer.id)),
            quantity_before_meters: sea_orm::ActiveValue::Set(Some(stock.quantity_meters)),
            quantity_before_kg: sea_orm::ActiveValue::Set(Some(stock.quantity_kg)),
            quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_meters)),
            quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_kg)),
            notes: sea_orm::ActiveValue::Set(Some(format!(
                "调拨出库 - 调拨单号: {}",
                transfer.transfer_no
            ))),
            created_by: sea_orm::ActiveValue::Set(transfer.created_by),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        transaction.insert(txn).await.map_err(Into::into)
    }

    /// 处理单个明细行的接收：增加库存或新建库存、写流水、收集事件、更新明细
    async fn process_receive_item(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: inventory_transfer_item::Model,
        target_map: &StockMap,
        source_map: &StockMap,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
    ) -> Result<(), AppError> {
        let source_stock = source_map.get(&item.product_id).cloned();
        if let Some(stock) = target_map.get(&item.product_id).cloned() {
            let source_kg_per_meter = source_stock
                .as_ref()
                .filter(|s| s.quantity_meters > rust_decimal::Decimal::ZERO)
                .map(|s| s.quantity_kg / s.quantity_meters)
                .unwrap_or(rust_decimal::Decimal::ZERO);
            let (new_meters, new_kg) =
                Self::increment_target_stock(txn, &stock, item.quantity, source_kg_per_meter)
                    .await?;
            let inserted = Self::insert_transfer_in_transaction(
                txn,
                transfer,
                &item,
                &stock,
                new_meters,
                new_kg,
            )
            .await?;
            pending_events.push(Self::make_transaction_event(&inserted));
            Self::update_received_item(txn, item).await?;
        } else {
            let quantity_kg =
                Self::create_target_stock_record(txn, transfer, &item, source_stock.as_ref())
                    .await?;
            let inserted = Self::insert_transfer_in_transaction_new(
                txn,
                transfer,
                &item,
                source_stock.as_ref(),
                quantity_kg,
            )
            .await?;
            pending_events.push(Self::make_transaction_event(&inserted));
        }
        Ok(())
    }

    /// 乐观锁增加目标仓库库存，返回 (new_meters, new_kg)
    async fn increment_target_stock(
        txn: &sea_orm::DatabaseTransaction,
        stock: &inventory_stock::Model,
        quantity: rust_decimal::Decimal,
        source_kg_per_meter: rust_decimal::Decimal,
    ) -> Result<(rust_decimal::Decimal, rust_decimal::Decimal), AppError> {
        let new_meters = stock.quantity_meters + quantity;
        let new_kg = (stock.quantity_kg + (quantity * source_kg_per_meter)).round_dp(4);
        let result = inventory_stock::Entity::update_many()
            .col_expr(
                inventory_stock::Column::QuantityOnHand,
                Expr::col(inventory_stock::Column::QuantityOnHand)
                    .binary(BinOper::Add, Expr::val(quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                Expr::col(inventory_stock::Column::QuantityAvailable)
                    .binary(BinOper::Add, Expr::val(quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityMeters,
                Expr::val(new_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityKg,
                Expr::val(new_kg).into(),
            )
            .col_expr(
                inventory_stock::Column::Version,
                Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::Version.eq(stock.version))
            .exec(txn)
            .await?;
        if result.rows_affected == 0 {
            tracing::error!("产品 {} 并发冲突", stock.product_id);
            return Err(AppError::business(format!(
                "产品 {} 库存记录已被其他用户修改，请重试",
                stock.product_id
            )));
        }
        Ok((new_meters, new_kg))
    }

    /// 写入 TRANSFER_IN 库存流水（已有库存记录）
    async fn insert_transfer_in_transaction(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: &inventory_transfer_item::Model,
        stock: &inventory_stock::Model,
        new_meters: rust_decimal::Decimal,
        new_kg: rust_decimal::Decimal,
    ) -> Result<inventory_transaction::Model, AppError> {
        let transaction = inventory_transaction::ActiveModel {
            id: sea_orm::ActiveValue::Set(0),
            transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
            product_id: sea_orm::ActiveValue::Set(item.product_id),
            warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
            batch_no: sea_orm::ActiveValue::Set(stock.batch_no.clone()),
            color_no: sea_orm::ActiveValue::Set(stock.color_no.clone()),
            dye_lot_no: sea_orm::ActiveValue::Set(stock.dye_lot_no.clone()),
            grade: sea_orm::ActiveValue::Set(stock.grade.clone()),
            quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
            quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
            source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(transfer.id)),
            quantity_before_meters: sea_orm::ActiveValue::Set(Some(stock.quantity_meters)),
            quantity_before_kg: sea_orm::ActiveValue::Set(Some(stock.quantity_kg)),
            quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_meters)),
            quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_kg)),
            notes: sea_orm::ActiveValue::Set(Some(format!(
                "调拨入库 - 调拨单号: {}",
                transfer.transfer_no
            ))),
            created_by: sea_orm::ActiveValue::Set(transfer.created_by),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        transaction.insert(txn).await.map_err(Into::into)
    }

    /// 在目标仓库新建库存记录，返回 quantity_kg
    async fn create_target_stock_record(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: &inventory_transfer_item::Model,
        source_stock: Option<&inventory_stock::Model>,
    ) -> Result<rust_decimal::Decimal, AppError> {
        let batch_no = source_stock
            .map(|s| s.batch_no.clone())
            .unwrap_or_default();
        let color_no = source_stock
            .map(|s| s.color_no.clone())
            .unwrap_or_default();
        let dye_lot_no = source_stock.and_then(|s| s.dye_lot_no.clone());
        let grade = source_stock
            .map(|s| s.grade.clone())
            .unwrap_or_else(|| "一等品".to_string());
        let gram_weight = source_stock.and_then(|s| s.gram_weight);
        let width = source_stock.and_then(|s| s.width);
        let production_date = source_stock.and_then(|s| s.production_date);
        let expiry_date = source_stock.and_then(|s| s.expiry_date);
        let source_kg_per_meter = source_stock
            .filter(|s| s.quantity_meters > rust_decimal::Decimal::ZERO)
            .map(|s| s.quantity_kg / s.quantity_meters)
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let quantity_kg = (item.quantity * source_kg_per_meter).round_dp(4);
        let new_stock = inventory_stock::ActiveModel {
            id: Default::default(),
            warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
            product_id: sea_orm::ActiveValue::Set(item.product_id),
            quantity_on_hand: sea_orm::ActiveValue::Set(item.quantity),
            quantity_available: sea_orm::ActiveValue::Set(item.quantity),
            quantity_reserved: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            quantity_incoming: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            reorder_point: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            max_stock_point: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            reorder_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            last_count_date: sea_orm::ActiveValue::NotSet,
            last_movement_date: sea_orm::ActiveValue::NotSet,
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            batch_no: sea_orm::ActiveValue::Set(batch_no),
            color_no: sea_orm::ActiveValue::Set(color_no),
            dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
            grade: sea_orm::ActiveValue::Set(grade),
            production_date: sea_orm::ActiveValue::Set(production_date),
            expiry_date: sea_orm::ActiveValue::Set(expiry_date),
            quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
            quantity_kg: sea_orm::ActiveValue::Set(quantity_kg),
            gram_weight: sea_orm::ActiveValue::Set(gram_weight),
            width: sea_orm::ActiveValue::Set(width),
            location_id: sea_orm::ActiveValue::NotSet,
            shelf_no: sea_orm::ActiveValue::NotSet,
            layer_no: sea_orm::ActiveValue::NotSet,
            bin_location: sea_orm::ActiveValue::NotSet,
            stock_status: sea_orm::ActiveValue::Set("正常".to_string()),
            quality_status: sea_orm::ActiveValue::Set("合格".to_string()),
            version: sea_orm::ActiveValue::Set(0),
            quantity_shipped: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
        };
        new_stock.insert(txn).await?;
        Ok(quantity_kg)
    }

    /// 写入 TRANSFER_IN 库存流水（新建库存记录）
    async fn insert_transfer_in_transaction_new(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: &inventory_transfer_item::Model,
        source_stock: Option<&inventory_stock::Model>,
        quantity_kg: rust_decimal::Decimal,
    ) -> Result<inventory_transaction::Model, AppError> {
        let f = Self::extract_target_stock_fields(source_stock);
        let transaction = inventory_transaction::ActiveModel {
            id: sea_orm::ActiveValue::Set(0),
            transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
            product_id: sea_orm::ActiveValue::Set(item.product_id),
            warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
            batch_no: sea_orm::ActiveValue::Set(f.batch_no),
            color_no: sea_orm::ActiveValue::Set(f.color_no),
            dye_lot_no: sea_orm::ActiveValue::Set(f.dye_lot_no),
            grade: sea_orm::ActiveValue::Set(f.grade),
            quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
            quantity_kg: sea_orm::ActiveValue::Set(quantity_kg),
            source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
            source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(transfer.id)),
            quantity_before_meters: sea_orm::ActiveValue::Set(Some(rust_decimal::Decimal::ZERO)),
            quantity_before_kg: sea_orm::ActiveValue::Set(Some(rust_decimal::Decimal::ZERO)),
            quantity_after_meters: sea_orm::ActiveValue::Set(Some(item.quantity)),
            quantity_after_kg: sea_orm::ActiveValue::Set(Some(quantity_kg)),
            notes: sea_orm::ActiveValue::Set(Some(format!(
                "调拨入库（新建库存） - 调拨单号: {}",
                transfer.transfer_no
            ))),
            created_by: sea_orm::ActiveValue::Set(transfer.created_by),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        transaction.insert(txn).await.map_err(Into::into)
    }

    /// 从源仓库库存提取流水所需的面料行业字段（仅流水使用，库存创建在 create_target_stock_record 内独立处理）
    fn extract_target_stock_fields(
        source_stock: Option<&inventory_stock::Model>,
    ) -> TargetStockFields {
        TargetStockFields {
            batch_no: source_stock.map(|s| s.batch_no.clone()).unwrap_or_default(),
            color_no: source_stock.map(|s| s.color_no.clone()).unwrap_or_default(),
            dye_lot_no: source_stock.and_then(|s| s.dye_lot_no.clone()),
            grade: source_stock.map(|s| s.grade.clone()).unwrap_or_else(|| "一等品".to_string()),
        }
    }

    /// 从已插入的流水构造 InventoryTransactionCreated 事件
    fn make_transaction_event(
        inserted: &inventory_transaction::Model,
    ) -> crate::services::event_bus::BusinessEvent {
        crate::services::event_bus::BusinessEvent::InventoryTransactionCreated {
            transaction_id: inserted.id,
            transaction_type: inserted.transaction_type.clone(),
            product_id: inserted.product_id,
            warehouse_id: inserted.warehouse_id,
            quantity_meters: inserted.quantity_meters,
            quantity_kg: inserted.quantity_kg,
            source_bill_type: inserted.source_bill_type.clone(),
            source_bill_no: inserted.source_bill_no.clone(),
            source_bill_id: inserted.source_bill_id,
            batch_no: inserted.batch_no.clone(),
            color_no: inserted.color_no.clone(),
            created_by: inserted.created_by,
        }
    }

    /// 更新明细项已发出数量（带审计）
    async fn update_shipped_item(
        txn: &sea_orm::DatabaseTransaction,
        item: inventory_transfer_item::Model,
    ) -> Result<(), AppError> {
        let item_quantity = item.quantity;
        let mut item_update: inventory_transfer_item::ActiveModel = item.into();
        item_update.shipped_quantity = sea_orm::ActiveValue::Set(item_quantity);
        item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            item_update,
            Some(0),
        )
        .await?;
        Ok(())
    }

    /// 更新明细项已接收数量（带审计）
    async fn update_received_item(
        txn: &sea_orm::DatabaseTransaction,
        item: inventory_transfer_item::Model,
    ) -> Result<(), AppError> {
        let item_quantity = item.quantity;
        let mut item_update: inventory_transfer_item::ActiveModel = item.into();
        item_update.received_quantity = sea_orm::ActiveValue::Set(item_quantity);
        item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            item_update,
            Some(0),
        )
        .await?;
        Ok(())
    }

    /// 更新调拨单状态为 shipped（带审计）
    async fn update_transfer_to_shipped(
        txn: &sea_orm::DatabaseTransaction,
        transfer: inventory_transfer::Model,
    ) -> Result<(), AppError> {
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("shipped".to_string());
        transfer_update.shipped_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;
        Ok(())
    }

    /// 更新调拨单状态为 completed（带审计）
    async fn update_transfer_to_completed(
        txn: &sea_orm::DatabaseTransaction,
        transfer: inventory_transfer::Model,
    ) -> Result<(), AppError> {
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        transfer_update.received_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;
        Ok(())
    }

    /// commit 成功后统一发布事件，避免回滚造成幻事件
    fn publish_pending_events(
        pending_events: Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
        success_msg: &'static str,
    ) {
        let events_count = pending_events.len();
        for event in pending_events {
            crate::services::event_bus::EVENT_BUS.publish(event);
        }
        if events_count > 0 {
            tracing::info!(transfer_id, events_count, "{}", success_msg);
        }
    }

    /// 列出调拨单的所有明细项
    pub async fn list_items(
        &self,
        transfer_id: i32,
    ) -> Result<Vec<InventoryTransferItemDetail>, AppError> {
        // 批次 113 P1-8：移除 `let _ =` 显式丢弃，直接表达式语句校验存在性
        self.get_transfer_detail(transfer_id, None).await?;
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .order_by(inventory_transfer_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(items
            .into_iter()
            .map(|item| InventoryTransferItemDetail {
                id: item.id,
                transfer_id: item.transfer_id,
                product_id: item.product_id,
                quantity: item.quantity,
                shipped_quantity: item.shipped_quantity,
                received_quantity: item.received_quantity,
                unit_cost: item.unit_cost,
                notes: item.notes,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect())
    }

    /// 向调拨单添加明细
    pub async fn add_item(
        &self,
        transfer_id: i32,
        req: InventoryTransferItemRequest,
    ) -> Result<InventoryTransferItemDetail, AppError> {
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;
        if transfer.status == "shipped" || transfer.status == "completed" {
            return Err(AppError::business(format!(
                "调拨单状态 {} 不允许添加明细",
                transfer.status
            )));
        }
        let txn = (*self.db).begin().await?;
        // 物料 ID 缺失时拒绝创建批次库存，避免脏 product_id=0 记录
        let product_id = req
            .product_id
            .ok_or_else(|| AppError::validation("批次缺少物料ID"))?;
        let quantity = req.quantity.unwrap_or(rust_decimal::Decimal::ZERO);
        let item = inventory_transfer_item::ActiveModel {
            id: Default::default(),
            transfer_id: sea_orm::ActiveValue::Set(transfer_id),
            product_id: sea_orm::ActiveValue::Set(product_id),
            quantity: sea_orm::ActiveValue::Set(quantity),
            shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            unit_cost: sea_orm::ActiveValue::NotSet,
            notes: sea_orm::ActiveValue::Set(req.notes),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            // v14 批次 417：面料行业追溯字段（T-P0-1），使用 NotSet 让 DB 默认值处理
            color_no: sea_orm::ActiveValue::NotSet,
            dye_lot_no: sea_orm::ActiveValue::NotSet,
            batch_no: sea_orm::ActiveValue::NotSet,
        };
        let item_model = item.insert(&txn).await?;
        // 重新计算总数量
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(&txn)
            .await?;
        let total_quantity: rust_decimal::Decimal = items.iter().map(|i| i.quantity).sum();
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        transfer_update.update(&txn).await?;
        txn.commit().await?;
        Ok(InventoryTransferItemDetail {
            id: item_model.id,
            transfer_id: item_model.transfer_id,
            product_id: item_model.product_id,
            quantity: item_model.quantity,
            shipped_quantity: item_model.shipped_quantity,
            received_quantity: item_model.received_quantity,
            unit_cost: item_model.unit_cost,
            notes: item_model.notes,
            created_at: item_model.created_at,
            updated_at: item_model.updated_at,
        })
    }

    /// 更新调拨单明细
    pub async fn update_item(
        &self,
        item_id: i32,
        req: InventoryTransferItemRequest,
    ) -> Result<InventoryTransferItemDetail, AppError> {
        let item_model = InventoryTransferItemEntity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调拨明细 {} 未找到", item_id)))?;
        let transfer = InventoryTransferEntity::find_by_id(item_model.transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("调拨单不存在"))?;
        if transfer.status == "shipped" || transfer.status == "completed" {
            return Err(AppError::business(format!(
                "调拨单状态 {} 不允许修改明细",
                transfer.status
            )));
        }
        let mut active: inventory_transfer_item::ActiveModel = item_model.into_active_model();
        if let Some(product_id) = req.product_id {
            active.product_id = sea_orm::ActiveValue::Set(product_id);
        }
        if let Some(quantity) = req.quantity {
            active.quantity = sea_orm::ActiveValue::Set(quantity);
        }
        if let Some(notes) = req.notes {
            active.notes = sea_orm::ActiveValue::Set(Some(notes));
        }
        active.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        let updated = active.update(&*self.db).await?;
        // 重新计算总数量
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(updated.transfer_id))
            .all(&*self.db)
            .await?;
        let total_quantity: rust_decimal::Decimal = items.iter().map(|i| i.quantity).sum();
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        transfer_update.update(&*self.db).await?;
        Ok(InventoryTransferItemDetail {
            id: updated.id,
            transfer_id: updated.transfer_id,
            product_id: updated.product_id,
            quantity: updated.quantity,
            shipped_quantity: updated.shipped_quantity,
            received_quantity: updated.received_quantity,
            unit_cost: updated.unit_cost,
            notes: updated.notes,
            created_at: updated.created_at,
            updated_at: updated.updated_at,
        })
    }

    /// 删除调拨单明细
    pub async fn delete_item(&self, item_id: i32) -> Result<(), AppError> {
        let item_model = InventoryTransferItemEntity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("调拨明细 {} 未找到", item_id)))?;
        let transfer = InventoryTransferEntity::find_by_id(item_model.transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("调拨单不存在"))?;
        if transfer.status == "shipped" || transfer.status == "completed" {
            return Err(AppError::business(format!(
                "调拨单状态 {} 不允许删除明细",
                transfer.status
            )));
        }
        let txn = (*self.db).begin().await?;
        InventoryTransferItemEntity::delete_by_id(item_id)
            .exec(&txn)
            .await?;
        // 重新计算总数量
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(item_model.transfer_id))
            .all(&txn)
            .await?;
        let total_quantity: rust_decimal::Decimal = items.iter().map(|i| i.quantity).sum();
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.total_quantity = sea_orm::ActiveValue::Set(total_quantity);
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        transfer_update.update(&txn).await?;
        txn.commit().await?;
        Ok(())
    }
}

/// 流水所需的面料行业字段（从源仓库库存复制，仅含流水实际使用的字段）
struct TargetStockFields {
    batch_no: String,
    color_no: String,
    dye_lot_no: Option<String>,
    grade: String,
}