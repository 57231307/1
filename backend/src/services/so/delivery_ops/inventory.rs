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

use crate::models::status::inventory_reservation as reservation_status;
use crate::models::{inventory_reservation, inventory_stock};
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
        let product_ids: Vec<i32> = items.iter().map(|i| i.product_id).collect();
        let existing_ids =
            Self::query_existing_reservation_ids(order_id, &product_ids, txn).await?;
        let stock_map = Self::query_locked_stock_map(&product_ids, &existing_ids, txn).await?;
        let reservations = Self::build_and_lock_reservations(
            order_id,
            items,
            user_id,
            &existing_ids,
            &stock_map,
            txn,
        )
        .await?;
        Self::batch_insert_reservations(reservations, txn).await
    }

    /// 查询订单已存在的 pending 预留 product_id 集合
    async fn query_existing_reservation_ids(
        order_id: i32,
        product_ids: &[i32],
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<std::collections::HashSet<i32>, AppError> {
        if product_ids.is_empty() {
            return Ok(std::collections::HashSet::new());
        }
        let ids: std::collections::HashSet<i32> = inventory_reservation::Entity::find()
            .filter(inventory_reservation::Column::OrderId.eq(order_id))
            .filter(inventory_reservation::Column::ProductId.is_in(product_ids.to_vec()))
            .filter(inventory_reservation::Column::Status.eq(reservation_status::PENDING))
            .all(txn)
            .await?
            .into_iter()
            .map(|r| r.product_id)
            .collect();
        Ok(ids)
    }

    /// 批量加锁查询需锁定的库存记录
    async fn query_locked_stock_map(
        product_ids: &[i32],
        existing_ids: &std::collections::HashSet<i32>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<std::collections::HashMap<i32, inventory_stock::Model>, AppError> {
        let need_lock: Vec<i32> = product_ids
            .iter()
            .filter(|pid| !existing_ids.contains(pid))
            .copied()
            .collect();
        if need_lock.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        let map: std::collections::HashMap<i32, inventory_stock::Model> =
            inventory_stock::Entity::find()
                .filter(inventory_stock::Column::ProductId.is_in(need_lock))
                .lock_exclusive()
                .all(txn)
                .await?
                .into_iter()
                .map(|s| (s.product_id, s))
                .collect();
        Ok(map)
    }

    /// 遍历 items 构建预留记录并逐条锁定库存
    async fn build_and_lock_reservations(
        order_id: i32,
        items: &[super::super::SalesOrderItemRequest],
        user_id: i32,
        existing_ids: &std::collections::HashSet<i32>,
        stock_map: &std::collections::HashMap<i32, inventory_stock::Model>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<Vec<inventory_reservation::ActiveModel>, AppError> {
        let mut reservations: Vec<inventory_reservation::ActiveModel> = Vec::new();
        for item in items {
            if existing_ids.contains(&item.product_id) {
                tracing::info!("产品 {} 已存在预留记录，跳过创建", item.product_id);
                continue;
            }
            let stock = stock_map.get(&item.product_id).cloned().ok_or_else(|| {
                AppError::business(format!("产品 {} 没有库存记录，无法锁定", item.product_id))
            })?;
            Self::check_stock_sufficient(&stock, item)?;
            reservations.push(Self::build_reservation_active_model(
                order_id, item, user_id, &stock,
            ));
            Self::execute_stock_lock(&stock, item, txn).await?;
        }
        Ok(reservations)
    }

    /// 校验库存是否充足
    fn check_stock_sufficient(
        stock: &inventory_stock::Model,
        item: &super::super::SalesOrderItemRequest,
    ) -> Result<(), AppError> {
        if stock.quantity_available < item.quantity {
            return Err(AppError::business(format!(
                "产品 {} 库存不足，无法锁定",
                item.product_id
            )));
        }
        Ok(())
    }

    /// 构建单条预留记录 ActiveModel
    fn build_reservation_active_model(
        order_id: i32,
        item: &super::super::SalesOrderItemRequest,
        user_id: i32,
        stock: &inventory_stock::Model,
    ) -> inventory_reservation::ActiveModel {
        inventory_reservation::ActiveModel {
            id: Default::default(),
            order_id: Set(order_id),
            product_id: Set(item.product_id),
            warehouse_id: Set(stock.warehouse_id),
            quantity: Set(item.quantity),
            status: Set(reservation_status::PENDING.to_string()),
            reserved_at: Set(chrono::Utc::now()),
            released_at: Set(None),
            notes: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    /// 执行库存锁定 UPDATE（带防御性 WHERE 条件）
    async fn execute_stock_lock(
        stock: &inventory_stock::Model,
        item: &super::super::SalesOrderItemRequest,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        let lock_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
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
        Ok(())
    }

    /// 批量插入预留记录
    async fn batch_insert_reservations(
        reservations: Vec<inventory_reservation::ActiveModel>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        if !reservations.is_empty() {
            inventory_reservation::Entity::insert_many(reservations)
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
        Ok((
            qty_before,
            qty_after,
            stock.color_no.clone(),
            stock.dye_lot_no.clone(),
        ))
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
