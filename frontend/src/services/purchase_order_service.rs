//! 采购订单服务 API 客户端
//! 提供采购订单相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrder {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: String,
    pub expected_delivery_date: Option<String>,
    pub status: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseOrderItem {
    pub id: i32,
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: String,
    pub quantity_ordered: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: i32,
    pub order_date: String,
    pub expected_delivery_date: Option<String>,
    pub warehouse_id: i32,
    pub department_id: i32,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateOrderItemRequest {
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: String,
    pub currency: Option<String>,
    pub quantity_ordered: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<String>,
    pub expected_delivery_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<String>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateOrderItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_ordered: Option<String>,
    pub tax_rate: Option<String>,
    pub delivery_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RejectOrderRequest {
    pub reason: String,
}

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

        let response: serde_json::Value = ApiService::get(&format!("/purchases/orders{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let orders: Vec<PurchaseOrder> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(orders)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get(id: i32) -> Result<PurchaseOrder, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/orders/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取采购订单详情失败".to_string())
    }

    pub async fn create(req: CreatePurchaseOrderRequest) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/purchases/orders", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建采购订单失败".to_string())
    }

    pub async fn update(id: i32, req: UpdatePurchaseOrderRequest) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/orders/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新采购订单失败".to_string())
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/orders/{}", id)).await
    }

    pub async fn submit(id: i32) -> Result<PurchaseOrder, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/orders/{}/submit", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "提交采购订单失败".to_string())
    }

    pub async fn approve(id: i32) -> Result<PurchaseOrder, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/orders/{}/approve", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审批采购订单失败".to_string())
    }

    pub async fn reject(id: i32, reason: String) -> Result<PurchaseOrder, String> {
        let body = serde_json::to_value(&RejectOrderRequest { reason }).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchases/orders/{}/reject", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "拒绝采购订单失败".to_string())
    }

    pub async fn close(id: i32) -> Result<PurchaseOrder, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/orders/{}/close", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "关闭采购订单失败".to_string())
    }

    pub async fn list_items(order_id: i32) -> Result<Vec<PurchaseOrderItem>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/orders/{}/items", order_id)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let items: Vec<PurchaseOrderItem> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn create_item(order_id: i32, req: CreateOrderItemRequest) -> Result<PurchaseOrderItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchases/orders/{}/items", order_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建订单明细失败".to_string())
    }

    pub async fn update_item(order_id: i32, item_id: i32, req: UpdateOrderItemRequest) -> Result<PurchaseOrderItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/orders/{}/items/{}", order_id, item_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新订单明细失败".to_string())
    }

    pub async fn delete_item(order_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/orders/{}/items/{}", order_id, item_id)).await
    }
}