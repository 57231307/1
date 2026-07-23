//! 销售发货-库存辅助子模块（delivery_ops/inventory）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L747-1082 迁移。
//! 包含 4 个库存辅助方法：
//! - check_inventory（库存充足性校验，批量查询消除 N+1）
//! - lock_inventory（锁定库存，创建预留记录）
//! - reduce_inventory（扣减库存，返回变更前后数量 + 色号/缸号）
//! - release_reservations（释放订单的库存预留记录）

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};

use crate::models::{inventory_reservation, inventory_stock};
use crate::models::status::inventory_reservation as reservation_status;
use crate::utils::error::AppError;

use super::super::delivery::ShipOrderItemRequest;
use super::super::order::SalesService;

impl SalesService {
    // ========== 库存辅助方法（私有） ==========

    /// 检查库存是否充足
    pub(crate) async fn check_inventory(
        &self,
        order_id: i32,
        items: &[ShipOrderItemRequest],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if items.is_empty() {
            return Ok(());
        }

        // v11 批次 38 修复：批量查询所有预留记录和库存记录，避免循环内逐个查询（N+1，最坏 2N 次查询）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();

        // 批量查询该订单所有 pending 预留记录，按 product_id 索引（取每组第一条，与原 .one() 语义一致）
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids.clone()))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?;
        let reservation_map: std::collections::HashMap<i32, &inventory_reservation::Model> =
            reservations
                .iter()
                .fold(std::collections::HashMap::new(), |mut acc, r| {
                    // 仅保留每个 product_id 的第一条（与原 .one() 语义一致）
                    acc.entry(r.product_id).or_insert(r);
                    acc
                });

        // 批量查询所有相关库存记录，按 product_id 索引
        let stocks = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.is_in(product_ids))
            .all(txn)
            .await?;
        let stock_map: std::collections::HashMap<i32, &inventory_stock::Model> =
            stocks.iter().map(|s| (s.product_id, s)).collect();

        for item in items {
            // 优先从预留记录查询
            if let Some(res) = reservation_map.get(&item.product_id) {
                if res.quantity < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 预留数量 {} 小于发货数量 {}",
                        item.product_id, res.quantity, item.quantity
                    )));
                }
                continue;
            }

            // 没有预留记录时直接查询库存
            match stock_map.get(&item.product_id) {
                Some(s) => {
                    if s.quantity_available < item.quantity {
                        return Err(AppError::business(format!(
                            "产品 {} 库存 {} 小于发货数量 {}",
                            item.product_id, s.quantity_available, item.quantity
                        )));
                    }
                }
                None => {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不存在",
                        item.product_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// 锁定库存（创建预留记录）
    pub(crate) async fn lock_inventory(
        &self,
        order_id: i32,
        items: &[super::super::SalesOrderItemRequest],
        user_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // v15 批次 42 修复：循环外批量查询该订单所有已存在的 pending 预留记录，
        // 避免循环内逐个查询（N+1）
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let existing_reservation_ids: std::collections::HashSet<i32> = if product_ids.is_empty() {
            std::collections::HashSet::new()
        } else {
            inventory_reservation::Entity::find()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.is_in(product_ids))
                .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
                .all(txn)
                .await?
                .into_iter()
                .map(|r| r.product_id)
                .collect()
        };

        // v17 批次 46 修复：循环外批量锁定所有需锁定的 product_id 的库存，避免循环内逐个 lock_exclusive（N+1）
        // PostgreSQL SELECT FOR UPDATE 支持 WHERE IN 批量加锁，行锁在事务内持续到 commit
        let need_lock_product_ids: Vec<i32> = items
            .iter()
            .map(|i| i.product_id)
            .filter(|pid| !existing_reservation_ids.contains(pid))
            .collect();
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            if need_lock_product_ids.is_empty() {
                std::collections::HashMap::new()
            } else {
                inventory_stock::Entity::find()
                    .filter(inventory_stock::Column::ProductId.is_in(need_lock_product_ids))
                    .lock_exclusive()
                    .all(txn)
                    .await?
                    .into_iter()
                    .map(|s| (s.product_id, s))
                    .collect()
            };

        // v13 P1-3：预留记录批量 INSERT，库存 update_many 保持逐条以确保防御性 WHERE 语义
        let mut reservations_to_insert: Vec<inventory_reservation::ActiveModel> = Vec::new();
        for item in items {
            if existing_reservation_ids.contains(&item.product_id) {
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }

            // v17 批次 46 修复：从批量查询结果获取（行锁已在批量查询时获取）
            let stock = stock_map.get(&item.product_id).cloned();

            if let Some(s) = stock {
                if s.quantity_available < item.quantity {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足，无法锁定",
                        item.product_id
                    )));
                }

                // 收集预留记录（不立即 INSERT）
                reservations_to_insert.push(inventory_reservation::ActiveModel {
                    id: Default::default(),
                    order_id: Set(order_id),
                    product_id: Set(item.product_id),
                    warehouse_id: Set(s.warehouse_id),
                    quantity: Set(item.quantity),
                    status: Set(reservation_status::PENDING.to_string()),
                    reserved_at: Set(chrono::Utc::now()),
                    released_at: Set(None),
                    notes: Set(None),
                    created_by: Set(Some(user_id)),
                    created_at: Set(chrono::Utc::now()),
                    updated_at: Set(chrono::Utc::now()),
                });

                // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= quantity，
                // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
                let lock_result = inventory_stock::Entity::update_many()
                    .filter(inventory_stock::Column::Id.eq(s.id))
                    .filter(inventory_stock::Column::QuantityAvailable.gte(item.quantity))
                    .col_expr(
                        inventory_stock::Column::QuantityAvailable,
                        sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                            .sub(item.quantity),
                    )
                    .col_expr(
                        inventory_stock::Column::UpdatedAt,
                        sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                    )
                    .exec(txn)
                    .await?;

                if lock_result.rows_affected == 0 {
                    return Err(AppError::business(format!(
                        "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                        item.product_id
                    )));
                }
            } else {
                return Err(AppError::business(format!(
                    "产品 {} 没有库存记录，无法锁定",
                    item.product_id
                )));
            }
        }
        // 批量 INSERT 预留记录，替代逐条 INSERT
        if !reservations_to_insert.is_empty() {
            inventory_reservation::Entity::insert_many(reservations_to_insert)
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 扣减库存
    /// 返回 (变更前可用数量, 变更后可用数量)，用于记录库存流水
    pub(crate) async fn reduce_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(Decimal, Decimal, String, Option<String>), AppError> {
        // 批次 9（2026-06-28）：加 FOR UPDATE 行锁，防止并发发货导致超扣
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        if stock.quantity_available < quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存 {} 小于发货数量 {}",
                product_id, stock.quantity_available, quantity
            )));
        }

        // 批次 9（2026-06-28）：UPDATE 加防御性 WHERE 条件 quantity_available >= quantity，
        // 即使并发绕过 SELECT FOR UPDATE（理论上不会发生），也能阻止超扣
        let reduce_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityAvailable.gte(quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        if reduce_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存不足（并发冲突或库存已被其他事务扣减）",
                product_id
            )));
        }

        // 标记预留为已完成
        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.eq(product_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CONSUMED.to_string()).into(),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        // 批次 356 v13 复审 B-P0-2 修复：返回变更前后的可用数量，供调用方记录库存流水
        // v14 批次 418 修复 D-P0-5：同时返回库存的 color_no/dye_lot_no，
        // 供调用方在库存流水中记录真实缸号/色号，替代原 None/空字符串硬编码
        let qty_before = stock.quantity_available;
        let qty_after = qty_before - quantity;
        Ok((qty_before, qty_after, stock.color_no.clone(), stock.dye_lot_no.clone()))
    }

    /// 释放订单的库存预留记录
    pub(crate) async fn release_reservations(
        &self,
        order_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let reservations = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?;

        // P2 5-14 修复：按 (product_id, warehouse_id) 聚合后批量更新库存，
        // 原为循环内逐条 update_many 导致 N 个=N 次 UPDATE；聚合后仅 G 次 UPDATE（G=唯一 product+warehouse 组合数）
        use std::collections::HashMap;
        let mut grouped: HashMap<(i32, i32), Decimal> = HashMap::new();
        for res in reservations {
            *grouped
                .entry((res.product_id, res.warehouse_id))
                .or_insert(Decimal::ZERO) += res.quantity;
        }

        let now = chrono::Utc::now();
        for ((product_id, warehouse_id), total_qty) in grouped {
            inventory_stock::Entity::update_many()
                .filter(inventory_stock::Column::ProductId.eq(product_id))
                .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                        .add(total_qty),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(now).into(),
                )
                .exec(txn)
                .await?;
        }

        inventory_reservation::Entity::update_many()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .col_expr(
                inventory_reservation::Column::Status,
                sea_orm::sea_query::Expr::val(reservation_status::CANCELLED.to_string()).into(),
            )
            .col_expr(
                inventory_reservation::Column::ReleasedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .col_expr(
                inventory_reservation::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        Ok(())
    }
}
