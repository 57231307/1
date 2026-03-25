//! 库存盘点服务 API 客户端
//! 提供库存盘点相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 库存盘点单数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCount {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: String,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 库存盘点明细项数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountItem {
    pub id: i32,
    pub count_id: i32,
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_before: String,
    pub quantity_actual: String,
    pub quantity_difference: String,
    pub unit_cost: String,
    pub total_cost: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 库存盘点详情（包含明细项）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryCountDetail {
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: String,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<InventoryCountItem>,
}

/// 库存盘点查询参数
#[derive(Debug, Clone, Serialize)]
pub struct InventoryCountQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub warehouse_id: Option<i32>,
    pub count_no: Option<String>,
}

/// 创建库存盘点明细项请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryCountItemRequest {
    pub product_id: i32,
    pub stock_id: i32,
    pub warehouse_id: i32,
    pub quantity_actual: String,
    pub unit_cost: String,
    pub notes: Option<String>,
}

/// 创建库存盘点请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateInventoryCountRequest {
    pub warehouse_id: i32,
    pub count_date: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryCountItemRequest>>,
}

/// 更新库存盘点请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInventoryCountRequest {
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<CreateInventoryCountItemRequest>>,
}

/// 审核库存盘点请求
#[derive(Debug, Clone, Serialize)]
pub struct ApproveCountRequest {
    pub approved: bool,
    pub notes: Option<String>,
}

/// 库存盘点服务
pub struct InventoryCountService;

impl InventoryCountService {
    /// 获取库存盘点列表
    pub async fn list(query: InventoryCountQuery) -> Result<Vec<InventoryCount>, String> {
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
        if let Some(warehouse_id) = query.warehouse_id {
            params.push(format!("warehouse_id={}", warehouse_id));
        }
        if let Some(ref count_no) = query.count_no {
            params.push(format!("count_no={}", count_no));
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: serde_json::Value = ApiService::get(&format!("/inventory/counts{}", query_string)).await?;

        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let counts: Vec<InventoryCount> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(counts)
        } else {
            Ok(Vec::new())
        }
    }

    /// 获取库存盘点详情
    pub async fn get(id: i32) -> Result<InventoryCountDetail, String> {
        let response: serde_json::Value = ApiService::get(&format!("/inventory/counts/{}", id)).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取库存盘点详情失败".to_string())
    }

    /// 创建库存盘点
    pub async fn create(req: CreateInventoryCountRequest) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/inventory/counts", &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建库存盘点失败".to_string())
    }

    /// 更新库存盘点
    pub async fn update(id: i32, req: UpdateInventoryCountRequest) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/inventory/counts/{}", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新库存盘点失败".to_string())
    }

    /// 审核库存盘点
    pub async fn approve(id: i32, approved: bool, notes: Option<String>) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&ApproveCountRequest { approved, notes })
            .map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/inventory/counts/{}/approve", id), &body).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审核库存盘点失败".to_string())
    }

    /// 完成库存盘点
    pub async fn complete(id: i32) -> Result<InventoryCountDetail, String> {
        let response: serde_json::Value = ApiService::post(&format!("/inventory/counts/{}/complete", id), &serde_json::json!({})).await?;

        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "完成库存盘点失败".to_string())
    }
}