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
    TransactionTrait,
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

impl InventoryTransferService {
    /// 发出库存调拨
    pub async fn ship_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有已审核的调拨单可以发出
        if transfer.status != "approved" {
            return Err(AppError::business(
                "只有已审核状态的调拨单可以发出".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

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
                let new_quantity_kg = if quantity_meters > rust_decimal::Decimal::ZERO {
                    quantity_kg - (quantity_kg * item.quantity / quantity_meters)
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
                        Expr::val(new_quantity_meters),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityKg,
                        Expr::val(new_quantity_kg),
                    )
                    .col_expr(
                        inventory_stock::Column::Version,
                        Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()),
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
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
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
                transaction.insert(&txn).await?;

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

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 接收库存调拨
    pub async fn receive_transfer(
        &self,
        transfer_id: i32,
    ) -> Result<InventoryTransferDetail, AppError> {
        // 检查调拨单是否存在
        let transfer = InventoryTransferEntity::find_by_id(transfer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("库存调拨单 {} 未找到", transfer_id)))?;

        // 检查状态，只有已发出的调拨单可以接收
        if transfer.status != "shipped" {
            return Err(AppError::business(
                "只有已发出状态的调拨单可以接收".to_string(),
            ));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 获取调拨明细项
        let items = InventoryTransferItemEntity::find()
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .all(&txn)
            .await?;

        // 批量获取目标仓库库存记录（优化N+1查询）
        let product_ids: Vec<i32> = items.iter().map(|item| item.product_id).collect();
        let stocks = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(&txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.product_id, s)).collect();

        // 增加目标仓库库存
        for item in items {
            // 查找目标仓库库存记录
            let stock = stock_map.get(&item.product_id);

            if let Some(stock_model) = stock {
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

                // 增加库存（带乐观锁）
                let new_quantity_meters = quantity_meters + item.quantity;
                // Calculate kg increase proportionally based on source stock
                let source_stock = InventoryStockEntity::find()
                    .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                    .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                    .one(&txn)
                    .await?;
                let source_kg_per_meter = if let Some(ref src) = source_stock {
                    if src.quantity_meters > rust_decimal::Decimal::ZERO {
                        src.quantity_kg / src.quantity_meters
                    } else {
                        rust_decimal::Decimal::ZERO
                    }
                } else {
                    rust_decimal::Decimal::ZERO
                };
                let new_quantity_kg = quantity_kg + (item.quantity * source_kg_per_meter);

                // 使用乐观锁条件更新：只有 version 匹配时才更新
                let update_result = inventory_stock::Entity::update_many()
                    .col_expr(
                        inventory_stock::Column::QuantityOnHand,
                        Expr::col(inventory_stock::Column::QuantityOnHand).binary(BinOper::Add, Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        Expr::col(inventory_stock::Column::QuantityAvailable).binary(BinOper::Add, Expr::val(item.quantity)),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityMeters,
                        Expr::val(new_quantity_meters),
                    )
                    .col_expr(
                        inventory_stock::Column::QuantityKg,
                        Expr::val(new_quantity_kg),
                    )
                    .col_expr(
                        inventory_stock::Column::Version,
                        Expr::col(inventory_stock::Column::Version).binary(BinOper::Add, Expr::val(1)),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()),
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

                // 记录 TRANSFER_IN 库存流水
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(quantity_meters)),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(quantity_kg)),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_quantity_meters)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_quantity_kg)),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨入库 - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;

                // 更新明细项已接收数量
                let item_quantity = item.quantity;
                let mut item_update: inventory_transfer_item::ActiveModel = item.into();
                item_update.received_quantity = sea_orm::ActiveValue::Set(item_quantity);
                item_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &txn,
                    "auto_audit",
                    item_update,
                    Some(0),
                )
                .await?;
            } else {
                // 如果目标仓库没有库存记录，创建新记录
                // 需要从源仓库的库存记录中获取面料行业字段
                let source_stock = InventoryStockEntity::find()
                    .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                    .filter(inventory_stock::Column::ProductId.eq(item.product_id))
                    .one(&txn)
                    .await?;

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

                // 计算源仓库的公斤/米比率
                let source_kg_per_meter = if let Some(ref src) = source_stock {
                    if src.quantity_meters > rust_decimal::Decimal::ZERO {
                        src.quantity_kg / src.quantity_meters
                    } else {
                        rust_decimal::Decimal::ZERO
                    }
                } else {
                    rust_decimal::Decimal::ZERO
                };

                let new_stock = inventory_stock::ActiveModel {
                    id: Default::default(),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    quantity_on_hand: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_available: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_reserved: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    quantity_incoming: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_point: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    reorder_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
                    last_count_date: sea_orm::ActiveValue::NotSet,
                    last_movement_date: sea_orm::ActiveValue::NotSet,
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                    // 面料行业特色字段 - 从源仓库库存复制
                    batch_no: sea_orm::ActiveValue::Set(batch_no.clone()),
                    color_no: sea_orm::ActiveValue::Set(color_no.clone()),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no.clone()),
                    grade: sea_orm::ActiveValue::Set(grade.clone()),
                    production_date: sea_orm::ActiveValue::Set(production_date),
                    expiry_date: sea_orm::ActiveValue::Set(expiry_date),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(item.quantity * source_kg_per_meter),
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
                new_stock.insert(&txn).await?;

                // 记录 TRANSFER_IN 库存流水（新建库存记录的情况）
                let transaction = inventory_transaction::ActiveModel {
                    id: sea_orm::ActiveValue::Set(0),
                    transaction_type: sea_orm::ActiveValue::Set("TRANSFER_IN".to_string()),
                    product_id: sea_orm::ActiveValue::Set(item.product_id),
                    warehouse_id: sea_orm::ActiveValue::Set(transfer.to_warehouse_id),
                    batch_no: sea_orm::ActiveValue::Set(batch_no),
                    color_no: sea_orm::ActiveValue::Set(color_no),
                    dye_lot_no: sea_orm::ActiveValue::Set(dye_lot_no),
                    grade: sea_orm::ActiveValue::Set(grade),
                    quantity_meters: sea_orm::ActiveValue::Set(item.quantity),
                    quantity_kg: sea_orm::ActiveValue::Set(item.quantity * source_kg_per_meter),
                    source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
                    source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
                    source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
                    quantity_before_meters: sea_orm::ActiveValue::Set(Some(
                        rust_decimal::Decimal::ZERO,
                    )),
                    quantity_before_kg: sea_orm::ActiveValue::Set(Some(
                        rust_decimal::Decimal::ZERO,
                    )),
                    quantity_after_meters: sea_orm::ActiveValue::Set(Some(item.quantity)),
                    quantity_after_kg: sea_orm::ActiveValue::Set(Some(
                        item.quantity * source_kg_per_meter,
                    )),
                    notes: sea_orm::ActiveValue::Set(Some(format!(
                        "调拨入库（新建库存） - 调拨单号: {}",
                        transfer.transfer_no
                    ))),
                    created_by: sea_orm::ActiveValue::Set(transfer.created_by),
                    created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                };
                transaction.insert(&txn).await?;
            }
        }

        // 更新调拨单状态
        let mut transfer_update: inventory_transfer::ActiveModel = transfer.into();
        transfer_update.status = sea_orm::ActiveValue::Set("completed".to_string());
        transfer_update.received_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
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

        // 返回调拨详情
        self.get_transfer_detail(transfer_id).await
    }

    /// 列出调拨单的所有明细项
    pub async fn list_items(
        &self,
        transfer_id: i32,
    ) -> Result<Vec<InventoryTransferItemDetail>, AppError> {
        let _ = self.get_transfer_detail(transfer_id).await?;
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
