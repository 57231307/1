//! 采购退货服务 API 客户端
//! 提供采购退货单相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::purchase_return::{
    CreatePurchaseReturnRequest, CreatePurchaseReturnItemRequest, PaginatedReturns, PurchaseReturn,
    PurchaseReturnItem, PurchaseReturnQuery, RejectReturnRequest, UpdatePurchaseReturnRequest,
    UpdateReturnItemRequest,
};
use crate::services::api::ApiService;

pub struct PurchaseReturnService;

impl PurchaseReturnService {
    /// 获取退货单列表
    pub async fn list(query: PurchaseReturnQuery) -> Result<PaginatedReturns, String> {
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

        let response: ApiResponse<PaginatedReturns> =
            ApiService::get(&format!("/purchases/returns{}", query_string)).await?;
        response.into_result()
    }

    /// 获取退货单详情
    pub async fn get(id: i32) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::get(&format!("/purchases/returns/{}", id)).await?;
        response.into_result()
    }

    /// 创建退货单
    pub async fn create(req: CreatePurchaseReturnRequest) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::post("/purchases/returns", &req).await?;
        response.into_result()
    }

    /// 更新退货单
    pub async fn update(id: i32, req: UpdatePurchaseReturnRequest) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::put(&format!("/purchases/returns/{}", id), &req).await?;
        response.into_result()
    }

    /// 提交退货单
    pub async fn submit(id: i32) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::post(&format!("/purchases/returns/{}/submit", id), &serde_json::json!({})).await?;
        response.into_result()
    }

    /// 审批退货单
    pub async fn approve(id: i32) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::post(&format!("/purchases/returns/{}/approve", id), &serde_json::json!({})).await?;
        response.into_result()
    }

    /// 拒绝退货单
    pub async fn reject(id: i32, reason: String) -> Result<PurchaseReturn, String> {
        let response: ApiResponse<PurchaseReturn> =
            ApiService::post(&format!("/purchases/returns/{}/reject", id), &RejectReturnRequest { reason }).await?;
        response.into_result()
    }

    /// 获取退货单明细列表
    pub async fn list_items(return_id: i32) -> Result<Vec<PurchaseReturnItem>, String> {
        let response: ApiResponse<Vec<PurchaseReturnItem>> =
            ApiService::get(&format!("/purchases/returns/{}/items", return_id)).await?;
        response.into_result()
    }

    /// 添加退货明细
    pub async fn create_item(return_id: i32, req: CreatePurchaseReturnItemRequest) -> Result<PurchaseReturnItem, String> {
        let response: ApiResponse<PurchaseReturnItem> =
            ApiService::post(&format!("/purchases/returns/{}/items", return_id), &req).await?;
        response.into_result()
    }

    /// 更新退货明细
    pub async fn update_item(return_id: i32, item_id: i32, req: UpdateReturnItemRequest) -> Result<PurchaseReturnItem, String> {
        let response: ApiResponse<PurchaseReturnItem> =
            ApiService::put(&format!("/purchases/returns/{}/items/{}", return_id, item_id), &req).await?;
        response.into_result()
    }

    /// 删除退货明细
    pub async fn delete_item(return_id: i32, item_id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/returns/{}/items/{}", return_id, item_id)).await
    }
}
