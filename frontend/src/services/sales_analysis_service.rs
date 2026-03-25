use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTrendAnalysis {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub average_daily_sales: String,
    pub growth_rate: String,
    pub trend_direction: String,
    pub peak_date: Option<String>,
    pub lowest_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductRanking {
    pub rank: i32,
    pub product_id: i32,
    pub product_name: Option<String>,
    pub product_code: Option<String>,
    pub category_id: Option<i32>,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub gross_profit: String,
    pub gross_margin: String,
    pub customer_count: i64,
    pub order_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerRanking {
    pub rank: i32,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub customer_type: String,
    pub total_sales_amount: String,
    pub total_sales_quantity: i64,
    pub gross_profit: String,
    pub order_count: i64,
    pub average_order_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesTarget {
    pub id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: String,
    pub actual_amount: String,
    pub completion_rate: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesTargetRequest {
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: String,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSalesTargetRequest {
    pub target_amount: Option<String>,
    pub status: Option<String>,
}

pub struct SalesAnalysisService;

impl SalesAnalysisService {
    pub async fn get_trend_analysis(
        period: &str,
        start_date: &str,
        end_date: &str,
        product_id: Option<i32>,
        customer_id: Option<i32>,
    ) -> Result<SalesTrendAnalysis, String> {
        let mut query = format!(
            "period={}&start_date={}&end_date={}",
            period, start_date, end_date
        );
        if let Some(pid) = product_id {
            query.push_str(&format!("&product_id={}", pid));
        }
        if let Some(cid) = customer_id {
            query.push_str(&format!("&customer_id={}", cid));
        }
        
        ApiService::get(&format!("/api/v1/erp/sales-analysis/trend?{}", query)).await
    }

    pub async fn get_product_ranking(
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        category_id: Option<i32>,
        limit: i32,
    ) -> Result<Vec<ProductRanking>, String> {
        let mut query = String::new();
        if let Some(p) = period {
            query.push_str(&format!("period={}&", p));
        }
        if let Some(sd) = start_date {
            query.push_str(&format!("start_date={}&", sd));
        }
        if let Some(ed) = end_date {
            query.push_str(&format!("end_date={}&", ed));
        }
        if let Some(cid) = category_id {
            query.push_str(&format!("category_id={}&", cid));
        }
        query.push_str(&format!("limit={}", limit));
        
        ApiService::get(&format!("/api/v1/erp/sales-analysis/product-ranking?{}", query)).await
    }

    pub async fn get_customer_ranking(
        period: Option<&str>,
        start_date: Option<&str>,
        end_date: Option<&str>,
        customer_type: Option<&str>,
        limit: i32,
    ) -> Result<Vec<CustomerRanking>, String> {
        let mut query = String::new();
        if let Some(p) = period {
            query.push_str(&format!("period={}&", p));
        }
        if let Some(sd) = start_date {
            query.push_str(&format!("start_date={}&", sd));
        }
        if let Some(ed) = end_date {
            query.push_str(&format!("end_date={}&", ed));
        }
        if let Some(ct) = customer_type {
            query.push_str(&format!("customer_type={}&", ct));
        }
        query.push_str(&format!("limit={}", limit));
        
        ApiService::get(&format!("/api/v1/erp/sales-analysis/customer-ranking?{}", query)).await
    }

    pub async fn list_targets(
        target_type: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<SalesTarget>, String> {
        let mut query = String::new();
        if let Some(tt) = target_type {
            query.push_str(&format!("target_type={}&", tt));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));
        
        ApiService::get(&format!("/api/v1/erp/sales-analysis/targets?{}", query)).await
    }

    pub async fn get_target(id: i32) -> Result<SalesTarget, String> {
        ApiService::get(&format!("/api/v1/erp/sales-analysis/targets/{}", id)).await
    }

    pub async fn create_target(req: CreateSalesTargetRequest) -> Result<SalesTarget, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/sales-analysis/targets", &payload).await
    }

    pub async fn update_target(id: i32, req: UpdateSalesTargetRequest) -> Result<SalesTarget, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/api/v1/erp/sales-analysis/targets/{}", id), &payload).await
    }

    pub async fn delete_target(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/sales-analysis/targets/{}", id)).await
    }
}
