//! AI 模型管理 handler（AI 模型版本 + 评估 + 决策日志 + 质量核对）

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::ai_model_management_service::{
    AiModelManagementService, AiQualityReconciliationService, ApproveModelVersionRequest,
    ChangeModelStatusRequest, CreateDecisionLogRequest, CreateModelEvaluationRequest,
    CreateModelVersionRequest, DecisionLogQuery,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

/// 查询参数：模型名称
#[derive(Debug, Deserialize)]
pub struct ModelNameQuery {
    pub model_name: Option<String>,
}

/// 查询参数：报告周期
#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    pub report_period: String,
}

/// 查询参数：limit
#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u64>,
}

/// 创建模型版本
pub async fn create_model_version(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateModelVersionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let model = service.create_model_version(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 列出模型版本
pub async fn list_model_versions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ModelNameQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let list = service.list_model_versions(params.model_name).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 获取活跃模型版本
pub async fn get_active_model_version(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(model_name): Path<String>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let model = service.get_active_model_version(&model_name).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 审批模型版本
pub async fn approve_model_version(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(version_id): Path<i32>,
    Json(req): Json<ApproveModelVersionRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let model = service.approve_model_version(version_id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 修改模型状态
pub async fn change_model_status(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(version_id): Path<i32>,
    Json(req): Json<ChangeModelStatusRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let model = service.change_model_status(version_id, req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 创建模型评估
pub async fn create_model_evaluation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateModelEvaluationRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let eval = service.create_model_evaluation(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(eval)?)))
}

/// 列出模型评估
pub async fn list_model_evaluations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(model_version_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let list = service.list_model_evaluations(model_version_id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}

/// 检测模型漂移
pub async fn detect_model_drift(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(model_version_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let (has_drift, expected, actual, score) = service.detect_model_drift(model_version_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "has_drift": has_drift,
        "expected_accuracy": expected,
        "actual_accuracy": actual,
        "drift_score": score,
    }))))
}

/// 记录 AI 决策日志
pub async fn log_decision(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateDecisionLogRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let log = service.log_decision(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(log)?)))
}

/// 列出决策日志
pub async fn list_decision_logs(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<DecisionLogQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiModelManagementService::new(state.db.clone());
    let vo = service.list_decision_logs(q).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(vo)?)))
}

/// AI 质量月度对账
pub async fn reconcile_monthly(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<PeriodQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiQualityReconciliationService::new(state.db.clone());
    let result = service.reconcile_monthly(params.report_period).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// 列出准确率报告
pub async fn list_accuracy_reports(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<LimitQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = AiQualityReconciliationService::new(state.db.clone());
    let list = service
        .list_accuracy_reports(params.limit.unwrap_or(30))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(list)?)))
}
