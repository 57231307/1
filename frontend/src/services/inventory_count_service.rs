//! 库存盘点服务 API 客户端
//! 提供库存盘点相关的 API 调用方法

use crate::models::inventory_count::*;
use crate::services::api::ApiService;

pub struct InventoryCountService;

impl InventoryCountService {
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

        ApiService::get(&format!("/inventory/counts{}", query_string)).await
    }

    pub async fn get(id: i32) -> Result<InventoryCountDetail, String> {
        ApiService::get(&format!("/inventory/counts/{}", id)).await
    }

    pub async fn create(req: CreateInventoryCountRequest) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post("/inventory/counts", &body).await
    }

    pub async fn update(
        id: i32,
        req: UpdateInventoryCountRequest,
    ) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::put(&format!("/inventory/counts/{}", id), &body).await
    }

    pub async fn approve(
        id: i32,
        approved: bool,
        notes: Option<String>,
    ) -> Result<InventoryCountDetail, String> {
        let body = serde_json::to_value(&ApproveCountRequest { approved, notes })
            .map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/inventory/counts/{}/approve", id), &body).await
    }

    pub async fn complete(id: i32) -> Result<InventoryCountDetail, String> {
        ApiService::post(
            &format!("/inventory/counts/{}/complete", id),
            &serde_json::json!({}),
        )
        .await
    }
}
