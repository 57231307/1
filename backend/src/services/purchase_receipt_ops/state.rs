//! 采购入库-状态流转子模块（purchase_receipt_ops/state）
//!
//! 批次 D10 拆分：从原 `purchase_receipt_service.rs` 迁移。
//! 包含 `PurchaseReceiptService` 的状态流转方法 + helper：
//! - `confirm_receipt`：确认入库单（DRAFT → COMPLETED），事务内完成库存入库与订单已收数量推进，
//!   commit 后发布事件 + 自动生成应付账单
//! - `lock_and_validate_receipt_txn`：锁定入库单并校验状态（私有 helper）
//! - `publish_events_and_generate_ap`：commit 后发布事件并自动生成应付账单（私有 helper）
//! - `concede_receipt`：让步接收（PENDING/REJECTED → CONCESSION_ACCEPTED，理由必填）
//! - `rejudge_receipt`：复检改判（CONCESSION_ACCEPTED → PASSED/REJECTED，结论对齐
//!   质检结论权威词表，理由必填，改判次数累加）
//! - `require_trimmed_reason`：理由必填校验（私有 helper，VALIDATION_ERROR 族）
//!
//! 跨模块调用：
//! - `confirm_receipt` 调用 `purchase_receipt_private` 中的 update_order_received_quantity / update_inventory_txn（已 `pub`，跨 impl 块可访问）
//! - `confirm_receipt` 调用 facade 的纯函数 `build_completed_receipt_active_model`（`pub(crate)`）

use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
    TransactionTrait,
};

use crate::models::{purchase_receipt, purchase_receipt_item, status};
use crate::services::event_bus::EVENT_BUS;
use crate::services::purchase_receipt_dto::{ConcedeReceiptRequest, RejudgeReceiptRequest};
use crate::services::purchase_receipt_service::PurchaseReceiptService;
use crate::services::supplier_blacklist_service::SupplierBlacklistService;
use crate::utils::error::AppError;

impl PurchaseReceiptService {
    /// 确认采购入库单
    /// 批次 16（2026-06-28）：入库单状态门查询加 lock_exclusive，；防止并发 confirm_receipt 同一入库单导致重复入库 + 重复生成应付账单 + 重复累加采购单已收数量。；原状态门无锁，两并发 confirm 均通过 DRAFT 检查，第二个 confirm 重复执行库存入库与；order_item received_quantity 累加，commit 后还会重复触发 auto_generate_from_receipt 生成应付账单。
    pub async fn confirm_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 锁定并校验入库单（DRAFT + 明细数 > 0），串行化并发 confirm
        let receipt = self.lock_and_validate_receipt_txn(receipt_id, &txn).await?;

        // 采购门控：确认收货前再次校验供应商是否在有效黑名单中（事务内执行，消除 TOCTOU）
        SupplierBlacklistService::new(self.db.clone())
            .check_supplier_not_blacklisted(&txn, receipt.supplier_id)
            .await?;

        // 质检门控（合格方可入库）：inspection_status 非 PASSED 的收货单不得推进库存与
        // PO 进度，判定必须先于 update_order_received_quantity / update_inventory_txn /
        // COMPLETED 状态写入；与应付结算复用同一判定入口
        // （PurchaseReceiptService::ensure_receipt_inspection_allows_flow，判定依据与
        // PENDING/REJECTED/词表外三族的裁定见该函数文档），本处不另写第二套比较。
        // 被拒时事务未写入任何行，drop 即整体回滚（零漂移由契约测试钉住）。
        Self::ensure_receipt_inspection_allows_flow(&receipt, "确认入库")?;

        // 关联采购单时更新已收数量，并在**同一事务**内回写实际到货日
        // （决策定案 #7：actual_delivery_date = 该单已确认收货的最大 receipt_date；
        // 回写失败随本事务整体回滚，不允许进度与到货日半成功）
        if let Some(order_id) = receipt.order_id {
            self.update_order_received_quantity(
                order_id,
                receipt_id,
                receipt.receipt_date,
                &txn,
                user_id,
            )
            .await?;
        }

        // 事务内更新库存（入库明细口径：色号/缸号/批次/等级/克重/门幅）
        let pending_events = self.update_inventory_txn(&receipt, &txn).await?;

        // 库存与订单已收数量在同一事务内落账，落账成功即收货终态 COMPLETED
        let receipt_active = Self::build_completed_receipt_active_model(receipt, user_id);
        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            receipt_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        // commit 后发布事件并自动生成应付账单
        self.publish_events_and_generate_ap(&receipt, pending_events, user_id)
            .await;

        Ok(receipt)
    }

    /// 锁定入库单并校验状态为 DRAFT 且明细数 > 0
    async fn lock_and_validate_receipt_txn(
        &self,
        receipt_id: i32,
        txn: &sea_orm::DatabaseTransaction,
    ) -> Result<purchase_receipt::Model, AppError> {
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business(format!(
                "入库单状态不允许确认，当前状态：{}",
                receipt.receipt_status
            )));
        }

        let item_count = purchase_receipt_item::Entity::find()
            .filter(purchase_receipt_item::Column::ReceiptId.eq(receipt_id))
            .count(txn)
            .await?;

        if item_count == 0 {
            return Err(AppError::business("入库单至少需要一行明细".to_string()));
        }

        Ok(receipt)
    }

    /// commit 后发布库存事件并自动生成应付账单（失败仅告警不阻塞）
    async fn publish_events_and_generate_ap(
        &self,
        receipt: &purchase_receipt::Model,
        pending_events: Vec<crate::services::event_bus::BusinessEvent>,
        user_id: i32,
    ) {
        for ev in pending_events {
            EVENT_BUS.publish(ev);
        }

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
    }
}

/// 让步接收 / 复检改判状态流转（权威词表 models/status/purchase_inventory.rs
/// 的 purchase_receipt_inspection；改判目标取值对齐 purchase_inspection_result）
impl PurchaseReceiptService {
    /// 理由必填校验（让步接收与复检改判共用）：`None`、空串、纯空白一律拒
    /// `VALIDATION_ERROR`（字段校验归 VALIDATION，本仓裁定），合法值返回 trim 后
    /// 原文用于落专用真实列（禁挪用 notes/remarks）。
    ///
    /// 调用方：`concede_receipt` / `rejudge_receipt`。文案只说"该做什么"，
    /// 不含记录 ID、不外泄内部结构。
    fn require_trimmed_reason(
        reason: Option<&str>,
        action_label: &str,
    ) -> Result<String, AppError> {
        let filled = reason
            .map(str::trim)
            .filter(|r| !r.is_empty())
            .ok_or_else(|| {
                AppError::validation_displayable(format!("请先填写{action_label}理由再提交"))
            })?;
        Ok(filled.to_string())
    }

    /// 让步接收：把收货单检验状态转入 CONCESSION_ACCEPTED（特采降级接收）。
    ///
    /// 合法前驱（以词表与 DB CHECK 现值域为准的真实态）：
    /// - PENDING：收货时即选让步（用户终裁"收货时可选让步接收"）；
    /// - REJECTED：质检判不合格后特采（"不合格特采/降级接收"语义）。
    /// 其余前驱（PASSED/已处于让步态）⇒ `BUSINESS_ERROR`（状态门归 BUSINESS，本仓裁定）。
    /// 单据门控：仅 receipt_status=DRAFT 可操作——已确认入库（COMPLETED）必然已经
    /// 质检合格放行，事后转让步属越权状态流转。
    ///
    /// 留痕：理由/操作人/时间落 purchase_receipt.concession_reason/concession_by/
    /// concession_at 专用列（操作人取会话 user_id，请求体不承载身份）；
    /// `update_with_audit` 同事务写 audit_log 前后全行快照（既有审计范式）。
    /// 事务内 lock_exclusive 串行化并发状态变更（口径同 `confirm_receipt`），
    /// 任一步失败 `?` 上抛、整体回滚，不允许半成功。
    pub async fn concede_receipt(
        &self,
        receipt_id: i32,
        req: ConcedeReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let reason = Self::require_trimmed_reason(req.reason.as_deref(), "让步接收")?;

        let txn = (*self.db).begin().await?;
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business_displayable(
                "已确认入库的收货单不允许办理让步接收",
            ));
        }
        let inspection = receipt.inspection_status.as_str();
        let predecessor_legal = inspection == status::purchase_receipt_inspection::PENDING
            || inspection == status::purchase_receipt_inspection::REJECTED;
        if !predecessor_legal {
            return Err(AppError::business_displayable(format!(
                "当前质检状态「{inspection}」不允许让步接收：仅待检或质检不合格的收货单可办理"
            )));
        }

        let mut active: purchase_receipt::ActiveModel = receipt.into();
        active.inspection_status =
            Set(status::purchase_receipt_inspection::CONCESSION_ACCEPTED.to_string());
        active.concession_reason = Set(Some(reason));
        active.concession_by = Set(Some(user_id));
        active.concession_at = Set(Some(chrono::Utc::now()));
        active.updated_by = Set(Some(user_id));
        active.updated_at = Set(chrono::Utc::now());

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;

        tracing::info!(
            receipt_id = receipt.id,
            user_id,
            "收货单已让步接收（检验状态 CONCESSION_ACCEPTED，入库/结算门控维持不放行）"
        );
        Ok(receipt)
    }

    /// 复检改判：把处于让步态的同一张收货单改判为 PASSED / REJECTED（显式端点，
    /// 不再靠"新建一张质检单"隐式覆写）。
    ///
    /// 改判结论取值对齐既有质检结论权威词表 purchase_inspection_result
    /// （pass/fail/partial），目标状态经同源映射 `to_receipt_inspection_status`
    /// （pass→PASSED，fail/partial→REJECTED）——词表外结论 ⇒ `VALIDATION_ERROR`
    /// （字段校验归 VALIDATION）；前驱非 CONCESSION_ACCEPTED ⇒ `BUSINESS_ERROR`
    /// （R1：离开让步态只允许→合格/不合格）。单据门控同样要求 receipt_status=DRAFT。
    ///
    /// 留痕：改判理由/操作人/时间落 rejudge_reason/rejudge_by/rejudge_at 专用列，
    /// rejudge_count 累加；audit_log 前后快照记录改判前后状态（update_with_audit，
    /// 与让步接收同事务范式）。
    pub async fn rejudge_receipt(
        &self,
        receipt_id: i32,
        req: RejudgeReceiptRequest,
        user_id: i32,
    ) -> Result<purchase_receipt::Model, AppError> {
        let reason = Self::require_trimmed_reason(req.reason.as_deref(), "复检改判")?;
        let target = req
            .inspection_result
            .as_deref()
            .and_then(status::purchase_inventory::purchase_inspection_result::to_receipt_inspection_status)
            .ok_or_else(|| {
                AppError::validation_displayable(format!(
                    "改判结论只能是{}之一",
                    status::purchase_inventory::purchase_inspection_result::ALL.join("/")
                ))
            })?;

        let txn = (*self.db).begin().await?;
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;

        if receipt.receipt_status != status::purchase_receipt::DRAFT {
            return Err(AppError::business_displayable(
                "已确认入库的收货单不允许复检改判",
            ));
        }
        if receipt.inspection_status != status::purchase_receipt_inspection::CONCESSION_ACCEPTED {
            return Err(AppError::business_displayable(
                "仅处于让步接收状态的收货单可复检改判为合格或不合格",
            ));
        }

        let rejudge_count = receipt.rejudge_count + 1;
        let mut active: purchase_receipt::ActiveModel = receipt.into();
        active.inspection_status = Set(target.to_string());
        active.rejudge_reason = Set(Some(reason));
        active.rejudge_by = Set(Some(user_id));
        active.rejudge_at = Set(Some(chrono::Utc::now()));
        active.rejudge_count = Set(rejudge_count);
        active.updated_by = Set(Some(user_id));
        active.updated_at = Set(chrono::Utc::now());

        let receipt = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;

        tracing::info!(
            receipt_id = receipt.id,
            user_id,
            new_inspection_status = %receipt.inspection_status,
            rejudge_count = receipt.rejudge_count,
            "收货单复检改判完成（改判前后状态与操作人已落审计快照）"
        );
        Ok(receipt)
    }
}
