use axum::{extract::State, Json};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::crm_lead;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 转化耗时统计
#[derive(Serialize)]
pub struct ConversionTimeStats {
    /// 平均转化时间（天）
    pub avg_conversion_days: f64,
    /// 最短转化时间（天）
    pub min_conversion_days: f64,
    /// 最长转化时间（天）
    pub max_conversion_days: f64,
    /// 已转化线索数
    pub converted_count: i64,
    /// 未转化线索数
    pub unconverted_count: i64,
}

/// GET /api/v1/erp/crm/leads/conversion-stats - 转化耗时分析
/// batch-15 P3: 转化耗时分析
pub async fn get_conversion_time_stats(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<ConversionTimeStats>>, AppError> {
    // 查询已转化的线索
    let converted_leads = cpm_lead::Entity::find()
        .filter(crm_lead::Column::ConvertedAt.is_not_null())
        .filter(crm_lead::Column::CreatedAt.is_not_null())
        .all(&*state.db)
        .await?;

    let converted_count = converted_leads.len() as i64;

    if converted_count == 0 {
        return Ok(Json(ApiResponse::success(ConversionTimeStats {
            avg_conversion_days: 0.0,
            min_conversion_days: 0.0,
            max_conversion_days: 0.0,
            converted_count: 0,
            unconverted_count: 0,
        })));
    }

    // 计算转化时间
    let mut conversion_days: Vec<f64> = Vec::new();
    for lead in &converted_leads {
        if let (Some(created), Some(converted)) = (lead.created_at, lead.converted_at) {
            let days = (converted - created).num_days() as f64;
            conversion_days.push(days);
        }
    }

    let avg_conversion_days = if conversion_days.is_empty() {
        0.0
    } else {
        conversion_days.iter().sum::<f64>() / conversion_days.len() as f64
    };

    let min_conversion_days = conversion_days
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max_conversion_days = conversion_days
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // 查询未转化线索数
    let unconverted_count = crm_lead::Entity::find()
        .filter(crm_lead::Column::ConvertedAt.is_null())
        .count(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(ConversionTimeStats {
        avg_conversion_days,
        min_conversion_days,
        max_conversion_days,
        converted_count,
        unconverted_count,
    })))
}
