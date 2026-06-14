//! 采购收货服务（po/receipt）
//!
//! 包含采购订单的收货确认（含库存入库联动）、收货单号生成等。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{product, purchase_order, purchase_order_item, purchase_receipt, status};
use crate::services::po::CreateOrderItemRequest;
use crate::services::po::UpdateOrderItemRequest;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use std::sync::Arc;

use super::order::PurchaseOrderService;
use sea_orm::DatabaseConnection;

impl PurchaseOrderService {
    // 生成入库单号
    // 格式：PR + 年月日 + 三位序号（PR20260315001）
    crate::impl_generate_no!(
        generate_receipt_no,
        "PR",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo
    );

    /// 标记采购订单为已收货（含库存入库联动）
    pub async fn receive_order(&self, order_id: i32) -> Result<purchase_order::Model, AppError> {
        // 1. 开启事务保证数据一致性
        let txn = (*self.db).begin().await?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 3. 检查状态 - 只有已审批的订单才能收货
        if order.order_status != status::purchase_order::APPROVED
            && order.order_status != status::purchase_order::PARTIAL_RECEIVED
        {
            return Err(AppError::business(format!(
                "订单状态不允许收货，当前状态：{}，需要状态：APPROVED 或 PARTIAL_RECEIVED",
                order.order_status
            )));
        }

        // 4. 查询订单明细
        let order_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&txn)
            .await?;

        // 5. 创建库存服务实例（使用事务版本的静态方法）
        // 6. 遍历订单明细，执行库存入库
        for item in &order_items {
            // 查询产品信息获取批次相关字段
            let product = product::Entity::find_by_id(item.product_id)
                .one(&txn)
                .await?
                .ok_or_else(|| {
                    AppError::not_found(format!("产品 ID {} 不存在", item.product_id))
                })?;

            // 计算入库数量
            let receive_quantity_meters = item.quantity - item.received_quantity;
            let receive_quantity_alt = item.quantity_alt - item.received_quantity_alt;

            // 只处理有入库数量的明细
            if receive_quantity_meters > Decimal::ZERO {
                // 查找现有库存记录（使用事务版本）
                let existing_stock = crate::services::inventory_stock_service::InventoryStockService::find_by_product_and_warehouse_txn(
                    &txn, item.product_id, order.warehouse_id,
                )
                .await
                .map_err(|e| {
                    tracing::error!("查询库存失败: 产品ID={}, 仓库ID={}, 错误: {}", item.product_id, order.warehouse_id, e);
                    AppError::internal(format!("查询库存失败: {}", e))
                })?;

                let before_quantity_meters;
                let before_quantity_kg;

                match existing_stock {
                    Some(stock) => {
                        before_quantity_meters = stock.quantity_meters;
                        before_quantity_kg = stock.quantity_kg;

                        // 更新现有库存（使用事务版本）
                        let new_quantity_meters = stock.quantity_meters + receive_quantity_meters;
                        let new_quantity_kg = stock.quantity_kg + receive_quantity_alt;

                        crate::services::inventory_stock_service::InventoryStockService::update_stock_quantity_with_optimistic_lock_txn(
                            &txn,
                            stock.id,
                            new_quantity_meters,
                            new_quantity_kg,
                            stock.version,
                        )
                        .await
                        .map_err(|e| {
                            tracing::error!("更新库存失败: 库存ID={}, 错误: {}", stock.id, e);
                            AppError::internal(format!("更新库存失败: {}", e))
                        })?;
                    }
                    None => {
                        before_quantity_meters = Decimal::ZERO;
                        before_quantity_kg = Decimal::ZERO;

                        // 创建新库存记录（使用事务版本）
                        crate::services::inventory_stock_service::InventoryStockService::create_stock_fabric_txn(
                            &txn,
                            order.warehouse_id,
                            item.product_id,
                            "DEFAULT".to_string(),
                            "DEFAULT".to_string(),
                            None,
                            "A".to_string(),
                            receive_quantity_meters,
                            receive_quantity_alt,
                            product.gram_weight,
                            product.width,
                            None,
                            None,
                            None,
                        )
                        .await
                        .map_err(|e| {
                            tracing::error!("创建库存记录失败: 产品ID={}, 仓库ID={}, 错误: {}", item.product_id, order.warehouse_id, e);
                            AppError::internal(format!("创建库存记录失败: {}", e))
                        })?;
                    }
                };

                // 记录库存流水（使用事务版本，正确的前后数量）
                crate::services::inventory_stock_service::InventoryStockService::record_transaction_txn(
                    &txn,
                    "PURCHASE_RECEIPT".to_string(),
                    item.product_id,
                    order.warehouse_id,
                    "DEFAULT".to_string(),
                    "DEFAULT".to_string(),
                    None,
                    "A".to_string(),
                    receive_quantity_meters,
                    receive_quantity_alt,
                    Some("purchase_order".to_string()),
                    Some(order.order_no.clone()),
                    Some(order.id),
                    Some(before_quantity_meters),
                    Some(before_quantity_kg),
                    Some(before_quantity_meters + receive_quantity_meters),
                    Some(before_quantity_kg + receive_quantity_alt),
                    Some(format!("采购入库 - 订单 {}", order.order_no)),
                    None,
                )
                .await
                .map_err(|e| {
                    tracing::error!("记录库存流水失败: 产品ID={}, 仓库ID={}, 错误: {}", item.product_id, order.warehouse_id, e);
                    AppError::internal(format!("记录库存流水失败: {}", e))
                })?;

                // 更新订单明细已入库数量（累加而非覆盖）
                let mut item_active: purchase_order_item::ActiveModel = item.clone().into();
                item_active.received_quantity =
                    Set(item.received_quantity + receive_quantity_meters);
                item_active.received_quantity_alt =
                    Set(item.received_quantity_alt + receive_quantity_alt);
                item_active.updated_at = Set(Utc::now());
                purchase_order_item::Entity::update(item_active)
                    .exec(&txn)
                    .await?;
            }
        }

        // 7. 判断订单状态（全部收货还是部分收货）
        let all_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&txn)
            .await?;

        let mut is_fully_received = true;
        for item in &all_items {
            if item.received_quantity < item.quantity {
                is_fully_received = false;
                break;
            }
        }

        let new_status = if is_fully_received {
            status::purchase_order::COMPLETED.to_string()
        } else {
            status::purchase_order::PARTIAL_RECEIVED.to_string()
        };

        // 7. 更新订单状态
        let now = chrono::Utc::now();
        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.order_status = Set(new_status);
        order_active.actual_delivery_date = Set(Some(now.date_naive()));
        order_active.updated_at = Set(now);

        let order = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        // 8. 提交事务
        txn.commit().await?;

        Ok(order)
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
        // 1. 查询订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        // 2. 检查状态
        if order.order_status != status::purchase_order::DRAFT {
            return Err(AppError::business(format!(
                "订单状态不允许添加明细，当前状态：{}",
                order.order_status
            )));
        }

        // 3. 检查权限
        if order.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能为自己创建的订单添加明细".to_string(),
            ));
        }

        // 4. 创建明细
        let quantity_ordered = req.quantity_ordered.unwrap_or(Decimal::ZERO);
        let unit_price = req.unit_price.unwrap_or(Decimal::ZERO);
        let amount = quantity_ordered * unit_price;
        let tax_percent = req.tax_rate.unwrap_or(Decimal::new(13, 2));
        let tax_amount = amount * tax_percent / Decimal::new(100, 0);
        let discount_percent = req.discount_percent.unwrap_or(Decimal::ZERO);
        let discount_amount = amount * discount_percent / Decimal::new(100, 0);
        let quantity_alt_ordered = req.quantity_alt_ordered.unwrap_or(Decimal::ZERO);

        let item = purchase_order_item::ActiveModel {
            id: Set(0),
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
        }
        .insert(&*self.db)
        .await?;

        // 5. 更新订单总金额
        self.calculate_order_total(order_id).await?;

        Ok(item)
    }

    /// 更新订单明细
    pub async fn update_order_item(
        &self,
        item_id: i32,
        req: UpdateOrderItemRequest,
        user_id: i32,
    ) -> Result<purchase_order_item::Model, AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
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

        // 5. 更新明细
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
            &*self.db,
            "auto_audit",
            item_active,
            Some(0),
        )
        .await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(item)
    }

    /// 删除订单明细
    pub async fn delete_order_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 1. 查询明细
        let item = purchase_order_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("订单明细 {}", item_id)))?;

        // 2. 查询订单
        let order = purchase_order::Entity::find_by_id(item.order_id)
            .one(&*self.db)
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
            .exec(&*self.db)
            .await?;

        // 6. 更新订单总金额
        self.calculate_order_total(order.id).await?;

        Ok(())
    }

    /// 计算订单总金额
    pub async fn calculate_order_total(&self, order_id: i32) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(&*self.db)
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

        // 3. 更新订单
        let order = purchase_order::Entity::find_by_id(order_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购订单 {}", order_id)))?;

        let mut order_active: purchase_order::ActiveModel = order.into();
        order_active.total_amount = Set(total_amount);
        order_active.total_quantity = Set(total_quantity);
        order_active.total_quantity_alt = Set(total_quantity_alt);
        order_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            order_active,
            Some(0),
        )
        .await?;

        Ok(())
    }
}

/// 引用 Arc 别名，避免子模块中重复声明
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;
