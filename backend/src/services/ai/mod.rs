//! AI 智能分析服务模块（ai = artificial intelligence）
//!
//! 由原 `services/ai_analysis_service.rs`（1202 行）按业务子领域拆分而来。
//! 子模块：
//! - `pred`         预测：销售预测（移动平均 + 指数平滑 + 季节性因子）
//! - `detect`       异常检测：销售突增/突降、库存零/积压/滞销（Z-score / IQR）
//! - `rec`          推荐：智能补货、关联推荐、趋势推荐、价格调整推荐
//! - `recipe_opt`   工艺优化：染色配方 k-NN 相似度推荐（温度/时间/pH/浴比）
//! - `quality_pred` 质量预测：基于历史检验记录的趋势 / 风险评分 / 建议措施
//!
//! 兼容说明：原 `crate::services::ai::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::ai::*;` 重新导出以保持向后兼容。

use moka::future::Cache;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub mod detect;
pub mod pred;
pub mod quality_pred;
pub mod rec;
pub mod recipe_opt;

// =====================================================
// 共享 DTO（与原 ai_analysis_service.rs 保持一致）
// =====================================================

/// 销售预测结果
#[derive(Debug, Clone)]
pub struct SalesForecast {
    pub product_id: i32,
    pub forecast_date: chrono::NaiveDate,
    pub predicted_quantity: rust_decimal::Decimal,
    pub confidence: f64,
    pub trend: String,
}

/// 库存优化建议
#[derive(Debug, Clone)]
pub struct InventorySuggestion {
    pub product_id: i32,
    pub current_stock: rust_decimal::Decimal,
    pub suggested_stock: rust_decimal::Decimal,
    pub reorder_point: rust_decimal::Decimal,
    pub reorder_quantity: rust_decimal::Decimal,
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
    pub detected_at: chrono::DateTime<chrono::Utc>,
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
    pub total_sales: rust_decimal::Decimal,
    pub cumulative_ratio: f64,
}

/// 库存周转率结果
#[derive(Debug, Clone)]
pub struct InventoryTurnover {
    pub product_id: i32,
    pub turnover_rate: f64,
    pub avg_stock: rust_decimal::Decimal,
    pub total_outbound: rust_decimal::Decimal,
    pub period_days: i64,
}

// =====================================================
// 共享 Service 结构体（子模块均通过 impl AiAnalysisService 扩展）
// =====================================================

/// V15 P1 5.2：AI 推理并发上限（permits=10），防止 CPU 过载
pub const AI_CONCURRENCY_LIMIT: usize = 10;
/// V15 P1 5.3：AI 推理缓存 TTL（5 分钟），相同入参命中缓存直接返回
pub const AI_CACHE_TTL_SECS: u64 = 300;
/// V15 P1 5.3：AI 推理缓存容量上限
pub const AI_CACHE_CAPACITY: u64 = 1_000;
/// V15 P1 5.1+9.5：AI 推理超时阈值（2s），超时返回降级结果
pub const AI_INFERENCE_TIMEOUT_MS: u64 = 2_000;

/// AI分析 Service
///
/// V15 P1 5.2+5.3：内置 Semaphore（permits=10）控制并发；moka 缓存（TTL 5min）避免重复计算。
pub struct AiAnalysisService {
    pub(crate) db: Arc<DatabaseConnection>,
    /// V15 P1 5.2：并发许可，所有 AI 推理方法在进入算法前 acquire
    pub(crate) semaphore: Arc<Semaphore>,
    /// V15 P1 5.3：工艺优化结果缓存（key 为入参指纹，TTL 5min）
    pub(crate) recipe_cache: Arc<Cache<String, recipe_opt::RecipeOptResponse>>,
    /// V15 P1 5.3：质量预测结果缓存（key 为入参指纹，TTL 5min）
    pub(crate) quality_cache: Arc<Cache<String, quality_pred::QualityPredResponse>>,
}

impl AiAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            semaphore: Arc::new(Semaphore::new(AI_CONCURRENCY_LIMIT)),
            recipe_cache: Arc::new(
                Cache::builder()
                    .time_to_live(std::time::Duration::from_secs(AI_CACHE_TTL_SECS))
                    .max_capacity(AI_CACHE_CAPACITY)
                    .build(),
            ),
            quality_cache: Arc::new(
                Cache::builder()
                    .time_to_live(std::time::Duration::from_secs(AI_CACHE_TTL_SECS))
                    .max_capacity(AI_CACHE_CAPACITY)
                    .build(),
            ),
        }
    }

    /// V15 P1 5.2：获取并发许可，所有 AI 推理方法在进入算法前调用
    ///
    /// 返回 permit 守卫，drop 时自动释放；permits=10 防止多用户并发调用 CPU 过载。
    pub(crate) async fn acquire_inference_permit(
        &self,
    ) -> Result<tokio::sync::OwnedSemaphorePermit, crate::utils::error::AppError> {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| crate::utils::error::AppError::internal("AI 推理并发许可获取失败"))
    }
}

// =====================================================
// 共享统计工具函数（供各子模块使用）
// =====================================================

/// 计算均值
pub(crate) fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// 计算标准差
pub(crate) fn std_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

/// 计算 IQR 四分位数 (Q1, Q3)
pub(crate) fn iqr_quartiles(values: &[f64]) -> (f64, f64) {
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
