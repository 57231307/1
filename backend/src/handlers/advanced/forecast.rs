//! 销售预测 / 库存优化 handler
//!
//! 包含销售预测（基于时间序列算法）以及库存优化建议。

use axum::{Json, extract::State};
use rust_decimal::prelude::ToPrimitive;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::product::Entity as ProductEntity;
use crate::services::ai::AiAnalysisService;
use crate::utils::error::AppError;
use crate::utils::optional_json::OptionalJson;
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
        None => return Err(AppError::validation_displayable("预测请求缺少物料ID")),
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
            tracing::error!("销售预测失败: {e}");
            Err(e)
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
    payload: OptionalJson<InventoryOptimizationRequest>,
) -> Result<Json<ApiResponse<InventoryOptimizationResponse>>, AppError> {
    let db = state.db.clone();
    let service = AiAnalysisService::new(state.db);

    let product_id = payload
        .0
        .as_ref()
        .and_then(|p| p.product_id.map(|pid| pid as i32));

    let suggestions = service.optimize_inventory(product_id).await?;

    let product_name_map = build_product_name_map(&db, &suggestions).await?;
    // 产品名称只取产品主数据真值；取不到真值的建议行整体剔除，预警计数与列表行同源
    let named = attach_product_names(suggestions, &product_name_map);
    let (low_count, overstock_count) = count_inventory_alerts(&named);
    let items = build_inventory_suggestions(named);

    let summary = format!(
        "检测到 {} 个产品需要补货，{} 个产品库存积压",
        low_count, overstock_count
    );

    let response = InventoryOptimizationResponse { summary, items };
    Ok(Json(ApiResponse::success(response)))
}

/// 统计库存预警数量（需补货/积压）
fn count_inventory_alerts(named: &[NamedSuggestion]) -> (usize, usize) {
    let low_count = named
        .iter()
        .filter(|n| {
            let current = n.suggestion.current_stock.to_f64().unwrap_or(0.0);
            current < n.suggestion.reorder_point.to_f64().unwrap_or(0.0)
        })
        .count();
    let overstock_count = named
        .iter()
        .filter(|n| {
            let current = n.suggestion.current_stock.to_f64().unwrap_or(0.0);
            let suggested = n.suggestion.suggested_stock.to_f64().unwrap_or(0.0);
            current > suggested * 2.0
        })
        .count();
    (low_count, overstock_count)
}

/// 批量查询产品名称映射（避免 N+1 查询）
///
/// 入参：建议行集合（取其 product_id）；返回：product_id → 产品主数据名称。
/// 查询失败按 `AppError` 上抛——吞成空表会让整列建议失去真实名称来源。
async fn build_product_name_map(
    db: &sea_orm::DatabaseConnection,
    suggestions: &[crate::services::ai::InventorySuggestion],
) -> Result<std::collections::HashMap<i32, String>, AppError> {
    let product_ids: Vec<i32> = suggestions.iter().map(|s| s.product_id).collect();
    if product_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    use sea_orm::{ColumnTrait, QueryFilter};
    let products = ProductEntity::find()
        .filter(crate::models::product::Column::Id.is_in(product_ids))
        .all(db)
        .await?;
    Ok(products.into_iter().map(|p| (p.id, p.name)).collect())
}

/// 建议行配上真实产品名称后的形态（名称与后续预警计数、列表行同源）
struct NamedSuggestion {
    product_name: String,
    suggestion: crate::services::ai::InventorySuggestion,
}

/// 逐行配产品主数据名称；主数据无对应行的建议剔除并 warn 点名。
///
/// 传给 `count_inventory_alerts` 与 `build_inventory_suggestions`，使汇总口径与
/// 响应列表都只覆盖能如实展示的行；禁止用占位字符串把无真实名称的行留在响应里。
fn attach_product_names(
    suggestions: Vec<crate::services::ai::InventorySuggestion>,
    product_name_map: &std::collections::HashMap<i32, String>,
) -> Vec<NamedSuggestion> {
    let mut named = Vec::with_capacity(suggestions.len());
    for s in suggestions {
        match product_name_map.get(&s.product_id) {
            Some(product_name) => named.push(NamedSuggestion {
                product_name: product_name.clone(),
                suggestion: s,
            }),
            None => {
                tracing::warn!(
                    product_id = s.product_id,
                    "库存优化建议剔除：产品主数据无对应行，无法提供真实产品名称"
                );
            }
        }
    }
    named
}

/// 构建库存优化建议项列表（过滤正常库存，分级优先级）
fn build_inventory_suggestions(named: Vec<NamedSuggestion>) -> Vec<InventorySuggestion> {
    let mut items = Vec::new();
    for n in named {
        let s = n.suggestion;
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

        items.push(InventorySuggestion {
            product_name: n.product_name,
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
    items
}

// ============================================================================
// 数据结构
// ============================================================================

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastRequest {
    pub period: String,
    pub product_id: Option<u32>,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct SalesForecastResponse {
    pub sales_amount: f64,
    pub order_count: u32,
    pub confidence: u32,
    pub trend: String,
    pub period: String,
    pub detail: Vec<ForecastDetail>,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct ForecastDetail {
    pub date: String,
    pub predicted_quantity: String,
    pub confidence: f64,
    pub trend: String,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationRequest {
    pub warehouse_id: Option<u32>,
    pub product_id: Option<u32>,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryOptimizationResponse {
    pub summary: String,
    pub items: Vec<InventorySuggestion>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code, reason = "序列化/反序列化字段")]
pub struct InventorySuggestion {
    pub product_name: String,
    pub suggestion: String,
    pub priority: String,
}
