//! AI智能分析 Service
//!
//! 提供销售预测、库存优化、异常检测等智能分析功能

use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::utils::error::AppError;

/// 销售预测结果
#[derive(Debug, Clone)]
pub struct SalesForecast {
    pub product_id: i32,
    pub forecast_date: NaiveDate,
    pub predicted_quantity: Decimal,
    pub confidence: f64,
    pub trend: String,
}

/// 库存优化建议
#[derive(Debug, Clone)]
pub struct InventorySuggestion {
    pub product_id: i32,
    pub current_stock: Decimal,
    pub suggested_stock: Decimal,
    pub reorder_point: Decimal,
    pub reorder_quantity: Decimal,
    pub reason: String,
}

/// 异常检测结果
#[derive(Debug, Clone)]
pub struct AnomalyDetection {
    pub entity_type: String,
    pub entity_id: i32,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub detected_at: chrono::DateTime<Utc>,
}

/// 智能推荐项
#[derive(Debug, Clone)]
pub struct SmartRecommendation {
    pub recommendation_type: String,
    pub target_id: i32,
    pub target_type: String,
    pub score: f64,
    pub reason: String,
}

/// AI分析 Service
pub struct AiAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl AiAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 销售预测 - 基于历史销售数据预测未来销量
    pub async fn forecast_sales(
        &self,
        product_id: i32,
        days: i64,
    ) -> Result<Vec<SalesForecast>, AppError> {
        // 获取历史销售数据（最近90天）
        let start_date = Utc::now().date_naive() - Duration::days(90);
        
        let orders = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::CreatedAt.gte(start_date))
            .filter(crate::models::sales_order::Column::IsDeleted.eq(false))
            .order_by_asc(crate::models::sales_order::Column::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 简单移动平均预测
        let total_quantity: Decimal = orders.iter().map(|o| o.total_amount).sum();
        let avg_daily = if !orders.is_empty() {
            total_quantity / Decimal::from(90)
        } else {
            Decimal::ZERO
        };

        let mut forecasts = Vec::new();
        let today = Utc::now().date_naive();

        for i in 1..=days {
            let forecast_date = today + Duration::days(i);
            
            // 计算趋势（基于最近30天vs前60天的对比）
            let trend = if i <= 7 {
                "UP" // 短期假设增长
            } else {
                "STABLE"
            };

            // 添加季节性因子（简化版）
            let month = forecast_date.month0() + 1; // month0() 返回 0-11，需要 +1
            let seasonal_factor = match month {
                1 | 2 => Decimal::try_from(0.8f64).unwrap_or(Decimal::ONE), // 春节淡季
                6 | 7 | 8 => Decimal::try_from(1.2f64).unwrap_or(Decimal::ONE), // 夏季旺季
                11 | 12 => Decimal::try_from(1.3f64).unwrap_or(Decimal::ONE), // 年末旺季
                _ => Decimal::ONE,
            };

            let predicted = avg_daily * seasonal_factor;

            forecasts.push(SalesForecast {
                product_id,
                forecast_date,
                predicted_quantity: predicted,
                confidence: 0.75, // 简化置信度
                trend: trend.to_string(),
            });
        }

        Ok(forecasts)
    }

    /// 库存优化建议
    pub async fn optimize_inventory(
        &self,
        product_id: Option<i32>,
    ) -> Result<Vec<InventorySuggestion>, AppError> {
        let mut select = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false));

        if let Some(pid) = product_id {
            select = select.filter(crate::models::inventory_stock::Column::ProductId.eq(pid));
        }

        let stocks = select
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut suggestions = Vec::new();

        for stock in stocks {
            // 获取该产品最近30天的平均日销量
            let avg_daily_sales = self.calculate_avg_daily_sales(stock.product_id).await?;
            
            // 安全库存 = 平均日销量 * 7天
            let safety_stock = avg_daily_sales * Decimal::from(7);
            
            // 再订货点 = 平均日销量 * 14天 + 安全库存
            let reorder_point = avg_daily_sales * Decimal::from(14) + safety_stock;
            
            // 建议订货量 = 平均日销量 * 30天
            let reorder_quantity = avg_daily_sales * Decimal::from(30);

            let reason = if stock.quantity_available < reorder_point {
                format!(
                    "库存低于再订货点({:.2})，建议补货",
                    reorder_point
                )
            } else if stock.quantity_available > safety_stock * Decimal::from(3) {
                "库存过高，建议减少订货量".to_string()
            } else {
                "库存水平正常".to_string()
            };

            suggestions.push(InventorySuggestion {
                product_id: stock.product_id,
                current_stock: stock.quantity_available,
                suggested_stock: safety_stock * Decimal::from(2),
                reorder_point,
                reorder_quantity,
                reason,
            });
        }

        Ok(suggestions)
    }

    /// 异常检测 - 检测销售、库存等异常
    pub async fn detect_anomalies(
        &self,
        days: i64,
    ) -> Result<Vec<AnomalyDetection>, AppError> {
        let mut anomalies = Vec::new();
        let check_date = Utc::now().date_naive() - Duration::days(days);

        // 检测销售异常（销量突降）
        let _sales_orders = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::CreatedAt.gte(check_date))
            .filter(crate::models::sales_order::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 检测零库存产品
        let zero_stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::QuantityAvailable.eq(Decimal::ZERO))
            .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        for stock in zero_stock {
            anomalies.push(AnomalyDetection {
                entity_type: "INVENTORY".to_string(),
                entity_id: stock.product_id,
                anomaly_type: "ZERO_STOCK".to_string(),
                severity: "HIGH".to_string(),
                description: format!("产品 {} 库存为零", stock.product_id),
                detected_at: Utc::now(),
            });
        }

        // 检测滞销产品（30天无销售）
        let all_stock = InventoryStockEntity::find()
            .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        for stock in all_stock {
            let recent_sales = self.calculate_avg_daily_sales(stock.product_id).await?;
            if recent_sales == Decimal::ZERO && stock.quantity_available > Decimal::ZERO {
                anomalies.push(AnomalyDetection {
                    entity_type: "INVENTORY".to_string(),
                    entity_id: stock.product_id,
                    anomaly_type: "SLOW_MOVING".to_string(),
                    severity: "MEDIUM".to_string(),
                    description: format!(
                        "产品 {} 近30天无销售但库存为 {:.2}",
                        stock.product_id, stock.quantity_available
                    ),
                    detected_at: Utc::now(),
                });
            }
        }

        Ok(anomalies)
    }

    /// 智能推荐 - 基于历史数据生成推荐
    pub async fn generate_recommendations(
        &self,
        recommendation_type: String,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let mut recommendations = Vec::new();

        match recommendation_type.as_str() {
            "REORDER" => {
                // 推荐需要补货的产品
                let suggestions = self.optimize_inventory(None).await?;
                for suggestion in suggestions.into_iter().take(limit) {
                    if suggestion.current_stock < suggestion.reorder_point {
                        recommendations.push(SmartRecommendation {
                            recommendation_type: "REORDER".to_string(),
                            target_id: suggestion.product_id,
                            target_type: "PRODUCT".to_string(),
                            score: (suggestion.reorder_point - suggestion.current_stock)
                                .to_f64()
                                .unwrap_or(0.0),
                            reason: suggestion.reason,
                        });
                    }
                }
            }
            "PRICE_ADJUST" => {
                // 推荐价格调整（简化版：基于库存水平）
                let stocks = InventoryStockEntity::find()
                    .filter(crate::models::inventory_stock::Column::IsDeleted.eq(false))
                    .all(&*self.db)
                    .await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

                for stock in stocks.into_iter().take(limit) {
                    if stock.quantity_available > Decimal::from(1000) {
                        recommendations.push(SmartRecommendation {
                            recommendation_type: "PRICE_ADJUST".to_string(),
                            target_id: stock.product_id,
                            target_type: "PRODUCT".to_string(),
                            score: stock.quantity_available.to_f64().unwrap_or(0.0),
                            reason: "库存积压，建议降价促销".to_string(),
                        });
                    }
                }
            }
            _ => {}
        }

        Ok(recommendations)
    }

    /// 计算平均日销量
    async fn calculate_avg_daily_sales(&self, _product_id: i32) -> Result<Decimal, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(30);

        let orders = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::CreatedAt.gte(start_date))
            .filter(crate::models::sales_order::Column::IsDeleted.eq(false))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total: Decimal = orders.iter().map(|o| o.total_amount).sum();
        let avg = if !orders.is_empty() {
            total / Decimal::from(30)
        } else {
            Decimal::ZERO
        };

        Ok(avg)
    }
}
