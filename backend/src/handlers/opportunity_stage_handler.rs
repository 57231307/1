use axum::{extract::State, Json};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::crm_opportunity;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 阶段停留时长统计
#[derive(Serialize)]
pub struct StageDurationStats {
    /// 阶段名称
    pub stage: String,
    /// 平均停留时长（天）
    pub avg_duration_days: f64,
    /// 最短停留时长（天）
    pub min_duration_days: f64,
    /// 最长停留时长（天）
    pub max_duration_days: f64,
    /// 商机数量
    pub opportunity_count: i64,
}

/// GET /api/v1/erp/crm/opportunities/stage-stats - 商机阶段停留时长统计
/// batch-14 P3: 人工复核状态机不完整
pub async fn get_opportunity_stage_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<StageDurationStats>>>, AppError> {
    // 查询所有已转化的商机
    let opportunities = crm_opportunity::Entity::find()
        .filter(crm_opportunity::Column::ConvertedAt.is_not_null())
        .filter(crm_opportunity::Column::CreatedAt.is_not_null())
        .all(&*state.db)
        .await?;

    // 按阶段分组统计
    let mut stage_map: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

    for opp in &opportunities {
        if let (Some(created), Some(converted)) = (opp.created_at, opp.converted_at) {
            let days = (converted - created).num_days() as f64;
            let stage = opp.stage.clone().unwrap_or_else(|| "unknown".to_string());
            stage_map.entry(stage).or_default().push(days);
        }
    }

    let stats: Vec<StageDurationStats> = stage_map
        .into_iter()
        .map(|(stage, durations)| {
            let avg = durations.iter().sum::<f64>() / durations.len() as f64;
            let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            StageDurationStats {
                stage,
                avg_duration_days: avg,
                min_duration_days: min,
                max_duration_days: max,
                opportunity_count: durations.len() as i64,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(stats)))
}
