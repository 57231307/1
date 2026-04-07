//! 仪表板模型

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub total_products: i64,
    pub total_warehouses: i64,
    pub total_inventory_value: String,
    pub total_orders: i64,
    pub pending_orders: i64,
    pub total_users: i64,
    pub active_users: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SalesStatistics {
    pub total_sales_amount: String,
    pub order_count: i64,
    pub avg_order_amount: String,
    pub completed_orders: i64,
    pub pending_orders: i64,
    pub cancelled_orders: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryStatistics {
    pub total_quantity: String,
    pub total_value: String,
    pub low_stock_count: i64,
    pub zero_stock_count: i64,
    pub warehouse_distribution: Vec<WarehouseStockStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseStockStat {
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub total_quantity: String,
    pub total_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockAlert {
    pub product_id: i32,
    pub product_name: String,
    pub product_code: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub current_quantity: String,
    pub min_stock: String,
    pub shortage: String,
}
