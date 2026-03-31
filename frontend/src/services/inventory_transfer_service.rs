//! 库存调拨服务 API 客户端
//! 提供库存调拨相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 库存调拨单数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransfer {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: String,
    pub status: String,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub shipped_at: Option<String>,
    pub received_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 库存调拨明细项数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransferItem {
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: String,
    pub shipped_quantity: String,
    pub received_quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 库存调拨详情（包含明细项）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTransferDetail {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: String,
    pub status: String,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub shipped_at: Option<String>,
    pub received_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<InventoryTransferItem>,
}

/// 库存调拨查询参数
#[derive(Debug, Clone, Serialize)]
pub struct InventoryTransferQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_no: Option<String>,
}

/// 创建库存调拨明细项请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryTransferItemRequest {
    pub product_id: i32,
    pub quantity: String,
    pub notes: Option<String>,
}

/// 创建库存调拨请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryTransferRequest {
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Vec<CreateInventoryTransferItemRequest>,
}

/// 更新库存调拨请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInventoryTransferRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryTransferItemRequest>>,
}

/// 审核库存调拨请求
#[derive(Debug, Clone, Serialize)]
pub struct ApproveTransferRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 库存调拨服务
pub struct InventoryTransferService;

impl InventoryTransferService {
    /// 获取库存调拨列表
    pub async fn list(query: InventoryTransferQuery) -> Result<Vec<InventoryTransfer>, String> {
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
        if let Some(from_warehouse_id) = query.from_warehouse_id {
            params.push(format!("from_warehouse_id={}", from_warehouse_id));
        }
        if let Some(to_warehouse_id) = query.to_warehouse_id {
            params.push(format!("to_warehouse_id={}", to_warehouse_id));
        }
        if let Some(ref transfer_no) = query.transfer_no {
            params.push(format!("transfer_no={}", transfer_no));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/inventory/transfers{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let transfers: Vec<InventoryTransfer> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(transfers)
        } else {
            Ok(Vec::new())
        }
    }

    /// 获取库存调拨详情
    pub async fn get(id: i32) -> Result<InventoryTransferDetail, String> {
        let response: serde_json::Value = ApiService::get(&format!("/inventory/transfers/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取库存调拨详情失败".to_string())
    }

    /// 创建库存调拨
    pub async fn create(req: CreateInventoryTransferRequest) -> Result<InventoryTransferDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/inventory/transfers", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建库存调拨失败".to_string())
    }

    /// 更新库存调拨
    pub async fn update(id: i32, req: UpdateInventoryTransferRequest) -> Result<InventoryTransferDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/inventory/transfers/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新库存调拨失败".to_string())
    }

    /// 审核库存调拨
    pub async fn approve(id: i32, approved: bool, notes: Option<String>) -> Result<InventoryTransferDetail, String> {
        let body = serde_json::to_value(&ApproveTransferRequest { approved, notes })
            .map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/inventory/transfers/{}/approve", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审核库存调拨失败".to_string())
    }

    /// 发出库存调拨
    pub async fn ship(id: i32) -> Result<InventoryTransferDetail, String> {
        let response: serde_json::Value = ApiService::post(&format!("/inventory/transfers/{}/ship", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "发出库存调拨失败".to_string())
    }

    /// 接收库存调拨
    pub async fn receive(id: i32) -> Result<InventoryTransferDetail, String> {
        let response: serde_json::Value = ApiService::post(&format!("/inventory/transfers/{}/receive", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "接收库存调拨失败".to_string())
    }

    /// 删除库存调拨
    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/inventory/transfers/{}", id)).await
    }
}