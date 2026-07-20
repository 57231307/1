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
        let _recent_items = self.fetch_recent_sales_items(check_start).await?;
        let historical_items = self.fetch_historical_sales_items(long_start).await?;

        // --- 销售异常检测 (Z-score) ---
        let product_daily_sales = aggregate_product_daily_sales(&historical_items);
        anomalies.extend(detect_sales_zscore_anomalies(
            &product_daily_sales,
            check_start,
        ));

        // --- 库存异常检测 (IQR 方法) ---
        // P3 维度 6 修复（批次 87）：补 LIMIT 兜底防止全表加载 OOM
        let all_stocks = self.fetch_all_stocks().await?;
        let stock_quantities: Vec<f64> = all_stocks
            .iter()
            .map(|s| s.quantity_available.to_f64().unwrap_or(0.0))
            .collect();

        if let Some((lower_fence, upper_fence)) = compute_iqr_fences(&stock_quantities) {
            anomalies.extend(detect_inventory_iqr_anomalies(
                &all_stocks,
                lower_fence,
                upper_fence,
            ));
        }

        // --- 滞销产品检测 ---
        anomalies.extend(detect_slow_moving_products(
            &all_stocks,
            &product_daily_sales,
            days,
        ));

        Ok(anomalies)
    }

    /// 获取最近一段时间的销售订单明细（保留以维持原查询行为，结果未使用）
    async fn fetch_recent_sales_items(
        &self,
        check_start: NaiveDate,
    ) -> Result<Vec<crate::models::sales_order_item::Model>, AppError> {
        Ok(SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    check_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?)
    }

    /// 获取长历史窗口内的销售订单明细（用于 Z-score 计算）
    async fn fetch_historical_sales_items(
        &self,
        long_start: NaiveDate,
    ) -> Result<Vec<crate::models::sales_order_item::Model>, AppError> {
        Ok(SalesOrderItemEntity::find()
            .filter(
                crate::models::sales_order_item::Column::CreatedAt.gte(
                    long_start
                        .and_hms_opt(0, 0, 0)
                        .unwrap_or_default()
                        .and_utc(),
                ),
            )
            .all(&*self.db)
            .await?)
    }

    /// 获取所有库存记录（带 LIMIT 兜底，防 OOM）
    async fn fetch_all_stocks(
        &self,
    ) -> Result<Vec<crate::models::inventory_stock::Model>, AppError> {
        Ok(InventoryStockEntity::find().limit(10_000).all(&*self.db).await?)
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

// ---- 自由函数 helper：纯函数，无 DB / txn 依赖 ----

/// 按产品聚合每日销售量：HashMap<product_id, HashMap<date, total_qty>>
fn aggregate_product_daily_sales(
    items: &[crate::models::sales_order_item::Model],
) -> HashMap<i32, HashMap<NaiveDate, f64>> {
    let mut product_daily_sales: HashMap<i32, HashMap<NaiveDate, f64>> = HashMap::new();
    for item in items {
        let date = item.created_at.date_naive();
        let qty = item.quantity_meters.to_f64().unwrap_or(0.0);
        product_daily_sales
            .entry(item.product_id)
            .or_default()
            .entry(date)
            .and_modify(|v| *v += qty)
            .or_insert(qty);
    }
    product_daily_sales
}

/// 销售异常检测（Z-score）：SPIKE / DROP，CRITICAL / WARNING
fn detect_sales_zscore_anomalies(
    product_daily_sales: &HashMap<i32, HashMap<NaiveDate, f64>>,
    check_start: NaiveDate,
) -> Vec<AnomalyDetection> {
    let mut anomalies = Vec::new();
    for (pid, daily_map) in product_daily_sales {
        anomalies.extend(detect_sales_anomalies_for_product(*pid, daily_map, check_start));
    }
    anomalies
}

/// 单产品的 Z-score 异常检测：样本 <7 跳过，std==0 跳过
fn detect_sales_anomalies_for_product(
    pid: i32,
    daily_map: &HashMap<NaiveDate, f64>,
    check_start: NaiveDate,
) -> Vec<AnomalyDetection> {
    let all_values: Vec<f64> = daily_map.values().copied().collect();
    if all_values.len() < 7 {
        return Vec::new();
    }

    let mean_val = mean(&all_values);
    let std_val = std_deviation(&all_values);

    // 检测最近 period 内的异常
    let check_dates: Vec<NaiveDate> = daily_map
        .keys()
        .filter(|d| **d >= check_start)
        .cloned()
        .collect();

    let mut anomalies = Vec::new();
    for date in check_dates {
        if let Some(&value) = daily_map.get(&date) {
            if std_val > 0.0 {
                let z_score = (value - mean_val) / std_val;
                if let Some(anomaly) = build_sales_zscore_anomaly(pid, date, value, mean_val, z_score) {
                    anomalies.push(anomaly);
                }
            }
        }
    }
    anomalies
}

/// 根据 Z-score 构造销售异常（SPIKE / DROP），无异常返回 None
fn build_sales_zscore_anomaly(
    pid: i32,
    date: NaiveDate,
    value: f64,
    mean_val: f64,
    z_score: f64,
) -> Option<AnomalyDetection> {
    if z_score > 2.5 {
        Some(AnomalyDetection {
            entity_type: "SALES".to_string(),
            entity_id: pid,
            anomaly_type: "SPIKE".to_string(),
            severity: if z_score > 3.5 { "CRITICAL" } else { "WARNING" }.to_string(),
            description: format!(
                "产品 {} 在 {} 销售量异常突增: {:.0} (均值={:.0}, Z-score={:.2})",
                pid, date, value, mean_val, z_score
            ),
            detected_at: Utc::now(),
        })
    } else if z_score < -2.5 {
        Some(AnomalyDetection {
            entity_type: "SALES".to_string(),
            entity_id: pid,
            anomaly_type: "DROP".to_string(),
            severity: if z_score < -3.5 { "CRITICAL" } else { "WARNING" }.to_string(),
            description: format!(
                "产品 {} 在 {} 销售量异常突降: {:.0} (均值={:.0}, Z-score={:.2})",
                pid, date, value, mean_val, z_score
            ),
            detected_at: Utc::now(),
        })
    } else {
        None
    }
}

/// 计算库存 IQR 下/上界；样本不足（<4）返回 None
fn compute_iqr_fences(quantities: &[f64]) -> Option<(f64, f64)> {
    if quantities.len() < 4 {
        return None;
    }
    let (q1, q3) = iqr_quartiles(quantities);
    let iqr = q3 - q1;
    let lower_fence = (q1 - 1.5 * iqr).max(0.0);
    let upper_fence = q3 + 1.5 * iqr;
    Some((lower_fence, upper_fence))
}

/// 库存异常检测（IQR 方法）：ZERO_STOCK / LOW_STOCK / OVERSTOCK
fn detect_inventory_iqr_anomalies(
    stocks: &[crate::models::inventory_stock::Model],
    lower_fence: f64,
    upper_fence: f64,
) -> Vec<AnomalyDetection> {
    let mut anomalies = Vec::new();
    for stock in stocks {
        if let Some(anomaly) = build_inventory_iqr_anomaly(stock, lower_fence, upper_fence) {
            anomalies.push(anomaly);
        }
    }
    anomalies
}

/// 根据 IQR 上下界构造单条库存异常，无异常返回 None
fn build_inventory_iqr_anomaly(
    stock: &crate::models::inventory_stock::Model,
    lower_fence: f64,
    upper_fence: f64,
) -> Option<AnomalyDetection> {
    let qty = stock.quantity_available.to_f64().unwrap_or(0.0);

    // 零库存
    if qty <= 0.0 {
        Some(inventory_anomaly(
            stock,
            "ZERO_STOCK",
            "CRITICAL",
            format!("产品 {} 库存为零，仓库={}", stock.product_id, stock.warehouse_id),
        ))
    }
    // 库存异常低 (IQR 下界)
    else if qty < lower_fence && qty > 0.0 {
        Some(inventory_anomaly(
            stock,
            "LOW_STOCK",
            "WARNING",
            format!(
                "产品 {} 库存({:.0})低于IQR下界({:.0}), 仓库={}",
                stock.product_id, qty, lower_fence, stock.warehouse_id
            ),
        ))
    }
    // 库存异常高 (积压)
    else if qty > upper_fence {
        Some(inventory_anomaly(
            stock,
            "OVERSTOCK",
            "WARNING",
            format!(
                "产品 {} 库存({:.0})超过IQR上界({:.0}), 疑似积压, 仓库={}",
                stock.product_id, qty, upper_fence, stock.warehouse_id
            ),
        ))
    } else {
        None
    }
}

/// 构造一条 INVENTORY 类型的异常记录（统一 entity_type 与 detected_at）
fn inventory_anomaly(
    stock: &crate::models::inventory_stock::Model,
    anomaly_type: &str,
    severity: &str,
    description: String,
) -> AnomalyDetection {
    AnomalyDetection {
        entity_type: "INVENTORY".to_string(),
        entity_id: stock.product_id,
        anomaly_type: anomaly_type.to_string(),
        severity: severity.to_string(),
        description,
        detected_at: Utc::now(),
    }
}

/// 滞销产品检测：有库存但历史窗口内无出库记录
fn detect_slow_moving_products(
    stocks: &[crate::models::inventory_stock::Model],
    product_daily_sales: &HashMap<i32, HashMap<NaiveDate, f64>>,
    days: i64,
) -> Vec<AnomalyDetection> {
    let mut anomalies = Vec::new();
    for stock in stocks {
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
    anomalies
}
