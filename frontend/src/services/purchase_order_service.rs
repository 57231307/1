#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
//! 采购订单服务 API 客户端
//! 提供采购订单相关的 API 调用方法

use crate::models::purchase_order::*;
use crate::services::api::ApiService;

pub struct PurchaseOrderService;

impl PurchaseOrderService {
    pub async fn list(query: PurchaseOrderQuery) -> Result<Vec<PurchaseOrder>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(supplier_id) = query.supplier_id {
            params.push(format!("supplier_id={}", supplier_id));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        ApiService::get(&format!("/purchases/orders{}", query_string)).await
    }

    pub async fn get(id: i32) -> Result<PurchaseOrder, String> {
        ApiService::get(&format!("/purchases/orders/{}", id)).await
    }

    pub async fn create(req: CreatePurchaseOrderRequest) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post("/purchases/orders", &body).await
    }

    pub async fn update(id: i32, req: UpdatePurchaseOrderRequest) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::put(&format!("/purchases/orders/{}", id), &body).await
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/orders/{}", id)).await
    }

    pub async fn submit(id: i32) -> Result<PurchaseOrder, String> {
        ApiService::post(
            &format!("/purchases/orders/{}/submit", id),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn approve(id: i32) -> Result<PurchaseOrder, String> {
        ApiService::post(
            &format!("/purchases/orders/{}/approve", id),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn reject(id: i32, reason: String) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&RejectOrderRequest { reason })
            .map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/purchases/orders/{}/reject", id), &body).await
    }

    pub async fn close(id: i32) -> Result<PurchaseOrder, String> {
        ApiService::post(
            &format!("/purchases/orders/{}/close", id),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn list_items(order_id: i32) -> Result<Vec<PurchaseOrderItem>, String> {
        ApiService::get(&format!("/purchases/orders/{}/items", order_id)).await
    }

    pub async fn create_item(
        order_id: i32,
        req: CreateOrderItemRequest,
    ) -> Result<PurchaseOrderItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/purchases/orders/{}/items", order_id), &body).await
    }

    pub async fn update_item(
        order_id: i32,
        item_id: i32,
        req: UpdateOrderItemRequest,
    ) -> Result<PurchaseOrderItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::put(
            &format!("/purchases/orders/{}/items/{}", order_id, item_id),
            &body,
        )
        .await
    }

    pub async fn delete_item(order_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/orders/{}/items/{}", order_id, item_id)).await
    }
}
