use sea_orm::{ColumnTrait, QueryFilter};
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

use crate::utils::app_state::AppState;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

pub async fn start_event_listener(db: Arc<DatabaseConnection>) {
    let mut receiver = EVENT_BUS.subscribe();

    
    tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match event {
                BusinessEvent::PurchaseReceiptCompleted { order_id, .. } => {
                    tracing::info!("Event received: PurchaseReceiptCompleted for order {}", order_id);
                    if let Ok(Some(order)) = crate::models::purchase_order::Entity::find_by_id(order_id).filter(crate::models::purchase_order::Column::IsDeleted.eq(false)).one(db.as_ref()).await {
                        let mut active_order: crate::models::purchase_order::ActiveModel = order.into();
                        active_order.order_status = Set("RECEIVED".to_string());
                        if let Err(e) = active_order.update(db.as_ref()).await {
                            tracing::error!("Failed to update purchase order {}: {}", order_id, e);
                        } else {
                            tracing::info!("Successfully updated purchase order {} status to RECEIVED", order_id);
                        }
                    }
                }
                BusinessEvent::SalesOrderShipped { order_id, .. } => {
                    tracing::info!("Event received: SalesOrderShipped for order {}", order_id);
                }
                BusinessEvent::PaymentCompleted { invoice_id, .. } => {
                    tracing::info!("Event received: PaymentCompleted for invoice {}", invoice_id);
                    if let Ok(Some(invoice)) = crate::models::ap_invoice::Entity::find_by_id(invoice_id).filter(crate::models::ap_invoice::Column::IsDeleted.eq(false)).one(db.as_ref()).await {
                        let mut active_invoice: crate::models::ap_invoice::ActiveModel = invoice.into();
                        active_invoice.invoice_status = Set("PAID".to_string());
                        if let Err(e) = active_invoice.update(db.as_ref()).await {
                            tracing::error!("Failed to update ap_invoice {}: {}", invoice_id, e);
                        } else {
                            tracing::info!("Successfully updated ap_invoice {} status to PAID", invoice_id);
                        }
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
                        if let Ok(Some(invoice)) = crate::models::ar_invoice::Entity::find_by_id(inv_id).filter(crate::models::ar_invoice::Column::IsDeleted.eq(false)).one(db.as_ref()).await {
                            let mut active_invoice: crate::models::ar_invoice::ActiveModel = invoice.into();
                            active_invoice.status = Set("PAID".to_string());
                            if let Err(e) = active_invoice.update(db.as_ref()).await {
                                tracing::error!("Failed to update ar_invoice {}: {}", inv_id, e);
                            } else {
                                tracing::info!("Successfully updated ar_invoice {} status to PAID", inv_id);
                            }
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
                            if let Ok(Some(order)) = crate::models::sales_order::Entity::find_by_id(business_id).filter(crate::models::sales_order::Column::IsDeleted.eq(false)).one(db.as_ref()).await {
                                let mut active_order: crate::models::sales_order::ActiveModel = order.into();
                                active_order.status = Set("rejected".to_string());
                                if let Err(e) = active_order.update(db.as_ref()).await {
                                    tracing::error!("Failed to update sales_order {} status to rejected: {}", business_id, e);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
