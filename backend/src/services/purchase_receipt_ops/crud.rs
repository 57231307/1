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

        // 3b. 关联采购订单的入库单必须把入库明细挂到被入的订单明细行，
        // 否则确认入库时无处累加 received_quantity，订单永远停在已审批态
        if let Some(order_id) = req.order_id {
            Self::link_receipt_items_to_order_items(&txn, receipt.id, order_id).await?;
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

        // 6. 业务追溯 chain head 接入（best-effort，失败不阻塞采购收货）
        let trace_service =
            crate::services::business_trace_service::BusinessTraceService::new(self.db.clone());
        let items_models: Vec<purchase_receipt_item::Model> = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt.id))
            .all(&*self.db)
            .await
            .unwrap_or_default();
        trace_service
            .record_purchase_receipt(&receipt, &items_models, user_id)
            .await;

        Ok(receipt)
    }

    /// 把入库明细挂到被入的采购订单明细行（事务内调用；`pub(crate)`：`add_receipt_item`
    /// 与 `update_receipt_item` 也需在明细写入后挂接，否则经明细端点增改的行永不累加订单进度）
    ///
    /// 确认入库按入库明细的 `order_item_id` 累加订单明细 received_quantity 并据此推进
    /// 订单状态（全部收货 COMPLETED / 部分收货 PARTIAL_RECEIVED）。请求未指定
    /// `order_item_id` 时按产品与剩余可收量在订单明细行间顺序分配（同产品多行落同一
    /// 明细行）；显式指定的明细必须属于本订单，否则拒绝建单，不允许挂错单。
    /// 找不到可收明细行（产品不在订单中或已收满）时按行记录错误：货物照入，
    /// 但该行的收货量不计入订单进度，便于从日志定位建单数据错误。
    pub(crate) async fn link_receipt_items_to_order_items(
        txn: &sea_orm::DatabaseTransaction,
        receipt_id: i32,
        order_id: i32,
    ) -> Result<(), AppError> {
        use crate::models::purchase_order_item;

        let order_items = purchase_order_item::Entity::find()
            .filter(purchase_order_item::Column::OrderId.eq(order_id))
            .all(txn)
            .await?;
        if order_items.is_empty() {
            return Err(AppError::bad_request(format!(
                "采购订单 {} 没有明细行，无法按单建入库单",
                order_id
            )));
        }

        let receipt_items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .all(txn)
            .await?;
        for item in receipt_items {
            if let Some(declared) = item.order_item_id {
                if !order_items.iter().any(|oi| oi.id == declared) {
                    return Err(AppError::bad_request(format!(
                        "入库单第 {} 行指定的订单明细 {} 不属于采购订单 {}",
                        item.line_no, declared, order_id
                    )));
                }
                continue;
            }
            let target = order_items
                .iter()
                .find(|oi| oi.product_id == item.product_id && oi.received_quantity < oi.quantity);
            match target {
                Some(oi) => {
                    let active = purchase_receipt_item::ActiveModel {
                        id: Set(item.id),
                        order_item_id: Set(Some(oi.id)),
                        ..Default::default()
                    };
                    purchase_receipt_item::Entity::update(active)
                        .exec(txn)
                        .await?;
                }
                None => tracing::error!(
                    receipt_id,
                    line_no = item.line_no,
                    product_id = item.product_id,
                    order_id,
                    "入库明细在产品上与采购订单明细不匹配（产品不在订单中或已收满），该行收货量不会累加到订单进度"
                ),
            }
        }
        Ok(())
    }

    /// 更新入库单总金额和数量（含审计日志），返回更新后的入库单（仅 `create_receipt` 调用，保持私有。）
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
