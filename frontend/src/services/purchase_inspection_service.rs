//! 采购检验服务 API 客户端
//! 提供采购检验相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::purchase_inspection::{
    CompleteInspectionRequest, CreatePurchaseInspectionRequest, InspectionListResponse,
    PurchaseInspection, PurchaseInspectionQuery, UpdatePurchaseInspectionRequest,
};
use crate::services::api::ApiService;

pub struct PurchaseInspectionService;

impl PurchaseInspectionService {
    /// 获取检验单列表
    pub async fn list(query: PurchaseInspectionQuery) -> Result<InspectionListResponse, String> {
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

        let response: ApiResponse<InspectionListResponse> =
            ApiService::get(&format!("/purchase-inspections{}", query_string)).await?;
        response.into_result()
    }

    /// 获取检验单详情
    pub async fn get(id: i32) -> Result<PurchaseInspection, String> {
        let response: ApiResponse<PurchaseInspection> =
            ApiService::get(&format!("/purchase-inspections/{}", id)).await?;
        response.into_result()
    }

    /// 创建采购检验单
    pub async fn create(req: CreatePurchaseInspectionRequest) -> Result<PurchaseInspection, String> {
        let response: ApiResponse<PurchaseInspection> =
            ApiService::post("/purchase-inspections", &req).await?;
        response.into_result()
    }

    /// 更新采购检验单
    #[allow(dead_code)]
    pub async fn update(id: i32, req: UpdatePurchaseInspectionRequest) -> Result<PurchaseInspection, String> {
        let response: ApiResponse<PurchaseInspection> =
            ApiService::put(&format!("/purchase-inspections/{}", id), &req).await?;
        response.into_result()
    }

    /// 完成采购检验单
    pub async fn complete(id: i32, req: CompleteInspectionRequest) -> Result<PurchaseInspection, String> {
        let response: ApiResponse<PurchaseInspection> =
            ApiService::post(&format!("/purchase-inspections/{}/complete", id), &req).await?;
        response.into_result()
    }
}
