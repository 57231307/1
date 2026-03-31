//! 采购退货服务 API 客户端
//! 提供采购退货单相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 采购退货单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturn {
    pub id: i32,
    pub return_no: String,
    pub order_id: i32,
    pub order_no: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub return_date: String,
    pub status: String,
    pub total_quantity: String,
    pub total_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 采购退货单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturnItem {
    pub id: i32,
    pub return_id: i32,
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
    pub quantity_returned: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 采购退货单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReturnQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 创建采购退货单请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseReturnRequest {
    pub order_id: i32,
    pub return_date: String,
    pub warehouse_id: i32,
    pub department_id: i32,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateReturnItemRequest>,
}

/// 创建退货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateReturnItemRequest {
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
    pub quantity_returned: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 更新采购退货单请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseReturnRequest {
    pub return_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub reason: Option<String>,
    pub notes: Option<String>,
}

/// 更新退货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReturnItemRequest {
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_returned: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 拒绝退货单请求
#[derive(Debug, Clone, Serialize)]
pub struct RejectReturnRequest {
    pub reason: String,
}

/// 分页结果
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedReturns {
    pub items: Vec<PurchaseReturn>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

pub struct PurchaseReturnService;

impl PurchaseReturnService {
    /// 获取退货单列表
    pub async fn list(query: PurchaseReturnQuery) -> Result<Vec<PurchaseReturn>, String> {
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

        let response: serde_json::Value = ApiService::get(&format!("/purchases/returns{}", query_string)).await?;

        if let Some(data) = response.get("data") {
            if let Some(items) = data.get("items").and_then(|v| v.as_array()) {
                let returns: Vec<PurchaseReturn> = items
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                return Ok(returns);
            }
        }
        Ok(Vec::new())
    }

    /// 获取退货单详情
    pub async fn get(id: i32) -> Result<PurchaseReturn, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/returns/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取退货单详情失败".to_string())
    }

    /// 创建退货单
    pub async fn create(req: CreatePurchaseReturnRequest) -> Result<PurchaseReturn, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/purchases/returns", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建退货单失败".to_string())
    }

    /// 更新退货单
    pub async fn update(id: i32, req: UpdatePurchaseReturnRequest) -> Result<PurchaseReturn, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/returns/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新退货单失败".to_string())
    }

    /// 提交退货单
    pub async fn submit(id: i32) -> Result<PurchaseReturn, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/returns/{}/submit", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "提交退货单失败".to_string())
    }

    /// 审批退货单
    pub async fn approve(id: i32) -> Result<PurchaseReturn, String> {
        let response: serde_json::Value = ApiService::post(&format!("/purchases/returns/{}/approve", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审批退货单失败".to_string())
    }

    /// 拒绝退货单
    pub async fn reject(id: i32, reason: String) -> Result<PurchaseReturn, String> {
        let body = serde_json::to_value(&RejectReturnRequest { reason }).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchases/returns/{}/reject", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "拒绝退货单失败".to_string())
    }

    /// 获取退货单明细列表
    pub async fn list_items(return_id: i32) -> Result<Vec<PurchaseReturnItem>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/purchases/returns/{}/items", return_id)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let items: Vec<PurchaseReturnItem> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// 添加退货明细
    pub async fn create_item(return_id: i32, req: CreateReturnItemRequest) -> Result<PurchaseReturnItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/purchases/returns/{}/items", return_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "添加退货明细失败".to_string())
    }

    /// 更新退货明细
    pub async fn update_item(return_id: i32, item_id: i32, req: UpdateReturnItemRequest) -> Result<PurchaseReturnItem, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/purchases/returns/{}/items/{}", return_id, item_id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新退货明细失败".to_string())
    }

    /// 删除退货明细
    pub async fn delete_item(return_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/returns/{}/items/{}", return_id, item_id)).await
    }
}