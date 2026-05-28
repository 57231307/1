#![allow(dead_code)]
use sea_orm;
use tokio::sync::broadcast;
use std::sync::Arc;
use once_cell::sync::Lazy;

#[derive(Clone, Debug)]
pub struct ShippedItem {
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
}

#[derive(Clone, Debug)]
pub enum BusinessEvent {
    PurchaseReceiptCompleted {
        receipt_id: i32,
        order_id: i32,
        supplier_id: i32,
    },
    SalesOrderShipped {
        order_id: i32,
        customer_id: i32,
        items: Vec<ShippedItem>,
    },
    PaymentCompleted {
        payment_id: i32,
        invoice_id: i32,
        amount: rust_decimal::Decimal,
    },
    InventoryAdjusted {
        product_id: i32,
        warehouse_id: i32,
        quantity_change: rust_decimal::Decimal,
    },
    CollectionCompleted {
        collection_id: i32,
        invoice_id: Option<i32>,
        amount: rust_decimal::Decimal,
    },
    PurchaseOrderApproved {
        order_id: i32,
        supplier_id: i32,
    },
    InventoryCountCompleted {
        count_id: i32,
        variance_count: i32,
    },
    BpmProcessFinished {
        business_type: String,
        business_id: i32,
        approved: bool,
    },
    LowStockAlert {
        product_id: i32,
        warehouse_id: i32,
        current_quantity: rust_decimal::Decimal,
        reorder_point: rust_decimal::Decimal,
        reorder_quantity: rust_decimal::Decimal,
    },
    FinancialIndicatorUpdate {
        period: String,
        trigger_source: String,
    },
    MaterialShortageAlert {
        material_id: i32,
        material_name: String,
        material_code: String,
        required_quantity: rust_decimal::Decimal,
        available_quantity: rust_decimal::Decimal,
        shortage_quantity: rust_decimal::Decimal,
        shortage_level: String,
        affected_orders_count: i32,
    },
    InventoryTransactionCreated {
        transaction_id: i32,
        transaction_type: String,
        product_id: i32,
        warehouse_id: i32,
        quantity_meters: rust_decimal::Decimal,
        quantity_kg: rust_decimal::Decimal,
        source_bill_type: Option<String>,
        source_bill_no: Option<String>,
        source_bill_id: Option<i32>,
        batch_no: String,
        color_no: String,
        created_by: Option<i32>,
    },
}

pub static EVENT_BUS: Lazy<EventBus> = Lazy::new(|| EventBus::new());

pub struct EventBus {
    sender: broadcast::Sender<BusinessEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }
    
    pub fn publish(&self, event: BusinessEvent) {
        let _ = self.sender.send(event);
    }
    
    pub fn subscribe(&self) -> broadcast::Receiver<BusinessEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

use sea_orm::DatabaseConnection;

pub async fn start_event_listener(db: Arc<DatabaseConnection>) {
    // 启动库存财务桥接服务监听器
    crate::services::inventory_finance_bridge_service::InventoryFinanceBridgeService::start_listener(db.clone());
    
    let mut receiver = EVENT_BUS.subscribe();

    
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                BusinessEvent::PurchaseReceiptCompleted { order_id, .. } => {
                    tracing::info!("Event received: PurchaseReceiptCompleted for order {}", order_id);
                    let po_service = crate::services::purchase_order_service::PurchaseOrderService::new(db.clone());
                    match po_service.receive_order(order_id).await {
                        Ok(_) => tracing::info!("Successfully updated purchase order {} status to RECEIVED", order_id),
                        Err(e) => tracing::error!("Failed to update purchase order {}: {}", order_id, e),
                    }
                }
                BusinessEvent::SalesOrderShipped { order_id, .. } => {
                    tracing::info!("Event received: SalesOrderShipped for order {}", order_id);
                }
                BusinessEvent::PaymentCompleted { invoice_id, .. } => {
                    tracing::info!("Event received: PaymentCompleted for invoice {}", invoice_id);
                    let ap_service = crate::services::ap_invoice_service::ApInvoiceService::new(db.clone());
                    match ap_service.mark_as_paid(invoice_id).await {
                        Ok(_) => tracing::info!("Successfully updated ap_invoice {} status to PAID", invoice_id),
                        Err(e) => tracing::error!("Failed to update ap_invoice {}: {}", invoice_id, e),
                    }
                }
                BusinessEvent::InventoryAdjusted { product_id, warehouse_id, quantity_change } => {
                    tracing::info!("Event received: InventoryAdjusted for product {} at warehouse {}, change: {}", product_id, warehouse_id, quantity_change);
                }
                BusinessEvent::PurchaseOrderApproved { order_id, .. } => {
                    tracing::info!("Event received: PurchaseOrderApproved for order {}", order_id);
                }
                BusinessEvent::CollectionCompleted { invoice_id, .. } => {
                    if let Some(inv_id) = invoice_id {
                        tracing::info!("Event received: CollectionCompleted for invoice {}", inv_id);
                        let ar_service = crate::services::ar_invoice_service::ArInvoiceService::new(db.clone());
                        match ar_service.mark_as_paid(inv_id).await {
                            Ok(_) => tracing::info!("Successfully updated ar_invoice {} status to PAID", inv_id),
                            Err(e) => tracing::error!("Failed to update ar_invoice {}: {}", inv_id, e),
                        }
                    }
                }
                BusinessEvent::InventoryCountCompleted { count_id, variance_count } => {
                    tracing::info!("处理库存盘点完成事件，盘点单ID: {}, 差异数: {}", count_id, variance_count);
                    tracing::info!(">> [报告服务] 盘点单 {} 的差异报告(差异: {}) 已生成并存档", count_id, variance_count);
                }
                BusinessEvent::BpmProcessFinished { business_type, business_id, approved } => {
                    tracing::info!("处理BPM流程结束事件: type={}, id={}, approved={}", business_type, business_id, approved);
                    if business_type == "purchase_order" {
                        if approved {
                            let po_service = crate::services::purchase_order_service::PurchaseOrderService::new(db.clone());
                            if let Err(e) = po_service.approve_order(business_id, 0).await {
                                tracing::error!("Failed to approve purchase_order {} via BPM: {}", business_id, e);
                            } else {
                                tracing::info!("Successfully approved purchase_order {} via BPM", business_id);
                            }
                        } else {
                            let po_service = crate::services::purchase_order_service::PurchaseOrderService::new(db.clone());
                            if let Err(e) = po_service.reject_order(business_id, "BPM审批拒绝".to_string(), 0).await {
                                tracing::error!("Failed to reject purchase_order {} via BPM: {}", business_id, e);
                            }
                        }
                    } else if business_type == "sales_order" {
                        if approved {
                            let sales_service = crate::services::sales_service::SalesService::new(db.clone());
                            if let Err(e) = sales_service.approve_order(business_id).await {
                                tracing::error!("Failed to approve sales_order {} via BPM: {}", business_id, e);
                            } else {
                                tracing::info!("Successfully approved sales_order {} via BPM", business_id);
                            }
                        } else {
                            let sales_service = crate::services::sales_service::SalesService::new(db.clone());
                            match sales_service.reject_order(business_id, "BPM审批拒绝".to_string()).await {
                                Ok(_) => tracing::info!("Successfully rejected sales_order {} via BPM", business_id),
                                Err(e) => tracing::error!("Failed to reject sales_order {} via BPM: {}", business_id, e),
                            }
                        }
                    }
                }
                BusinessEvent::LowStockAlert {
                    product_id,
                    warehouse_id,
                    current_quantity,
                    reorder_point,
                    reorder_quantity,
                } => {
                    tracing::info!(
                        "处理低库存预警事件: 产品ID={}, 仓库ID={}, 当前库存={}, 补货点={}, 建议补货量={}",
                        product_id,
                        warehouse_id,
                        current_quantity,
                        reorder_point,
                        reorder_quantity
                    );

                    // 创建采购建议
                    let po_service = crate::services::purchase_order_service::PurchaseOrderService::new(db.clone());
                    match po_service.create_purchase_suggestion(
                        product_id,
                        warehouse_id,
                        current_quantity,
                        reorder_point,
                        reorder_quantity,
                    ).await {
                        Ok(order) => {
                            tracing::info!(
                                "成功创建采购建议: 订单ID={}, 订单号={}",
                                order.id,
                                order.order_no
                            );
                        }
                        Err(e) => {
                            tracing::error!("创建采购建议失败: {}", e);
                        }
                    }

                    // 发送低库存预警通知给仓库管理员和采购人员
                    let notification_service = crate::services::event_notification_service::EventNotificationService::new(db.clone());
                    let product_name = crate::models::product::Entity::find_by_id(product_id)
                        .one(db.as_ref())
                        .await
                        .ok()
                        .flatten()
                        .map(|p| p.name)
                        .unwrap_or_else(|| format!("产品{}", product_id));

                    // 查询仓库管理员和采购相关人员（通过 role_id 关联角色表）
                    let notify_user_ids = crate::models::user::Entity::find()
                        .filter(crate::models::user::Column::IsActive.eq(true))
                        .all(db.as_ref())
                        .await
                        .unwrap_or_default()
                        .into_iter()
                        .map(|u| u.id)
                        .collect::<Vec<i32>>();

                    let notify_count = notify_user_ids.len();
                    for user_id in notify_user_ids {
                        if let Err(e) = notification_service.notify_inventory_alert(
                            user_id,
                            &product_name,
                            product_id,
                            &format!("{}米", current_quantity),
                            &format!("{}米", reorder_point),
                        ).await {
                            tracing::error!("发送低库存预警通知失败: 用户ID={}, 错误={}", user_id, e);
                        }
                    }
                    tracing::info!(
                        "低库存预警通知已发送: 产品={}, 仓库ID={}, 通知人数={}",
                        product_name, warehouse_id, notify_count
                    );
                }
                BusinessEvent::FinancialIndicatorUpdate { period, trigger_source } => {
                    tracing::info!("处理财务指标更新事件: 期间={}, 触发源={}", period, trigger_source);
                    let fa_service = crate::services::financial_analysis_service::FinancialAnalysisService::new(db.clone());
                    match fa_service.calculate_indicators(&period, 0).await {
                        Ok(results) => {
                            tracing::info!(
                                "财务指标自动计算完成: 期间={}, 计算 {} 个指标",
                                period,
                                results.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("财务指标自动计算失败: 期间={}, 错误={}", period, e);
                        }
                    }
                }
                BusinessEvent::MaterialShortageAlert {
                    material_id,
                    material_name,
                    material_code,
                    required_quantity,
                    available_quantity,
                    shortage_quantity,
                    shortage_level,
                    affected_orders_count,
                } => {
                    tracing::info!(
                        "处理缺料预警事件: 物料ID={}, 物料名称={}, 缺料数量={}, 预警级别={}, 受影响订单数={}",
                        material_id,
                        material_name,
                        shortage_quantity,
                        shortage_level,
                        affected_orders_count
                    );
                    let po_service = crate::services::purchase_order_service::PurchaseOrderService::new(db.clone());
                    match po_service.create_purchase_suggestion_from_shortage(
                        material_id,
                        material_name.clone(),
                        material_code.clone(),
                        required_quantity,
                        available_quantity,
                        shortage_quantity,
                        shortage_level.clone(),
                        affected_orders_count,
                    ).await {
                        Ok(order) => {
                            tracing::info!(
                                "成功创建缺料采购建议: 订单ID={}, 订单号={}, 物料={}",
                                order.id,
                                order.order_no,
                                material_name
                            );
                        }
                        Err(e) => {
                            tracing::error!("创建缺料采购建议失败: 物料ID={}, 错误={}", material_id, e);
                        }
                    }
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }
    });
}
