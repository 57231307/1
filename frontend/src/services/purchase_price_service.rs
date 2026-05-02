//! 采购价格服务 API 客户端
//! 提供采购价格相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::purchase_price::{
    ApprovePriceRequest, CreatePurchasePriceRequest, PriceTrendAnalysis, PurchasePrice,
    UpdatePurchasePriceRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

pub struct PurchasePriceService;

impl CrudService for PurchasePriceService {
    type Model = PurchasePrice;
    type ListResponse = Vec<PurchasePrice>;
    type CreateRequest = CreatePurchasePriceRequest;
    type UpdateRequest = UpdatePurchasePriceRequest;

    fn base_path() -> &'static str {
        "/purchases/prices"
    }
}


impl PurchasePriceService {
    /// 查询采购价格列表
    pub async fn list(
        product_id: Option<i32>,
        supplier_id: Option<i32>,
        price_type: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<PurchasePrice>, String> {
        let mut params = Vec::new();
        if let Some(pid) = product_id {
            params.push(format!("product_id={}", pid));
        }
        if let Some(sid) = supplier_id {
            params.push(format!("supplier_id={}", sid));
        }
        if let Some(pt) = price_type {
            params.push(format!("price_type={}", pt));
        }
        if let Some(s) = status {
            params.push(format!("status={}", s));
        }
        params.push(format!("page={}", page));
        params.push(format!("page_size={}", page_size));

        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        let response: ApiResponse<Vec<PurchasePrice>> =
            ApiService::get(&format!("/purchases/prices{}", query)).await?;
        response.into_result()
    }

    /// 获取采购价格详情

    /// 创建采购价格

    /// 更新采购价格

    /// 删除采购价格

    /// 审批采购价格
    pub async fn approve(id: i32, req: ApprovePriceRequest) -> Result<PurchasePrice, String> {
        let response: ApiResponse<PurchasePrice> =
            ApiService::post(&format!("/purchases/prices/{}/approve", id), &req).await?;
        response.into_result()
    }

    /// 获取价格历史
    pub async fn history(product_id: i32, supplier_id: i32, limit: i64) -> Result<Vec<PurchasePrice>, String> {
        let response: ApiResponse<Vec<PurchasePrice>> = ApiService::get(
            &format!("/purchases/prices/history/{}/{}?limit={}", product_id, supplier_id, limit)
        ).await?;
        response.into_result()
    }

    /// 分析价格趋势
    pub async fn analyze_trend(product_id: i32, supplier_id: i32) -> Result<PriceTrendAnalysis, String> {
        let response: ApiResponse<PriceTrendAnalysis> = ApiService::get(
            &format!("/purchases/prices/trend/{}/{}", product_id, supplier_id)
        ).await?;
        response.into_result()
    }
}
