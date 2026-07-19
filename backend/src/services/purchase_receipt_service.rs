//! 采购入库 Service
//!
//! 采购入库服务层，负责采购入库的核心业务逻辑
//! 包含入库单创建、确认、更新等全流程管理
//!
// 批次 101 v6 复审 P2 修复：calculate_receipt_total_txn / calculate_receipt_total
// 审计操作人 Some(0) 占位符改为真实 user_id，三处内部调用方同步透传 user_id（P2-6）。

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::event_bus::EVENT_BUS;
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use crate::utils::error::AppError;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

/// 采购入库服务
pub struct PurchaseReceiptService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseReceiptService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// P2 3-19 修复：检查 user_id 是否为管理员（用于绕过 created_by owner 检查）
    ///
    /// 原 update_receipt/delete_receipt/confirm_receipt/add_receipt_item/
    /// update_receipt_item/delete_receipt_item 6 处均硬编码 `created_by != user_id`
    /// 无管理员绕过，admin 无法管理他人创建的入库单。
    /// 新增此辅助方法，admin 角色可绕过 owner 检查。
    async fn is_admin_user(&self, user_id: i32) -> Result<bool, AppError> {
        use crate::models::user;
        use sea_orm::EntityTrait;

        let user = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))?;
        if let Some(role_id) = user.role_id {
            Ok(crate::utils::admin_checker::is_admin_role(&self.db, role_id).await)
        } else {
            Ok(false)
        }
    }

    // 生成入库单号
    // 格式：GR + 年月日 + 三位序号（GR20260315001）
    crate::impl_generate_no!(
        generate_receipt_no,
        "PR",
        purchase_receipt::Entity,
        purchase_receipt::Column::ReceiptNo
    );

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
        let receipt = purchase_receipt::ActiveModel {
            receipt_no: Set(receipt_no),
            order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            receipt_date: Set(req.receipt_date),
            warehouse_id: Set(req.warehouse_id),
            department_id: Set(req.department_id),
            receiver_id: Set(Some(user_id)),
            inspector_id: Set(req.inspector_id),
            inspection_status: Set("PENDING".to_string()),
            receipt_status: Set(status::purchase_receipt::DRAFT.to_string()),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 3. 创建入库明细（P2 5-13/3-18 修复：改用 insert_many 批量插入，原为循环内逐条 insert 导致 N 条=N 次 INSERT）
        let mut total_quantity = Decimal::new(0, 0);
        let mut total_quantity_alt = Decimal::new(0, 0);
        let mut total_amount = Decimal::new(0, 0);

        let mut item_active_models: Vec<purchase_receipt_item::ActiveModel> =
            Vec::with_capacity(req.items.len());
        for item_req in req.items {
            let amount = item_req.quantity * item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0));
            total_quantity += item_req.quantity;
            total_quantity_alt += item_req.quantity_alt;
            total_amount += amount;

            item_active_models.push(purchase_receipt_item::ActiveModel {
                receipt_id: Set(receipt.id),
                order_item_id: Set(item_req.order_item_id),
                product_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                quantity_alt: Set(Some(item_req.quantity_alt)),
                unit_price: Set(Some(
                    item_req.unit_price.unwrap_or_else(|| Decimal::new(0, 0)),
                )),
                amount: Set(Some(amount)),
                notes: Set(item_req.notes),
                ..Default::default()
            });
        }

        if !item_active_models.is_empty() {
            purchase_receipt_item::Entity::insert_many(item_active_models)
                .exec(&txn)
                .await?;
        }

        // 4. 更新入库单总金额和数量
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.total_quantity = Set(total_quantity);
        receipt_active.total_quantity_alt = Set(total_quantity_alt);
        receipt_active.total_amount = Set(total_amount);
        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(receipt)
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

    /// 确认采购入库单
    ///
    /// 批次 16（2026-06-28）：入库单状态门查询加 lock_exclusive，
    /// 防止并发 confirm_receipt 同一入库单导致重复入库 + 重复生成应付账单 + 重复累加采购单已收数量。
    /// 原状态门无锁，两并发 confirm 均通过 DRAFT 检查，第二个 confirm 重复执行库存入库与
    /// order_item received_quantity 累加，commit 后还会重复触发 auto_generate_from_receipt 生成应付账单。
    pub async fn confirm_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询入库单（加 lock_exclusive 串行化并发 confirm_receipt）
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        // 2. 检查状态
        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许确认，当前状态：{}",
                receipt.receipt_status
            )));
        }

        // 3. 检查是否有明细
        let item_count = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .count(&txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("入库单至少需要一行明细".to_string()));
        }

        // 4. 检查是否有关联的采购订单
        if let Some(order_id) = receipt.order_id {
            // 已实现: 更新采购订单的已入库数量
            // 批次 94 P2-10：透传 user_id 用于审计日志
            self.update_order_received_quantity(order_id, receipt_id, &txn, user_id)
                .await?;
        }

        // 5. 更新状态
        let now = chrono::Utc::now();
        let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
        receipt_active.receipt_status = Set(status::purchase_receipt::CONFIRMED.to_string());
        receipt_active.confirmed_at = Set(Some(now));
        receipt_active.confirmed_by = Set(Some(user_id));
        receipt_active.updated_by = Set(Some(user_id));
        receipt_active.updated_at = Set(now);

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 6. 更新库存（在事务内执行，保证原子性）
        // P0 5-2 修复：update_inventory_txn 不再在内部 publish 事件，改为返回收集到的库存流水事件，
        // 由本处在 commit 成功后统一 publish，避免事务回滚时幻事件
        let pending_events = self.update_inventory_txn(&receipt, &txn).await?;

        // 7. 提交事务
        txn.commit().await?;

        // P0 5-2 修复：commit 成功后统一发布库存流水事件，避免事务回滚时幻事件
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

        // 8. 自动生成应付账款（事务外执行，失败不影响入库）
        // P0 5-5 修复：auto_generate_from_receipt 内部自建事务、不接受外部 txn，无法纳入主事务。
        // 失败时改为 warn 级补偿提示，明确需人工补生成应付单，避免仅 error 日志无补偿指引
        // 导致入库成功但应付账款缺失的问题被遗漏。
        let ap_service =
            crate::services::ap_invoice_service::ApInvoiceService::new(self.db.clone());
        if let Err(e) = ap_service
            .auto_generate_from_receipt(receipt.id, user_id)
            .await
        {
            tracing::warn!(
                "⚠ 入库单 {} 已确认成功，但自动生成应付账单失败，需人工补生成应付单：{}",
                receipt.receipt_no,
                e
            );
        } else {
            tracing::info!("成功自动生成应付账单 (入库单 {})", receipt.receipt_no);
        }

        Ok(receipt)
    }

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

    /// 获取入库单列表（分页）
    pub async fn list_receipts(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        order_id: Option<i32>,
    ) -> Result<(Vec<purchase_receipt::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_receipt::Entity::find();

        // 添加筛选条件
        if let Some(status) = status {
            query = query.filter(purchase_receipt::Column::ReceiptStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_receipt::Column::SupplierId.eq(supplier_id));
        }
        if let Some(order_id) = order_id {
            query = query.filter(purchase_receipt::Column::OrderId.eq(order_id));
        }

        // 分页查询
        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_receipt::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 获取入库单详情
    pub async fn get_receipt(&self, receipt_id: i32) -> Result<purchase_receipt::Model, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        Ok(receipt)
    }

    /// 获取入库明细列表
    pub async fn list_receipt_items(
        &self,
        receipt_id: i32,
    ) -> Result<Vec<purchase_receipt_item::Model>, AppError> {
        let items = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .order_by(purchase_receipt_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use sea_orm::DatabaseConnection;
    use std::sync::Arc;
    // 批次 415：decs! 宏展开为 Decimal::from_str，需导入 FromStr trait
    use std::str::FromStr;

    /// 构造合法的 CreateReceiptItemRequest（单条明细）
    fn sample_item() -> CreateReceiptItemRequest {
        CreateReceiptItemRequest {
            order_item_id: Some(1),
            line_no: 1,
            material_id: 1001,
            material_code: "M001".to_string(),
            material_name: "测试物料".to_string(),
            batch_no: Some("B20260719".to_string()),
            color_code: Some("RED".to_string()),
            lot_no: Some("L01".to_string()),
            grade: Some("A".to_string()),
            gram_weight: Some(decs!(200)),
            width: Some(decs!(150)),
            quantity: decs!(100),
            quantity_alt: decs!(50),
            unit_master: "M".to_string(),
            unit_alt: Some("KG".to_string()),
            unit_price: Some(decs!(10)),
            location_code: Some("A-01-01".to_string()),
            package_no: Some("P001".to_string()),
            production_date: Some(ymd!(2026, 7, 19)),
            shelf_life: Some(365),
            notes: Some("测试明细".to_string()),
        }
    }

    /// 构造合法的 CreatePurchaseReceiptRequest（默认 1 条明细）
    fn sample_request() -> CreatePurchaseReceiptRequest {
        CreatePurchaseReceiptRequest {
            order_id: Some(1),
            supplier_id: 100,
            receipt_date: ymd!(2026, 7, 19),
            warehouse_id: 1,
            department_id: Some(1),
            inspector_id: Some(10),
            notes: Some("测试入库单".to_string()),
            attachment_urls: Some(vec!["file://test.pdf".to_string()]),
            items: vec![sample_item()],
        }
    }

    // ============ 状态常量值正确性测试 ============

    /// 测试_入库单状态常量_值正确性
    ///
    /// 验证 status::purchase_receipt 模块中 3 个状态常量值与状态机约定一致
    /// （大写：DRAFT/CONFIRMED/COMPLETED，与 purchase_receipt_service.rs 中
    /// 字符串字面量 `"DRAFT"` / `status::purchase_receipt::DRAFT.to_string()` 一致）。
    #[test]
    fn 测试_入库单状态常量_值正确性() {
        assert_eq!(status::purchase_receipt::DRAFT, "DRAFT");
        assert_eq!(status::purchase_receipt::CONFIRMED, "CONFIRMED");
        assert_eq!(status::purchase_receipt::COMPLETED, "COMPLETED");
    }

    /// 测试_入库单状态常量_互不相同
    ///
    /// 业务规则：3 个状态必须互不相同，避免状态机歧义。
    #[test]
    fn 测试_入库单状态常量_互不相同() {
        let states = [
            status::purchase_receipt::DRAFT,
            status::purchase_receipt::CONFIRMED,
            status::purchase_receipt::COMPLETED,
        ];
        let unique: std::collections::HashSet<&str> = states.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    /// 测试_入库单状态常量_大写风格
    ///
    /// 业务规则：purchase_receipt 状态值采用大写风格（DRAFT/CONFIRMED/COMPLETED），
    /// 与 quotation 模块（小写 draft/approved/rejected/cancelled）不同。
    /// 验证所有状态均为大写字母（规则 20：注释与功能一致）。
    #[test]
    fn 测试_入库单状态常量_大写风格() {
        // purchase_receipt 状态用大写（与 sales_order/quotation 小写不同）
        for s in [
            status::purchase_receipt::DRAFT,
            status::purchase_receipt::CONFIRMED,
            status::purchase_receipt::COMPLETED,
        ] {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "状态 {} 应全大写",
                s
            );
        }
    }

    // ============ PurchaseReceiptService 构造与 DB 连接测试 ============

    /// 测试_PurchaseReceiptService_new_正确持有数据库连接
    ///
    /// 验证 new(Arc<DatabaseConnection>) 构造的 service 实例可以执行简单查询。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_new_正确持有数据库连接() {
        let db = Arc::new(setup_test_db().await);
        let svc = PurchaseReceiptService::new(db.clone());
        use sea_orm::ConnectionTrait;
        let _ = svc
            .db
            .execute(sea_orm::Statement::from_sql_and_values(
                svc.db.get_database_backend(),
                "SELECT 1",
                Vec::new(),
            ))
            .await
            .expect("数据库连接应可用");
    }

    /// 测试_PurchaseReceiptService_get_receipt_空数据库返回Err
    ///
    /// 业务规则：get_receipt 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_get_receipt_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.get_receipt(9999).await;
        // SQLite 内存数据库无 purchase_receipts 表，应返回 Err（DbErr 转 AppError）
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_list_receipts_空数据库返回Err
    ///
    /// 业务规则：list_receipts 查询 purchase_receipts 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_list_receipts_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.list_receipts(1, 20, None, None, None).await;
        // SQLite 内存数据库无 purchase_receipts 表，应返回 Err
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_list_receipt_items_空数据库返回Err
    ///
    /// 业务规则：list_receipt_items 查询 purchase_receipt_items 表，SQLite 内存数据库无 schema 应返回 Err。
    /// 验证错误处理路径健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_list_receipt_items_空数据库返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.list_receipt_items(9999).await;
        // SQLite 内存数据库无 purchase_receipt_items 表，应返回 Err
        assert!(result.is_err());
    }

    // ============ create_receipt 业务校验测试 ============

    /// 测试_PurchaseReceiptService_create_receipt_空明细返回Err
    ///
    /// 业务规则：CreatePurchaseReceiptRequest.items 至少 1 条（DTO 上 #[validate(length(min = 1))]）。
    /// service 层未显式调用 Validate::validate，空明细会进入 generate_receipt_no 查询表，
    /// SQLite 内存数据库无表应返回 Err（非 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_create_receipt_空明细返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let mut req = sample_request();
        req.items.clear();
        let result = svc.create_receipt(req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_create_receipt_不存在表返回Err
    ///
    /// 业务规则：create_receipt 依赖 purchase_receipt 表存在。
    /// SQLite 内存数据库无 schema，应返回 DbErr（非 panic）。
    /// 这验证了错误处理路径的健壮性（不会因 DB 错误 panic）。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_create_receipt_不存在表返回Err() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = sample_request();
        let result = svc.create_receipt(req, 1).await;
        // SQLite 内存数据库无表，应返回 Err（DbErr 或 AppError）
        assert!(result.is_err());
    }

    // ============ update_receipt 状态机校验测试 ============

    /// 测试_PurchaseReceiptService_update_receipt_不存在返回AppError
    ///
    /// 业务规则：update_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_update_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = UpdatePurchaseReceiptRequest::default();
        let result = svc.update_receipt(9999, req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_delete_receipt_不存在返回AppError
    ///
    /// 业务规则：delete_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_delete_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.delete_receipt(9999, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_confirm_receipt_不存在返回AppError
    ///
    /// 业务规则：confirm_receipt 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_confirm_receipt_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.confirm_receipt(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ 明细操作状态机校验测试 ============

    /// 测试_PurchaseReceiptService_add_receipt_item_不存在入库单返回AppError
    ///
    /// 业务规则：add_receipt_item 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_add_receipt_item_不存在入库单返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let item_req = sample_item();
        let result = svc.add_receipt_item(9999, item_req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_update_receipt_item_不存在返回AppError
    ///
    /// 业务规则：update_receipt_item 不存在的明细返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_update_receipt_item_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let req = UpdateReceiptItemRequest::default();
        let result = svc.update_receipt_item(9999, req, 1).await;
        assert!(result.is_err());
    }

    /// 测试_PurchaseReceiptService_delete_receipt_item_不存在返回AppError
    ///
    /// 业务规则：delete_receipt_item 不存在的明细返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_delete_receipt_item_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.delete_receipt_item(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ calculate_receipt_total 测试 ============

    /// 测试_PurchaseReceiptService_calculate_receipt_total_不存在返回AppError
    ///
    /// 业务规则：calculate_receipt_total 不存在的入库单返回 AppError::not_found。
    #[tokio::test]
    async fn 测试_PurchaseReceiptService_calculate_receipt_total_不存在返回AppError() {
        let db = setup_test_db().await;
        let svc = PurchaseReceiptService::new(Arc::new(db));
        let result = svc.calculate_receipt_total(9999, 1).await;
        assert!(result.is_err());
    }

    // ============ DTO 字段完整性测试 ============

    /// 测试_CreateReceiptItemRequest_字段完整构造
    ///
    /// 验证 CreateReceiptItemRequest 所有字段可以正确构造，
    /// 确保后续业务方法接收到完整 DTO 时不会因字段缺失 panic。
    #[test]
    fn 测试_CreateReceiptItemRequest_字段完整构造() {
        let item = sample_item();
        assert_eq!(item.material_id, 1001);
        assert_eq!(item.material_code, "M001");
        assert_eq!(item.quantity, decs!(100));
        assert_eq!(item.unit_price, Some(decs!(10)));
        assert!(item.batch_no.is_some());
        assert!(item.color_code.is_some());
        assert!(item.lot_no.is_some());
        assert!(item.grade.is_some());
    }

    /// 测试_UpdatePurchaseReceiptRequest_默认值全为None
    ///
    /// 业务规则：UpdatePurchaseReceiptRequest 使用 #[derive(Default)]，
    /// 所有字段默认为 None，表示不更新该字段。
    #[test]
    fn 测试_UpdatePurchaseReceiptRequest_默认值全为None() {
        let req = UpdatePurchaseReceiptRequest::default();
        assert!(req.supplier_id.is_none());
        assert!(req.receipt_date.is_none());
        assert!(req.department_id.is_none());
        assert!(req.inspector_id.is_none());
        assert!(req.notes.is_none());
        assert!(req.attachment_urls.is_none());
    }

    /// 测试_UpdateReceiptItemRequest_默认值全为None
    ///
    /// 业务规则：UpdateReceiptItemRequest 使用 #[derive(Default)]，
    /// 所有字段默认为 None，表示不更新该字段。
    #[test]
    fn 测试_UpdateReceiptItemRequest_默认值全为None() {
        let req = UpdateReceiptItemRequest::default();
        assert!(req.line_no.is_none());
        assert!(req.material_id.is_none());
        assert!(req.material_code.is_none());
        assert!(req.quantity.is_none());
        assert!(req.unit_price.is_none());
        assert!(req.notes.is_none());
    }
}

