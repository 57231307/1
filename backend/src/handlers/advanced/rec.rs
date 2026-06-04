//! 智能推荐 handler
//!
//! 提供基于关联规则与趋势的智能推荐能力。

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::ai_analysis_service::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// ============================================================================
// 智能推荐 - 关联规则推荐
// ============================================================================

/// 智能推荐
pub async fn recommendations(
    State(state): State<AppState>,
    _auth: AuthContext,
    payload: Option<Json<RecommendationRequest>>,
) -> impl IntoResponse {
    let service = AiAnalysisService::new(state.db);

    let rec_type = payload
        .as_ref()
        .and_then(|p| p.recommendation_type.clone())
        .unwrap_or_else(|| "all".to_string());
    let limit = payload.as_ref().and_then(|p| p.limit).unwrap_or(10);

    match service.generate_recommendations(rec_type, limit).await {
        Ok(recs) => {
            let items: Vec<Recommendation> = recs
                .into_iter()
                .map(|r| {
                    let rec_type_label = match r.recommendation_type.as_str() {
                        "REORDER" => "补货建议",
                        "BUNDLE" => "关联推荐",
                        "TREND" => "趋势推荐",
                        "PRICE_ADJUST" => "价格调整",
                        _ => "综合推荐",
                    };

                    Recommendation {
                        content: r.reason,
                        recommendation_type: rec_type_label.to_string(),
                        created_at: Utc::now().format("%Y-%m-%d %H:%M").to_string(),
                        score: r.score,
                        target_id: r.target_id,
                    }
                })
                .collect();

            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("生成推荐失败: {}", e);
            Json(ApiResponse::error(format!("生成推荐失败: {}", e)))
        }
    }
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationRequest {
    pub recommendation_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub content: String,
    pub recommendation_type: String,
    pub created_at: String,
    pub score: f64,
    pub target_id: i32,
}
