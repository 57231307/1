//! AI 异常检测服务（ai/detect）
//!
//! 提供销售/库存的异常检测能力：
//! - 销售异常（Z-score）：SPIKE 突增 / DROP 突降，CRITICAL 与 WARNING 两级
//! - 库存异常（IQR）：ZERO_STOCK / LOW_STOCK / OVERSTOCK / SLOW_MOVING 滞销
//!
//! 拆分自原 `ai_analysis_service.rs` 的 `detect_anomalies` 与
//! `aggregate_daily_from_transactions` 两个方法。

use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

use crate::models::inventory_stock::Entity as InventoryStockEntity;
use crate::models::sales_order_item::Entity as SalesOrderItemEntity;
use crate::utils::error::AppError;

use super::{iqr_quartiles, mean, std_deviation, AiAnalysisService, AnomalyDetection};

impl AiAnalysisService {
    /// 异常检测 - 检测销售和库存异常
    pub async fn detect_anomalies(&self, days: i64) -> Result<Vec<AnomalyDetection>, AppError> {
        let mut anomalies = Vec::new();
        let check_start = Utc::now().date_naive() - Duration::days(days);
        let long_start = Utc::now().date_naive() - Duration::days(days * 4); // 更长的历史窗口

        // 获取销售订单明细
        let _recent_items = SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    check_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

        let historical_items = SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    long_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?;

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
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载 OOM
        let all_stocks = InventoryStockEntity::find()
            .limit(10_000)
            .all(&*self.db)
            .await?;

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
                        description: format!(
                            "产品 {} 库存为零，仓库={}",
                            stock.product_id, stock.warehouse_id
                        ),
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

    /// 从交易记录中聚合每日出库量（供 optimize_inventory 调用，跨子模块）
    pub(crate) fn aggregate_daily_from_transactions(
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
}
