//! 采购收货服务 API 客户端
//! 提供采购收货单相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 采购收货单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceipt {
    pub id: i32,
    pub receipt_no: String,
    pub order_id: i32,
    pub order_no: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub receipt_date: String,
    pub status: String,
    pub total_quantity: String,
    pub total_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub inspector: Option<String>,
    pub inspection_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 采购收货单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptItem {
    pub id: i32,
    pub receipt_id: i32,
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
    pub quantity_received: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 采购收货单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReceiptQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub order_id: Option<i32>,
}

/// 创建采购收货单请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseReceiptRequest {
    pub order_id: i32,
    pub receipt_date: String,
    pub warehouse_id: i32,
    pub department_id: i32,
    pub inspector: Option<String>,
    pub notes: Option<String>,
}

/// 更新采购收货单请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseReceiptRequest {
    pub receipt_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub inspector: Option<String>,
    pub notes: Option<String>,
}

/// 创建收货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateReceiptItemRequest {
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
    pub quantity_received: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 更新收货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReceiptItemRequest {
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_received: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 分页结果
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedReceipts {
    pub items: Vec<PurchaseReceipt>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

pub struct PurchaseReceiptService;

impl PurchaseReceiptService {
    /// 获取收货单列表
    pub async fn list(query: PurchaseReceiptQuery) -> Result<Vec<PurchaseReceipt>, String> {
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
        if let Some(order_id) = query.order_id {
            params.push(format!("order_id={}", order_id));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/purchases/receipts{}", query_string)).await?;

        if let Some(data) = response.get("data") {
            if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
                let receipts: Vec<PurchaseReceipt> = items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(receipts);
            }
        }
        Ok(Vec::new())
    }

    /// 获取收货单详情
    pub async fn get(id: i32) -> Result<PurchaseReceipt, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/receipts/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取收货单详情失败".to_string())
    }

    /// 创建收货单
    pub async fn create(req: CreatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/purchases/receipts", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建收货单失败".to_string())
    }

    /// 更新收货单
    pub async fn update(id: i32, req: UpdatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/receipts/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新收货单失败".to_string())
    }

    /// 确认收货单
    pub async fn confirm(id: i32) -> Result<PurchaseReceipt, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/receipts/{}/confirm", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "确认收货单失败".to_string())
    }

    /// 获取收货单明细列表
    pub async fn list_items(receipt_id: i32) -> Result<Vec<PurchaseReceiptItem>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/receipts/{}/items", receipt_id)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let items: Vec<PurchaseReceiptItem> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// 添加收货明细
    pub async fn create_item(receipt_id: i32, req: CreateReceiptItemRequest) -> Result<PurchaseReceiptItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchases/receipts/{}/items", receipt_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "添加收货明细失败".to_string())
    }

    /// 更新收货明细
    pub async fn update_item(receipt_id: i32, item_id: i32, req: UpdateReceiptItemRequest) -> Result<PurchaseReceiptItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/receipts/{}/items/{}", receipt_id, item_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新收货明细失败".to_string())
    }

    /// 删除收货明细
    pub async fn delete_item(receipt_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/receipts/{}/items/{}", receipt_id, item_id)).await
    }
}