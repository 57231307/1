use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Serialize)]
pub struct SalesForecastResponse {
    pub product_id: i32,
    pub forecast_date: String,
    pub predicted_quantity: String,
    pub confidence: f64,
    pub trend: String,
}

#[derive(Debug, Deserialize)]
pub struct ForecastSalesQuery {
    pub product_id: i32,
    pub days: Option<i64>,
}

pub async fn forecast_sales(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ForecastSalesQuery>,
) -> Result<Json<ApiResponse<Vec<SalesForecastResponse>>>, AppError> {
    // V15 P0-S26：AI 端点权限码 ai-forecast:read（权限由 middleware 强制）
    // V15 P0-S27 预备：记录调用者身份，为后续 data_scope 过滤铺路
    tracing::info!(
        user_id = auth.user_id,
        username = %auth.username,
        product_id = query.product_id,
        "AI 销售预测调用"
    );
    let service = AiAnalysisService::new(state.db);
    let days = query.days.unwrap_or(30);

    let forecasts = service
        .forecast_sales(query.product_id, days)
        .await
        .map_err(|e| {
            tracing::error!("销售预测失败: {}", e);
            AppError::internal(format!("销售预测失败: {}", e))
        })?;

    let responses: Vec<SalesForecastResponse> = forecasts
        .into_iter()
        .map(|f| SalesForecastResponse {
            product_id: f.product_id,
            forecast_date: f.forecast_date.to_string(),
            predicted_quantity: f.predicted_quantity.to_string(),
            confidence: f.confidence,
            trend: f.trend,
        })
        .collect();
    Ok(Json(ApiResponse::success(responses)))
}

#[derive(Debug, Serialize)]
pub struct InventorySuggestionResponse {
    pub product_id: i32,
    pub current_stock: String,
    pub suggested_stock: String,
    pub reorder_point: String,
    pub reorder_quantity: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct OptimizeInventoryQuery {
    pub product_id: Option<i32>,
}

pub async fn optimize_inventory(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<OptimizeInventoryQuery>,
) -> Result<Json<ApiResponse<Vec<InventorySuggestionResponse>>>, AppError> {
    // V15 P0-S26：AI 端点权限码 ai-inventory-opt:read
    // V15 P0-S27 预备：记录调用者身份
    tracing::info!(
        user_id = auth.user_id,
        username = %auth.username,
        product_id = ?query.product_id,
        "AI 库存优化调用"
    );
    let service = AiAnalysisService::new(state.db);

    let suggestions = service
        .optimize_inventory(query.product_id)
        .await
        .map_err(|e| {
            tracing::error!("库存优化失败: {}", e);
            AppError::internal(format!("库存优化失败: {}", e))
        })?;

    let responses: Vec<InventorySuggestionResponse> = suggestions
        .into_iter()
        .map(|s| InventorySuggestionResponse {
            product_id: s.product_id,
            current_stock: s.current_stock.to_string(),
            suggested_stock: s.suggested_stock.to_string(),
            reorder_point: s.reorder_point.to_string(),
            reorder_quantity: s.reorder_quantity.to_string(),
            reason: s.reason,
        })
        .collect();
    Ok(Json(ApiResponse::success(responses)))
}

#[derive(Debug, Serialize)]
pub struct AnomalyDetectionResponse {
    pub entity_type: String,
    pub entity_id: i32,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub detected_at: String,
}

#[derive(Debug, Deserialize)]
pub struct DetectAnomaliesQuery {
    pub days: Option<i64>,
}

pub async fn detect_anomalies(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DetectAnomaliesQuery>,
) -> Result<Json<ApiResponse<Vec<AnomalyDetectionResponse>>>, AppError> {
    // V15 P0-S26：AI 端点权限码 ai-anomaly:read
    // V15 P0-S27 预备：记录调用者身份
    tracing::info!(
        user_id = auth.user_id,
        username = %auth.username,
        days = query.days.unwrap_or(7),
        "AI 异常检测调用"
    );
    let service = AiAnalysisService::new(state.db);
    let days = query.days.unwrap_or(7);

    let anomalies = service.detect_anomalies(days).await.map_err(|e| {
        tracing::error!("异常检测失败: {}", e);
        AppError::internal(format!("异常检测失败: {}", e))
    })?;

    let responses: Vec<AnomalyDetectionResponse> = anomalies
        .into_iter()
        .map(|a| AnomalyDetectionResponse {
            entity_type: a.entity_type,
            entity_id: a.entity_id,
            anomaly_type: a.anomaly_type,
            severity: a.severity,
            description: a.description,
            detected_at: a.detected_at.to_rfc3339(),
        })
        .collect();
    Ok(Json(ApiResponse::success(responses)))
}

#[derive(Debug, Serialize)]
pub struct SmartRecommendationResponse {
    pub recommendation_type: String,
    pub target_id: i32,
    pub target_type: String,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RecommendationsQuery {
    pub recommendation_type: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_recommendations(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<RecommendationsQuery>,
) -> Result<Json<ApiResponse<Vec<SmartRecommendationResponse>>>, AppError> {
    // V15 P0-S26：AI 端点权限码 ai-recommendation:read
    // V15 P0-S27 预备：记录调用者身份
    tracing::info!(
        user_id = auth.user_id,
        username = %auth.username,
        rec_type = ?query.recommendation_type,
        "AI 智能推荐调用"
    );
    let service = AiAnalysisService::new(state.db);
    let rec_type = query
        .recommendation_type
        .unwrap_or_else(|| "all".to_string());
    let limit = query.limit.unwrap_or(10).clamp(1, 100);

    let recommendations = service
        .generate_recommendations(rec_type, limit)
        .await
        .map_err(|e| {
            tracing::error!("生成推荐失败: {}", e);
            AppError::internal(format!("生成推荐失败: {}", e))
        })?;

    let responses: Vec<SmartRecommendationResponse> = recommendations
        .into_iter()
        .map(|r| SmartRecommendationResponse {
            recommendation_type: r.recommendation_type,
            target_id: r.target_id,
            target_type: r.target_type,
            score: r.score,
            reason: r.reason,
        })
        .collect();
    Ok(Json(ApiResponse::success(responses)))
}
