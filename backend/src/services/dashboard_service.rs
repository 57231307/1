use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::prelude::*;
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::models::{inventory_stock, product, sales_order, user, warehouse};
use crate::utils::cache::{AppCache, Cache};

/// 仪表板概览数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct DashboardOverview {
    /// 总产品数
    pub total_products: i64,
    /// 总仓库数
    pub total_warehouses: i64,
    /// 总库存金额
    pub total_inventory_value: Decimal,
    /// 总订单数
    pub total_orders: i64,
    /// 待处理订单数
    pub pending_orders: i64,
    /// 总用户数
    pub total_users: i64,
    /// 活跃用户数（最近 7 天登录）
    pub active_users: i64,
}

/// 销售统计数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct SalesStatistics {
    /// 销售总额
    pub total_sales_amount: Decimal,
    /// 订单总数
    pub order_count: i64,
    /// 平均每单金额
    pub avg_order_amount: Decimal,
    /// 已完成订单数
    pub completed_orders: i64,
    /// 待处理订单数
    pub pending_orders: i64,
    /// 已取消订单数
    pub cancelled_orders: i64,
}

/// 库存统计数据
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct InventoryStatistics {
    /// 总库存数量
    pub total_quantity: Decimal,
    /// 总库存金额
    pub total_value: Decimal,
    /// 低库存产品数
    pub low_stock_count: i64,
    /// 零库存产品数
    pub zero_stock_count: i64,
    /// 仓库分布统计
    pub warehouse_distribution: Vec<WarehouseStockStat>,
}

/// 仓库库存统计
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct WarehouseStockStat {
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub total_quantity: Decimal,
    pub total_value: Decimal,
}

/// 低库存预警项
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct LowStockAlert {
    pub product_id: i32,
    pub product_name: String,
    pub product_code: String,
    pub warehouse_id: i32,
    pub warehouse_name: String,
    pub current_quantity: Decimal,
    pub min_stock: Decimal,
    pub shortage: Decimal,
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
        // 总产品数
        let total_products = product::Entity::find().count(&*self.db).await?;

        // 总仓库数
        let total_warehouses = warehouse::Entity::find().count(&*self.db).await?;

        // 总库存金额 - 暂时跳过此字段
        let total_inventory_value = Decimal::ZERO;

        // 总订单数
        let total_orders = sales_order::Entity::find().count(&*self.db).await?;

        // 待处理订单数
        let pending_orders = sales_order::Entity::find()
            .filter(sales_order::Column::Status.eq("pending"))
            .count(&*self.db)
            .await?;

        // 总用户数
        let total_users = user::Entity::find().count(&*self.db).await?;

        // 活跃用户数（最近 7 天登录）
        let seven_days_ago = Utc::now() - chrono::Duration::days(7);
        let active_users = user::Entity::find()
            .filter(user::Column::LastLoginAt.gte(seven_days_ago))
            .count(&*self.db)
            .await?;

        let overview = DashboardOverview {
            total_products: total_products as i64,
            total_warehouses: total_warehouses as i64,
            total_inventory_value,
            total_orders: total_orders as i64,
            pending_orders: pending_orders as i64,
            total_users: total_users as i64,
            active_users: active_users as i64,
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

        let mut query = sales_order::Entity::find();

        // 应用日期范围过滤
        if let Some(start) = start_date {
            query = query.filter(sales_order::Column::OrderDate.gte(start.date_naive()));
        }
        if let Some(end) = end_date {
            query = query.filter(sales_order::Column::OrderDate.lte(end.date_naive()));
        }

        // 销售总额
        let total_sales_amount = query
            .clone()
            .select_only()
            .column_as(Expr::col(sales_order::Column::TotalAmount).sum(), "total")
            .into_tuple::<Option<Decimal>>()
            .one(self.db.as_ref())
            .await?
            .flatten()
            .unwrap_or(Decimal::ZERO);

        // 订单总数
        let order_count = query.clone().count(self.db.as_ref()).await?;

        // 平均每单金额
        let avg_order_amount = if order_count > 0 {
            total_sales_amount / Decimal::from(order_count as i32)
        } else {
            Decimal::ZERO
        };

        // 已完成订单数
        let completed_orders = sales_order::Entity::find()
            .filter(sales_order::Column::Status.eq("completed"))
            .count(self.db.as_ref())
            .await?;

        // 待处理订单数
        let pending_orders = sales_order::Entity::find()
            .filter(sales_order::Column::Status.eq("pending"))
            .count(self.db.as_ref())
            .await?;

        // 已取消订单数
        let cancelled_orders = sales_order::Entity::find()
            .filter(sales_order::Column::Status.eq("cancelled"))
            .count(self.db.as_ref())
            .await?;

        let statistics = SalesStatistics {
            total_sales_amount,
            order_count: order_count as i64,
            avg_order_amount,
            completed_orders: completed_orders as i64,
            pending_orders: pending_orders as i64,
            cancelled_orders: cancelled_orders as i64,
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
                .one(self.db.as_ref())
                .await?;
            if let Some(warehouse_model) = wh {
                warehouse_stats.push(WarehouseStockStat {
                    warehouse_id: wh_id,
                    warehouse_name: warehouse_model.name,
                    total_quantity: qty.unwrap_or(Decimal::ZERO),
                    total_value: Decimal::ZERO,
                });
            }
        }

        let statistics = InventoryStatistics {
            total_quantity,
            total_value: Decimal::ZERO,
            low_stock_count,
            zero_stock_count,
            warehouse_distribution: warehouse_stats,
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
                .one(&*self.db)
                .await?;
            // 获取仓库信息
            let wh = warehouse::Entity::find_by_id(item.warehouse_id)
                .one(&*self.db)
                .await?;

            if let (Some(p), Some(w)) = (product, wh) {
                let shortage = item.reorder_point - item.quantity_available;
                alerts.push(LowStockAlert {
                    product_id: item.product_id,
                    product_name: p.name,
                    product_code: p.code,
                    warehouse_id: item.warehouse_id,
                    warehouse_name: w.name,
                    current_quantity: item.quantity_available,
                    min_stock: item.reorder_point,
                    shortage,
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
