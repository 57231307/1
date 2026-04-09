use crate::services::api::ApiService;
use crate::models::dashboard::{
    DashboardOverview, InventoryStatistics, LowStockAlert, SalesStatistics,
};

/// 仪表板服务
pub struct DashboardService;

impl DashboardService {
    /// 获取仪表板概览数据
    pub async fn get_overview(start_date: &str, end_date: &str) -> Result<DashboardOverview, String> {
        let url = format!(
            "/dashboard/overview?start_date={}&end_date={}",
            urlencoding::encode(start_date), urlencoding::encode(end_date)
        );
        ApiService::get(&url).await
    }

    /// 获取销售统计
    pub async fn get_sales_statistics(start_date: &str, end_date: &str) -> Result<SalesStatistics, String> {
        let url = format!(
            "/dashboard/sales-stats?start_date={}&end_date={}",
            urlencoding::encode(start_date), urlencoding::encode(end_date)
        );
        ApiService::get(&url).await
    }

    /// 获取库存统计
    pub async fn get_inventory_statistics() -> Result<InventoryStatistics, String> {
        ApiService::get("/dashboard/inventory-stats").await
    }

    /// 获取低库存预警
    pub async fn get_low_stock_alerts() -> Result<Vec<LowStockAlert>, String> {
        ApiService::get("/dashboard/low-stock-alerts").await
    }
}
