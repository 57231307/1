use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use crate::utils::storage;

/// 仪表板概览数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub total_products: i64,
    pub total_warehouses: i64,
    pub total_orders: i64,
    pub total_sales: String,
    pub low_stock_count: i64,
    pub pending_orders: i64,
    pub monthly_sales: String,
}

/// 销售统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesStatistics {
    pub daily_sales: Vec<SalesDataPoint>,
    pub weekly_sales: Vec<SalesDataPoint>,
    pub monthly_sales: Vec<SalesDataPoint>,
    pub by_customer: Vec<SalesByDimension>,
    pub by_product: Vec<SalesByDimension>,
    pub by_salesperson: Vec<SalesByDimension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesDataPoint {
    pub date: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesByDimension {
    pub name: String,
    pub amount: String,
    pub count: i64,
}

/// 库存统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryStatistics {
    pub total_inventory: String,
    pub by_warehouse: Vec<InventoryByWarehouse>,
    pub by_category: Vec<InventoryByCategory>,
    pub turnover_rate: String,
    pub aging_analysis: Vec<AgingData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryByWarehouse {
    pub warehouse_name: String,
    pub quantity: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryByCategory {
    pub category_name: String,
    pub quantity: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingData {
    pub age_range: String,
    pub quantity: String,
    pub percentage: f64,
}

/// 低库存预警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockAlert {
    pub product_id: i32,
    pub product_name: String,
    pub warehouse_name: String,
    pub current_quantity: String,
    pub min_stock: String,
    pub shortage: String,
}

/// 仪表板服务
pub struct DashboardService;

impl DashboardService {
    /// 获取仪表板概览数据
    pub async fn get_overview(start_date: &str, end_date: &str) -> Result<DashboardOverview, String> {
        let token = storage::Storage::get_token().ok_or("未登录")?;
        
        let url = format!(
            "/api/v1/erp/dashboard/overview?start_date={}&end_date={}",
            start_date, end_date
        );
        
        let response = Request::get(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("请求失败：{}", e))?;
        
        if !response.ok() {
            return Err(format!("请求失败：{}", response.status()));
        }
        
        let api_response: ApiResponse<DashboardOverview> = response
            .json()
            .await
            .map_err(|e| format!("解析失败：{}", e))?;
        
        if api_response.success {
            Ok(api_response.data.ok_or("数据为空")?)
        } else {
            Err(api_response.message.unwrap_or("未知错误".to_string()))
        }
    }

    /// 获取销售统计
    pub async fn get_sales_statistics(start_date: &str, end_date: &str) -> Result<SalesStatistics, String> {
        let token = storage::Storage::get_token().ok_or("未登录")?;
        
        let url = format!(
            "/api/v1/erp/dashboard/sales-stats?start_date={}&end_date={}",
            start_date, end_date
        );
        
        let response = Request::get(&url)
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("请求失败：{}", e))?;
        
        if !response.ok() {
            return Err(format!("请求失败：{}", response.status()));
        }
        
        let api_response: ApiResponse<SalesStatistics> = response
            .json()
            .await
            .map_err(|e| format!("解析失败：{}", e))?;
        
        if api_response.success {
            Ok(api_response.data.ok_or("数据为空")?)
        } else {
            Err(api_response.message.unwrap_or("未知错误".to_string()))
        }
    }

    /// 获取库存统计
    pub async fn get_inventory_statistics() -> Result<InventoryStatistics, String> {
        let token = storage::Storage::get_token().ok_or("未登录")?;
        
        let response = Request::get("/api/v1/erp/dashboard/inventory-stats")
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("请求失败：{}", e))?;
        
        if !response.ok() {
            return Err(format!("请求失败：{}", response.status()));
        }
        
        let api_response: ApiResponse<InventoryStatistics> = response
            .json()
            .await
            .map_err(|e| format!("解析失败：{}", e))?;
        
        if api_response.success {
            Ok(api_response.data.ok_or("数据为空")?)
        } else {
            Err(api_response.message.unwrap_or("未知错误".to_string()))
        }
    }

    /// 获取低库存预警
    pub async fn get_low_stock_alerts() -> Result<Vec<LowStockAlert>, String> {
        let token = storage::Storage::get_token().ok_or("未登录")?;
        
        let response = Request::get("/api/v1/erp/dashboard/low-stock-alerts")
            .header("Authorization", &format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("请求失败：{}", e))?;
        
        if !response.ok() {
            return Err(format!("请求失败：{}", response.status()));
        }
        
        let api_response: ApiResponse<Vec<LowStockAlert>> = response
            .json()
            .await
            .map_err(|e| format!("解析失败：{}", e))?;
        
        if api_response.success {
            Ok(api_response.data.ok_or("数据为空")?)
        } else {
            Err(api_response.message.unwrap_or("未知错误".to_string()))
        }
    }
}

/// API 响应结构
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    message: Option<String>,
    data: Option<T>,
}
