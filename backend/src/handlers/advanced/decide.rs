//! 决策类 handler
//!
//! 提供异常检测端点（Z-score + IQR 统计法）。

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::ai::AiAnalysisService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 异常检测 - 使用统计方法（Z-score + IQR）
// ============================================================================

/// 异常检测
pub async fn anomaly_detection(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<AnomalyDetectionRequest>,
) -> Result<Json<ApiResponse<Vec<AnomalyItem>>>, AppError> {
    let service = AiAnalysisService::new(state.db);

    let days = payload
        .date_range
        .as_ref()
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);

    // V15 P2 14-9.3：异常检测失败时降级返回空列表（不阻塞前端）
    let anomalies = match service.detect_anomalies(days).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("异常检测失败，降级返回空列表: {}", e);
            return Ok(Json(ApiResponse::success(vec![])));
        }
    };

    let filtered = match payload.data_type.as_str() {
        "sales" => anomalies
            .into_iter()
            .filter(|a| a.entity_type == "SALES")
            .collect::<Vec<_>>(),
        "inventory" => anomalies
            .into_iter()
            .filter(|a| a.entity_type == "INVENTORY")
            .collect::<Vec<_>>(),
        "quality" => anomalies,
        _ => anomalies,
    };

    let items: Vec<AnomalyItem> = filtered
        .into_iter()
        .map(|a| {
            let severity = match a.severity.as_str() {
                "CRITICAL" => "critical",
                "WARNING" => "warning",
                "MEDIUM" => "warning",
                _ => "info",
            };

            let anomaly_type = match a.anomaly_type.as_str() {
                "SPIKE" => "突增",
                "DROP" => "突降",
                "ZERO_STOCK" => "零库存",
                "LOW_STOCK" => "低于安全线",
                "OVERSTOCK" => "库存积压",
                "SLOW_MOVING" => "滞销",
                other => other,
            };

            AnomalyItem {
                item: format!("{} #{}", a.entity_type, a.entity_id),
                anomaly_type: anomaly_type.to_string(),
                description: a.description,
                severity: severity.to_string(),
                detected_at: a.detected_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyDetectionRequest {
    pub data_type: String,
    pub date_range: Option<String>,
}

#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyItem {
    pub item: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
    pub detected_at: String,
}
