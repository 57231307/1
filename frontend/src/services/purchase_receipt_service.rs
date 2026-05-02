//! 采购收货服务 API 客户端
//! 提供采购收货单相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::purchase_receipt::{
    CreatePurchaseReceiptRequest, CreateReceiptItemRequest, PaginatedReceipts, PurchaseReceipt,
    PurchaseReceiptItem, PurchaseReceiptQuery, UpdatePurchaseReceiptRequest,
    UpdateReceiptItemRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct PurchaseReceiptService;

impl CrudService for PurchaseReceiptService {
    type Model = PurchaseReceipt;
    type ListResponse = PaginatedReceipts;
    type CreateRequest = CreatePurchaseReceiptRequest;
    type UpdateRequest = UpdatePurchaseReceiptRequest;

    fn base_path() -> &'static str {
        "/purchases/receipts"
    }
}


impl PurchaseReceiptService {
    /// 获取收货单列表
    pub async fn list(query: PurchaseReceiptQuery) -> Result<PaginatedReceipts, String> {
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

        let response: ApiResponse<PaginatedReceipts> =
            ApiService::get(&format!("/purchases/receipts{}", query_string)).await?;
        response.into_result()
    }

    /// 获取收货单详情

    /// 创建收货单

    /// 更新收货单

    /// 确认收货单
    pub async fn confirm(id: i32) -> Result<PurchaseReceipt, String> {
        let response: ApiResponse<PurchaseReceipt> =
            ApiService::post(&format!("/purchases/receipts/{}/confirm", id), &serde_json::json!({})).await?;
        response.into_result()
    }

    /// 获取收货单明细列表
    pub async fn list_items(receipt_id: i32) -> Result<Vec<PurchaseReceiptItem>, String> {
        let response: ApiResponse<Vec<PurchaseReceiptItem>> =
            ApiService::get(&format!("/purchases/receipts/{}/items", receipt_id)).await?;
        response.into_result()
    }

    /// 添加收货明细
    pub async fn create_item(receipt_id: i32, req: CreateReceiptItemRequest) -> Result<PurchaseReceiptItem, String> {
        let response: ApiResponse<PurchaseReceiptItem> =
            ApiService::post(&format!("/purchases/receipts/{}/items", receipt_id), &req).await?;
        response.into_result()
    }

    /// 更新收货明细
    pub async fn update_item(receipt_id: i32, item_id: i32, req: UpdateReceiptItemRequest) -> Result<PurchaseReceiptItem, String> {
        let response: ApiResponse<PurchaseReceiptItem> =
            ApiService::put(&format!("/purchases/receipts/{}/items/{}", receipt_id, item_id), &req).await?;
        response.into_result()
    }

    /// 删除收货明细
    pub async fn delete_item(receipt_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/receipts/{}/items/{}", receipt_id, item_id)).await
    }
}
