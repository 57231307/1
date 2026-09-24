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
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, IntoActiveModel, JoinType, Order,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, TransactionTrait,
};

use crate::models::inventory_stock::{self, Entity as InventoryStockEntity};
use crate::models::inventory_transaction;
use crate::models::inventory_transfer::{self, Entity as InventoryTransferEntity};
use crate::models::inventory_transfer_item::{self, Entity as InventoryTransferItemEntity};
use crate::models::product;
use crate::models::status::purchase_inventory::inventory_stock_grade;
use crate::models::status::purchase_inventory::inventory_stock_quality_status as quality_status;
use crate::models::status::purchase_inventory::inventory_stock_status;
use crate::models::status::purchase_inventory::inventory_transfer as transfer_status;
use crate::services::inventory_deduction::{
    AllocationSource, DeductionCandidate, DeductionError, plan_deduction,
    require_outbound_dimensions,
};
use crate::utils::error::AppError;

use super::fabric_class::{self, FabricTrace};
use super::{
    InventoryTransferDetail, InventoryTransferItemDetail, InventoryTransferItemRequest,
    InventoryTransferService,
};

/// 调拨明细面料行业追溯字段的校验与归一化委托给全仓唯一实现
/// [`fabric_class::validate_fabric_trace`]（款号由 product_id 承载，此处含色号/缸号/批次）。
///
/// 三字段与 `inventory_transfer_item::ActiveModel` 对应列一一对应，供 `add_item` 落库与单元测试断言共用。
type TransferTraceFields = FabricTrace;

/// 库存四维定位键：款号(product_id) + 色号(color_no) + 缸号(dye_lot_no) + 批次(batch_no)。
/// 与出库侧 `require_outbound_dimensions` 的精确扣减口径对称——调拨入库定位/新建目标库存行
/// 必须用同一组四维，不能退化为只按 product_id 匹配。
type StockDimKey = (i32, String, Option<String>, String);

/// 新建库存的面料行业追溯字段（从源仓库复制，封装避免参数过多）。
struct NewStockFabricFields<'a> {
    batch_no: &'a str,
    color_no: &'a str,
    dye_lot_no: Option<&'a str>,
    grade: &'a str,
    gram_weight: Option<rust_decimal::Decimal>,
    width: Option<rust_decimal::Decimal>,
    // 与 inventory_stock::Model 字段类型保持一致（DateTime<Utc>），避免类型转换
    production_date: Option<chrono::DateTime<chrono::Utc>>,
    expiry_date: Option<chrono::DateTime<chrono::Utc>>,
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
        let txn = (*self.db).begin().await?;
        let mut pending_events: Vec<crate::services::event_bus::BusinessEvent> = Vec::new();
        let transfer = Self::lock_and_validate_transfer_for_ship(&txn, transfer_id).await?;
        let items = Self::load_transfer_items(&txn, transfer_id).await?;
        for item in items {
            Self::apply_ship_item_deduction(
                &txn,
                &transfer,
                item,
                &mut pending_events,
                transfer_id,
            )
            .await?;
        }
        Self::update_transfer_to_shipped(&txn, transfer).await?;
        txn.commit().await?;
        Self::publish_ship_events(pending_events, transfer_id);
        self.get_transfer_detail(transfer_id, None).await
    }

    /// 锁定调拨单并校验状态为 approved（串行化并发状态变更）。
    async fn lock_and_validate_transfer_for_ship(
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

    /// 处理单个调拨明细项的库存扣减：四维校验→四维候选查询（源仓+款号+色号+批次）→
    /// 指定缸精确扣、不足时显式跨缸回退（确定性次序）→逐行扣减→逐行 TRANSFER_OUT 流水→事件→更新明细。
    ///
    /// 用户拍板规则（不做兜底）：调拨明细必须显式携带色号/缸号/批次；缺维度或四维组合
    /// （含可回退其他缸）无库存时报业务错误；每笔实际扣到的缸号/批次如实写入流水（跨缸回退留痕）。
    /// batch-18 P2-6：扣减源仓库后，同步增加目标仓库的 quantity_incoming（在途库存）
    async fn apply_ship_item_deduction(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: inventory_transfer_item::Model,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        let dims = require_outbound_dimensions(
            "调拨出库明细",
            item.product_id,
            Some(item.color_no.as_str()),
            item.dye_lot_no.as_deref(),
            Some(item.batch_no.as_str()),
        )?;
        // 四维候选：源仓 + 款号 + 色号 + 批次（缸号不过滤——跨缸回退允许扣其他缸，
        // 缸号维度取舍由 plan_deduction 统一负责）；FOR UPDATE 行锁防并发超扣
        let candidates = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
            .filter(inventory_stock::Column::ProductId.eq(item.product_id))
            .filter(inventory_stock::Column::ColorNo.eq(&dims.color_no))
            .filter(inventory_stock::Column::BatchNo.eq(&dims.batch_no))
            .lock_exclusive()
            .all(txn)
            .await?;
        if candidates.is_empty() {
            return Err(Self::no_stock_error(
                &transfer.transfer_no,
                item.product_id,
                &dims,
            ));
        }
        let plan_candidates: Vec<DeductionCandidate> = candidates
            .iter()
            .map(|s| DeductionCandidate {
                stock_id: s.id,
                dye_lot_no: s.dye_lot_no.clone(),
                quantity_available: s.quantity_available,
                created_at: s.created_at,
            })
            .collect();
        let allocations =
            plan_deduction(&plan_candidates, dims.dye_lot_no.as_deref(), item.quantity).map_err(
                |e| match e {
                    DeductionError::NoStockAtAll => {
                        Self::no_stock_error(&transfer.transfer_no, item.product_id, &dims)
                    }
                    DeductionError::Insufficient {
                        available_total,
                        required,
                    } => Self::insufficient_error(
                        &transfer.transfer_no,
                        item.product_id,
                        &dims,
                        available_total,
                        required,
                    ),
                },
            )?;

        for alloc in allocations {
            let stock_model = candidates
                .iter()
                .find(|s| s.id == alloc.stock_id)
                .ok_or_else(|| AppError::internal("扣减规划返回了候选之外的库存行"))?;
            let (new_quantity_meters, new_quantity_kg) =
                Self::compute_ship_new_quantities(stock_model, alloc.quantity);
            Self::update_stock_with_optimistic_lock_for_ship(
                txn,
                stock_model.id,
                stock_model.version,
                alloc.quantity,
                new_quantity_meters,
                new_quantity_kg,
                item.product_id,
            )
            .await?;

            // batch-18 P2-6：增加目标仓库的 quantity_incoming（在途库存）
            Self::update_target_warehouse_incoming(
                txn,
                transfer.to_warehouse_id,
                item.product_id,
                alloc.quantity,
            )
            .await?;

            let inserted = Self::build_and_insert_transfer_out_transaction(
                txn,
                transfer,
                &item,
                stock_model,
                alloc.quantity,
                new_quantity_meters,
                new_quantity_kg,
                dims.dye_lot_no.as_deref(),
                alloc.source == AllocationSource::CrossDyeLot,
                transfer_id,
            )
            .await?;
            pending_events.push(Self::build_inventory_transaction_created_event(&inserted));
        }
        Self::update_item_shipped_quantity(txn, item).await
    }

    /// 四维组合在源仓完全无库存的业务错误（不做兜底）
    fn no_stock_error(
        transfer_no: &str,
        product_id: i32,
        dims: &crate::services::inventory_deduction::OutboundDimensions,
    ) -> AppError {
        // 白坯/染色如实呈现：白坯色号为空、无缸号；染色带色号+缸号
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）"
        } else {
            dims.color_no.as_str()
        };
        let dims_disp = match dims.dye_lot_no.as_deref() {
            Some(lot) => format!(
                "色号 {} + 缸号 {} + 批次 {}",
                color_disp, lot, dims.batch_no
            ),
            None => format!("色号 {} + 批次 {}（白坯免缸号）", color_disp, dims.batch_no),
        };
        AppError::business(format!(
            "调拨单 {}：款号（产品 {}）+ {} 在源仓库无任何库存记录，出库被拒绝（不回退到产品+色号扣减）",
            transfer_no, product_id, dims_disp
        ))
    }

    /// 四维口径（含跨缸回退范围）库存不足的业务错误
    fn insufficient_error(
        transfer_no: &str,
        product_id: i32,
        dims: &crate::services::inventory_deduction::OutboundDimensions,
        available_total: rust_decimal::Decimal,
        required: rust_decimal::Decimal,
    ) -> AppError {
        let color_disp = if dims.color_no.is_empty() {
            "白坯（无颜色）"
        } else {
            dims.color_no.as_str()
        };
        let reason = match dims.dye_lot_no.as_deref() {
            // 染色：指定缸不足 + 可回退其他缸仍不足
            Some(lot) => format!(
                "款号（产品 {}）+ 色号 {} + 批次 {} 源仓可用库存合计 {}（含跨缸回退的其他缸）小于出库数量 {}，指定缸号 {} 数量不足且其他缸亦不足以补足",
                product_id, color_disp, dims.batch_no, available_total, required, lot
            ),
            // 白坯：无缸号维度，仅无缸号行可扣且仍不足
            None => format!(
                "款号（产品 {}）+ 色号 {} + 批次 {} 源仓无缸号库存合计 {} 小于出库数量 {}（白坯不做跨缸回退）",
                product_id, color_disp, dims.batch_no, available_total, required
            ),
        };
        AppError::business(format!("调拨单 {}：{}", transfer_no, reason))
    }

    /// batch-18 P2-6：更新目标仓库的 quantity_incoming（在途库存）
    async fn update_target_warehouse_incoming(
        txn: &sea_orm::DatabaseTransaction,
        to_warehouse_id: i32,
        product_id: i32,
        quantity: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        // 查找目标仓库的库存记录
        let target_stock = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .one(txn)
            .await?;

        if let Some(stock) = target_stock {
            // 更新 quantity_incoming
            let update_result = inventory_stock::Entity::update_many()
                .col_expr(
                    inventory_stock::Column::QuantityIncoming,
                    Expr::col(inventory_stock::Column::QuantityIncoming)
                        .binary(BinOper::Add, Expr::val(quantity)),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                )
                .filter(inventory_stock::Column::Id.eq(stock.id))
                .exec(txn)
                .await?;

            if update_result.rows_affected == 0 {
                tracing::warn!(
                    "更新目标仓库在途库存失败：产品 {} 仓库 {}",
                    product_id,
                    to_warehouse_id
                );
            }
        } else {
            // 目标仓库无库存记录，记录警告（不创建新记录，避免字段不完整）
            tracing::warn!(
                "目标仓库无库存记录，跳过在途库存更新：产品 {} 仓库 {}",
                product_id,
                to_warehouse_id
            );
        }

        Ok(())
    }

    /// 计算扣减后的新 quantity_meters 和 quantity_kg（按比例扣减 kg，round_dp(4) 防精度漂移）。
    fn compute_ship_new_quantities(
        stock_model: &inventory_stock::Model,
        item_quantity: rust_decimal::Decimal,
    ) -> (rust_decimal::Decimal, rust_decimal::Decimal) {
        let new_quantity_meters = stock_model.quantity_meters - item_quantity;
        let new_quantity_kg = if stock_model.quantity_meters > rust_decimal::Decimal::ZERO {
            (stock_model.quantity_kg
                - (stock_model.quantity_kg * item_quantity / stock_model.quantity_meters))
                .round_dp(4)
        } else {
            stock_model.quantity_kg
        };
        (new_quantity_meters, new_quantity_kg)
    }

    /// 乐观锁扣减库存：只有 version 匹配时才扣减（rows_affected=0 报并发冲突）。
    async fn update_stock_with_optimistic_lock_for_ship(
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
                    .binary(BinOper::Sub, Expr::val(item_quantity)),
            )
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                Expr::col(inventory_stock::Column::QuantityAvailable)
                    .binary(BinOper::Sub, Expr::val(item_quantity)),
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
            .exec(txn)
            .await?;
        Self::ensure_rows_affected(update_result.rows_affected, product_id)?;
        Ok(())
    }

    /// 校验乐观锁更新影响行数（0 行=并发冲突）
    fn ensure_rows_affected(rows: u64, product_id: i32) -> Result<(), AppError> {
        if rows == 0 {
            tracing::error!(
                "Transaction will rollback on drop: 产品 {} 并发冲突",
                product_id
            );
            return Err(AppError::business(format!(
                "产品 {} 库存记录已被其他用户修改，请重试",
                product_id
            )));
        }
        Ok(())
    }

    /// 构造并插入 TRANSFER_OUT 库存流水（记录扣减前后的米/kg 与源单据信息）。
    ///
    /// 流水如实记录**实际被扣库存行**的缸号/批次（来自 stock_model 行本身）；
    /// 染色布跨缸回退时备注写明"指定缸号 X 不足，实扣缸号 Y"，不允许静默换缸；
    /// 白坯布（`requested_dye_lot` 为 None）不做跨缸回退，`is_cross_dye_lot` 恒 false。
    #[allow(clippy::too_many_arguments)]
    async fn build_and_insert_transfer_out_transaction(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        item: &inventory_transfer_item::Model,
        stock_model: &inventory_stock::Model,
        deduct_quantity: rust_decimal::Decimal,
        new_quantity_meters: rust_decimal::Decimal,
        new_quantity_kg: rust_decimal::Decimal,
        requested_dye_lot: Option<&str>,
        is_cross_dye_lot: bool,
        transfer_id: i32,
    ) -> Result<inventory_transaction::Model, AppError> {
        let cross_note = if is_cross_dye_lot {
            format!(
                "（跨缸回退：指定缸号 {} 数量不足，本笔实扣缸号 {}）",
                requested_dye_lot.unwrap_or_default(),
                stock_model.dye_lot_no.clone().unwrap_or_default()
            )
        } else {
            String::new()
        };
        let transaction = inventory_transaction::ActiveModel {
            id: Default::default(),
            transaction_type: sea_orm::ActiveValue::Set("TRANSFER_OUT".to_string()),
            product_id: sea_orm::ActiveValue::Set(item.product_id),
            warehouse_id: sea_orm::ActiveValue::Set(transfer.from_warehouse_id),
            batch_no: sea_orm::ActiveValue::Set(stock_model.batch_no.clone()),
            color_no: sea_orm::ActiveValue::Set(stock_model.color_no.clone()),
            dye_lot_no: sea_orm::ActiveValue::Set(stock_model.dye_lot_no.clone()),
            grade: sea_orm::ActiveValue::Set(stock_model.grade.clone()),
            quantity_meters: sea_orm::ActiveValue::Set(deduct_quantity),
            quantity_kg: sea_orm::ActiveValue::Set(stock_model.quantity_kg - new_quantity_kg),
            source_bill_type: sea_orm::ActiveValue::Set(Some("TRANSFER".to_string())),
            source_bill_no: sea_orm::ActiveValue::Set(Some(transfer.transfer_no.clone())),
            source_bill_id: sea_orm::ActiveValue::Set(Some(transfer_id)),
            quantity_before_meters: sea_orm::ActiveValue::Set(Some(stock_model.quantity_meters)),
            quantity_before_kg: sea_orm::ActiveValue::Set(Some(stock_model.quantity_kg)),
            quantity_after_meters: sea_orm::ActiveValue::Set(Some(new_quantity_meters)),
            quantity_after_kg: sea_orm::ActiveValue::Set(Some(new_quantity_kg)),
            notes: sea_orm::ActiveValue::Set(Some(format!(
                "调拨出库 - 调拨单号: {}{}",
                transfer.transfer_no, cross_note
            ))),
            created_by: sea_orm::ActiveValue::Set(transfer.created_by),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };
        Ok(transaction.insert(txn).await?)
    }

    /// 更新调拨明细项的 shipped_quantity 为本次发出数量（带审计）。
    async fn update_item_shipped_quantity(
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

    /// 更新调拨单状态为 shipped 并设置 shipped_at（带审计）。
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

    /// commit 成功后统一发布 pending_events（避免回滚造成幻事件）。
    fn publish_ship_events(
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
                "调拨出库完成，已发布 InventoryTransactionCreated 事件触发财务凭证生成"
            );
        }
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
        // 与出库侧对称：目标仓库存按 (款号+色号+缸号+批次) 四维定位/落行，绝不按 product_id 单键匹配。
        // 出库按四维精确扣减（apply_ship_item_deduction / require_outbound_dimensions），入库若不落回
        // 同一四维组合，源仓被扣掉的缸/批在目标仓就查不到——即本用例暴露的"入库后按四维查询为空"缺陷。
        let (stock_map, source_stock_map) =
            Self::load_receive_stock_maps(&txn, &transfer, &items).await?;

        for item in items {
            let key = Self::stock_dim_key(
                item.product_id,
                &item.color_no,
                item.dye_lot_no.as_deref(),
                &item.batch_no,
            );
            if stock_map.contains_key(&key) {
                Self::apply_receive_existing_stock(
                    &txn,
                    &transfer,
                    &stock_map,
                    &source_stock_map,
                    key,
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
                    key,
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

    /// 构造库存四维定位键（款号+色号+缸号+批次），与 `inventory_stock::Model` 对应列口径一致。
    fn stock_dim_key(
        product_id: i32,
        color_no: &str,
        dye_lot_no: Option<&str>,
        batch_no: &str,
    ) -> StockDimKey {
        (
            product_id,
            color_no.to_string(),
            dye_lot_no.map(|s| s.to_string()),
            batch_no.to_string(),
        )
    }

    /// 从库存行提取四维定位键。
    fn stock_dim_key_of(model: &inventory_stock::Model) -> StockDimKey {
        Self::stock_dim_key(
            model.product_id,
            &model.color_no,
            model.dye_lot_no.as_deref(),
            &model.batch_no,
        )
    }

    /// 批量加载目标仓库与源仓库的库存记录（避免循环内 N+1 查询），按四维键索引。
    async fn load_receive_stock_maps(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        items: &[inventory_transfer_item::Model],
    ) -> Result<
        (
            std::collections::HashMap<StockDimKey, inventory_stock::Model>,
            std::collections::HashMap<StockDimKey, inventory_stock::Model>,
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
        let stock_map: std::collections::HashMap<StockDimKey, inventory_stock::Model> = stocks
            .into_iter()
            .map(|s| (Self::stock_dim_key_of(&s), s))
            .collect();

        let source_stocks = if product_ids.is_empty() {
            Vec::new()
        } else {
            InventoryStockEntity::find()
                .filter(inventory_stock::Column::WarehouseId.eq(transfer.from_warehouse_id))
                .filter(inventory_stock::Column::ProductId.is_in(product_ids))
                .all(txn)
                .await?
        };
        let source_stock_map: std::collections::HashMap<StockDimKey, inventory_stock::Model> =
            source_stocks
                .into_iter()
                .map(|s| (Self::stock_dim_key_of(&s), s))
                .collect();
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
    /// batch-18 P2-6：接收时同步扣减目标仓库的 quantity_incoming（在途转实收）
    async fn apply_receive_existing_stock(
        txn: &sea_orm::DatabaseTransaction,
        transfer: &inventory_transfer::Model,
        stock_map: &std::collections::HashMap<StockDimKey, inventory_stock::Model>,
        source_stock_map: &std::collections::HashMap<StockDimKey, inventory_stock::Model>,
        key: StockDimKey,
        item: inventory_transfer_item::Model,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        let stock_model = stock_map
            .get(&key)
            .ok_or_else(|| AppError::business(format!("产品 {} 库存记录缺失", item.product_id)))?;
        let (quantity_meters, quantity_kg, expected_version) = (
            stock_model.quantity_meters,
            stock_model.quantity_kg,
            stock_model.version,
        );
        let batch_no = stock_model.batch_no.clone();
        let color_no = stock_model.color_no.clone();
        let dye_lot_no = stock_model.dye_lot_no.clone();
        let grade = stock_model.grade.clone();

        let new_quantity_meters = quantity_meters + item.quantity;
        let source_kg_per_meter = source_stock_map
            .get(&key)
            .map(Self::compute_source_kg_per_meter)
            .unwrap_or(rust_decimal::Decimal::ZERO);
        // 批次 97 P1-12 修复（v5 复审）：kg 计算补 round_dp(4) 防止精度漂移
        let new_quantity_kg = (quantity_kg + (item.quantity * source_kg_per_meter)).round_dp(4);

        Self::update_existing_stock_with_optimistic_lock(
            txn,
            stock_model.id,
            expected_version,
            item.quantity,
            new_quantity_meters,
            new_quantity_kg,
            item.product_id,
        )
        .await?;

        // batch-18 P2-6：扣减目标仓库的 quantity_incoming（在途转实收）
        Self::deduct_target_warehouse_incoming(
            txn,
            transfer.to_warehouse_id,
            item.product_id,
            item.quantity,
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
        // 先提取 received_quantity 再 move item，避免 use of moved value
        let received_quantity = item.quantity;
        Self::update_item_received_quantity(txn, item, received_quantity).await?;
        Ok(())
    }

    /// batch-18 P2-6：扣减目标仓库的 quantity_incoming（在途转实收）
    async fn deduct_target_warehouse_incoming(
        txn: &sea_orm::DatabaseTransaction,
        to_warehouse_id: i32,
        product_id: i32,
        quantity: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let target_stock = InventoryStockEntity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(to_warehouse_id))
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .one(txn)
            .await?;

        if let Some(stock) = target_stock {
            let current_incoming = stock.quantity_incoming;
            let new_incoming = (current_incoming - quantity).max(rust_decimal::Decimal::ZERO);

            let update_result = inventory_stock::Entity::update_many()
                .col_expr(
                    inventory_stock::Column::QuantityIncoming,
                    sea_orm::sea_query::Expr::val(new_incoming),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()),
                )
                .filter(inventory_stock::Column::Id.eq(stock.id))
                .exec(txn)
                .await?;

            if update_result.rows_affected == 0 {
                tracing::warn!(
                    "扣减目标仓库在途库存失败：产品 {} 仓库 {}",
                    product_id,
                    to_warehouse_id
                );
            }
        }

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
            .exec(txn)
            .await?;
        if update_result.rows_affected == 0 {
            // 不在此处显式 rollback：txn 为共享引用，无法 take ownership。
            // 错误向上传播至 receive_transfer 主函数返回时，DatabaseTransaction drop 会自动回滚未提交事务。
            tracing::error!(
                "Transaction will rollback on drop: 产品 {} 并发冲突",
                product_id
            );
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
        source_stock_map: &std::collections::HashMap<StockDimKey, inventory_stock::Model>,
        key: StockDimKey,
        item: inventory_transfer_item::Model,
        pending_events: &mut Vec<crate::services::event_bus::BusinessEvent>,
        transfer_id: i32,
    ) -> Result<(), AppError> {
        // 四维追溯字段（色号/缸号/批次）以调拨明细项为唯一事实来源——出库即按这组四维扣的源仓行，
        // 入库必须在目标仓按同一组四维建行，否则源仓扣减的四维在目标仓查不到。
        let batch_no = item.batch_no.clone();
        let color_no = item.color_no.clone();
        let dye_lot_no = item.dye_lot_no.clone();
        // 面料物理属性（等级/克重/幅宽/日期/kg-per-米比率）复用同四维的源仓行派生（循环外批量查询避免 N+1）；
        // 源仓同四维行缺失时按纺织默认如实降级（等级=一等品、其余为空），不伪造四维。
        let s = source_stock_map.get(&key);
        let grade = s
            .map(|s| s.grade.clone())
            .unwrap_or_else(|| inventory_stock_grade::FIRST.to_string());
        let gram_weight = s.and_then(|s| s.gram_weight);
        let width = s.and_then(|s| s.width);
        let production_date = s.and_then(|s| s.production_date);
        let expiry_date = s.and_then(|s| s.expiry_date);
        let source_kg_per_meter = s
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
            stock_status: sea_orm::ActiveValue::Set(inventory_stock_status::NORMAL.to_string()),
            quality_status: sea_orm::ActiveValue::Set(quality_status::PASS.to_string()),
            version: sea_orm::ActiveValue::Set(0),
            quantity_shipped: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            replenishment_strategy: sea_orm::ActiveValue::Set("reorder_point".to_string()),
        }
    }

    /// 构造 TRANSFER_IN 库存流水 ActiveModel（existing/new 两条路径共用）。
    fn build_transfer_in_transaction(
        f: TransferInTxnFields<'_>,
    ) -> inventory_transaction::ActiveModel {
        inventory_transaction::ActiveModel {
            id: Default::default(),
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
        transfer_update.status = sea_orm::ActiveValue::Set(transfer_status::COMPLETED.to_string());
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
        // LEFT JOIN products 单次富化取 product_code/product_name/grade/unit（与详情明细口径一致，无逐行回查）
        let items = InventoryTransferItemEntity::find()
            .column_as(product::Column::Code, "product_code")
            .column_as(product::Column::Name, "product_name")
            .column_as(product::Column::ProductGrade, "grade")
            .column_as(product::Column::Unit, "unit")
            .join(
                JoinType::LeftJoin,
                inventory_transfer_item::Relation::Product.def(),
            )
            .filter(inventory_transfer_item::Column::TransferId.eq(transfer_id))
            .order_by(inventory_transfer_item::Column::Id, Order::Asc)
            .into_model::<InventoryTransferItemDetail>()
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 校验并归一化调拨明细的面料行业追溯字段（款号由 product_id 承载，此处含色号/缸号/批次）。
    ///
    /// 白坯/染色判定与缸号/批次必填口径的唯一实现是
    /// [`fabric_class::validate_fabric_trace`]，本方法仅委托之，避免多处判定漂移：
    /// - 色号为空 → 白坯布：免缸号（归一为 None），批次仍必填；
    /// - 色号非空 → 染色布：缸号、批次都必填，缺一返回明确业务错误；
    /// - 不以色号文本内容判定布种（带"白"字的色号是染色白色布）。
    fn validate_trace_fields(
        color_no: Option<String>,
        dye_lot_no: Option<String>,
        batch_no: Option<String>,
    ) -> Result<TransferTraceFields, AppError> {
        fabric_class::validate_fabric_trace(color_no, dye_lot_no, batch_no)
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

        if transfer.status == transfer_status::SHIPPED
            || transfer.status == transfer_status::COMPLETED
        {
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

        // 四维追溯字段（款号由 product_id 承载 + 色号 + 缸号 + 批次）如实校验并落库，
        // 禁止用 NotSet 丢弃入参（历史缺陷：本方法曾把三列写 NotSet → 落空值，调拨链路断链）。
        let trace = Self::validate_trace_fields(
            req.color_no.clone(),
            req.dye_lot_no.clone(),
            req.batch_no.clone(),
        )?;

        let item = inventory_transfer_item::ActiveModel {
            id: Default::default(),
            transfer_id: sea_orm::ActiveValue::Set(transfer_id),
            product_id: sea_orm::ActiveValue::Set(product_id),
            quantity: sea_orm::ActiveValue::Set(quantity),
            shipped_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            received_quantity: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ZERO),
            // unit_cost 是入参字段（mod.rs:93），建单批路径亦如实写入（inventory_move.rs:278），
            // 此处不得用 NotSet 丢弃（同"入参有值被丢"缺陷，列可空 → Set(Option) 直存）
            unit_cost: sea_orm::ActiveValue::Set(req.unit_cost),
            notes: sea_orm::ActiveValue::Set(req.notes),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            // v14 批次 417：面料行业追溯字段（T-P0-1），真实写入入参值（白坯布缸号为合法 NULL）
            color_no: sea_orm::ActiveValue::Set(trace.color_no),
            dye_lot_no: sea_orm::ActiveValue::Set(trace.dye_lot_no),
            batch_no: sea_orm::ActiveValue::Set(trace.batch_no),
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
            color_no: item_model.color_no,
            dye_lot_no: item_model.dye_lot_no,
            batch_no: item_model.batch_no,
            // 单品写入回显路径不做 products JOIN，产品名/编码/等级/单位如实回传为 None
            product_code: None,
            product_name: None,
            grade: None,
            unit: None,
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

        if transfer.status == transfer_status::SHIPPED
            || transfer.status == transfer_status::COMPLETED
        {
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
            color_no: updated.color_no,
            dye_lot_no: updated.dye_lot_no,
            batch_no: updated.batch_no,
            // 单品写入回显路径不做 products JOIN，产品名/编码/等级/单位如实回传为 None
            product_code: None,
            product_name: None,
            grade: None,
            unit: None,
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

        if transfer.status == transfer_status::SHIPPED
            || transfer.status == transfer_status::COMPLETED
        {
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

#[cfg(test)]
mod tests {
    use super::{InventoryTransferService, TransferTraceFields};
    use crate::models::inventory_transfer_item;
    use crate::utils::error::AppError;
    use sea_orm::ActiveValue;

    fn validate(
        color: Option<&str>,
        dye: Option<&str>,
        batch: Option<&str>,
    ) -> Result<TransferTraceFields, AppError> {
        InventoryTransferService::validate_trace_fields(
            color.map(str::to_string),
            dye.map(str::to_string),
            batch.map(str::to_string),
        )
    }

    #[test]
    fn four_dims_persisted_verbatim() {
        let f = validate(Some("C001"), Some("D9"), Some("B2026")).expect("四维应通过");
        assert_eq!(f.color_no, "C001");
        assert_eq!(f.batch_no, "B2026");
        assert_eq!(f.dye_lot_no.as_deref(), Some("D9"));
    }

    #[test]
    fn validated_fields_map_into_active_model_as_set() {
        // 复现 add_item 落库映射：三列必须是 Set(入参)，不得是 NotSet（历史丢列缺陷）
        let f = validate(Some("C001"), Some("D9"), Some("B2026")).unwrap();
        let am = inventory_transfer_item::ActiveModel {
            color_no: ActiveValue::Set(f.color_no.clone()),
            dye_lot_no: ActiveValue::Set(f.dye_lot_no.clone()),
            batch_no: ActiveValue::Set(f.batch_no.clone()),
            ..Default::default()
        };
        assert_eq!(am.color_no.unwrap(), "C001");
        assert_eq!(am.batch_no.unwrap(), "B2026");
        assert_eq!(am.dye_lot_no.unwrap(), Some("D9".to_string()));
    }

    #[test]
    fn values_are_trimmed() {
        let f = validate(Some(" C001 "), Some(" D9 "), Some(" B1 ")).unwrap();
        assert_eq!(f.color_no, "C001");
        assert_eq!(f.dye_lot_no.as_deref(), Some("D9"));
        assert_eq!(f.batch_no, "B1");
    }

    #[test]
    fn empty_color_no_is_greige_allows_null_dye_lot() {
        // 新口径：色号为空 = 白坯布，免缸号但批次必填（委托 fabric_class 单一实现）
        let f = validate(Some("   "), None, Some("B1")).expect("空色号应视为白坯布并允许免缸号");
        assert_eq!(f.color_no, "");
        assert_eq!(f.dye_lot_no, None);
        assert_eq!(f.batch_no, "B1");
    }

    #[test]
    fn missing_batch_no_rejected() {
        let err = validate(Some("C001"), Some("D9"), None).expect_err("缺批号必须报错");
        assert!(err.to_string().contains("批"), "实际: {}", err);
    }

    #[test]
    fn greige_missing_batch_no_rejected() {
        // 批次是入库批次，白坯布（空色号）亦必填
        let err = validate(None, None, Some("   ")).expect_err("白坯布缺批次必须报错");
        assert!(err.to_string().contains("批"), "实际: {}", err);
    }

    #[test]
    fn dyed_fabric_missing_dye_lot_rejected() {
        // 非空色号（染色布）缺缸号必须报明确业务错误，不得静默落空
        let err = validate(Some("C001"), None, Some("B1")).expect_err("染色布缺缸号必须报错");
        assert!(err.to_string().contains("缸号"), "实际: {}", err);
    }

    #[test]
    fn white_named_color_requires_dye_lot() {
        // 不再按名称判定：名字带"白"/WHITE 的色号是染色白色布，缺缸号必须报错
        let err =
            validate(Some("本白"), None, Some("B1")).expect_err("白色号是染色布，缺缸号必须报错");
        assert!(err.to_string().contains("缸号"), "实际: {}", err);
        let err2 = validate(Some("WHITE"), None, Some("B1"))
            .expect_err("WHITE 色号是染色布，缺缸号必须报错");
        assert!(err2.to_string().contains("缸号"), "实际: {}", err2);
    }
}
