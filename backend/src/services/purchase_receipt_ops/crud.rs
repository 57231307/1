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

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};

use crate::models::{purchase_order_item, purchase_receipt, purchase_receipt_item, status};
use crate::services::purchase_receipt_dto::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, UpdatePurchaseReceiptRequest,
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
        // 四维必入准入：产品/批次缺失的明细在建单期即拒绝，整单不落库。
        // 色号/缸号的「染色布必填」口径依赖白坯布共享判定（尚未落地），见
        // validate_receipt_item_dimensions 内 TODO。
        for item in &req.items {
            Self::validate_receipt_item_dimensions(item)?;
        }

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

    /// 入库明细四维准入校验（建单/追加明细期即拒绝）。
    ///
    /// 当前可无条件强制的两维：产品与批次——缺任一即返回定位到行的业务错误，
    /// 不允许靠库存行的 `DEFAULT ''` 把缺失维度落空串。
    ///
    /// 色号/缸号「染色布必填」需先判定白坯布 vs 染色布（色号为空 ⇒ 白坯布 ⇒ 免缸号；
    /// 色号非空 ⇒ 缸号与批次必填）。该判定口径由采购/库存共用，应调用共享函数
    /// `crate::services::inv` 侧统一落地的「白坯布判定」（doto iter31 第 3/5 条，本域同事负责，
    /// 尚未落地）。TODO：共享判定落地后，在此按色号是否为空追加
    /// 「染色布缺缸号 ⇒ 拒绝」分支，切勿在本文件按色号名称嗅探白色。
    pub(crate) fn validate_receipt_item_dimensions(
        item: &CreateReceiptItemRequest,
    ) -> Result<(), AppError> {
        if item.material_id <= 0 {
            return Err(AppError::business(format!(
                "入库单第 {} 行缺少有效的产品（material_id 非法），拒绝建单",
                item.line_no
            )));
        }
        let batch_missing = item
            .batch_no
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty();
        if batch_missing {
            return Err(AppError::business(format!(
                "入库单第 {} 行（产品 {}）缺少批次号，四维不全，拒绝建单",
                item.line_no, item.material_id
            )));
        }
        Ok(())
    }

    /// 按订单未收容量把入库明细匹配到订单明细行（纯决策，无 DB）。
    ///
    /// 返回 `(入库明细 id, 订单明细 id)` 分配表；任一明细触发以下情形即返回业务错误，
    /// 调用方据此整单拒绝（不落库、不改进度、不留部分成功）：
    /// - 显式指定的订单明细不属于本采购订单；
    /// - 明细产品完全不在订单中（产品对不上 ⇒ 硬拒绝）；
    /// - 同产品订单行未收数量不足以容纳本行入库量（超收 ⇒ 当前无容差配置即拒绝）。
    pub(crate) fn plan_order_item_links(
        order_id: i32,
        order_items: &[purchase_order_item::Model],
        receipt_items: &[purchase_receipt_item::Model],
    ) -> Result<Vec<(i32, i32)>, AppError> {
        use std::collections::HashMap;
        // 每个订单明细行的未收容量 = 采购数量 - 已收数量
        let capacity: HashMap<i32, Decimal> = order_items
            .iter()
            .map(|oi| (oi.id, oi.quantity - oi.received_quantity))
            .collect();
        let mut used: HashMap<i32, Decimal> = HashMap::new();
        let mut assignments: Vec<(i32, i32)> = Vec::new();

        for item in receipt_items {
            let target_id: i32 = match item.order_item_id {
                Some(declared) => {
                    if !capacity.contains_key(&declared) {
                        return Err(AppError::business(format!(
                            "入库单第 {} 行指定的采购订单明细 {} 不属于采购订单 {}，拒绝建单",
                            item.line_no, declared, order_id
                        )));
                    }
                    declared
                }
                None => {
                    // 未显式指定：贪心匹配同产品、且剩余可收容量足以容纳本行整量的订单行。
                    // 一张入库明细只对应一个订单明细行（累加口径即如此），故不允许跨行拆分。
                    let matched = order_items.iter().find(|oi| {
                        oi.product_id == item.product_id
                            && (capacity[&oi.id]
                                - used.get(&oi.id).copied().unwrap_or(Decimal::ZERO))
                                >= item.quantity
                    });
                    match matched {
                        Some(oi) => oi.id,
                        None => {
                            // 区分「产品对不上」与「超收」：产品是否出现在订单明细中
                            if order_items
                                .iter()
                                .any(|oi| oi.product_id == item.product_id)
                            {
                                return Err(AppError::business(format!(
                                    "入库单第 {} 行产品 {} 的入库量 {} 超过采购订单 {} 该产品的未收数量（暂无超收容差配置），拒绝建单",
                                    item.line_no, item.product_id, item.quantity, order_id
                                )));
                            } else {
                                return Err(AppError::business(format!(
                                    "入库单第 {} 行产品 {} 不在采购订单 {} 的明细中，入库与订单产品不符，拒绝建单",
                                    item.line_no, item.product_id, order_id
                                )));
                            }
                        }
                    }
                }
            };

            let cap = capacity.get(&target_id).copied().unwrap_or(Decimal::ZERO);
            let entry = used.entry(target_id).or_insert(Decimal::ZERO);
            *entry += item.quantity;
            if *entry > cap {
                return Err(AppError::business(format!(
                    "入库单第 {} 行产品 {} 累计入库量超过采购订单明细 {} 的未收数量（暂无超收容差配置），拒绝建单",
                    item.line_no, item.product_id, target_id
                )));
            }
            assignments.push((item.id, target_id));
        }
        Ok(assignments)
    }

    /// 把入库明细挂到被入的采购订单明细行（事务内调用；`pub(crate)`：`add_receipt_item`
    /// 与 `update_receipt_item` 也需在明细写入后挂接，否则经明细端点增改的行永不累加订单进度）
    ///
    /// 确认入库按入库明细的 `order_item_id` 累加订单明细 received_quantity 并据此推进
    /// 订单状态（全部收货 COMPLETED / 部分收货 PARTIAL_RECEIVED）。
    ///
    /// 挂接决策为 fail-closed：产品对不上订单、超收、或指定明细不属于本订单时返回业务错误
    /// 并整单拒绝（由 create/add/update 所在事务回滚，货物不入、进度不动），
    /// 不再「货物照入 + 订单进度不累加 + 只记错误日志」静默放行。
    pub(crate) async fn link_receipt_items_to_order_items(
        txn: &sea_orm::DatabaseTransaction,
        receipt_id: i32,
        order_id: i32,
    ) -> Result<(), AppError> {
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

        // 决策失败即整单拒绝（此处尚未写入，调用方事务回滚保证无部分成功中间态）
        let assignments = Self::plan_order_item_links(order_id, &order_items, &receipt_items)?;

        let mut assign_map: std::collections::HashMap<i32, i32> = assignments.into_iter().collect();
        for item in receipt_items {
            if let Some(target) = assign_map.remove(&item.id) {
                if item.order_item_id != Some(target) {
                    let active = purchase_receipt_item::ActiveModel {
                        id: Set(item.id),
                        order_item_id: Set(Some(target)),
                        ..Default::default()
                    };
                    purchase_receipt_item::Entity::update(active)
                        .exec(txn)
                        .await?;
                }
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

#[cfg(test)]
mod fail_closed_tests {
    //! 采购收货入库「拒绝建单 + 四维准入」决策层单元测试（真实接入生产代码路径）。
    //!
    //! 覆盖三条拍板规则的纯决策：
    //! ①产品与订单不符 / 超收 → plan_order_item_links 返回业务错误（整单拒绝依据）；
    //! ②缺产品 / 缺批次 → validate_receipt_item_dimensions 返回业务错误；
    //! ③齐套 → plan 正确分配到订单行，且 build_receipt_item_active_model 把四维如实写入。
    //! 端到端「库存无新增行 / 库存四列等于入参 / 进度累加」的真实落库行为由
    //! frontend/e2e/flow/01-p2p.spec.ts 对真实 PostgreSQL 链路钉住。

    use super::*;
    use chrono::Utc;
    use sea_orm::ActiveValue;
    use std::str::FromStr;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn order_item(
        id: i32,
        order_id: i32,
        product_id: i32,
        qty: &str,
        received: &str,
    ) -> purchase_order_item::Model {
        purchase_order_item::Model {
            id,
            order_id,
            line_no: 1,
            product_id,
            quantity: dec(qty),
            quantity_alt: Decimal::ZERO,
            unit_price: dec("10"),
            unit_price_foreign: dec("10"),
            discount_percent: Decimal::ZERO,
            tax_percent: dec("13"),
            subtotal: dec("0"),
            tax_amount: dec("0"),
            discount_amount: dec("0"),
            total_amount: dec("0"),
            received_quantity: dec(received),
            received_quantity_alt: Decimal::ZERO,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            color_code: None,
            lot_no: None,
            batch_no: None,
        }
    }

    fn receipt_item(
        id: i32,
        line_no: i32,
        product_id: i32,
        qty: &str,
        declared: Option<i32>,
    ) -> purchase_receipt_item::Model {
        purchase_receipt_item::Model {
            id,
            receipt_id: 1,
            order_item_id: declared,
            line_no,
            product_id,
            material_code: "M".to_string(),
            material_name: "n".to_string(),
            batch_no: Some("B001".to_string()),
            color_code: Some("RED".to_string()),
            lot_no: Some("L01".to_string()),
            grade: Some("A".to_string()),
            gram_weight: None,
            width: None,
            quantity: dec(qty),
            quantity_alt: Some(Decimal::ZERO),
            unit_master: "M".to_string(),
            unit_alt: None,
            unit_price: Some(dec("10")),
            amount: Some(dec("0")),
            location_code: None,
            piece_no: None,
            package_no: None,
            production_date: None,
            shelf_life: None,
            notes: None,
            created_at: None,
            internal_dye_lot_id: None,
            internal_dye_lot_no: None,
            internal_piece_ids: None,
            internal_piece_nos: None,
            supplier_dye_lot_no: None,
            supplier_piece_nos: None,
            batch_conversion_log_id: None,
        }
    }

    fn item_req(
        line_no: i32,
        material_id: i32,
        batch_no: Option<&str>,
    ) -> CreateReceiptItemRequest {
        CreateReceiptItemRequest {
            order_item_id: None,
            line_no,
            material_id,
            material_code: "M".to_string(),
            material_name: "n".to_string(),
            batch_no: batch_no.map(str::to_string),
            color_code: Some("RED".to_string()),
            lot_no: Some("L01".to_string()),
            piece_no: None,
            grade: Some("A".to_string()),
            gram_weight: None,
            width: None,
            quantity: dec("100"),
            quantity_alt: Decimal::ZERO,
            unit_master: "M".to_string(),
            unit_alt: None,
            unit_price: Some(dec("10")),
            location_code: None,
            package_no: None,
            production_date: None,
            shelf_life: None,
            notes: None,
        }
    }

    // ===== 规则②：四维准入（缺产品/缺批次）建单期即拒绝 =====

    #[test]
    fn missing_batch_is_rejected() {
        // 缺批次 → 业务错误
        let err = PurchaseReceiptService::validate_receipt_item_dimensions(&item_req(1, 500, None))
            .expect_err("缺批次应拒绝");
        assert!(
            matches!(err, AppError::BusinessError(_)),
            "应为业务错误：{:?}",
            err
        );
    }

    #[test]
    fn blank_batch_is_rejected() {
        // 批次为空白串（将触发库存行 DEFAULT '' 落空串）→ 拒绝
        let err = PurchaseReceiptService::validate_receipt_item_dimensions(&item_req(
            2,
            500,
            Some("   "),
        ))
        .expect_err("空白批次应拒绝");
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    #[test]
    fn missing_product_is_rejected() {
        let err =
            PurchaseReceiptService::validate_receipt_item_dimensions(&item_req(1, 0, Some("B001")))
                .expect_err("缺产品应拒绝");
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    #[test]
    fn complete_dimensions_pass() {
        PurchaseReceiptService::validate_receipt_item_dimensions(&item_req(1, 500, Some("B001")))
            .expect("产品+批次齐全应通过准入");
    }

    // ===== 规则①：产品对不上 / 超收 → 整单拒绝决策 =====

    #[test]
    fn product_not_in_order_is_rejected() {
        // 订单只有产品 500；入库明细产品 999 不在订单中 → 硬拒绝
        let order_items = vec![order_item(11, 1, 500, "1000", "0")];
        let receipt_items = vec![receipt_item(21, 1, 999, "100", None)];
        let err = PurchaseReceiptService::plan_order_item_links(1, &order_items, &receipt_items)
            .expect_err("产品与订单不符应拒绝");
        let msg = err.to_string();
        assert!(
            msg.contains("第 1 行") && msg.contains("999"),
            "错误需定位到行/产品：{}",
            msg
        );
        assert!(
            msg.contains("产品") && msg.contains("不在"),
            "应判定为产品不符：{}",
            msg
        );
    }

    #[test]
    fn over_receipt_without_tolerance_is_rejected() {
        // 产品匹配但未收数量不足（超收）→ 当前无容差配置即拒绝，且文案说明超收
        let order_items = vec![order_item(11, 1, 500, "100", "50")]; // 剩余 50
        let receipt_items = vec![receipt_item(21, 3, 500, "80", None)]; // 入 80 > 50
        let err = PurchaseReceiptService::plan_order_item_links(1, &order_items, &receipt_items)
            .expect_err("超收应拒绝");
        let msg = err.to_string();
        assert!(msg.contains("第 3 行"), "应定位到行：{}", msg);
        assert!(msg.contains("超过"), "应说明超收：{}", msg);
        assert!(
            !msg.contains("不在"),
            "产品在场时不得误判为产品不符：{}",
            msg
        );
    }

    #[test]
    fn declared_order_item_not_in_order_is_rejected() {
        let order_items = vec![order_item(11, 1, 500, "1000", "0")];
        // 显式指定了不属于本订单的明细 777
        let receipt_items = vec![receipt_item(21, 1, 500, "10", Some(777))];
        let err = PurchaseReceiptService::plan_order_item_links(1, &order_items, &receipt_items)
            .expect_err("指定明细不属于订单应拒绝");
        assert!(err.to_string().contains("777"));
    }

    // ===== 规则③：齐套 → 正确分配 + 四维如实写入 ActiveModel =====

    #[test]
    fn complete_link_assigns_matching_order_item() {
        let order_items = vec![
            order_item(11, 1, 500, "1000", "0"),
            order_item(12, 1, 600, "1000", "0"),
        ];
        let receipt_items = vec![receipt_item(21, 1, 600, "500", None)];
        let assign = PurchaseReceiptService::plan_order_item_links(1, &order_items, &receipt_items)
            .expect("齐套应通过");
        // 入库行应挂到同产品(600)的订单明细 12，而非首行
        assert_eq!(assign, vec![(21, 12)], "应按产品匹配到正确订单行");
    }

    #[test]
    fn build_receipt_item_persists_four_dimensions() {
        let active = PurchaseReceiptService::build_receipt_item_active_model(
            item_req(1, 500, Some("B001")),
            7,
            dec("1000"),
        );
        let product = match active.product_id {
            ActiveValue::Set(v) => v,
            _ => panic!("product_id 应被如实写入"),
        };
        let batch = match active.batch_no {
            ActiveValue::Set(v) => v,
            _ => panic!("batch_no 应被如实写入"),
        };
        let color = match active.color_code {
            ActiveValue::Set(v) => v,
            _ => panic!("color_code 应被如实写入"),
        };
        let lot = match active.lot_no {
            ActiveValue::Set(v) => v,
            _ => panic!("lot_no 应被如实写入"),
        };
        assert_eq!(product, 500);
        assert_eq!(batch.as_deref(), Some("B001"));
        assert_eq!(color.as_deref(), Some("RED"));
        assert_eq!(lot.as_deref(), Some("L01"));
    }
}
