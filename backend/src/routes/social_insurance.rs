//! 社保公积金域路由（V15 P1 batch-08 缺陷 23）
//!
//! 社保公积金扣缴 RESTful 接口，path 前缀 /social-insurance。
//! 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条

use crate::container::AppState;
use crate::handlers::social_insurance_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 社保公积金路由（path 前缀 /social-insurance）
///
/// 注意：静态子路径（如 /by-worker/:worker_id）需在 /:id 之前注册，
/// 避免 axum matchit 把静态段误当 :id 匹配。
pub fn social_insurance() -> Router<AppState> {
    Router::new()
        .route("/social-insurance", post(social_insurance_handler::create))
        .route("/social-insurance", get(social_insurance_handler::list))
        .route(
            "/social-insurance/by-worker/:worker_id",
            get(social_insurance_handler::get_by_worker_period),
        )
        .route(
            "/social-insurance/:id",
            get(social_insurance_handler::get_by_id),
        )
        .route(
            "/social-insurance/:id/mark-paid",
            post(social_insurance_handler::mark_paid),
        )
        .route(
            "/social-insurance/:id/cancel",
            post(social_insurance_handler::cancel),
        )
        // 打印路由
        .route(
            "/social-insurance/:id/print",
            get(crate::handlers::print_handler::social_insurance_record_print_docx),
        )
}

/// 社保公积金域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(social_insurance())
}
