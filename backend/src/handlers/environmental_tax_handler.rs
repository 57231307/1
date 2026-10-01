//! 环保税 handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::environmental_tax_service::{
    CreateDischargeRecordRequest, EnvironmentalTaxService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    Json,
    extract::{Query, State},
};
use serde::Deserialize;

/// 构造环保税服务：适用税额（地方可变值）从进程级部署配置注入。
///
/// 未配置时 `global_env_tax_rate_per_equivalent()` 返回 `None`，计税路径会在
/// 服务层显式失败并记 warn（启动/构造阶段不报错、不取默认值，决策定案 #6）。
fn env_tax_service(state: &AppState) -> EnvironmentalTaxService {
    EnvironmentalTaxService::new(
        state.db.clone(),
        crate::config::settings::global_env_tax_rate_per_equivalent(),
    )
}

/// 查询参数：申报期间
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub period_year: i32,
    pub period_month: i32,
}

/// 创建污染物排放记录（自动计算环保税）
pub async fn create_discharge_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(mut req): Json<CreateDischargeRecordRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.created_by = Some(auth.user_id);
    let service = env_tax_service(&state);
    let model = service.create_discharge_record(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 按期间查询污染物排放记录
pub async fn list_discharge_records(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = env_tax_service(&state);
    let list = service
        .list_by_period(params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 生成环保税申报表（按期间汇总）
pub async fn generate_tax_declaration(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = env_tax_service(&state);
    let result = service
        .generate_tax_declaration(params.period_year, params.period_month)
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}
