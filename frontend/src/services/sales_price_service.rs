//! 销售价格服务 API 客户端
//! 提供销售价格相关的 API 调用方法

use crate::models::api_response::ApiResponse;
use crate::models::sales_price::{
    ApprovePriceRequest, CreateSalesPriceRequest, SalesPrice, UpdateSalesPriceRequest,
};
use crate::services::api::ApiService;

pub struct SalesPriceService;

impl SalesPriceService {
    /// 查询销售价格列表
    pub async fn list(
        product_id: Option<i32>,
        customer_id: Option<i32>,
        customer_type: Option<&str>,
        price_level: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<SalesPrice>, String> {
        let mut params = Vec::new();
        if let Some(pid) = product_id {
            params.push(format!("product_id={}", pid));
        }
        if let Some(cid) = customer_id {
            params.push(format!("customer_id={}", cid));
        }
        if let Some(ct) = customer_type {
            params.push(format!("customer_type={}", ct));
        }
        if let Some(pl) = price_level {
            params.push(format!("price_level={}", pl));
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

        let response: ApiResponse<Vec<SalesPrice>> =
            ApiService::get(&format!("/sales-prices{}", query)).await?;
        response.into_result()
    }

    /// 获取销售价格详情
    pub async fn get(id: i32) -> Result<SalesPrice, String> {
        let response: ApiResponse<SalesPrice> =
            ApiService::get(&format!("/sales-prices/{}", id)).await?;
        response.into_result()
    }

    /// 创建销售价格
    pub async fn create(req: CreateSalesPriceRequest) -> Result<SalesPrice, String> {
        let response: ApiResponse<SalesPrice> = ApiService::post("/sales-prices", &req).await?;
        response.into_result()
    }

    /// 更新销售价格
    pub async fn update(id: i32, req: UpdateSalesPriceRequest) -> Result<SalesPrice, String> {
        let response: ApiResponse<SalesPrice> =
            ApiService::put(&format!("/sales-prices/{}", id), &req).await?;
        response.into_result()
    }

    /// 删除销售价格
    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/sales-prices/{}", id)).await
    }

    /// 审批销售价格
    pub async fn approve(id: i32, req: ApprovePriceRequest) -> Result<SalesPrice, String> {
        let response: ApiResponse<SalesPrice> =
            ApiService::post(&format!("/sales-prices/{}/approve", id), &req).await?;
        response.into_result()
    }

    /// 获取客户价格等级
    pub async fn get_customer_price_level(customer_type: &str) -> Result<Vec<SalesPrice>, String> {
        let response: ApiResponse<Vec<SalesPrice>> =
            ApiService::get(&format!("/sales-prices/customer-level/{}", customer_type)).await?;
        response.into_result()
    }

    /// 获取价格策略
    pub async fn get_strategies(customer_type: Option<&str>) -> Result<Vec<SalesPrice>, String> {
        let query = if let Some(ct) = customer_type {
            format!("?customer_type={}", ct)
        } else {
            String::new()
        };

        let response: ApiResponse<Vec<SalesPrice>> =
            ApiService::get(&format!("/sales-prices/strategies{}", query)).await?;
        response.into_result()
    }
}
