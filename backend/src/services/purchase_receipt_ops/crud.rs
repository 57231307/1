//! 采购入库-CRUD 子模块（purchase_receipt_ops/crud）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的 3 个入库单 CRUD 方法 + 1 个事务内总金额更新 helper：
//! - `create_receipt`：创建入库单（含明细），调用 facade 的 generate_receipt_no / build_receipt_active_model / build_receipt_items_and_totals
//! - `update_receipt`：更新入库单（仅 DRAFT，admin 可绕过 owner）
//! - `delete_receipt`：删除入库单（仅 DRAFT + 审计日志，admin 可绕过 owner）
//! - `update_receipt_totals`：事务内更新入库单总金额（仅 create_receipt 调用，私有）
//!
//! 跨模块调用：
//! - 调用 `auth::is_admin_user`（`pub(crate)`）做管理员绕过校验
//! - 调用 facade 的纯函数 `build_receipt_active_model` / `build_receipt_items_and_totals`（`pub(crate)`）

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, UpdatePurchaseReceiptRequest,
};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// 创建采购入库单（含明细）
    pub async fn create_receipt(
        &self,
        req: CreatePurchaseReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成入库单号
        let receipt_no = self.generate_receipt_no().await?;

        // 2. 创建入库单主表
        let receipt = Self::build_receipt_active_model(&req, receipt_no, user_id)
            .insert(&txn)
            .await?;

        // 3. 创建入库明细（批量 insert_many，避免循环逐条 INSERT）
        let (item_active_models, total_quantity, total_quantity_alt, total_amount) =
            Self::build_receipt_items_and_totals(req.items, receipt.id);
        if !item_active_models.is_empty() {
            purchase_receipt_item::Entity::insert_many(item_active_models)
                .exec(&txn)
                .await?;
        }

        // 4. 更新入库单总金额和数量
        let receipt = Self::update_receipt_totals(
            &txn,
            receipt,
            total_quantity,
            total_quantity_alt,
            total_amount,
            user_id,
        )
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(receipt)
    }

    /// 更新入库单总金额和数量（含审计日志），返回更新后的入库单
    ///
    /// 仅 `create_receipt` 调用，保持私有。
    async fn update_receipt_totals(
        txn: &sea_orm::DatabaseTransaction,
        receipt: purchase_receipt::Model,
        total_quantity: rust_decimal::Decimal,
        total_quantity_alt: rust_decimal::Decimal,
        total_amount: rust_decimal::Decimal,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.total_quantity = Set(total_quantity);
        receipt_active.total_quantity_alt = Set(total_quantity_alt);
        receipt_active.total_amount = Set(total_amount);
        // P1 1-1 修复：原 Some(0) 占位符改为真实操作人 user_id
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            receipt_active,
            Some(user_id),
        )
        .await
    }

    /// 更新采购入库单（仅草稿状态）
    pub async fn update_receipt(
        &self,
        receipt_id: i32,
        req: UpdatePurchaseReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        // 批次 18（2026-06-28）：补全事务边界，原实现无事务且 update_with_audit 传 &*self.db 非原子
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单（加 lock_exclusive 串行化并发修改）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许修改，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限（P2 3-19 修复：admin 可绕过 owner 检查）
        if !self.is_admin_user(user_id).await? && receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能修改自己创建的入库单".to_string(),
            ));
        }

        // 4. 更新入库单（update_with_audit 传 &txn 纳入事务，保证原子性）
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();

        if let Some(supplier_id) = req.supplier_id {
            receipt_active.supplier_id = Set(supplier_id);
        }
        if let Some(receipt_date) = req.receipt_date {
            receipt_active.receipt_date = Set(receipt_date);
        }
        if let Some(department_id) = req.department_id {
            receipt_active.department_id = Set(Some(department_id));
        }
        if let Some(inspector_id) = req.inspector_id {
            receipt_active.inspector_id = Set(Some(inspector_id));
        }
        if let Some(notes) = req.notes {
            receipt_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            receipt_active.attachment_urls = Set(Some(attachment_urls));
        }

        receipt_active.updated_by = Set(Some(user_id));

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(receipt)
    }

    /// 删除采购入库单（仅 DRAFT 状态）
    pub async fn delete_receipt(&self, receipt_id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态门用裸查询 &*self.db 无锁，且 txn 仅包裹删除；
        // 改为将状态门查询移入 txn 并加 lock_exclusive，防止并发删除/确认同入库单。
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单（加 lock_exclusive 串行化并发 delete_receipt）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许删除，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查权限（P2 3-19 修复：admin 可绕过 owner 检查）
        if !self.is_admin_user(user_id).await? && receipt.created_by != user_id {
            return Err(AppError::permission_denied(
                "只能删除自己创建的入库单".to_string(),
            ));
        }

        // 4. 先删除明细
        purchase_receipt_item::Entity::delete_many()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .exec(&txn)
            .await?;

        // 5. 删除入库单（P0 8-3 修复：补审计日志）
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            purchase_receipt::Entity,
            _,
        >(&txn, "purchase_receipt", receipt_id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }
}
