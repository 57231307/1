//! 采购收货相关服务（po/receipt）
//!
//! 包含采购订单明细的增删改与订单金额汇总。
//! 库存入库由入库单确认事务（`purchase_receipt_ops::state::confirm_receipt`）按入库明细
//! 单点完成——本模块不再有第二套按订单明细收货的实现，避免出现两套收货口径。
//! 拆分自原 `purchase_order_service.rs`。

use crate::models::{purchase_order, purchase_order_item, status};
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
        self.calculate_order_total_txn(order_id, &txn, user_id)
            .await?;
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
        if order.order_status != crate::models::status::purchase_inventory::purchase_order::DRAFT {
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
        self.calculate_order_total_txn(order.id, &txn, user_id)
            .await?;

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
        if order.order_status != crate::models::status::purchase_inventory::purchase_order::DRAFT {
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
        self.calculate_order_total_txn(order.id, &txn, user_id)
            .await?;

        txn.commit().await?;

        Ok(())
    }

    /// 计算订单总金额（事务版本）
    /// 批次 19（2026-06-28）：新增 _txn 变体，接受外部事务参数，；供已有事务的调用方使用，保证明细写与总金额重算原子性。；内部 3 处 DB 句柄全部使用 txn，主表查询加 lock_exclusive 串行化并发重算，；防止两个并发重算基于过期明细快照导致丢失更新。
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
    /// 批次 19（2026-06-28）：改为便捷入口，内部 begin + 调 _txn + commit。；已在事务内的调用方应直接调用 calculate_order_total_txn 以复用事务。
    pub async fn calculate_order_total(&self, order_id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        // 批次 94 P2-10：透传 user_id 用于审计日志
        self.calculate_order_total_txn(order_id, &txn, user_id)
            .await?;
        txn.commit().await?;
        Ok(())
    }
}
