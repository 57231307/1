//! 销售价格服务 API 客户端
//! 提供销售价格相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 销售价格模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesPrice {
    pub id: i32,
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: String,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: String,
    pub price_level: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateSalesPriceRequest {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub customer_type: Option<String>,
    pub price: String,
    pub currency: Option<String>,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: Option<String>,
    pub price_level: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

/// 更新销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateSalesPriceRequest {
    pub price: Option<String>,
    pub price_level: Option<String>,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

/// 审批销售价格请求
#[derive(Debug, Clone, Serialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

/// 销售价格服务
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

        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/sales/prices{}", query)).await?;
        
        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let prices: Vec<SalesPrice> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(prices)
        } else {
            Ok(Vec::new())
        }
    }

    /// 获取销售价格详情
    pub async fn get(id: i32) -> Result<SalesPrice, String> {
        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/sales/prices/{}", id)).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取价格详情失败".to_string())
    }

    /// 创建销售价格
    pub async fn create(req: CreateSalesPriceRequest) -> Result<SalesPrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/api/v1/erp/sales/prices", &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建价格失败".to_string())
    }

    /// 更新销售价格
    pub async fn update(id: i32, req: UpdateSalesPriceRequest) -> Result<SalesPrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/api/v1/erp/sales/prices/{}", id), &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新价格失败".to_string())
    }

    /// 删除销售价格
    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/sales/prices/{}", id)).await
    }

    /// 审批销售价格
    pub async fn approve(id: i32, req: ApprovePriceRequest) -> Result<SalesPrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/api/v1/erp/sales/prices/{}/approve", id), &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审批价格失败".to_string())
    }

    /// 获取客户价格等级
    pub async fn get_customer_price_level(customer_type: &str) -> Result<Vec<SalesPrice>, String> {
        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/sales/prices/customer-level/{}", customer_type)).await?;
        
        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let prices: Vec<SalesPrice> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(prices)
        } else {
            Ok(Vec::new())
        }
    }

    /// 获取价格策略
    pub async fn get_strategies(customer_type: Option<&str>) -> Result<Vec<SalesPrice>, String> {
        let query = if let Some(ct) = customer_type {
            format!("?customer_type={}", ct)
        } else {
            String::new()
        };
        
        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/sales/prices/strategies{}", query)).await?;
        
        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let prices: Vec<SalesPrice> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(prices)
        } else {
            Ok(Vec::new())
        }
    }
}
