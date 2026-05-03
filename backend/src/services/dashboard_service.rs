use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, QueryOrder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::models::{inventory_stock, product, sales_order, user, warehouse};
use crate::utils::cache::{AppCache, Cache};

/// 仪表板概览数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct DashboardOverview {
    pub total_products: i64,
    pub total_warehouses: i64,
    pub total_orders: i64,
    pub total_sales: String,
    pub low_stock_count: i64,
    pub pending_orders: i64,
    pub monthly_sales: String,
}

/// 销售统计数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SalesStatistics {
    pub daily_sales: Vec<SalesDataPoint>,
    pub weekly_sales: Vec<SalesDataPoint>,
    pub monthly_sales: Vec<SalesDataPoint>,
    pub by_customer: Vec<SalesByDimension>,
    pub by_product: Vec<SalesByDimension>,
    pub by_salesperson: Vec<SalesByDimension>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SalesDataPoint {
    pub date: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SalesByDimension {
    pub name: String,
    pub amount: String,
    pub count: i64,
}

/// 库存统计数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct InventoryStatistics {
    pub total_inventory: String,
    pub by_warehouse: Vec<InventoryByWarehouse>,
    pub by_category: Vec<InventoryByCategory>,
    pub turnover_rate: String,
    pub aging_analysis: Vec<AgingData>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct InventoryByWarehouse {
    pub warehouse_name: String,
    pub quantity: String,
    pub value: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct InventoryByCategory {
    pub category_name: String,
    pub quantity: String,
    pub value: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct AgingData {
    pub age_range: String,
    pub quantity: String,
    pub percentage: f64,
}

/// 低库存预警项
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct LowStockAlert {
    pub product_id: i32,
    pub product_name: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub current_quantity: String,
    pub min_stock: String,
    pub shortage: String,
}

/// 仪表板服务
pub struct DashboardService {
    db: Arc<DatabaseConnection>,
    cache: Arc<AppCache>,
}

impl DashboardService {
    pub fn new(db: Arc<DatabaseConnection>, cache: Arc<AppCache>) -> Self {
        Self { db, cache }
    }

    /// 获取仪表板概览数据
    pub async fn get_overview(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<DashboardOverview, sea_orm::DbErr> {
        // 生成缓存键
        let cache_key = format!("dashboard:overview:{}-{}", 
            start_date.map(|d| d.to_rfc3339()).unwrap_or("all".to_string()),
            end_date.map(|d| d.to_rfc3339()).unwrap_or("all".to_string())
        );

        // 尝试从缓存获取
        if let Some(cached) = self.cache.get_dashboard_cache().get(&cache_key) {
            if let Ok(overview) = serde_json::from_value(cached) {
                return Ok(overview);
            }
        }

        // 缓存未命中，从数据库获取
        let total_products = product::Entity::find().filter(product::Column::IsDeleted.eq(false)).count(&*self.db).await? as i64;
        let total_warehouses = warehouse::Entity::find().filter(warehouse::Column::IsDeleted.eq(false)).count(&*self.db).await? as i64;
        let total_orders = sales_order::Entity::find().filter(sales_order::Column::IsDeleted.eq(false)).count(&*self.db).await? as i64;

        let pending_orders = sales_order::Entity::find()
            .filter(sales_order::Column::IsDeleted.eq(false))
            .filter(sales_order::Column::Status.eq("pending"))
            .count(&*self.db)
            .await? as i64;
            
        let low_stock_count = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::IsDeleted.eq(false))
            .filter(
                Expr::col(inventory_stock::Column::QuantityMeters)
                    .lt(Expr::col(inventory_stock::Column::ReorderPoint)),
            )
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .count(self.db.as_ref())
            .await? as i64;
            
        // 本月销售额
        let now = Utc::now();
        use chrono::Datelike;
        let start_of_month = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
        let monthly_sales_dec = sales_order::Entity::find()
            .filter(sales_order::Column::IsDeleted.eq(false))
            .filter(sales_order::Column::OrderDate.gte(start_of_month))
            .select_only()
            .column_as(Expr::col(sales_order::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);
            
        // 总销售额
        let total_sales_dec = sales_order::Entity::find()
            .filter(sales_order::Column::IsDeleted.eq(false))
            .select_only()
            .column_as(Expr::col(sales_order::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        let overview = DashboardOverview {
            total_products,
            total_warehouses,
            total_orders,
            total_sales: total_sales_dec.to_string(),
            low_stock_count,
            pending_orders,
            monthly_sales: monthly_sales_dec.to_string(),
        };

        // 缓存结果，有效期5分钟
        if let Ok(overview_json) = serde_json::to_value(overview.clone()) {
            self.cache.get_dashboard_cache().set(cache_key, overview_json, Some(Duration::from_secs(300)));
        }

        Ok(overview)
    }

    /// 获取销售统计数据
    pub async fn get_sales_statistics(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<SalesStatistics, sea_orm::DbErr> {
        // 生成缓存键
        let cache_key = format!("dashboard:sales:{}-{}", 
            start_date.map(|d| d.to_rfc3339()).unwrap_or("all".to_string()),
            end_date.map(|d| d.to_rfc3339()).unwrap_or("all".to_string())
        );

        // 尝试从缓存获取
        if let Some(cached) = self.cache.get_dashboard_cache().get(&cache_key) {
            if let Ok(statistics) = serde_json::from_value(cached) {
                return Ok(statistics);
            }
        }

        let mut query = sales_order::Entity::find().filter(sales_order::Column::IsDeleted.eq(false));

        // 应用日期范围过滤
        if let Some(start) = start_date {
            query = query.filter(sales_order::Column::OrderDate.gte(start.date_naive()));
        }
        if let Some(end) = end_date {
            query = query.filter(sales_order::Column::OrderDate.lte(end.date_naive()));
        }

        // 生成 daily_sales 示例（实际需 GROUP BY）
        // 简化起见，按日分组聚合金额
        let daily_results = query
            .clone()
            .select_only()
            .column(sales_order::Column::OrderDate)
            .column_as(Expr::col(sales_order::Column::TotalAmount).sum(), "amount")
            .group_by(sales_order::Column::OrderDate)
            .order_by_asc(sales_order::Column::OrderDate)
            .into_tuple::<(chrono::NaiveDate, Option<Decimal>)>()
            .all(self.db.as_ref())
            .await?;

        let daily_sales = daily_results.into_iter().map(|(date, amt)| SalesDataPoint {
            date: date.to_string(),
            amount: amt.unwrap_or(Decimal::ZERO).to_string(),
        }).collect();

        let statistics = SalesStatistics {
            daily_sales,
            weekly_sales: vec![],
            monthly_sales: vec![],
            by_customer: vec![],
            by_product: vec![],
            by_salesperson: vec![],
        };

        // 缓存结果，有效期5分钟
        if let Ok(statistics_json) = serde_json::to_value(statistics.clone()) {
            self.cache.get_dashboard_cache().set(cache_key, statistics_json, Some(Duration::from_secs(300)));
        }

        Ok(statistics)
    }

    /// 获取库存统计数据
    pub async fn get_inventory_statistics(
        &self,
        _start_date: Option<DateTime<Utc>>,
        _end_date: Option<DateTime<Utc>>,
    ) -> Result<InventoryStatistics, sea_orm::DbErr> {
        // 生成缓存键
        let cache_key = "dashboard:inventory:all".to_string();

        // 尝试从缓存获取
        if let Some(cached) = self.cache.get_dashboard_cache().get(&cache_key) {
            if let Ok(statistics) = serde_json::from_value(cached) {
                return Ok(statistics);
            }
        }

        // 总库存数量 - 暂时使用简单查询
        let total_quantity = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::IsDeleted.eq(false))
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .select_only()
            .column_as(
                Expr::col(inventory_stock::Column::QuantityMeters).sum(),
                "total",
            )
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 低库存产品数
        let low_stock_count = inventory_stock::Entity::find()
            .filter(
                Expr::col(inventory_stock::Column::QuantityMeters)
                    .lt(Expr::col(inventory_stock::Column::ReorderPoint)),
            )
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .count(self.db.as_ref())
            .await? as i64;

        // 零库存产品数
        let zero_stock_count = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::QuantityMeters.eq(Decimal::ZERO))
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .count(self.db.as_ref())
            .await? as i64;

        // 仓库分布统计 - 暂时简化处理
        let warehouse_distribution = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::IsDeleted.eq(false))
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .select_only()
            .column(inventory_stock::Column::WarehouseId)
            .column_as(
                Expr::col(inventory_stock::Column::QuantityMeters).sum(),
                "total_qty",
            )
            .group_by(inventory_stock::Column::WarehouseId)
            .into_tuple::<(i32, Option<Decimal>)>()
            .all(self.db.as_ref())
            .await?;

        let mut warehouse_stats = Vec::new();
        for (wh_id, qty) in warehouse_distribution {
            // 获取仓库名称
            let wh = warehouse::Entity::find_by_id(wh_id)
                .filter(warehouse::Column::IsDeleted.eq(false))
                .one(self.db.as_ref())
                .await?;
            if let Some(warehouse_model) = wh {
                warehouse_stats.push(InventoryByWarehouse {
                    warehouse_name: warehouse_model.name,
                    quantity: qty.unwrap_or(Decimal::ZERO).to_string(),
                    value: "0.0".to_string(),
                });
            }
        }

        let statistics = InventoryStatistics {
            total_inventory: total_quantity.to_string(),
            turnover_rate: "0.0".to_string(),
            by_warehouse: warehouse_stats,
            by_category: vec![],
            aging_analysis: vec![],
        };

        // 缓存结果，有效期5分钟
        if let Ok(statistics_json) = serde_json::to_value(statistics.clone()) {
            self.cache.get_dashboard_cache().set(cache_key, statistics_json, Some(Duration::from_secs(300)));
        }

        Ok(statistics)
    }

    /// 获取低库存预警数据
    pub async fn get_low_stock_alerts(&self) -> Result<Vec<LowStockAlert>, sea_orm::DbErr> {
        // 生成缓存键
        let cache_key = "inventory:low_stock".to_string();

        // 尝试从缓存获取
        if let Some(cached) = self.cache.get_inventory_cache().get(&cache_key) {
            if let Ok(alerts) = serde_json::from_value(cached) {
                return Ok(alerts);
            }
        }

        let low_stock_items = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::IsDeleted.eq(false))
            .filter(
                Expr::col(inventory_stock::Column::QuantityMeters)
                    .lt(Expr::col(inventory_stock::Column::ReorderPoint)),
            )
            .filter(inventory_stock::Column::StockStatus.eq("active"))
            .all(&*self.db)
            .await?;

        let mut alerts = Vec::new();
        for item in low_stock_items {
            // 获取产品信息
            let product = product::Entity::find_by_id(item.product_id)
                .filter(product::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?;
            // 获取仓库信息
            let wh = warehouse::Entity::find_by_id(item.warehouse_id)
                .filter(warehouse::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?;

            if let (Some(p), Some(w)) = (product, wh) {
                let shortage = item.reorder_point - item.quantity_available;
                alerts.push(LowStockAlert {
                    product_id: item.product_id,
                    product_name: p.name,
                    warehouse_id: item.warehouse_id,
                    warehouse_name: w.name,
                    current_quantity: item.quantity_available.to_string(),
                    min_stock: item.reorder_point.to_string(),
                    shortage: shortage.to_string(),
                });
            }
        }

        // 缓存结果，有效期5分钟
        if let Ok(alerts_json) = serde_json::to_value(alerts.clone()) {
            self.cache.get_inventory_cache().set(cache_key, alerts_json, Some(Duration::from_secs(300)));
        }

        Ok(alerts)
    }
}
