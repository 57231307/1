use crate::models::inventory_adjustment::{
    AdjustmentItem, AdjustmentSummary, CreateAdjustmentItemRequest, CreateAdjustmentRequest,
    InventoryAdjustment, InventoryAdjustmentListResponse,
};
use crate::services::api::ApiService;

pub struct InventoryAdjustmentService;

impl InventoryAdjustmentService {
    pub async fn list_adjustments(
        page: Option<u64>,
        page_size: Option<u64>,
    ) -> Result<InventoryAdjustmentListResponse, String> {
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

    pub async fn get_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::get::<InventoryAdjustment>(&format!("/inventory-adjustments/{}", id)).await
    }

    pub async fn create_adjustment(request: CreateAdjustmentRequest) -> Result<InventoryAdjustment, String> {
        let payload = serde_json::to_value(&request).map_err(|e| e.to_string())?;
        ApiService::post("/inventory-adjustments", &payload).await
    }

    pub async fn approve_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::post::<InventoryAdjustment>(&format!("/inventory-adjustments/{}/approve", id), &serde_json::json!({})).await
    }

    pub async fn reject_adjustment(id: i32) -> Result<InventoryAdjustment, String> {
        ApiService::post::<InventoryAdjustment>(&format!("/inventory-adjustments/{}/reject", id), &serde_json::json!({})).await
    }
}
