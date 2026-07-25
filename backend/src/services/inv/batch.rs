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

/// 新建库存的面料行业追溯字段（从源仓库复制，封装避免参数过多）。
struct NewStockFabricFields<'a> {
    batch_no: &'a str,
    color_no: &'a str,
    dye_lot_no: Option<&'a str>,
    grade: &'a str,
    gram_weight: Option<rust_decimal::Decimal>,
    width: Option<rust_decimal::Decimal>,
    production_date: Option<chrono::NaiveDate>,
    expiry_date: Option<chrono::NaiveDate>,
    source_kg_per_meter: rust_decimal::Decimal,
}

/// TRANSFER_IN 库存流水构造参数（封装 before/after 数量与单号避免参数过多）。
struct TransferInTxnFields<'a> {
    product_id: i32,
    warehouse_id: i32,
    batch_no: &'a str,
    color_no: &'a str,
    dye_lot_no: Option<&'a str>,
    grade: &'a str,
    quantity_meters: rust_decimal::Decimal,
    quantity_kg: rust_decimal::Decimal,
    quantity_before_meters: Option<rust_decimal::Decimal>,
    quantity_before_kg: Option<rust_decimal::Decimal>,
    quantity_after_meters: Option<rust_decimal::Decimal>,
    quantity_after_kg: Option<rust_decimal::Decimal>,
    notes: &'a str,
    created_by: Option<i32>,
    transfer_id: i32,
    transfer_no: &'a str,
}

impl InventoryTransferService {
    /// 发出库存调拨
    pub async fn ship_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 开启事务
        let txn = (*self.db).begin().await?;

        // v14 批次 420 修复 T-P1-1：调拨流程事件收集
        // 原实现直接 insert 流水后未发布 InventoryTransactionCreated 事件，
        // 导致下游库存财务桥接服务无法感知调拨出/入库，库存账与财务账脱节。
        // 修复：事务内收集事件，commit 成功后统一 publish（避免回滚造成幻事件）。
        let mut pending_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();

        // 检查调拨单是否存在（行锁，串行化并发状态变更）
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有已审核的调拨单可以发出
        if transfer.status != "approved" {
            return Err(AppError::business(
                "只有已审核状态的调拨单可以发出".to_string(),
            ));
        }

        // 获取调拨明细项
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(&txn)
            .await?;

        // 批量获取源仓库库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        // 扣减源仓库库存
        for item in items {
            // 查找源仓库库存记录
            let stock = stock_map.get(&item.product_id);

            if let Some(stock_model) = stock {
                // 检查库存是否充足
                if stock_model.quantity_on_hand < item.quantity {
                    tracing::error!("Transaction rolled back: 产品 {} 库存不足", item.product_id);
                    txn.rollback().await?;
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足",
                        item.product_id
                    )));
                }

                // 保存需要使用的值
                let stock_id = stock_model.id;
                let _quantity_on_hand = stock_model.quantity_on_hand;
                let quantity_meters = stock_model.quantity_meters;
                let quantity_kg = stock_model.quantity_kg;
                let expected_version = stock_model.version;
                let batch_no = stock_model.batch_no.clone();
                let color_no = stock_model.color_no.clone();
                let dye_lot_no = stock_model.dye_lot_no.clone();
                let grade = stock_model.grade.clone();
                let _stock_model = stock_model.clone();

                // 扣减库存（带乐观锁）
                let new_quantity_meters = quantity_meters - item.quantity;
                // Calculate kg reduction proportionally
                // 批次 97 P1-12 修复（v5 复审）：kg 计算补 round_dp(4) 防止精度漂移
                let new_quantity_kg = if quantity_meters > rust_decimal::Decimal::ZERO {
                    (quantity_kg - (quantity_kg * item.quantity / quantity_meters)).round_dp(4)
                } else {
                    quantity_kg
                };

                // 使用乐观锁条件更新：只有 version 匹配时才更新
                let update_result = inventory_stock::Entity::update_many()
                    .col_expr(
                        inventory_stock::Column::QuantityOnHand,
                        Expr::col(inventory_stock::Column::QuantityOnHand).binary(BinOper::Sub, Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        Expr::col(inventory_stock::Column::QuantityAvailable).binary(BinOper::Sub, Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityMeters,
                        Expr::val(new_quantity_meters).into(),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityKg,
                        Expr::val(new_quantity_kg).into(),
                    )
                    .col_expr(
                        inventory_stock::Column::Version,
                        Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                    )
                    .filter(inventory_stock::Column::Id.eq(stock_id))
                    .filter(inventory_stock::Column::Version.eq(expected_version))
                    .exec(&txn)
                    .await?;

                // 检查乐观锁是否成功
                if update_result.rows_affected == 0 {
                    tracing::error!("Transaction rolled back: 产品 {} 并发冲突", item.product_id);
                    txn.rollback().await?;
                    return Err(AppError::business(format!(
                        "产品 {} 库存记录已被其他用户修改，请重试",
                        item.product_id
                    )));
                }

                // 记录 TRANSFER_OUT 库存流水
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_OUT".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.from_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no.clone()),
                    color_no: sea_orm::ActiveValue::Set(color_no.clone()),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no.clone()),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(quantity_kg - new_quantity_kg),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(quantity_meters)),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(quantity_kg)),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_quantity_meters)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_quantity_kg)),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨出库 - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                let inserted_transaction = transaction.insert(&txn).await?;

                // v14 批次 420 修复 T-P1-1：事务内收集 InventoryTransactionCreated 事件
                // 事务内仅收集不发布，避免 commit 失败导致幻事件
                pending_events.push(crate::services::event_bus::BusinessEvent::InventoryTransactionCreated {
                    transaction_id: inserted_transaction.id,
                    transaction_type: inserted_transaction.transaction_type.clone(),
                    product_id: inserted_transaction.product_id,
                    warehouse_id: inserted_transaction.warehouse_id,
                    quantity_meters: inserted_transaction.quantity_meters,
                    quantity_kg: inserted_transaction.quantity_kg,
                    source_bill_type: inserted_transaction.source_bill_type.clone(),
                    source_bill_no: inserted_transaction.source_bill_no.clone(),
                    source_bill_id: inserted_transaction.source_bill_id,
                    batch_no: inserted_transaction.batch_no.clone(),
                    color_no: inserted_transaction.color_no.clone(),
                    created_by: inserted_transaction.created_by,
                });

                // 更新明细项已发出数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.shipped_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &txn,
                    "auto_audit",
                    item_update,
                    Some(0),
                )
                .await?;
            } else {
                tracing::error!(
                    "Transaction rolled back: 产品 {} 在源仓库无库存记录",
                    item.product_id
                );
                txn.rollback().await?;
                return Err(AppError::business(format!(
                    "产品 {} 在源仓库无库存记录",
                    item.product_id
                )));
            }
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("shipped".to_string());
        transfer_update.shipped_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        transfer_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            transfer_update,
            Some(0),
        )
        .await?;

        // 提交事务
        txn.commit().await?;

        // v14 批次 420 修复 T-P1-1：commit 成功后统一发布事件，避免回滚造成幻事件
        let events_count = pending_events.len();
        for event in pending_events {
            crate::services::event_bus::EVENT_BUS.publish(event);
        }
        if events_count > 0 {
            tracing::info!(
                transfer_id,
                events_count,
                "调拨出库完成，已发布 InventoryTransactionCreated 事件触发财务凭证生成"
            );
        }

        // 返回调拨详情
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 接收库存调拨
    pub async fn receive_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        let txn = (*self.db).begin().await?;
        let mut pending_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();

        let transfer = Self::lock_and_validate_transfer_for_receive(&txn, transfer_id).await?;
        let items = Self::load_transfer_items(&txn, transfer_id).await?;
        let (stock_map, source_stock_map) =
            Self::load_receive_stock_maps(&txn, &transfer, &items).await?;

        for item in items {
            if stock_map.contains_key(&item.product_id) {
                Self::apply_receive_existing_stock(
                    &txn,
                    &transfer,
                    &stock_map,
                    &source_stock_map,
                    item,
                    &mut pending_events,
                    transfer_id,
                )
                .await?;
            } else {
                Self::apply_receive_new_stock(
                    &txn,
                    &transfer,
                    &source_stock_map,
                    item,
                    &mut pending_events,
                    transfer_id,
                )
                .await?;
            }
        }

        Self::update_transfer_to_completed(&txn, transfer).await?;
        txn.commit().await?;
        Self::publish_receive_events(pending_events, transfer_id);
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 锁定调拨单并校验状态为 shipped（串行化并发状态变更）。
    async fn lock_and_validate_transfer_for_receive(
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

    /// 加载调拨单明细项。
    async fn load_transfer_items(
        txn: &sea_orm::DatabaseTransaction,
        transfer_id: i32,
    ) -> Result<Vec<inventory_transfer_item::Model>, AppError> {
        Ok(InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(txn)
            .await?)
    }

    /// 批量加载目标仓库与源仓库的库存记录（避免循环内 N+1 查询）。
    async fn load_receive_stock_maps(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        items: &[inventory_transfer_item::Model],
    ) -> Result<
        (
            std::collections::HashMap<i32, inventory_stock::Model>,
            std::collections::HashMap<i32, inventory_stock::Model>,
        ),
        AppError,
    > {
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = if product_ids.is_empty() {
            Vec::new()
        } else {
            InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.to_warehouse_id))
                .filter(inventory_stock::Column::ProductId.is_in(product_ids.clone()))
                .all(txn)
                .await?
        };
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        let source_stocks = if product_ids.is_empty() {
            Vec::new()
        } else {
            InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                .filter(inventory_stock::Column::ProductId.is_in(product_ids))
                .all(txn)
                .await?
        };
        let source_stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            source_stocks.into_iter().map(|s| (s.product_id, s)).collect();
        Ok((stock_map, source_stock_map))
    }

    /// 计算源仓库的公斤/米比率（quantity_meters=0 时返回 0 避免除零）。
    fn compute_source_kg_per_meter(src: &inventory_stock::Model) -> rust_decimal::Decimal {
        if src.quantity_meters > rust_decimal::Decimal::ZERO {
            src.quantity_kg / src.quantity_meters
        } else {
            rust_decimal::Decimal::ZERO
        }
    }

    /// 处理已有库存记录的接收：乐观锁更新 + 写流水 + 收集事件 + 更新明细已收数量。
    async fn apply_receive_existing_stock(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
        source_stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
        item: inventory_transfer_item::Model,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        let stock_model = stock_map.get(&item.product_id).ok_or_else(|| {
            AppError::business(format!("产品 {} 库存记录缺失", item.product_id))
        })?;
        let stock_id = stock_model.id;
        let quantity_meters = stock_model.quantity_meters;
        let quantity_kg = stock_model.quantity_kg;
        let expected_version = stock_model.version;
        let batch_no = stock_model.batch_no.clone();
        let color_no = stock_model.color_no.clone();
        let dye_lot_no = stock_model.dye_lot_no.clone();
        let grade = stock_model.grade.clone();

        let new_quantity_meters = quantity_meters + item.quantity;
        let source_kg_per_meter = source_stock_map
            .get(&item.product_id)
            .map(Self::compute_source_kg_per_meter)
            .unwrap_or(rust_decimal::Decimal::ZERO);
        // 批次 97 P1-12 修复（v5 复审）：kg 计算补 round_dp(4) 防止精度漂移
        let new_quantity_kg =
            (quantity_kg + (item.quantity * source_kg_per_meter)).round_dp(4);

        Self::update_existing_stock_with_optimistic_lock(
            txn,
            stock_id,
            expected_version,
            item.quantity,
            new_quantity_meters,
            new_quantity_kg,
            item.product_id,
        )
        .await?;

        let transaction = Self::build_transfer_in_transaction(TransferInTxnFields {
            product_id: item.product_id,
            warehouse_id: transfer.to_warehouse_id,
            batch_no: &batch_no,
            color_no: &color_no,
            dye_lot_no: dye_lot_no.as_deref(),
            grade: &grade,
            quantity_meters: item.quantity,
            quantity_kg: rust_decimal::Decimal::ZERO,
            quantity_before_meters: Some(quantity_meters),
            quantity_before_kg: Some(quantity_kg),
            quantity_after_meters: Some(new_quantity_meters),
            quantity_after_kg: Some(new_quantity_kg),
            notes: &format!("调拨入库 - 调拨单号: {}", transfer.transfer_no),
            created_by: transfer.created_by,
            transfer_id,
            transfer_no: &transfer.transfer_no,
        });
        let inserted = transaction.insert(txn).await?;
        pending_events.push(Self::build_inventory_transaction_created_event(&inserted));

        Self::update_item_received_quantity(txn, item, item.quantity).await?;
        Ok(())
    }

    /// 乐观锁条件更新：只有 version 匹配时才更新（rows_affected=0 时回滚事务并报错）。
    async fn update_existing_stock_with_optimistic_lock(
        txn: &sea_orm::DatabaseTransaction,
        stock_id: i32,
        expected_version: i32,
        item_quantity: rust_decimal::Decimal,
        new_quantity_meters: rust_decimal::Decimal,
        new_quantity_kg: rust_decimal::Decimal,
        product_id: i32,
    ) -> Result<(), AppError> {
        let update_result = inventory_stock::Entity::update_many()
            .col_expr(
                inventory_stock::Column::QuantityOnHand,
                Expr::col(inventory_stock::Column::QuantityOnHand)
                    .binary(BinOper::Add, Expr::val(item_quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                Expr::col(inventory_stock::Column::QuantityAvailable)
                    .binary(BinOper::Add, Expr::val(item_quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityMeters,
                Expr::val(new_quantity_meters).into(),
            )
            .col_expr(
                inventory_stock::Column::QuantityKg,
                Expr::val(new_quantity_kg).into(),
            )
            .col_expr(
                inventory_stock::Column::Version,
                Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .filter(inventory_stock::Column::Id.eq(stock_id))
            .filter(inventory_stock::Column::Version.eq(expected_version))
            .exec(txn)
            .await?;
        if update_result.rows_affected == 0 {
            tracing::error!("Transaction rolled back: 产品 {} 并发冲突", product_id);
            txn.rollback().await?;
            return Err(AppError::business(format!(
                "产品 {} 库存记录已被其他用户修改，请重试",
                product_id
            )));
        }
        Ok(())
    }

    /// 处理目标仓库无库存记录的接收：新建库存 + 写流水 + 收集事件。
    async fn apply_receive_new_stock(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        source_stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
        item: inventory_transfer_item::Model,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        // v15 批次 42 修复：复用循环外批量查询的 source_stock_map，避免循环内逐个查询（N+1）
        let source_stock = source_stock_map.get(&item.product_id).cloned();

        let batch_no = source_stock
            .as_ref()
            .map(|s| s.batch_no.clone())
            .unwrap_or_default();
        let color_no = source_stock
            .as_ref()
            .map(|s| s.color_no.clone())
            .unwrap_or_default();
        let dye_lot_no = source_stock.as_ref().and_then(|s| s.dye_lot_no.clone());
        let grade = source_stock
            .as_ref()
            .map(|s| s.grade.clone())
            .unwrap_or_else(|| "一等品".to_string());
        let gram_weight = source_stock.as_ref().and_then(|s| s.gram_weight);
        let width = source_stock.as_ref().and_then(|s| s.width);
        let production_date = source_stock.as_ref().and_then(|s| s.production_date);
        let expiry_date = source_stock.as_ref().and_then(|s| s.expiry_date);
        let source_kg_per_meter = source_stock
            .as_ref()
            .map(Self::compute_source_kg_per_meter)
            .unwrap_or(rust_decimal::Decimal::ZERO);

        let new_stock = Self::build_new_stock_active_model(
            transfer.to_warehouse_id,
            &item,
            NewStockFabricFields {
                batch_no: &batch_no,
                color_no: &color_no,
                dye_lot_no: dye_lot_no.as_deref(),
                grade: &grade,
                gram_weight,
                width,
                production_date,
                expiry_date,
                source_kg_per_meter,
            },
        );
        new_stock.insert(txn).await?;

        let transaction = Self::build_transfer_in_transaction(TransferInTxnFields {
            product_id: item.product_id,
            warehouse_id: transfer.to_warehouse_id,
            batch_no: &batch_no,
            color_no: &color_no,
            dye_lot_no: dye_lot_no.as_deref(),
            grade: &grade,
            quantity_meters: item.quantity,
            quantity_kg: (item.quantity * source_kg_per_meter).round_dp(4),
            quantity_before_meters: Some(rust_decimal::Decimal::ZERO),
            quantity_before_kg: Some(rust_decimal::Decimal::ZERO),
            quantity_after_meters: Some(item.quantity),
            quantity_after_kg: Some((item.quantity * source_kg_per_meter).round_dp(4)),
            notes: &format!("调拨入库（新建库存） - 调拨单号: {}", transfer.transfer_no),
            created_by: transfer.created_by,
            transfer_id,
            transfer_no: &transfer.transfer_no,
        });
        let inserted = transaction.insert(txn).await?;
        pending_events.push(Self::build_inventory_transaction_created_event(&inserted));
        Ok(())
    }

    /// 构造新建库存的 ActiveModel（面料行业字段从源仓库复制）。
    fn build_new_stock_active_model(
        warehouse_id: i32,
        item: &inventory_transfer_item::Model,
        fields: NewStockFabricFields<'_>,
    ) -> inventory_stock::ActiveModel {
        inventory_stock::ActiveModel {
            id: Default::default(),
            warehouse_id: sea_orm::ActiveValue::Set(warehouse_id),
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
            batch_no: sea_orm::ActiveValue::Set(fields.batch_no.to_string()),
            color_no: sea_orm::ActiveValue::Set(fields.color_no.to_string()),
            dye_lot_no: sea_orm::ActiveValue::Set(fields.dye_lot_no.map(|s| s.to_string())),
            grade: sea_orm::ActiveValue::Set(fields.grade.to_string()),
            production_date: sea_orm::ActiveValue::Set(fields.production_date),
            expiry_date: sea_orm::ActiveValue::Set(fields.expiry_date),
            quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
            quantity_kg: sea_orm::ActiveValue::Set(
                (item.quantity * fields.source_kg_per_meter).round_dp(4),
            ),
            gram_weight: sea_orm::ActiveValue::Set(fields.gram_weight),
            width: sea_orm::ActiveValue::Set(fields.width),
            location_id: sea_orm::ActiveValue::NotSet,
            shelf_no: sea_orm::ActiveValue::NotSet,
            layer_no: sea_orm::ActiveValue::NotSet,
            bin_location: sea_orm::ActiveValue::NotSet,
            stock_status: sea_orm::ActiveValue::Set("正常".to_string()),
            quality_status: sea_orm::ActiveValue::Set("合格".to_string()),
            version: sea_orm::ActiveValue::Set(0),
            quantity_shipped: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
        }
    }

    /// 构造 TRANSFER_IN 库存流水 ActiveModel（existing/new 两条路径共用）。
    fn build_transfer_in_transaction(
        f: TransferInTxnFields<'_>,
    ) -> inventory_transaction::ActiveModel {
        inventory_transaction::ActiveModel {
            id: sea_orm::ActiveValue::Set(0),
            transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
            product_id: sea_orm::ActiveValue::Set(f.product_id),
            warehouse_id: sea_orm::ActiveValue::Set(f.warehouse_id),
            batch_no: sea_orm::ActiveValue::Set(f.batch_no.to_string()),
            color_no: sea_orm::ActiveValue::Set(f.color_no.to_string()),
            dye_lot_no: sea_orm::ActiveValue::Set(f.dye_lot_no.map(|s| s.to_string())),
            grade: sea_orm::ActiveValue::Set(f.grade.to_string()),
            quantity_meters: sea_orm::ActiveValue::Set(f.quantity_meters),
            quantity_kg: sea_orm::ActiveValue::Set(f.quantity_kg),
            source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
            source_bill_no: sea_orm::ActiveValue::Set(Some(f.transfer_no.to_string())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(f.transfer_id)),
            quantity_before_meters: sea_orm::ActiveValue::Set(f.quantity_before_meters),
            quantity_before_kg: sea_orm::ActiveValue::Set(f.quantity_before_kg),
            quantity_after_meters: sea_orm::ActiveValue::Set(f.quantity_after_meters),
            quantity_after_kg: sea_orm::ActiveValue::Set(f.quantity_after_kg),
            notes: sea_orm::ActiveValue::Set(Some(f.notes.to_string())),
            created_by: sea_orm::ActiveValue::Set(f.created_by),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        }
    }

    /// 从已插入的库存流水 Model 构造 InventoryTransactionCreated 事件。
    fn build_inventory_transaction_created_event(
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

    /// 更新调拨明细项的 received_quantity 为本次接收数量（带审计）。
    async fn update_item_received_quantity(
        txn: &sea_orm::DatabaseTransaction,
        item: inventory_transfer_item::Model,
        received_quantity: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let mut item_update: inventory_transfer_item::ActiveModel = item.into();
        item_update.received_quantity = sea_orm::ActiveValue::Set(received_quantity);
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

    /// 更新调拨单状态为 completed（带审计）。
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

    /// commit 成功后统一发布 pending_events（避免回滚造成幻事件）。
    fn publish_receive_events(
        pending_events: Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) {
        let events_count = pending_events.len();
        for event in pending_events {
            crate::services::event_bus::EVENT_BUS.publish(event);
        }
        if events_count > 0 {
            tracing::info!(
                transfer_id,
                events_count,
                "调拨入库完成，已发布 InventoryTransactionCreated 事件触发财务凭证生成"
            );
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
