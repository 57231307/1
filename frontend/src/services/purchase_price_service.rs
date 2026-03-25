//! 采购价格服务 API 客户端
//! 提供采购价格相关的 API 调用方法

use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};

/// 采购价格模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchasePrice {
    pub id: i32,
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: String,
    pub currency: String,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: String,
    pub effective_date: String,
    pub expiry_date: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchasePriceRequest {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: String,
    pub currency: Option<String>,
    pub unit: String,
    pub min_order_qty: Option<String>,
    pub price_type: Option<String>,
    pub effective_date: String,
    pub expiry_date: Option<String>,
}

/// 更新采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchasePriceRequest {
    pub price: Option<String>,
    pub expiry_date: Option<String>,
    pub status: Option<String>,
}

/// 审批采购价格请求
#[derive(Debug, Clone, Serialize)]
pub struct ApprovePriceRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

/// 价格趋势分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTrendAnalysis {
    pub product_id: i32,
    pub supplier_id: i32,
    pub current_price: String,
    pub average_price: String,
    pub min_price: String,
    pub max_price: String,
    pub price_change_rate: String,
    pub trend_direction: String,
    pub history_count: i64,
}

/// 采购价格服务
pub struct PurchasePriceService;

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

        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/purchases/prices{}", query)).await?;
        
        // 解析响应
        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let prices: Vec<PurchasePrice> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(prices)
        } else {
            Ok(Vec::new())
        }
    }

    /// 获取采购价格详情
    pub async fn get(id: i32) -> Result<PurchasePrice, String> {
        let response: serde_json::Value = ApiService::get(&format!("/api/v1/erp/purchases/prices/{}", id)).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "获取价格详情失败".to_string())
    }

    /// 创建采购价格
    pub async fn create(req: CreatePurchasePriceRequest) -> Result<PurchasePrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post("/api/v1/erp/purchases/prices", &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "创建价格失败".to_string())
    }

    /// 更新采购价格
    pub async fn update(id: i32, req: UpdatePurchasePriceRequest) -> Result<PurchasePrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::put(&format!("/api/v1/erp/purchases/prices/{}", id), &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "更新价格失败".to_string())
    }

    /// 删除采购价格
    pub async fn delete(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/purchases/prices/{}", id)).await
    }

    /// 审批采购价格
    pub async fn approve(id: i32, req: ApprovePriceRequest) -> Result<PurchasePrice, String> {
        let body = serde_json::to_value(&req).map_err(|e| format!("序列化失败：{}", e))?;
        let response: serde_json::Value = ApiService::post(&format!("/api/v1/erp/purchases/prices/{}/approve", id), &body).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "审批价格失败".to_string())
    }

    /// 获取价格历史
    pub async fn history(product_id: i32, supplier_id: i32, limit: i64) -> Result<Vec<PurchasePrice>, String> {
        let response: serde_json::Value = ApiService::get(
            &format!("/api/v1/erp/purchases/prices/history/{}/{}?limit={}", product_id, supplier_id, limit)
        ).await?;
        
        if let Some(data) = response.get("data").and_then(|v| v.as_array()) {
            let prices: Vec<PurchasePrice> = data
                .iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect();
            Ok(prices)
        } else {
            Ok(Vec::new())
        }
    }

    /// 分析价格趋势
    pub async fn analyze_trend(product_id: i32, supplier_id: i32) -> Result<PriceTrendAnalysis, String> {
        let response: serde_json::Value = ApiService::get(
            &format!("/api/v1/erp/purchases/prices/trend/{}/{}", product_id, supplier_id)
        ).await?;
        
        response
            .get("data")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "分析价格趋势失败".to_string())
    }
}
