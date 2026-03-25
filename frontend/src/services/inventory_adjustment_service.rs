use crate::services::api::ApiService;

/// 库存调整单数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct InventoryAdjustment {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub total_quantity: String,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: String,
    pub items: Vec<AdjustmentItem>,
}

/// 调整单明细项
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AdjustmentItem {
    pub id: i32,
    pub stock_id: i32,
    pub quantity: String,
    pub quantity_before: String,
    pub quantity_after: String,
    pub unit_cost: Option<String>,
    pub amount: Option<String>,
    pub notes: Option<String>,
}

/// 调整单列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct InventoryAdjustmentListResponse {
    pub adjustments: Vec<AdjustmentSummary>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 调整单摘要信息
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AdjustmentSummary {
    pub id: i32,
    pub adjustment_no: String,
    pub warehouse_id: i32,
    pub adjustment_type: String,
    pub reason_type: String,
    pub status: String,
    pub total_quantity: String,
    pub created_at: String,
}

/// 创建调整单请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAdjustmentRequest {
    pub warehouse_id: i32,
    pub adjustment_date: String,
    pub adjustment_type: String,
    pub reason_type: String,
    pub reason_description: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<CreateAdjustmentItemRequest>,
}

/// 创建调整单明细项请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateAdjustmentItemRequest {
    pub stock_id: i32,
    pub quantity: String,
    pub unit_cost: Option<String>,
    pub notes: Option<String>,
}

/// 库存调整单服务
pub struct InventoryAdjustmentService;

impl InventoryAdjustmentService {
    /// 获取调整单列表
    pub async fn list_adjustments(page: Option<u64>, page_size: Option<u64>) -> Result<InventoryAdjustmentListResponse, String> {
        let mut url = "/inventory-adjustments".to_string();
        if let Some(p) = page {
            url.push_str(&format!("?page={}", p));
        }
        if let Some(ps) = page_size {
            if url.contains('?') {
                url.push_str(&format!("&page_size={}", ps));
            } else {
                url.push_str(&format!("?page_size={}", ps));
            }
        }
        ApiService::get::<InventoryAdjustmentListResponse>(&url).await
    }

    /// 获取调整单详情
    pub async fn get_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::get::<InventoryAdjustment>(&format!("/inventory-adjustments/{}", id)).await
    }

    /// 创建调整单
    pub async fn create_adjustment(request: CreateAdjustmentRequest) -> Result<InventoryAdjustment, String> {
        let payload = serde_json::to_value(&request).map_err(|e| e.to_string())?;
        ApiService::post("/inventory-adjustments", &payload).await
    }

    /// 审核通过调整单
    pub async fn approve_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::post::<InventoryAdjustment>(&format!("/inventory-adjustments/{}/approve", id), &serde_json::json!({})).await
    }

    /// 驳回调整单
    pub async fn reject_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::post::<InventoryAdjustment>(&format!("/inventory-adjustments/{}/reject", id), &serde_json::json!({})).await
    }
}