//! 库存财务桥接事件监听器 impl 子模块（inventory_finance_bridge_ops/listener）
//!
//! 拆分：从原 inventory_finance_bridge_service.rs 迁移事件监听器启动/关闭 + 事件分发处理逻辑。
//! - start_listener：启动 EVENT_BUS 订阅 spawn（含 panic 隔离 catch_unwind）
//! - shutdown_listener：优雅关闭监听器 task（abort detached spawn，幂等）
//! - handle_inventory_event_safe：单事件处理（panic 隔离由调用方 catch_unwind 包裹）
//! - handle_inventory_transaction：交易类型分发到 voucher 子模块的凭证生成/成本维护方法
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
use tokio::sync::broadcast::error::RecvError;
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
            // 广播通道的 Lagged 只说明本消费者落后、若干条被丢弃，接收器本身仍可用。
            // 原先写作 `while let Ok(event) = receiver.recv().await` 把 Lagged 当成终止条件，
            // 一次积压就让库存交易的会计凭证生成永久停摆且不留任何日志。
            loop {
                let event = match receiver.recv().await {
                    Ok(event) => event,
                    Err(RecvError::Lagged(missed)) => {
                        error!(
                            missed_events = missed,
                            "库存财务桥接监听器消费落后，部分事件被丢弃，继续接收后续事件（不退出循环）"
                        );
                        continue;
                    }
                    Err(RecvError::Closed) => {
                        error!(
                            "事件总线已关闭，库存财务桥接监听器退出（此后不再自动生成会计凭证）"
                        );
                        break;
                    }
                };
                // 批次 8（2026-06-28）：单次事件处理 panic 隔离
                // 库存财务桥接监听器 panic 会导致库存交易不再生成会计凭证，
                // 财务报表与库存数据不一致。
                let result = AssertUnwindSafe(Self::handle_inventory_event_safe(db.clone(), event))
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

    /// 处理库存交易事件：按交易类型分发到凭证生成或成本维护
    ///
    /// 幂等：先查重，命中即跳过；业务失败清除幂等记录让事件可重放，
    /// 清除也失败则写入死信表兜底。
    /// 不套外层业务事务：凭证创建在 VoucherService 内部以独立事务提交，
    /// 外层事务回滚撤不回已提交的凭证，伪装原子性只会掩盖失败；
    /// 业务失败统一以 Err 向上传播，由监听器显式记日志。
    async fn handle_inventory_transaction(
        &self,
        transaction_id: i32,
        transaction_type: &str,
        args: VoucherCreateArgs<'_>,
    ) -> Result<(), AppError> {
        let idempotency_service =
            crate::services::event_idempotency_service::EventIdempotencyService::new(
                self.db.clone(),
            );
        let consumer_id = "inventory_finance_bridge";
        let event_key = format!("inventory_txn:{}", transaction_id);
        let event_type = "InventoryTransactionCreated";

        // 阶段 1：幂等查重（自身独立事务，命中即跳过）
        let already_processed = !idempotency_service
            .try_mark_processed(consumer_id, &event_key, event_type)
            .await?;
        if already_processed {
            info!(
                transaction_id = transaction_id,
                transaction_type = transaction_type,
                "库存交易事件已处理，幂等跳过凭证生成"
            );
            return Ok(());
        }

        // 阶段 2：执行业务动作（凭证创建自带事务并提交）
        let result: Result<(), AppError> = match transaction_type {
            "PURCHASE_RECEIPT" => self.maintain_purchase_receipt_cost(args).await,
            "PURCHASE_RETURN" => {
                info!(
                    transaction_id = transaction_id,
                    source_bill_no = args.source_bill_no.unwrap_or("-"),
                    "采购退货记账凭证已由应付侧红字凭证唯一承载，桥接臂不重复入账"
                );
                Ok(())
            }
            "SALES_DELIVERY" => self.create_sales_delivery_voucher(args).await,
            "SALES_RETURN" => self.create_sales_return_voucher(args).await,
            "INVENTORY_ADJUSTMENT" => self.create_inventory_adjustment_voucher(args).await,
            "PRODUCTION_RECEIPT" | "PRODUCTION_OUTPUT" => {
                self.create_production_receipt_voucher(args).await
            }
            "PRODUCTION_ISSUE" | "PRODUCTION_CONSUMPTION" => {
                self.create_production_issue_voucher(args).await
            }
            _ => {
                info!("未处理的库存交易类型: {}", transaction_type);
                Ok(())
            }
        };

        if let Err(err) = result {
            // 业务失败：删除幂等记录，允许事件重放；清除失败则推死信表由后台补偿兜底。
            let unmark_ok = idempotency_service
                .unmark_processed(consumer_id, &event_key)
                .await
                .is_ok();
            if !unmark_ok {
                let _ =
                    crate::services::event_retry_service::EventRetryService::new(self.db.clone())
                        .handle_failure(
                            event_type,
                            serde_json::json!({
                                "consumer_id": consumer_id,
                                "event_key": event_key,
                            }),
                            "业务失败后幂等清除失败",
                            &err.to_string(),
                            3,
                        )
                        .await;
            }
            error!(
                transaction_id = transaction_id,
                transaction_type = transaction_type,
                error = %err,
                "库存交易事件处理失败，已清除幂等记录等待重放（凭证/成本未落地）"
            );
            return Err(err);
        }
        Ok(())
    }
}
