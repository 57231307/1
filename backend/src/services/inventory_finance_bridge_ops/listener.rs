//! 库存财务桥接事件监听器 impl 子模块（inventory_finance_bridge_ops/listener）
//!
//! 拆分：从原 inventory_finance_bridge_service.rs 迁移事件监听器启动/关闭 + 事件分发处理逻辑。
//! - start_listener：启动 EVENT_BUS 订阅 spawn（含 panic 隔离 catch_unwind）
//! - shutdown_listener：优雅关闭监听器 task（abort detached spawn，幂等）
//! - handle_inventory_event_safe：单事件处理（panic 隔离由调用方 catch_unwind 包裹）
//! - handle_inventory_transaction：交易类型分发到 voucher 子模块的 create_*_voucher
//!
//! BRIDGE_LISTENER_HANDLE 静态句柄仅在本模块使用，随 impl 块一并迁入。

use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::services::inventory_finance_bridge_service::{
    InventoryFinanceBridgeService, VoucherCreateArgs,
};
use crate::utils::error::AppError;
use futures::FutureExt;
use sea_orm::DatabaseConnection;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tracing::{error, info};

/// L-29 修复（批次 373 v13 复审）：库存财务桥接监听器 spawn 句柄
/// 保存句柄以便 shutdown 时 abort，避免 detached task 泄漏
static BRIDGE_LISTENER_HANDLE: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>> =
    std::sync::Mutex::new(None);

impl InventoryFinanceBridgeService {
    /// 启动库存变动事件监听器
    pub fn start_listener(db: Arc<DatabaseConnection>) {
        let mut receiver = EVENT_BUS.subscribe();

        // L-29 修复（批次 373 v13 复审）：保存 spawn 句柄供 shutdown abort
        let listener_handle = tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                // 批次 8（2026-06-28）：单次事件处理 panic 隔离
                // 库存财务桥接监听器 panic 会导致库存交易不再生成会计凭证，
                // 财务报表与库存数据不一致。
                let result = AssertUnwindSafe(Self::handle_inventory_event_safe(
                    db.clone(),
                    event,
                ))
                .catch_unwind()
                .await;
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 库存财务桥接 spawn panic 已被隔离，继续运行（不退出循环）"
                    );
                }
            }
        });

        // L-29 修复（批次 373 v13 复审）：保存句柄到全局 static
        if let Ok(mut guard) = BRIDGE_LISTENER_HANDLE.lock() {
            *guard = Some(listener_handle);
        }
    }

    /// 处理单个库存交易创建事件（panic 隔离由调用方 catch_unwind 包裹）
    ///
    /// 仅处理 `InventoryTransactionCreated` 事件，调用 handle_inventory_transaction 生成会计凭证。
    async fn handle_inventory_event_safe(db: Arc<DatabaseConnection>, event: BusinessEvent) {
        if let BusinessEvent::InventoryTransactionCreated {
            transaction_id,
            transaction_type,
            product_id,
            warehouse_id,
            quantity_meters,
            quantity_kg,
            source_bill_type,
            source_bill_no,
            source_bill_id,
            batch_no,
            color_no,
            created_by,
        } = event
        {
            info!(
                "处理库存交易创建事件: 交易ID={}, 类型={}, 产品ID={}, 仓库ID={}",
                transaction_id, transaction_type, product_id, warehouse_id
            );

            let bridge_service = InventoryFinanceBridgeService::new(db);
            // 批次 337 v10 复审 P3 修复：使用 VoucherCreateArgs 参数对象替代多参数
            let voucher_args = VoucherCreateArgs {
                product_id,
                warehouse_id,
                quantity_meters,
                quantity_kg,
                source_bill_type: source_bill_type.as_deref(),
                source_bill_no: source_bill_no.as_deref(),
                source_bill_id,
                batch_no: &batch_no,
                color_no: &color_no,
                created_by,
            };
            if let Err(e) = bridge_service
                .handle_inventory_transaction(transaction_id, &transaction_type, voucher_args)
                .await
            {
                error!(
                    "处理库存交易事件失败: 交易ID={}, 错误={}",
                    transaction_id, e
                );
            }
        }
    }

    /// L-29 修复（批次 373 v13 复审）：优雅关闭库存财务桥接监听器
    /// abort 后台 spawn task，防止 detached task 泄漏。幂等：多次调用安全。
    pub fn shutdown_listener() {
        let handle = match BRIDGE_LISTENER_HANDLE.lock() {
            Ok(mut guard) => guard.take(),
            Err(e) => {
                tracing::error!(error = %e, "BRIDGE_LISTENER_HANDLE 锁中毒，无法关闭监听器");
                return;
            }
        };
        if let Some(h) = handle {
            h.abort();
            tracing::info!("库存财务桥接监听器 task 已关闭");
        }
    }

    /// 处理库存交易事件，生成相应的会计凭证
    ///
    /// 批次 337 v10 复审 P3 修复：签名从 12 参数改为单一参数对象 `VoucherCreateArgs`，
    /// 消除 `clippy::too_many_arguments` 警告。transaction_type 改为 args 内嵌字段处理
    /// 不再单独传递，通过 match 分发到 5 个 create_*_voucher 私有函数。
    async fn handle_inventory_transaction(
        &self,
        transaction_id: i32,
        transaction_type: &str,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        // B-P1-8 修复（批次 365 v13 复审）：事件幂等处理
        // 重复消费 InventoryTransactionCreated 会导致重复生成会计凭证 + 重复过账，
        // 科目余额累加失真。使用 transaction_id 作为幂等键，处理前检查是否已处理。
        let idempotency_service =
            crate::services::event_idempotency_service::EventIdempotencyService::new(self.db.clone());
        let consumer_id = "inventory_finance_bridge";
        let event_key = format!("inventory_txn:{}", transaction_id);
        let event_type = "InventoryTransactionCreated";
        let should_process = idempotency_service
            .try_mark_processed(consumer_id, &event_key, event_type)
            .await?;
        if !should_process {
            info!(
                transaction_id = transaction_id,
                transaction_type = transaction_type,
                "库存交易事件已处理，幂等跳过凭证生成"
            );
            return Ok(());
        }

        // 根据交易类型生成不同的凭证
        match transaction_type {
            "PURCHASE_RECEIPT" => {
                // 采购入库凭证：借：库存商品 / 贷：应付账款
                self.create_purchase_receipt_voucher(args).await?;
            }
            "PURCHASE_RETURN" => {
                // 批次 356 v13 复审 B-P0-5 修复：采购退货凭证
                // 借：应付账款（红字） / 贷：库存商品（红字）
                self.create_purchase_return_voucher(args).await?;
            }
            "SALES_DELIVERY" => {
                // 销售出库凭证：借：主营业务成本 / 贷：库存商品
                self.create_sales_delivery_voucher(args).await?;
            }
            "SALES_RETURN" => {
                // 批次 356 v13 复审 B-P0-6 修复：销售退货凭证
                // 借：库存商品 / 贷：主营业务成本（红字反转）
                self.create_sales_return_voucher(args).await?;
            }
            "INVENTORY_ADJUSTMENT" => {
                // 库存调整凭证
                self.create_inventory_adjustment_voucher(args).await?;
            }
            "PRODUCTION_RECEIPT" | "PRODUCTION_OUTPUT" => {
                // 生产入库凭证：借：库存商品 / 贷：生产成本
                // 批次 356 v13 复审 B-P0-4 修复：兼容 PRODUCTION_OUTPUT 事件类型
                self.create_production_receipt_voucher(args).await?;
            }
            "PRODUCTION_ISSUE" | "PRODUCTION_CONSUMPTION" => {
                // 生产领料凭证：借：生产成本 / 贷：库存商品
                // 批次 356 v13 复审 B-P0-4 修复：兼容 PRODUCTION_CONSUMPTION 事件类型
                self.create_production_issue_voucher(args).await?;
            }
            _ => {
                info!("未处理的库存交易类型: {}", transaction_type);
            }
        }

        Ok(())
    }
}
