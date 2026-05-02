//! 库存调拨服务 API 客户端
//! 提供库存调拨相关的 API 调用方法

use crate::models::inventory_transfer::*;
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct InventoryTransferService;

impl CrudService for InventoryTransferService {
    type Model = InventoryTransferDetail;
    type ListResponse = Vec<InventoryTransfer>;
    type CreateRequest = CreateInventoryTransferRequest;
    type UpdateRequest = UpdateInventoryTransferRequest;

    fn base_path() -> &'static str {
        "/inventory/transfers"
    }
}


impl InventoryTransferService {
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

        ApiService::get(&format!("/inventory/transfers{}", query_string)).await
    }

    pub async fn approve(id: i32, approved: bool, notes: Option<String>) -> Result<InventoryTransferDetail, String> {
        let body = serde_json::to_value(&ApproveTransferRequest { approved, notes })
            .map_err(|e| format!("序列化失败：{}", e))?;
        ApiService::post(&format!("/inventory/transfers/{}/approve", id), &body).await
    }

    pub async fn ship(id: i32) -> Result<InventoryTransferDetail, String> {
        ApiService::post(&format!("/inventory/transfers/{}/ship", id), &serde_json::json!({})).await
    }

    pub async fn receive(id: i32) -> Result<InventoryTransferDetail, String> {
        ApiService::post(&format!("/inventory/transfers/{}/receive", id), &serde_json::json!({})).await
    }
}
