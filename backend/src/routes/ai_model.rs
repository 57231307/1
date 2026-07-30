//! AI 模型管理 + 质量核对域路由

use crate::container::AppState;
use crate::handlers::ai_model_management_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// AI 模型管理路由（path 前缀 /ai-models）
pub fn ai_models() -> Router<AppState> {
    Router::new()
        .route(
            "/ai-models/versions",
            post(ai_model_management_handler::create_model_version),
        )
        .route(
            "/ai-models/versions",
            get(ai_model_management_handler::list_model_versions),
        )
        .route(
            "/ai-models/versions/active/:model_name",
            get(ai_model_management_handler::get_active_model_version),
        )
        .route(
            "/ai-models/versions/:version_id/approve",
            post(ai_model_management_handler::approve_model_version),
        )
        .route(
            "/ai-models/versions/:version_id/status",
            post(ai_model_management_handler::change_model_status),
        )
        .route(
            "/ai-models/evaluations",
            post(ai_model_management_handler::create_model_evaluation),
        )
        .route(
            "/ai-models/evaluations/:model_version_id",
            get(ai_model_management_handler::list_model_evaluations),
        )
        .route(
            "/ai-models/evaluations/:model_version_id/drift",
            get(ai_model_management_handler::detect_model_drift),
        )
        .route(
            "/ai-models/decisions",
            post(ai_model_management_handler::log_decision),
        )
        .route(
            "/ai-models/decisions",
            get(ai_model_management_handler::list_decision_logs),
        )
        .route(
            "/ai-models/reconcile",
            get(ai_model_management_handler::reconcile_monthly),
        )
        .route(
            "/ai-models/accuracy-reports",
            get(ai_model_management_handler::list_accuracy_reports),
        )
}

/// AI 域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(ai_models())
}
