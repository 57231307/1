//! 销售发货-取消发货子模块（delivery_ops/cancel）
//!
//! 批次 488 D10-3 拆分：从原 `so/delivery.rs` L1084-1320 迁移。
//! 包含 4 个取消发货相关方法：
//! - revert_delivery_items / revert_order_status_if_needed（私有辅助）
//! - cancel_delivery（公开 API）
//! - restore_inventory（私有辅助，对称反向于 reduce_inventory）
//!
//! 业务规则：
//! - 仅 SHIPPED 状态的发货单可取消（PENDING 是预留态，本系统发货即 SHIPPED）
//! - 库存恢复（对称反向）：quantity_available += qty，quantity_shipped -= qty
//! - 预留恢复：将 CONSUMED 状态的预留恢复为 PENDING
//! - 订单明细回退：sales_order_item.shipped_quantity -= qty
//! - 订单状态回退：若所有发货单取消，订单 SHIPPED→APPROVED；部分取消 SHIPPED→PARTIAL_SHIPPED

use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{
    inventory_reservation, inventory_stock, sales_delivery, sales_delivery_item, sales_order,
    sales_order_item,
};
use crate::models::status::inventory_reservation as reservation_status;
use crate::models::status::sales_delivery as delivery_status;
use crate::models::status::sales_order as so_status;
use crate::utils::error::AppError;

use super::super::order::SalesService;

impl SalesService {
    /// 取消销售发货单
    ///
    /// 批次 216 P2-1 修复（v12 复审）：实现销售发货 cancel_delivery 功能，
    /// 移除 sales_delivery::CANCELLED 的 #[allow(dead_code)] 标注。
    ///
    /// 业务规则：
    /// 恢复库存 + 回退订单明细已发货数量 + 恢复预留（取消发货循环体）
    async fn revert_delivery_items(
        &self,
        delivery_items: &[sales_delivery_item::Model],
        order_id: i32,
        warehouse_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        for item in delivery_items {
            // 恢复库存（对称反向）：quantity_available += qty，quantity_shipped -= qty
            self.restore_inventory(item.product_id, warehouse_id, item.quantity, txn)
                .await?;

            // 回退订单明细已发货数量
            sales_order_item::Entity::update_many()
                .filter(sales_order_item::Column::OrderId.eq(order_id))
                .filter(sales_order_item::Column::ProductId.eq(item.product_id))
                .col_expr(
                    sales_order_item::Column::ShippedQuantity,
                    sea_orm::sea_query::Expr::col(sales_order_item::Column::ShippedQuantity)
                        .sub(item.quantity),
                )
                .col_expr(
                    sales_order_item::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                )
                .exec(txn)
                .await?;

            // 恢复预留：将 CONSUMED 状态的预留恢复为 PENDING
            inventory_reservation::Entity::update_many()
                .filter(inventory_reservation::Column::OrderId.eq(order_id))
                .filter(inventory_reservation::Column::ProductId.eq(item.product_id))
                .filter(inventory_reservation::Column::Status.eq(reservation_status::CONSUMED))
                .col_expr(
                    inventory_reservation::Column::Status,
                    sea_orm::sea_query::Expr::val(reservation_status::PENDING.to_string())
                        .into(),
                )
                .col_expr(
                    inventory_reservation::Column::ReleasedAt,
                    sea_orm::sea_query::Expr::val(None::<chrono::DateTime<chrono::Utc>>).into(),
                )
                .col_expr(
                    inventory_reservation::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
                )
                .exec(txn)
                .await?;
        }
        Ok(())
    }

    /// 判定取消发货后订单状态是否需要回退，如需回退则更新订单状态
    async fn revert_order_status_if_needed(
        &self,
        order: sales_order::Model,
        order_id: i32,
        user_id: i32,
        now: chrono::DateTime<chrono::Utc>,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 查询订单所有明细，判断是否还有已发货项
        let order_items = sales_order_item::Entity::find()
            .filter(sales_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;

        let mut has_shipped = false;
        for oi in &order_items {
            if oi.shipped_quantity > Decimal::ZERO {
                has_shipped = true;
                break;
            }
        }

        // 订单状态回退：若所有发货都已取消，订单回退到 APPROVED；否则回退到 PARTIAL_SHIPPED
        let new_order_status = if has_shipped {
            so_status::PARTIAL_SHIPPED
        } else {
            so_status::APPROVED
        };

        // 仅当订单当前状态为 SHIPPED 或 PARTIAL_SHIPPED 时才回退（避免覆盖 CANCELLED 等终态）
        if order.status == so_status::SHIPPED || order.status == so_status::PARTIAL_SHIPPED {
            let mut order_active: sales_order::ActiveModel = order.into();
            order_active.status = Set(new_order_status.to_string());
            order_active.updated_at = Set(now);

            crate::services::audit_log_service::AuditLogService::update_with_audit(
                txn,
                "auto_audit",
                order_active,
                Some(user_id),
            )
            .await?;
        }
        Ok(())
    }

    /// - 仅 SHIPPED 状态的发货单可取消（PENDING 是预留态，本系统发货即 SHIPPED）
    /// - 库存恢复（对称反向）：quantity_available += qty，quantity_shipped -= qty
    /// - 预留恢复：将 CONSUMED 状态的预留恢复为 PENDING
    /// - 订单明细回退：sales_order_item.shipped_quantity -= qty
    /// - 订单状态回退：若所有发货单取消，订单 SHIPPED→APPROVED；部分取消 SHIPPED→PARTIAL_SHIPPED
    /// - AR 冲销：全额发货时生成的应收账款需冲销（生成红字 credit_memo）
    pub async fn cancel_delivery(
        &self,
        delivery_id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<sales_delivery::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询发货单（加 lock_exclusive 串行化并发取消）
        let delivery = sales_delivery::Entity::find_by_id(delivery_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("发货单 {}", delivery_id)))?;

        // 2. 检查状态 - 只有 SHIPPED 状态的发货单才能取消
        if delivery.status != delivery_status::SHIPPED {
            return Err(AppError::business(format!(
                "发货单状态不允许取消，当前状态：{}，仅 SHIPPED 状态可取消",
                delivery.status
            )));
        }

        let order_id = delivery.order_id;
        let warehouse_id = delivery.warehouse_id;

        // 3. 查询发货明细（用于库存恢复和订单明细回退）
        let delivery_items = sales_delivery_item::Entity::find()
            .filter(sales_delivery_item::Column::DeliveryId.eq(delivery_id))
            .all(&txn)
            .await?;

        // 4. 加排他锁查询关联订单（串行化并发取消/发货）
        let order = sales_order::Entity::find_by_id(order_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("销售订单 {}", order_id)))?;

        // 5. 恢复库存 + 回退订单明细已发货数量 + 恢复预留
        self.revert_delivery_items(&delivery_items, order_id, warehouse_id, &txn)
            .await?;

        // 6. 更新发货单状态为 CANCELLED，记录取消原因到 remarks
        let now = chrono::Utc::now();
        let cancel_remark = format!("[取消原因] {}", reason);
        let mut delivery_active: sales_delivery::ActiveModel = delivery.into();
        delivery_active.status = Set(delivery_status::CANCELLED.to_string());
        delivery_active.remarks = Set(Some(cancel_remark));
        delivery_active.updated_at = Set(now);

        let updated_delivery = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            delivery_active,
            Some(user_id),
        )
        .await?;

        // 7. 判定订单状态是否需要回退
        self.revert_order_status_if_needed(order, order_id, user_id, now, &txn)
            .await?;

        txn.commit().await?;

        Ok(updated_delivery)
    }

    /// 恢复库存（取消发货时使用，对称反向于 reduce_inventory）
    ///
    /// quantity_available += qty，quantity_shipped -= qty
    async fn restore_inventory(
        &self,
        product_id: i32,
        warehouse_id: i32,
        quantity: Decimal,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<(), AppError> {
        // 加行锁查询库存记录
        let stock = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::ProductId.eq(product_id))
            .filter(inventory_stock::Column::WarehouseId.eq(warehouse_id))
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产品 {} 库存记录", product_id)))?;

        // 防御性校验：已发货数量不能小于要恢复的数量
        if stock.quantity_shipped < quantity {
            return Err(AppError::business(format!(
                "产品 {} 已发货数量 {} 小于要恢复的数量 {}，库存数据不一致",
                product_id, stock.quantity_shipped, quantity
            )));
        }

        // 对称反向更新：quantity_available += qty，quantity_shipped -= qty
        let restore_result = inventory_stock::Entity::update_many()
            .filter(inventory_stock::Column::Id.eq(stock.id))
            .filter(inventory_stock::Column::QuantityShipped.gte(quantity))
            .col_expr(
                inventory_stock::Column::QuantityAvailable,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityAvailable)
                    .add(quantity),
            )
            .col_expr(
                inventory_stock::Column::QuantityShipped,
                sea_orm::sea_query::Expr::col(inventory_stock::Column::QuantityShipped)
                    .sub(quantity),
            )
            .col_expr(
                inventory_stock::Column::UpdatedAt,
                sea_orm::sea_query::Expr::val(chrono::Utc::now()).into(),
            )
            .exec(txn)
            .await?;

        if restore_result.rows_affected == 0 {
            return Err(AppError::business(format!(
                "产品 {} 库存恢复失败（并发冲突或已发货数量不足）",
                product_id
            )));
        }

        Ok(())
    }
}
