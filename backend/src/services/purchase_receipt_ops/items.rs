//! 采购入库-明细 CRUD 子模块（purchase_receipt_ops/items）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 3 个明细 CRUD 方法 + 2 个总金额重算方法：
//! - `add_receipt_item`：添加明细（DRAFT，admin 可绕过 owner）
//! - `update_receipt_item`：更新明细（DRAFT，admin 可绕过 owner）
//! - `delete_receipt_item`：删除明细（DRAFT，admin 可绕过 owner）
//! - `calculate_receipt_total_txn`：事务内重算入库单总金额（`pub`，供 add/update/delete_receipt_item 跨方法调用）
//! - `calculate_receipt_total`：便捷入口（自建事务），handler 直接调用，保持 `pub`
//!
//! 跨模块调用：
//! - 调用 `auth::is_admin_user`（`pub(crate)`）做管理员绕过校验

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::purchase_receipt_dto::{CreateReceiptItemRequest, UpdateReceiptItemRequest};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// 添加入库明细
    pub async fn add_receipt_item(
        &self,
        receipt_id: i32,
        req: CreateReceiptItemRequest,
        user_id: i32,
    ) -> Result<purchase_receipt_item::Model, AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细写与总金额重算原子化。
        // 原实现明细 insert 与 calculate_receipt_total 非原子，且均用 &*self.db 无锁，
        // 并发 add_receipt_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单（加 lock_exclusive 串行化并发明细操作）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许添加明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限（P2 3-19 修复：admin 可绕过 owner 检查）
        if !self.is_admin_user(user_id).await? && receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能为自己创建的入库单添加明细".to_string(),
            ));
        }

        // 4. 创建明细
        let amount = req.quantity * req.unit_price.unwrap_or_else(|| Decimal::new(0, 0));
        let item = purchase_receipt_item::ActiveModel {
            receipt_id: Set(receipt_id),
            order_item_id: Set(req.order_item_id),
            product_id: Set(req.material_id),
            quantity: Set(req.quantity),
            quantity_alt: Set(Some(req.quantity_alt)),
            unit_price: Set(Some(req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)))),
            amount: Set(Some(amount)),
            notes: Set(req.notes),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 5. 更新入库单总金额（事务内调用 _txn 变体，保证明细写与重算原子性）
        self.calculate_receipt_total_txn(receipt_id, &txn, user_id)
            .await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 更新入库明细
    pub async fn update_receipt_item(
        &self,
        item_id: i32,
        req: UpdateReceiptItemRequest,
        user_id: i32,
    ) -> Result<purchase_receipt_item::Model, AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细 update 与总金额重算原子化。
        // 原实现明细 update_with_audit 与 calculate_receipt_total 非原子且均用 &*self.db，
        // 并发 update_receipt_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;

        // 1. 查询明细
        let item = purchase_receipt_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("入库明细 {}", item_id)))?;

        // 2. 查询入库单（加 lock_exclusive 串行化并发明细操作）
        let receipt = purchase_receipt::Entity::find_by_id(item.receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", item.receipt_id)))?;

        // 3. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许修改明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 4. 检查权限（P2 3-19 修复：admin 可绕过 owner 检查）
        if !self.is_admin_user(user_id).await? && receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的入库明细".to_string(),
            ));
        }

        // 5. 更新明细（update_with_audit 传 &txn 纳入事务，保证原子性）
        let mut item_active: purchase_receipt_item::ActiveModel = item.into();

        if let Some(quantity) = req.quantity {
            item_active.quantity = Set(quantity);
        }
        if let Some(quantity_alt) = req.quantity_alt {
            item_active.quantity_alt = Set(Some(quantity_alt));
        }
        if let Some(unit_price) = req.unit_price {
            item_active.unit_price = Set(Some(unit_price));
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

        // 6. 更新入库单总金额（事务内调用 _txn 变体，保证明细写与重算原子性）
        self.calculate_receipt_total_txn(receipt.id, &txn, user_id)
            .await?;

        txn.commit().await?;

        Ok(item)
    }

    /// 删除入库明细
    pub async fn delete_receipt_item(&self, item_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 19（2026-06-28）：补全事务边界，明细 delete 与总金额重算原子化。
        // 原实现明细 delete 与 calculate_receipt_total 非原子且均用 &*self.db，
        // 并发 delete_receipt_item 会导致总金额丢失更新。
        let txn = (*self.db).begin().await?;

        // 1. 查询明细
        let item = purchase_receipt_item::Entity::find_by_id(item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("入库明细 {}", item_id)))?;

        // 2. 查询入库单（加 lock_exclusive 串行化并发明细操作）
        let receipt = purchase_receipt::Entity::find_by_id(item.receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", item.receipt_id)))?;

        // 3. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许删除明细，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 4. 检查权限（P2 3-19 修复：admin 可绕过 owner 检查）
        if !self.is_admin_user(user_id).await? && receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的入库明细".to_string(),
            ));
        }

        // 5. 删除明细
        purchase_receipt_item::Entity::delete_by_id(item_id)
            .exec(&txn)
            .await?;

        // 6. 更新入库单总金额（事务内调用 _txn 变体，保证明细写与重算原子性）
        self.calculate_receipt_total_txn(receipt.id, &txn, user_id)
            .await?;

        txn.commit().await?;

        Ok(())
    }

    /// 计算入库单总金额（事务版本）
    ///
    /// 批次 19（2026-06-28）：新增 _txn 变体，接受外部事务参数，
    /// 供已有事务的调用方使用，保证明细写与总金额重算原子性。
    /// 内部 3 处 DB 句柄全部使用 txn，主表查询加 lock_exclusive 串行化并发重算，
    /// 防止两个并发重算基于过期明细快照导致丢失更新。
    ///
    /// 批次 101 v6 复审 P2-6 修复：原 update_with_audit 调用 Some(0) 占位符导致审计日志操作人为 0，
    /// 改为透传真实操作人 user_id。user_id 由调用方（add_receipt_item / update_receipt_item /
    /// delete_receipt_item / calculate_receipt_total）注入。
    pub async fn calculate_receipt_total_txn(
        &self,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 1. 查询所有明细
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(txn)
            .await?;

        // 2. 计算总和
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);
        let mut total_amount = Decimal::new(0, 0);

        for item in items {
            total_quantity += item.quantity;
            total_quantity_alt += item.quantity_alt.unwrap_or_default();
            total_amount += item.amount.unwrap_or_default();
        }

        // 3. 更新入库单（加 lock_exclusive 串行化并发重算，防止丢失更新）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.total_quantity = Set(total_quantity);
        receipt_active.total_quantity_alt = Set(total_quantity_alt);
        receipt_active.total_amount = Set(total_amount);
        receipt_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            receipt_active,
            // 批次 101 v6 复审 P2-6：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        Ok(())
    }

    /// 计算入库单总金额（便捷入口，内部自建事务）
    ///
    /// 已在事务内的调用方应直接调用 calculate_receipt_total_txn 以复用事务。
    pub async fn calculate_receipt_total(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        self.calculate_receipt_total_txn(receipt_id, &txn, user_id)
            .await?;
        txn.commit().await?;
        Ok(())
    }
}
