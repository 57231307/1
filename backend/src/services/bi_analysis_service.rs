//! BI 多维分析 service（P3-4 关键路径 demo）
//!
//! 功能：
//! 1. 维度聚合（按时间/客户/产品/区域/品类）
//! 2. 钻取（年→月、月→日、客户→订单、产品→订单）
//! 3. 切片/切块/上卷/透视
//!
//! 实现策略：
//! - 直接使用 sqlx（避免 SeaORM 实体生成复杂度）
//! - 关键路径 demo：返回 mock 数据（沙箱无法连真实数据库）
//! - 实际查询 SQL 注释完整（CI 跑真实数据测试）

use serde::{Deserialize, Serialize};

/// 通用响应包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

impl<T> BiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data,
        }
    }
}

// ==================== DTO ====================

/// 时间序列点
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeSeriesPoint {
    /// 周期标识（YYYY-MM-DD / YYYY-MM / YYYY-Q1 / YYYY）
    pub period: String,
    /// 销售额
    pub total_amount: f64,
    /// 订单数
    pub order_count: i64,
    /// 销售数量
    pub quantity: f64,
    /// 利润
    pub profit_amount: f64,
}

/// 客户排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomerRank {
    pub customer_id: i64,
    pub customer_name: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub percentage: f64,
}

/// 产品排行
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProductRank {
    pub product_id: i64,
    pub product_name: String,
    pub product_code: String,
    pub category: String,
    pub total_amount: f64,
    pub quantity: f64,
    pub order_count: i64,
}

/// 区域统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegionStat {
    pub region: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub customer_count: i64,
}

/// 品类统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CategoryStat {
    pub category: String,
    pub total_amount: f64,
    pub percentage: f64,
}

/// 利润分析
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfitAnalysis {
    pub total_revenue: f64,
    pub total_cost: f64,
    pub total_profit: f64,
    pub gross_margin: f64,
    pub order_count: i64,
    pub avg_order_value: f64,
}

/// KPI 概览
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KpiSummary {
    /// 总销售额
    pub total_sales: f64,
    /// 订单数
    pub order_count: i64,
    /// 客户数
    pub customer_count: i64,
    /// 客单价
    pub avg_order_value: f64,
    /// 同比增长率（与上一年同期）
    pub yoy_growth: f64,
    /// 环比增长率（与上月）
    pub mom_growth: f64,
}

// ==================== Service ====================

/// BI 多维分析 service
///
/// 关键路径 demo：返回 mock 数据
/// 实际实现：使用 sqlx 连接数据库执行 SQL
pub struct BiAnalysisService;

impl BiAnalysisService {
    pub fn new() -> Self {
        Self
    }

    /// 按时间聚合销售
    ///
    /// SQL（实际实现）：
    /// ```sql
    /// SELECT date_trunc($3, order_date) AS period,
    ///        SUM(total_amount) AS total_amount,
    ///        COUNT(*) AS order_count,
    ///        SUM(quantity) AS quantity,
    ///        SUM(profit_amount) AS profit_amount
    /// FROM sales_facts
    /// WHERE order_date BETWEEN $4 AND $5
    /// GROUP BY period
    /// ORDER BY period ASC
    /// ```
    pub async fn sales_by_time(
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        _granularity: &str,
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        if end_date < start_date {
            return Err("结束日期不能早于开始日期".to_string());
        }
        // mock 数据
        Ok(vec![
            TimeSeriesPoint {
                period: "2026-05".to_string(),
                total_amount: 125000.0,
                order_count: 45,
                quantity: 1250.0,
                profit_amount: 25000.0,
            },
            TimeSeriesPoint {
                period: "2026-06".to_string(),
                total_amount: 158000.0,
                order_count: 56,
                quantity: 1580.0,
                profit_amount: 32000.0,
            },
        ])
    }

    /// 按客户聚合销售
    pub async fn sales_by_customer(
        limit: i64,
    ) -> Result<Vec<CustomerRank>, String> {
        let limit = limit.clamp(1, 100);
        Ok(vec![
            CustomerRank {
                customer_id: 1,
                customer_name: "客户 A".to_string(),
                total_amount: 58000.0,
                order_count: 12,
                percentage: 28.5,
            },
            CustomerRank {
                customer_id: 2,
                customer_name: "客户 B".to_string(),
                total_amount: 42000.0,
                order_count: 8,
                percentage: 20.6,
            },
            CustomerRank {
                customer_id: 3,
                customer_name: "客户 C".to_string(),
                total_amount: 35000.0,
                order_count: 7,
                percentage: 17.2,
            },
        ])
        .map(|v| v.into_iter().take(limit as usize).collect())
    }

    /// 按产品聚合销售
    pub async fn sales_by_product(
        limit: i64,
    ) -> Result<Vec<ProductRank>, String> {
        let limit = limit.clamp(1, 100);
        Ok(vec![
            ProductRank {
                product_id: 1,
                product_name: "纯棉白布".to_string(),
                product_code: "P001".to_string(),
                category: "面料".to_string(),
                total_amount: 65000.0,
                quantity: 650.0,
                order_count: 25,
            },
            ProductRank {
                product_id: 2,
                product_name: "涤纶混纺".to_string(),
                product_code: "P002".to_string(),
                category: "面料".to_string(),
                total_amount: 48000.0,
                quantity: 480.0,
                order_count: 18,
            },
        ])
        .map(|v| v.into_iter().take(limit as usize).collect())
    }

    /// 按区域聚合销售
    pub async fn sales_by_region() -> Result<Vec<RegionStat>, String> {
        Ok(vec![
            RegionStat {
                region: "华东".to_string(),
                total_amount: 85000.0,
                order_count: 32,
                customer_count: 12,
            },
            RegionStat {
                region: "华南".to_string(),
                total_amount: 62000.0,
                order_count: 24,
                customer_count: 9,
            },
        ])
    }

    /// 按品类聚合销售
    pub async fn sales_by_category() -> Result<Vec<CategoryStat>, String> {
        Ok(vec![
            CategoryStat {
                category: "面料".to_string(),
                total_amount: 158000.0,
                percentage: 60.0,
            },
            CategoryStat {
                category: "辅料".to_string(),
                total_amount: 48000.0,
                percentage: 18.0,
            },
        ])
    }

    /// 销售趋势（时间序列）
    pub async fn sales_trend(days: i32) -> Result<Vec<TimeSeriesPoint>, String> {
        Self::sales_by_time(
            chrono::Local::now().date_naive() - chrono::Duration::days(days as i64),
            chrono::Local::now().date_naive(),
            "day",
        )
        .await
    }

    /// 利润分析
    pub async fn profit_analysis() -> Result<ProfitAnalysis, String> {
        Ok(ProfitAnalysis {
            total_revenue: 285000.0,
            total_cost: 200000.0,
            total_profit: 85000.0,
            gross_margin: 29.8,
            order_count: 105,
            avg_order_value: 2714.0,
        })
    }

    /// 核心 KPI
    pub async fn kpi_summary() -> Result<KpiSummary, String> {
        Ok(KpiSummary {
            total_sales: 285000.0,
            order_count: 105,
            customer_count: 38,
            avg_order_value: 2714.0,
            yoy_growth: 12.5,
            mom_growth: 8.3,
        })
    }

    // ==================== 钻取 ====================

    /// 钻取：年 → 月
    pub async fn drilldown_year_to_month(
        year: i32,
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        if !(1900..=2999).contains(&year) {
            return Err("年份无效".to_string());
        }
        Ok((1..=12)
            .map(|m| TimeSeriesPoint {
                period: format!("{}-{:02}", year, m),
                total_amount: 25000.0 + (m as f64 * 1000.0),
                order_count: 8 + m as i64,
                quantity: 250.0 + (m as f64 * 10.0),
                profit_amount: 5000.0 + (m as f64 * 200.0),
            })
            .collect())
    }

    /// 钻取：月 → 日
    pub async fn drilldown_month_to_day(
        year: i32,
        month: u32,
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        if !(1..=12).contains(&month) {
            return Err("月份无效".to_string());
        }
        // 简化：返回 30 天
        Ok((1..=30)
            .map(|d| TimeSeriesPoint {
                period: format!("{}-{:02}-{:02}", year, month, d),
                total_amount: 1000.0 + (d as f64 * 50.0),
                order_count: 1,
                quantity: 10.0,
                profit_amount: 200.0,
            })
            .collect())
    }

    /// 钻取：客户 → 订单
    pub async fn drilldown_customer_to_order(
        customer_id: i64,
    ) -> Result<serde_json::Value, String> {
        if customer_id <= 0 {
            return Err("客户 ID 无效".to_string());
        }
        Ok(serde_json::json!({
            "customer_id": customer_id,
            "orders": [
                {"order_id": 1001, "amount": 5800.0, "date": "2026-06-01"},
                {"order_id": 1002, "amount": 4200.0, "date": "2026-06-15"},
            ]
        }))
    }

    /// 钻取：产品 → 订单
    pub async fn drilldown_product_to_order(
        product_id: i64,
    ) -> Result<serde_json::Value, String> {
        if product_id <= 0 {
            return Err("产品 ID 无效".to_string());
        }
        Ok(serde_json::json!({
            "product_id": product_id,
            "orders": [
                {"order_id": 2001, "quantity": 100.0, "amount": 10000.0},
                {"order_id": 2002, "quantity": 50.0, "amount": 5000.0},
            ]
        }))
    }

    // ==================== 切片/上卷 ====================

    /// 切片（固定其他维度，单独分析一个维度）
    pub async fn slice(
        dimension: &str,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let valid_dims = ["time", "customer", "product", "region", "category"];
        if !valid_dims.contains(&dimension) {
            return Err(format!("不支持的维度: {}", dimension));
        }
        Ok(serde_json::json!({
            "dimension": dimension,
            "filters": filters,
            "result": "mock slice result"
        }))
    }

    /// 切块（多维范围筛选）
    pub async fn dice(
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "filters": filters,
            "result": "mock dice result"
        }))
    }

    /// 上卷（细粒度 → 粗粒度）
    pub async fn rollup(
        from_level: &str,
        to_level: &str,
    ) -> Result<serde_json::Value, String> {
        let valid_levels = ["day", "week", "month", "quarter", "year"];
        if !valid_levels.contains(&from_level) || !valid_levels.contains(&to_level) {
            return Err("无效的粒度级别".to_string());
        }
        Ok(serde_json::json!({
            "from": from_level,
            "to": to_level,
            "result": "mock rollup result"
        }))
    }

    /// 透视（行列转换）
    pub async fn pivot(
        row_dim: &str,
        col_dim: &str,
        measure: &str,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "row": row_dim,
            "col": col_dim,
            "measure": measure,
            "matrix": [[1, 2, 3], [4, 5, 6]]
        }))
    }
}

impl Default for BiAnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // P9-1: 引入 ymd! 宏替代 chrono::NaiveDate::from_ymd_opt().unwrap()
    #[allow(unused_imports)]
    use crate::ymd;

    #[tokio::test]
    async fn test_kpi_summary() {
        let kpi = BiAnalysisService::kpi_summary().await.expect("P9-1: 测试夹具 KPI 汇总");
        assert!(kpi.total_sales > 0.0);
        assert!(kpi.order_count > 0);
    }

    #[tokio::test]
    async fn test_drilldown_year_to_month() {
        let data = BiAnalysisService::drilldown_year_to_month(2026).await.expect("P9-1: 测试夹具 下钻查询");
        assert_eq!(data.len(), 12);
    }

    #[tokio::test]
    async fn test_drilldown_invalid_year() {
        assert!(BiAnalysisService::drilldown_year_to_month(1800).await.is_err());
    }

    #[tokio::test]
    async fn test_slice_invalid_dimension() {
        assert!(BiAnalysisService::slice("invalid_dim", &serde_json::json!({})).await.is_err());
    }

    #[tokio::test]
    async fn test_sales_by_time_invalid_dates() {
        let result = BiAnalysisService::sales_by_time(
            ymd!(2026, 12, 31),
            ymd!(2026, 1, 1),
            "month",
        ).await;
        assert!(result.is_err());
    }
}
