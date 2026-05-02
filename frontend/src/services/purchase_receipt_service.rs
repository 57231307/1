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
        <Self as CrudService>::list_with_query(&query).await
    }

    /// 获取收货单详情
    pub async fn get(id: i32) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::get(id).await
    }

    /// 创建收货单
    pub async fn create(req: CreatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::create(req).await
    }

    /// 更新收货单
    pub async fn update(id: i32, req: UpdatePurchaseReceiptRequest) -> Result<PurchaseReceipt, String> {
        <Self as CrudService>::update(id, req).await
    }

    /// 删除收货单
    pub async fn delete(id: i32) -> Result<(), String> {
        <Self as CrudService>::delete(id).await
    }

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
