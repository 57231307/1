//! 销售预测 / 库存优化 handler
//!
//! 包含销售预测（基于时间序列算法）以及库存优化建议。

use axum::{extract::State, Json};
use rust_decimal::prelude::ToPrimitive;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::product::Entity as ProductEntity;
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 销售预测 - 使用真实时间序列算法（移动平均 + 指数平滑）
// ============================================================================

/// 销售预测
pub async fn sales_forecast(
    State(state): State<AppState>,
    Json(payload): Json<SalesForecastRequest>,
) -> Result<Json<ApiResponse<SalesForecastResponse>>, AppError> {
    let service = AiAnalysisService::new(state.db);

    let days = match payload.period.as_str() {
        "3m" => 90,
        "6m" => 180,
        "12m" => 365,
        _ => 30,
    };

    // 物料 ID 缺失时拒绝预测，避免脏 product_id=0 污染
    let product_id: i32 = match payload.product_id {
        Some(id) => id as i32,
        None => return Err(AppError::validation("预测请求缺少物料ID")),
    };

    match service.forecast_sales(product_id, days).await {
        Ok(forecasts) => {
            let total_amount: f64 = forecasts
                .iter()
                .map(|f| f.predicted_quantity.to_f64().unwrap_or(0.0))
                .sum();
            let order_count = forecasts.len() as u32;
            let avg_confidence = if forecasts.is_empty() {
                0.0
            } else {
                forecasts.iter().map(|f| f.confidence).sum::<f64>() / forecasts.len() as f64
            };
            let trend = if forecasts.is_empty() {
                "无数据".to_string()
            } else {
                forecasts[0].trend.clone()
            };

            let response = SalesForecastResponse {
                sales_amount: total_amount,
                order_count,
                confidence: (avg_confidence * 100.0).round() as u32,
                trend,
                period: payload.period,
                detail: forecasts
                    .into_iter()
                    .take(30)
                    .map(|f| ForecastDetail {
                        date: f.forecast_date.to_string(),
                        predicted_quantity: f.predicted_quantity.to_string(),
                        confidence: f.confidence,
                        trend: f.trend,
                    })
                    .collect(),
            };

            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("销售预测失败: {}", e);
            Ok(Json(ApiResponse::error(format!("销售预测失败: {}", e))))
        }
    }
}

// ============================================================================
// 库存优化 - 基于历史出库数据
// ============================================================================

/// 库存优化建议
pub async fn inventory_optimization(
    State(state): State<AppState>,
    _auth: AuthContext,
    payload: Option<Json<InventoryOptimizationRequest>>,
) -> Result<Json<ApiResponse<InventoryOptimizationResponse>>, AppError> {
    let db = state.db.clone();
    let service = AiAnalysisService::new(state.db);

    let product_id = payload
        .as_ref()
        .and_then(|p| p.product_id.map(|pid| pid as i32));

    let suggestions = service.optimize_inventory(product_id).await?;

    let high_count = suggestions
        .iter()
        .filter(|s| {
            let current = s.current_stock.to_f64().unwrap_or(0.0);
            current < s.reorder_point.to_f64().unwrap_or(0.0)
        })
        .count();
    let overstock_count = suggestions
        .iter()
        .filter(|s| {
            let current = s.current_stock.to_f64().unwrap_or(0.0);
            let suggested = s.suggested_stock.to_f64().unwrap_or(0.0);
            current > suggested * 2.0
        })
        .count();

    let mut items: Vec<InventorySuggestion> = Vec::new();

    // v16 批次 44 修复：循环外批量查询所有产品名称，避免循环内逐个查询（N+1）
    let product_ids: Vec<i32> = suggestions.iter().map(|s| s.product_id).collect();
    let product_name_map: std::collections::HashMap<i32, String> = if product_ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        use sea_orm::ColumnTrait;
        ProductEntity::find()
            .filter(crate::models::product::Column::Id.is_in(product_ids))
            .all(&*db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect()
    };

    for s in suggestions {
        let current = s.current_stock.to_f64().unwrap_or(0.0);
        let reorder_point = s.reorder_point.to_f64().unwrap_or(0.0);
        let suggested = s.suggested_stock.to_f64().unwrap_or(0.0);

        if current >= reorder_point && current <= suggested * 2.0 && current > 0.0 {
            continue;
        }

        let priority = if current <= 0.0 || current < reorder_point * 0.5 {
            "high"
        } else if current < reorder_point {
            "medium"
        } else {
            "low"
        };

        // v16 批次 44 修复：从批量查询结果获取产品名称（O(1) 查找）
        let product_name = product_name_map
            .get(&s.product_id)
            .cloned()
            .unwrap_or_else(|| format!("产品 {}", s.product_id));

        items.push(InventorySuggestion {
            product_name,
            suggestion: format!(
                "{} (当前库存: {:.0}, 再订货点: {:.0}, 建议订货: {:.0})",
                s.reason,
                current,
                reorder_point,
                s.reorder_quantity.to_f64().unwrap_or(0.0)
            ),
            priority: priority.to_string(),
        });
    }

    let summary = format!(
        "检测到 {} 个产品需要补货，{} 个产品库存积压",
        high_count, overstock_count
    );

    let response = InventoryOptimizationResponse { summary, items };
    Ok(Json(ApiResponse::success(response)))
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastRequest {
    pub period: String,
    pub product_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastResponse {
    pub sales_amount: f64,
    pub order_count: u32,
    pub confidence: u32,
    pub trend: String,
    pub period: String,
    pub detail: Vec<ForecastDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ForecastDetail {
    pub date: String,
    pub predicted_quantity: String,
    pub confidence: f64,
    pub trend: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationRequest {
    pub warehouse_id: Option<u32>,
    pub product_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationResponse {
    pub summary: String,
    pub items: Vec<InventorySuggestion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventorySuggestion {
    pub product_name: String,
    pub suggestion: String,
    pub priority: String,
}
