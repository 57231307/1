//! 职业健康合规域路由

use crate::container::AppState;
use crate::handlers::occupational_health_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 职业危害因素检测记录路由（path 前缀 /occupational-health/hazard-monitorings）
pub fn hazard_monitorings() -> Router<AppState> {
    Router::new()
        .route(
            "/occupational-health/hazard-monitorings",
            post(occupational_health_handler::create_hazard_monitoring),
        )
        .route(
            "/occupational-health/hazard-monitorings",
            get(occupational_health_handler::list_hazard_monitorings),
        )
}

/// 职业健康体检档案路由（path 前缀 /occupational-health/health-exams）
pub fn health_exams() -> Router<AppState> {
    Router::new()
        .route(
            "/occupational-health/health-exams",
            post(occupational_health_handler::create_health_exam),
        )
        .route(
            "/occupational-health/health-exams",
            get(occupational_health_handler::list_health_exams),
        )
        .route(
            "/occupational-health/health-exams/scan-expiry-warnings",
            post(occupational_health_handler::scan_exam_expiry_warnings),
        )
}

/// PPE 发放记录路由（path 前缀 /occupational-health/ppe-distributions）
pub fn ppe_distributions() -> Router<AppState> {
    Router::new()
        .route(
            "/occupational-health/ppe-distributions",
            post(occupational_health_handler::create_ppe_distribution),
        )
        .route(
            "/occupational-health/ppe-distributions",
            get(occupational_health_handler::list_ppe_distributions),
        )
        .route(
            "/occupational-health/ppe-distributions/scan-expired",
            post(occupational_health_handler::scan_expired_ppe),
        )
        .route(
            "/occupational-health/ppe-distributions/:id/return",
            post(occupational_health_handler::return_ppe),
        )
}

/// 职业健康合规域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(hazard_monitorings())
        .merge(health_exams())
        .merge(ppe_distributions())
}
