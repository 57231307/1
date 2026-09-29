//! AI 模型管理 + 质量核对域路由

use crate::container::AppState;
use crate::handlers::ai_model_management_handler;
use axum::{
    Router,
    routing::{get, post},
};

/// AI 模型管理路由（挂载于 nest 前缀 /api/v1/erp/ai-models 之下，路由为相对路径）
pub fn ai_models() -> Router<AppState> {
    Router::new()
        .route(
            "/versions",
            post(ai_model_management_handler::create_model_version),
        )
        .route(
            "/versions",
            get(ai_model_management_handler::list_model_versions),
        )
        .route(
            "/versions/active/{model_name}",
            get(ai_model_management_handler::get_active_model_version),
        )
        .route(
            "/versions/{version_id}/approve",
            post(ai_model_management_handler::approve_model_version),
        )
        .route(
            "/versions/{version_id}/status",
            post(ai_model_management_handler::change_model_status),
        )
        .route(
            "/evaluations",
            post(ai_model_management_handler::create_model_evaluation),
        )
        .route(
            "/evaluations/{model_version_id}",
            get(ai_model_management_handler::list_model_evaluations),
        )
        .route(
            "/evaluations/{model_version_id}/drift",
            get(ai_model_management_handler::detect_model_drift),
        )
        .route(
            "/decisions",
            post(ai_model_management_handler::log_decision),
        )
        .route(
            "/decisions",
            get(ai_model_management_handler::list_decision_logs),
        )
        .route(
            "/reconcile",
            post(ai_model_management_handler::reconcile_monthly),
        )
        .route(
            "/accuracy-reports",
            get(ai_model_management_handler::list_accuracy_reports),
        )
}

/// AI 域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(ai_models())
}
