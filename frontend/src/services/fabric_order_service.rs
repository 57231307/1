//! 面料订单服务 API 客户端
//! 提供面料订单相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricOrder {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub order_date: String,
    pub required_date: String,
    pub status: String,
    pub total_amount: String,
    pub paid_amount: String,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricOrderItem {
    pub id: i32,
    pub fabric_order_id: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub unit_price_meters: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub base_price: Option<f64>,
    pub color_extra_cost: Option<f64>,
    pub grade_price_diff: Option<f64>,
    pub final_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FabricOrderQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i32>,
    pub order_no: Option<String>,
    pub status: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateFabricOrderRequest {
    pub customer_id: i32,
    pub order_date: String,
    pub required_date: String,
    pub items: Vec<FabricOrderItemRequest>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FabricOrderItemRequest {
    pub product_id: i32,
    pub product_name: Option<String>,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub unit_price_meters: f64,
    pub gram_weight: Option<f64>,
    pub width: Option<f64>,
    pub color_no: String,
    pub batch_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
    pub pantone_code: Option<String>,
    pub color_name: Option<String>,
    pub base_price: Option<f64>,
    pub color_extra_cost: Option<f64>,
    pub grade_price_diff: Option<f64>,
    pub final_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateFabricOrderRequest {
    pub required_date: Option<String>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub delivery_address: Option<String>,
    pub payment_terms: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<FabricOrderItemRequest>>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
}

pub struct FabricOrderService;

impl FabricOrderService {
    pub async fn list(query: FabricOrderQuery) -> Result<Vec<FabricOrder>, String> {
        let mut params = Vec::new();
        if let Some(page) = query.page {
            params.push(format!("page={}", page));
        }
        if let Some(page_size) = query.page_size {
            params.push(format!("page_size={}", page_size));
        }
        if let Some(customer_id) = query.customer_id {
            params.push(format!("customer_id={}", customer_id));
        }
        if let Some(ref order_no) = query.order_no {
            params.push(format!("order_no={}", order_no));
        }
        if let Some(ref status) = query.status {
            params.push(format!("status={}", status));
        }
        if let Some(ref batch_no) = query.batch_no {
            params.push(format!("batch_no={}", batch_no));
        }
        if let Some(ref color_no) = query.color_no {
            params.push(format!("color_no={}", color_no));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/sales/fabric-orders{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let orders: Vec<FabricOrder> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(orders)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get(id: i32) -> Result<FabricOrder, String> {
        let response: serde_json::Value = ApiService::get(&format!("/sales/fabric-orders/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取面料订单详情失败".to_string())
    }

    pub async fn create(req: CreateFabricOrderRequest) -> Result<FabricOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/sales/fabric-orders", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建面料订单失败".to_string())
    }

    pub async fn update(id: i32, req: UpdateFabricOrderRequest) -> Result<FabricOrder, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/sales/fabric-orders/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新面料订单失败".to_string())
    }

    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/sales/fabric-orders/{}", id)).await
    }

    pub async fn approve(id: i32) -> Result<FabricOrder, String> {
        let response: serde_json::Value = ApiService::post(&format!("/sales/fabric-orders/{}/approve", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审批面料订单失败".to_string())
    }
}