//! AI 销售预测服务（ai/pred）
//!
//! 提供基于历史销售数据的预测能力：
//! - 移动平均 (WMA, 加权 7 日)
//! - 指数平滑 (Holt 线性趋势)
//! - 季节性因子（按月聚合）
//! - 数据不足时的降级预测（fallback_forecast）
//!
//! 算法组合：60% 指数平滑 + 40% 加权移动平均，再乘以季节性因子。
//! 置信度随预测距离衰减，并基于拟合残差标准差计算。
//!
//! 拆分自原 `ai_analysis_service.rs` 的 `forecast_sales` / `fallback_forecast` /
//! `build_seasonal_factors` 三个方法。

use chrono::{Datelike, Duration, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

use crate::models::sales_order::Entity as SalesOrderEntity;
use crate::models::sales_order_item::Entity as SalesOrderItemEntity;
use crate::utils::error::AppError;

use super::{mean, std_deviation, AiAnalysisService, SalesForecast};

impl AiAnalysisService {
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
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .order_by_asc(crate::models::sales_order_item::Column::CreatedAt)
            .all(&*self.db)
            .await?;

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
        let beta = 0.1; // 趋势平滑因子

        let mut level = values[0];
        let mut trend = if n > 1 { values[1] - values[0] } else { 0.0 };

        #[allow(clippy::needless_range_loop)]
        for i in 1..n {
            let prev_level = level;
            level = alpha * values[i] + (1.0 - alpha) * (level + trend);
            trend = beta * (level - prev_level) + (1.0 - beta) * trend;
        }

        // 计算拟合残差的标准差，用于置信度
        let mut residuals = Vec::new();
        let mut fit_level = values[0];
        let mut fit_trend = if n > 1 { values[1] - values[0] } else { 0.0 };
        #[allow(clippy::needless_range_loop)]
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
        let earlier_avg = if n > 14 {
            mean(&values[..n - 14])
        } else {
            mean(&values[..n / 2])
        };
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
            .filter(
                crate::models::sales_order::Column::CreatedAt.gte(
                    start_date
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

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
                predicted_quantity: Decimal::try_from(avg_daily * seasonal)
                    .unwrap_or(Decimal::ZERO),
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
            monthly_totals
                .entry(date.month() as usize)
                .or_default()
                .push(values[i]);
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
}
