//! AI智能分析 Service
//!
//! 提供销售预测、库存优化、异常检测、智能推荐等真实业务分析功能

#![allow(dead_code)]

use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::sales_order_item::Entity as SalesOrderItemEntity;
use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::inventory_transaction::Entity as InventoryTransactionEntity;
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

/// ABC 分类结果
#[derive(Debug, Clone)]
pub struct AbcClassification {
    pub product_id: i32,
    pub category: String, // A / B / C
    pub total_sales: Decimal,
    pub cumulative_ratio: f64,
}

/// 库存周转率结果
#[derive(Debug, Clone)]
pub struct InventoryTurnover {
    pub product_id: i32,
    pub turnover_rate: f64,
    pub avg_stock: Decimal,
    pub total_outbound: Decimal,
    pub period_days: i64,
}

/// AI分析 Service
pub struct AiAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl AiAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ========================================================================
    // 销售预测 - 移动平均 + 指数平滑
    // ========================================================================

    /// 销售预测 - 基于历史销售数据预测未来销量
    /// 使用加权移动平均(WMA) + 指数平滑(Exponential Smoothing)组合算法
    pub async fn forecast_sales(
        &self,
        product_id: i32,
        days: i64,
    ) -> Result<Vec<SalesForecast>, AppError> {
        // 获取该产品最近 180 天的每日销售数据
        let start_date = Utc::now().date_naive() - Duration::days(180);

        let items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::ProductId.eq(product_id))
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc()))
            .order_by_asc(crate::models::sales_order_item::Column::CreatedAt)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按日期聚合每日销量
        let mut daily_sales: HashMap<NaiveDate, f64> = HashMap::new();
        for item in &items {
            let date = item.created_at.date_naive();
            let qty = item.quantity_meters.to_f64().unwrap_or(0.0);
            *daily_sales.entry(date).or_insert(0.0) += qty;
        }

        // 如果数据不足，返回基于全局订单的粗略预测
        if daily_sales.len() < 7 {
            return self.fallback_forecast(product_id, days).await;
        }

        // 构建有序时间序列
        let mut sorted_dates: Vec<NaiveDate> = daily_sales.keys().cloned().collect();
        sorted_dates.sort();

        let values: Vec<f64> = sorted_dates.iter().map(|d| daily_sales[d]).collect();
        let n = values.len();

        // --- 移动平均 (7日加权移动平均) ---
        let window = 7.min(n);
        let mut wma_sum = 0.0;
        let mut wma_weight_sum = 0.0;
        for i in 0..window {
            let weight = (i + 1) as f64;
            wma_sum += values[n - window + i] * weight;
            wma_weight_sum += weight;
        }
        let wma = wma_sum / wma_weight_sum;

        // --- 指数平滑 (Holt 线性趋势) ---
        let alpha = 0.3; // 水平平滑因子
        let beta = 0.1;  // 趋势平滑因子

        let mut level = values[0];
        let mut trend = if n > 1 { values[1] - values[0] } else { 0.0 };

        for i in 1..n {
            let prev_level = level;
            level = alpha * values[i] + (1.0 - alpha) * (level + trend);
            trend = beta * (level - prev_level) + (1.0 - beta) * trend;
        }

        // 计算拟合残差的标准差，用于置信度
        let mut residuals = Vec::new();
        let mut fit_level = values[0];
        let mut fit_trend = if n > 1 { values[1] - values[0] } else { 0.0 };
        for i in 0..n {
            let predicted = fit_level + fit_trend;
            residuals.push(values[i] - predicted);
            if i < n - 1 {
                let prev = fit_level;
                fit_level = alpha * values[i] + (1.0 - alpha) * (fit_level + fit_trend);
                fit_trend = beta * (fit_level - prev) + (1.0 - beta) * fit_trend;
            }
        }
        let residual_std = std_deviation(&residuals);

        // 计算趋势方向
        let recent_avg = mean(&values[n.saturating_sub(14)..]);
        let earlier_avg = if n > 14 { mean(&values[..n - 14]) } else { mean(&values[..n / 2]) };
        let trend_direction = if recent_avg > earlier_avg * 1.1 {
            "UP"
        } else if recent_avg < earlier_avg * 0.9 {
            "DOWN"
        } else {
            "STABLE"
        };

        // 季节性因子
        let seasonal_factors = self.build_seasonal_factors(&sorted_dates, &values);

        // 生成预测
        let today = Utc::now().date_naive();
        let mut forecasts = Vec::new();

        for i in 1..=days {
            let forecast_date = today + Duration::days(i);
            let month = forecast_date.month() as usize;

            // 组合预测: 60% 指数平滑 + 40% 加权移动平均
            let holt_pred = level + trend * (i as f64);
            let combined = 0.6 * holt_pred + 0.4 * wma;

            // 应用季节性因子
            let seasonal = seasonal_factors.get(&month).copied().unwrap_or(1.0);
            let predicted = (combined * seasonal).max(0.0);

            // 置信度随预测距离衰减
            let base_confidence = if residual_std > 0.0 {
                (1.0 - (residual_std / (recent_avg.max(1.0)))).clamp(0.3, 0.95)
            } else {
                0.85
            };
            let confidence = (base_confidence * (0.99_f64).powi(i as i32 - 1)).clamp(0.3, 0.95);

            forecasts.push(SalesForecast {
                product_id,
                forecast_date,
                predicted_quantity: Decimal::try_from(predicted).unwrap_or(Decimal::ZERO),
                confidence: (confidence * 100.0).round() / 100.0,
                trend: trend_direction.to_string(),
            });
        }

        Ok(forecasts)
    }

    /// 数据不足时的降级预测
    async fn fallback_forecast(
        &self,
        product_id: i32,
        days: i64,
    ) -> Result<Vec<SalesForecast>, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(90);

        let orders = SalesOrderEntity::find()
            .filter(crate::models::sales_order::Column::CreatedAt.gte(start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc()))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total: Decimal = orders.iter().map(|o| o.total_amount).sum();
        let _days_count = orders.len().max(1) as f64;
        let avg_daily = total.to_f64().unwrap_or(0.0) / 90.0;

        let today = Utc::now().date_naive();
        let mut forecasts = Vec::new();

        for i in 1..=days {
            let forecast_date = today + Duration::days(i);
            let month = forecast_date.month();
            let seasonal = match month {
                1 | 2 => 0.8,
                6..=8 => 1.2,
                11 | 12 => 1.3,
                _ => 1.0,
            };

            forecasts.push(SalesForecast {
                product_id,
                forecast_date,
                predicted_quantity: Decimal::try_from(avg_daily * seasonal).unwrap_or(Decimal::ZERO),
                confidence: 0.55,
                trend: "STABLE".to_string(),
            });
        }

        Ok(forecasts)
    }

    /// 构建月度季节性因子
    fn build_seasonal_factors(&self, dates: &[NaiveDate], values: &[f64]) -> HashMap<usize, f64> {
        let mut monthly_totals: HashMap<usize, Vec<f64>> = HashMap::new();
        for (i, date) in dates.iter().enumerate() {
            monthly_totals.entry(date.month() as usize).or_default().push(values[i]);
        }

        let overall_mean = mean(values);
        if overall_mean == 0.0 {
            return HashMap::new();
        }

        let mut factors = HashMap::new();
        for (month, vals) in &monthly_totals {
            let month_mean = mean(vals);
            factors.insert(*month, month_mean / overall_mean);
        }
        factors
    }

    // ========================================================================
    // 库存优化 - 安全库存 + ABC 分类 + 周转率
    // ========================================================================

    /// 库存优化建议
    /// 基于历史出库数据计算安全库存，结合 ABC 分类给出建议
    pub async fn optimize_inventory(
        &self,
        product_id: Option<i32>,
    ) -> Result<Vec<InventorySuggestion>, AppError> {
        let mut select = InventoryStockEntity::find();

        if let Some(pid) = product_id {
            select = select.filter(crate::models::inventory_stock::Column::ProductId.eq(pid));
        }

        let stocks = select
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if stocks.is_empty() {
            return Ok(Vec::new());
        }

        // 获取所有产品的出库历史数据
        let start_date = Utc::now().date_naive() - Duration::days(90);
        let transactions = InventoryTransactionEntity::find()
            .filter(crate::models::inventory_transaction::Column::CreatedAt.gte(
                start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按产品聚合出库数据
        let mut outbound_by_product: HashMap<i32, Vec<f64>> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                outbound_by_product.entry(tx.product_id).or_default().push(qty);
            }
        }

        // ABC 分类
        let abc_classifications = self.compute_abc_classification(&stocks, &outbound_by_product);
        let abc_map: HashMap<i32, &str> = abc_classifications
            .iter()
            .map(|c| (c.product_id, c.category.as_str()))
            .collect();

        let mut suggestions = Vec::new();

        for stock in stocks {
            let pid = stock.product_id;
            let current = stock.quantity_available.to_f64().unwrap_or(0.0);

            // 计算出库统计
            let outbound_qtys = outbound_by_product.get(&pid);
            let (avg_daily_demand, demand_std, _total_outbound) = if let Some(qtys) = outbound_qtys {
                let daily_map = self.aggregate_daily_from_transactions(pid, &transactions);
                let daily_values: Vec<f64> = daily_map.values().copied().collect();
                let avg = if daily_values.is_empty() { 0.0 } else { mean(&daily_values) };
                let std = if daily_values.len() > 1 { std_deviation(&daily_values) } else { avg * 0.3 };
                let total: f64 = qtys.iter().sum();
                (avg, std, total)
            } else {
                (0.0, 0.0, 0.0)
            };

            // ABC 分类影响服务水平
            let abc = abc_map.get(&pid).copied().unwrap_or("C");
            let service_level_z = match abc {
                "A" => 2.33, // 99% 服务水平
                "B" => 1.65, // 95% 服务水平
                _ => 1.28,   // 90% 服务水平
            };

            // 安全库存 = Z * σ * √(LT)  假设提前期 LT = 7 天
            let lead_time = 7.0_f64;
            let safety_stock = service_level_z * demand_std * lead_time.sqrt();

            // 再订货点 = 平均日需求 * 提前期 + 安全库存
            let reorder_point = avg_daily_demand * lead_time + safety_stock;

            // 建议订货量 = 30 天需求量（EOQ 简化版）
            let reorder_quantity = avg_daily_demand * 30.0;

            // 建议库存水平 = 安全库存 + 30天需求
            let suggested = safety_stock + avg_daily_demand * 30.0;

            let reason = if current <= 0.0 {
                format!("库存为零! ABC分类={}, 安全库存={:.0}, 建议立即补货 {:.0}", abc, safety_stock, reorder_quantity)
            } else if current < reorder_point {
                format!(
                    "库存({:.0})低于再订货点({:.0}), ABC分类={}, 安全库存={:.0}, 建议补货 {:.0}",
                    current, reorder_point, abc, safety_stock, reorder_quantity
                )
            } else if current > suggested * 2.0 {
                format!(
                    "库存({:.0})过高, 超过建议水平({:.0})的2倍, ABC分类={}, 建议减少采购或促销",
                    current, suggested, abc
                )
            } else {
                format!("库存水平正常, ABC分类={}, 安全库存={:.0}", abc, safety_stock)
            };

            suggestions.push(InventorySuggestion {
                product_id: pid,
                current_stock: stock.quantity_available,
                suggested_stock: Decimal::try_from(suggested.max(0.0)).unwrap_or(Decimal::ZERO),
                reorder_point: Decimal::try_from(reorder_point.max(0.0)).unwrap_or(Decimal::ZERO),
                reorder_quantity: Decimal::try_from(reorder_quantity.max(0.0)).unwrap_or(Decimal::ZERO),
                reason,
            });
        }

        Ok(suggestions)
    }

    /// ABC 分类分析
    /// A 类: 累计销售额占比 0-80%
    /// B 类: 累计销售额占比 80-95%
    /// C 类: 累计销售额占比 95-100%
    pub async fn get_abc_classification(&self) -> Result<Vec<AbcClassification>, AppError> {
        let stocks = InventoryStockEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let start_date = Utc::now().date_naive() - Duration::days(90);
        let transactions = InventoryTransactionEntity::find()
            .filter(crate::models::inventory_transaction::Column::CreatedAt.gte(
                start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut outbound_by_product: HashMap<i32, Vec<f64>> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                outbound_by_product.entry(tx.product_id).or_default().push(qty);
            }
        }

        Ok(self.compute_abc_classification(&stocks, &outbound_by_product))
    }

    fn compute_abc_classification(
        &self,
        stocks: &[crate::models::inventory_stock::Model],
        outbound_by_product: &HashMap<i32, Vec<f64>>,
    ) -> Vec<AbcClassification> {
        // 计算每个产品的总出库量
        let mut product_sales: Vec<(i32, f64)> = stocks
            .iter()
            .map(|s| {
                let total = outbound_by_product
                    .get(&s.product_id)
                    .map(|v| v.iter().sum::<f64>())
                    .unwrap_or(0.0);
                (s.product_id, total)
            })
            .collect();

        // 按销售额降序排列
        product_sales.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let grand_total: f64 = product_sales.iter().map(|(_, v)| v).sum();
        if grand_total == 0.0 {
            return product_sales
                .into_iter()
                .map(|(pid, _)| AbcClassification {
                    product_id: pid,
                    category: "C".to_string(),
                    total_sales: Decimal::ZERO,
                    cumulative_ratio: 1.0,
                })
                .collect();
        }

        let mut cumulative = 0.0;
        product_sales
            .into_iter()
            .map(|(pid, sales)| {
                cumulative += sales;
                let ratio = cumulative / grand_total;
                let category = if ratio <= 0.80 {
                    "A"
                } else if ratio <= 0.95 {
                    "B"
                } else {
                    "C"
                };
                AbcClassification {
                    product_id: pid,
                    category: category.to_string(),
                    total_sales: Decimal::try_from(sales).unwrap_or(Decimal::ZERO),
                    cumulative_ratio: (ratio * 1000.0).round() / 1000.0,
                }
            })
            .collect()
    }

    /// 库存周转率计算
    pub async fn get_inventory_turnover(
        &self,
        product_id: Option<i32>,
        days: i64,
    ) -> Result<Vec<InventoryTurnover>, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(days);

        let mut tx_select = InventoryTransactionEntity::find()
            .filter(crate::models::inventory_transaction::Column::CreatedAt.gte(
                start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ));
        if let Some(pid) = product_id {
            tx_select = tx_select.filter(crate::models::inventory_transaction::Column::ProductId.eq(pid));
        }

        let transactions = tx_select
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按产品聚合出库量
        let mut outbound_map: HashMap<i32, f64> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                *outbound_map.entry(tx.product_id).or_insert(0.0) += qty;
            }
        }

        // 获取当前库存
        let mut stock_select = InventoryStockEntity::find();
        if let Some(pid) = product_id {
            stock_select = stock_select.filter(crate::models::inventory_stock::Column::ProductId.eq(pid));
        }
        let stocks = stock_select
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按产品聚合当前库存
        let mut stock_map: HashMap<i32, f64> = HashMap::new();
        for s in &stocks {
            let qty = s.quantity_on_hand.to_f64().unwrap_or(0.0);
            *stock_map.entry(s.product_id).or_insert(0.0) += qty;
        }

        let mut results = Vec::new();
        let all_pids: std::collections::HashSet<i32> = outbound_map
            .keys()
            .chain(stock_map.keys())
            .cloned()
            .collect();

        for pid in all_pids {
            let total_outbound = outbound_map.get(&pid).copied().unwrap_or(0.0);
            let current_stock = stock_map.get(&pid).copied().unwrap_or(0.0);

            // 平均库存近似为当前库存（简化处理）
            let avg_stock = current_stock;
            let turnover_rate = if avg_stock > 0.0 {
                (total_outbound / avg_stock * 365.0 / days as f64 * 100.0).round() / 100.0
            } else {
                0.0
            };

            results.push(InventoryTurnover {
                product_id: pid,
                turnover_rate,
                avg_stock: Decimal::try_from(avg_stock).unwrap_or(Decimal::ZERO),
                total_outbound: Decimal::try_from(total_outbound).unwrap_or(Decimal::ZERO),
                period_days: days,
            });
        }

        // 按周转率降序排列
        results.sort_by(|a, b| b.turnover_rate.partial_cmp(&a.turnover_rate).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// 从交易记录中聚合每日出库量
    fn aggregate_daily_from_transactions(
        &self,
        product_id: i32,
        transactions: &[crate::models::inventory_transaction::Model],
    ) -> HashMap<NaiveDate, f64> {
        let mut daily: HashMap<NaiveDate, f64> = HashMap::new();
        for tx in transactions {
            if tx.product_id == product_id
                && (tx.transaction_type == "销售出库" || tx.transaction_type == "出库")
            {
                let date = tx.created_at.date_naive();
                let qty = tx.quantity_meters.to_f64().unwrap_or(0.0);
                *daily.entry(date).or_insert(0.0) += qty;
            }
        }
        daily
    }

    // ========================================================================
    // 异常检测 - Z-score / IQR 统计方法
    // ========================================================================

    /// 异常检测 - 检测销售和库存异常
    pub async fn detect_anomalies(
        &self,
        days: i64,
    ) -> Result<Vec<AnomalyDetection>, AppError> {
        let mut anomalies = Vec::new();
        let check_start = Utc::now().date_naive() - Duration::days(days);
        let long_start = Utc::now().date_naive() - Duration::days(days * 4); // 更长的历史窗口

        // 获取销售订单明细
        let _recent_items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(
                check_start.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let historical_items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(
                long_start.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // --- 销售异常检测 (Z-score) ---
        let mut product_daily_sales: HashMap<i32, HashMap<NaiveDate, f64>> = HashMap::new();
        for item in &historical_items {
            let date = item.created_at.date_naive();
            let qty = item.quantity_meters.to_f64().unwrap_or(0.0);
            product_daily_sales
                .entry(item.product_id)
                .or_default()
                .entry(date)
                .and_modify(|v| *v += qty)
                .or_insert(qty);
        }

        for (pid, daily_map) in &product_daily_sales {
            let all_values: Vec<f64> = daily_map.values().copied().collect();
            if all_values.len() < 7 {
                continue;
            }

            let mean_val = mean(&all_values);
            let std_val = std_deviation(&all_values);

            // 检测最近 period 内的异常
            let check_dates: Vec<NaiveDate> = daily_map
                .keys()
                .filter(|d| **d >= check_start)
                .cloned()
                .collect();

            for date in check_dates {
                if let Some(&value) = daily_map.get(&date) {
                    if std_val > 0.0 {
                        let z_score = (value - mean_val) / std_val;

                        if z_score > 2.5 {
                            anomalies.push(AnomalyDetection {
                                entity_type: "SALES".to_string(),
                                entity_id: *pid,
                                anomaly_type: "SPIKE".to_string(),
                                severity: if z_score > 3.5 { "CRITICAL" } else { "WARNING" }.to_string(),
                                description: format!(
                                    "产品 {} 在 {} 销售量异常突增: {:.0} (均值={:.0}, Z-score={:.2})",
                                    pid, date, value, mean_val, z_score
                                ),
                                detected_at: Utc::now(),
                            });
                        } else if z_score < -2.5 {
                            anomalies.push(AnomalyDetection {
                                entity_type: "SALES".to_string(),
                                entity_id: *pid,
                                anomaly_type: "DROP".to_string(),
                                severity: if z_score < -3.5 { "CRITICAL" } else { "WARNING" }.to_string(),
                                description: format!(
                                    "产品 {} 在 {} 销售量异常突降: {:.0} (均值={:.0}, Z-score={:.2})",
                                    pid, date, value, mean_val, z_score
                                ),
                                detected_at: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        // --- 库存异常检测 (IQR 方法) ---
        let all_stocks = InventoryStockEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 收集所有库存量用于 IQR 计算
        let stock_quantities: Vec<f64> = all_stocks
            .iter()
            .map(|s| s.quantity_available.to_f64().unwrap_or(0.0))
            .collect();

        if stock_quantities.len() >= 4 {
            let (q1, q3) = iqr_quartiles(&stock_quantities);
            let iqr = q3 - q1;
            let lower_fence = (q1 - 1.5 * iqr).max(0.0);
            let upper_fence = q3 + 1.5 * iqr;

            for stock in &all_stocks {
                let qty = stock.quantity_available.to_f64().unwrap_or(0.0);

                // 零库存
                if qty <= 0.0 {
                    anomalies.push(AnomalyDetection {
                        entity_type: "INVENTORY".to_string(),
                        entity_id: stock.product_id,
                        anomaly_type: "ZERO_STOCK".to_string(),
                        severity: "CRITICAL".to_string(),
                        description: format!("产品 {} 库存为零，仓库={}", stock.product_id, stock.warehouse_id),
                        detected_at: Utc::now(),
                    });
                }
                // 库存异常低 (IQR 下界)
                else if qty < lower_fence && qty > 0.0 {
                    anomalies.push(AnomalyDetection {
                        entity_type: "INVENTORY".to_string(),
                        entity_id: stock.product_id,
                        anomaly_type: "LOW_STOCK".to_string(),
                        severity: "WARNING".to_string(),
                        description: format!(
                            "产品 {} 库存({:.0})低于IQR下界({:.0}), 仓库={}",
                            stock.product_id, qty, lower_fence, stock.warehouse_id
                        ),
                        detected_at: Utc::now(),
                    });
                }
                // 库存异常高 (积压)
                else if qty > upper_fence {
                    anomalies.push(AnomalyDetection {
                        entity_type: "INVENTORY".to_string(),
                        entity_id: stock.product_id,
                        anomaly_type: "OVERSTOCK".to_string(),
                        severity: "WARNING".to_string(),
                        description: format!(
                            "产品 {} 库存({:.0})超过IQR上界({:.0}), 疑似积压, 仓库={}",
                            stock.product_id, qty, upper_fence, stock.warehouse_id
                        ),
                        detected_at: Utc::now(),
                    });
                }
            }
        }

        // --- 滞销产品检测 ---
        for stock in &all_stocks {
            let outbound_qty = product_daily_sales
                .get(&stock.product_id)
                .map(|m| m.values().sum::<f64>())
                .unwrap_or(0.0);

            if outbound_qty <= 0.0 && stock.quantity_available.to_f64().unwrap_or(0.0) > 0.0 {
                anomalies.push(AnomalyDetection {
                    entity_type: "INVENTORY".to_string(),
                    entity_id: stock.product_id,
                    anomaly_type: "SLOW_MOVING".to_string(),
                    severity: "MEDIUM".to_string(),
                    description: format!(
                        "产品 {} 在 {} 天内无出库记录，但库存为 {:.0}，疑似滞销",
                        stock.product_id,
                        days * 4,
                        stock.quantity_available
                    ),
                    detected_at: Utc::now(),
                });
            }
        }

        Ok(anomalies)
    }

    // ========================================================================
    // 智能推荐 - 关联规则 + 销售趋势
    // ========================================================================

    /// 智能推荐 - 基于历史数据生成推荐
    pub async fn generate_recommendations(
        &self,
        recommendation_type: String,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let mut recommendations = Vec::new();

        match recommendation_type.as_str() {
            "REORDER" => {
                // 基于库存优化的补货推荐
                let suggestions = self.optimize_inventory(None).await?;
                let mut reorder_items: Vec<SmartRecommendation> = suggestions
                    .into_iter()
                    .filter(|s| s.current_stock < s.reorder_point)
                    .map(|s| {
                        let urgency = (s.reorder_point - s.current_stock).to_f64().unwrap_or(0.0);
                        SmartRecommendation {
                            recommendation_type: "REORDER".to_string(),
                            target_id: s.product_id,
                            target_type: "PRODUCT".to_string(),
                            score: urgency,
                            reason: s.reason,
                        }
                    })
                    .collect();

                reorder_items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                recommendations = reorder_items.into_iter().take(limit).collect();
            }
            "BUNDLE" => {
                // 基于关联规则的捆绑采购推荐
                // 分析经常一起被销售的产品组合
                recommendations = self.generate_association_recommendations(limit).await?;
            }
            "TREND" => {
                // 基于销售趋势的产品推荐
                recommendations = self.generate_trend_recommendations(limit).await?;
            }
            "PRICE_ADJUST" => {
                // 基于库存水平的价格调整推荐
                recommendations = self.generate_price_recommendations(limit).await?;
            }
            _ => {
                // 默认: 综合推荐
                let mut reorder = Box::pin(self.generate_recommendations("REORDER".to_string(), limit / 2)).await?;
                let mut trend = Box::pin(self.generate_recommendations("TREND".to_string(), limit / 2)).await?;
                recommendations.append(&mut reorder);
                recommendations.append(&mut trend);
            }
        }

        Ok(recommendations)
    }

    /// 基于关联规则的采购推荐
    /// 分析哪些产品经常在同一时期被销售，推荐关联产品
    async fn generate_association_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let start_date = Utc::now().date_naive() - Duration::days(60);

        let items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(
                start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按订单分组产品
        let mut order_products: HashMap<i32, Vec<i32>> = HashMap::new();
        for item in &items {
            order_products.entry(item.order_id).or_default().push(item.product_id);
        }

        // 计算产品共现频率
        let mut co_occurrence: HashMap<(i32, i32), usize> = HashMap::new();
        let mut product_count: HashMap<i32, usize> = HashMap::new();

        for products in order_products.values() {
            let unique: std::collections::HashSet<i32> = products.iter().cloned().collect();
            for &pid in &unique {
                *product_count.entry(pid).or_insert(0) += 1;
            }
            let sorted: Vec<i32> = {
                let mut v: Vec<i32> = unique.into_iter().collect();
                v.sort();
                v
            };
            for i in 0..sorted.len() {
                for j in (i + 1)..sorted.len() {
                    *co_occurrence.entry((sorted[i], sorted[j])).or_insert(0) += 1;
                }
            }
        }

        let total_orders = order_products.len().max(1) as f64;

        // 找出强关联规则 (support > 5%, confidence > 30%)
        let mut assoc_scores: Vec<(i32, i32, f64, String)> = Vec::new();
        for ((p1, p2), &count) in &co_occurrence {
            let support = count as f64 / total_orders;
            let conf1 = count as f64 / product_count.get(p1).copied().unwrap_or(1).max(1) as f64;
            let _conf2 = count as f64 / product_count.get(p2).copied().unwrap_or(1).max(1) as f64;

            if support > 0.05 && conf1 > 0.3 {
                let lift = support / (
                    (product_count.get(p1).unwrap_or(&0).to_f64().unwrap_or(0.0) / total_orders)
                    * (product_count.get(p2).unwrap_or(&0).to_f64().unwrap_or(0.0) / total_orders)
                );
                assoc_scores.push((*p1, *p2, lift, format!(
                    "产品 {} 与产品 {} 经常一起被购买 (共现率={:.0}%, 提升度={:.2})",
                    p1, p2, conf1 * 100.0, lift
                )));
            }
        }

        assoc_scores.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        // 为销量较低的产品推荐关联的高销量产品
        let recommendations: Vec<SmartRecommendation> = assoc_scores
            .into_iter()
            .take(limit)
            .map(|(source, target, score, reason)| SmartRecommendation {
                recommendation_type: "BUNDLE".to_string(),
                target_id: target,
                target_type: "PRODUCT".to_string(),
                score,
                reason: format!("与产品 {} 关联推荐: {}", source, reason),
            })
            .collect();

        Ok(recommendations)
    }

    /// 基于销售趋势的产品推荐
    async fn generate_trend_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let now = Utc::now().date_naive();
        let recent_start = now - Duration::days(30);
        let earlier_start = now - Duration::days(60);

        // 最近 30 天的销售
        let recent_items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(
                recent_start.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 前 30 天的销售
        let earlier_items = SalesOrderItemEntity::find()
            .filter(crate::models::sales_order_item::Column::CreatedAt.gte(
                earlier_start.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .filter(crate::models::sales_order_item::Column::CreatedAt.lt(
                recent_start.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 按产品聚合
        let mut recent_sales: HashMap<i32, f64> = HashMap::new();
        for item in &recent_items {
            *recent_sales.entry(item.product_id).or_insert(0.0) += item.quantity_meters.to_f64().unwrap_or(0.0);
        }

        let mut earlier_sales: HashMap<i32, f64> = HashMap::new();
        for item in &earlier_items {
            *earlier_sales.entry(item.product_id).or_insert(0.0) += item.quantity_meters.to_f64().unwrap_or(0.0);
        }

        // 计算增长率
        let mut growth_items: Vec<(i32, f64, f64, f64)> = Vec::new();
        for (pid, &recent) in &recent_sales {
            let earlier = earlier_sales.get(pid).copied().unwrap_or(0.0);
            let growth = if earlier > 0.0 {
                (recent - earlier) / earlier
            } else if recent > 0.0 {
                1.0 // 新品
            } else {
                0.0
            };
            growth_items.push((*pid, recent, earlier, growth));
        }

        // 按增长率降序排列
        growth_items.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let recommendations: Vec<SmartRecommendation> = growth_items
            .into_iter()
            .take(limit)
            .map(|(pid, recent, earlier, growth)| {
                let reason = if earlier <= 0.0 && recent > 0.0 {
                    format!("新品热销: 最近30天销量={:.0}", recent)
                } else if growth > 0.5 {
                    format!(
                        "销量快速增长: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent, earlier, growth * 100.0
                    )
                } else if growth > 0.0 {
                    format!(
                        "销量稳步增长: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent, earlier, growth * 100.0
                    )
                } else {
                    format!(
                        "销量下降: 最近30天={:.0}, 前30天={:.0}, 增长率={:.0}%",
                        recent, earlier, growth * 100.0
                    )
                };
                SmartRecommendation {
                    recommendation_type: "TREND".to_string(),
                    target_id: pid,
                    target_type: "PRODUCT".to_string(),
                    score: (growth * 100.0).round() / 100.0,
                    reason,
                }
            })
            .collect();

        Ok(recommendations)
    }

    /// 基于库存水平的价格调整推荐
    async fn generate_price_recommendations(
        &self,
        limit: usize,
    ) -> Result<Vec<SmartRecommendation>, AppError> {
        let stocks = InventoryStockEntity::find()
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let start_date = Utc::now().date_naive() - Duration::days(30);
        let transactions = InventoryTransactionEntity::find()
            .filter(crate::models::inventory_transaction::Column::CreatedAt.gte(
                start_date.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc(),
            ))
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut outbound_map: HashMap<i32, f64> = HashMap::new();
        for tx in &transactions {
            if tx.transaction_type == "销售出库" || tx.transaction_type == "出库" {
                *outbound_map.entry(tx.product_id).or_insert(0.0) += tx.quantity_meters.to_f64().unwrap_or(0.0);
            }
        }

        let mut recommendations = Vec::new();

        for stock in &stocks {
            let qty = stock.quantity_available.to_f64().unwrap_or(0.0);
            let outbound = outbound_map.get(&stock.product_id).copied().unwrap_or(0.0);

            if qty > 500.0 && outbound < qty * 0.1 {
                recommendations.push(SmartRecommendation {
                    recommendation_type: "PRICE_ADJUST".to_string(),
                    target_id: stock.product_id,
                    target_type: "PRODUCT".to_string(),
                    score: qty / outbound.max(1.0),
                    reason: format!(
                        "库存积压({:.0})且近30天出库量({:.0})低，建议降价促销或捆绑销售",
                        qty, outbound
                    ),
                });
            }
        }

        recommendations.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        recommendations.truncate(limit);

        Ok(recommendations)
    }
}

// ============================================================================
// 统计工具函数
// ============================================================================

/// 计算均值
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// 计算标准差
fn std_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

/// 计算 IQR 四分位数 (Q1, Q3)
fn iqr_quartiles(values: &[f64]) -> (f64, f64) {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    if n < 4 {
        return (sorted[0], sorted[n - 1]);
    }

    let q1_idx = n / 4;
    let q3_idx = (3 * n) / 4;

    (sorted[q1_idx], sorted[q3_idx])
}
