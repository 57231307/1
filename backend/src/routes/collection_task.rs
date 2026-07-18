//! 催收任务路由（V15 P0-B03 Batch 481）
//!
//! 7 端点：
//!   - POST   /auto-generate          自动生成催收任务
//!   - POST   /                        手动创建催收任务
//!   - GET    /                        任务列表
//!   - GET    /:id                     任务详情
//!   - POST   /:id/contact             记录催收结果
//!   - POST   /:id/reassign            重新分配
//!   - POST   /:id/cancel              取消任务
//!
//! 路由注册顺序：静态路径（/ 和 /auto-generate）必须在 /:id 之前，
//! 避免 axum matchit 把 "auto-generate" 当 :id 匹配。

use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::collection_task_handler;
use crate::utils::app_state::AppState;

/// 催收任务路由（nest 到 /api/v1/erp/collection-tasks）
pub fn routes() -> Router<AppState> {
    Router::new()
        // 静态路径必须在 /:id 之前注册，避免 axum matchit 把 "auto-generate" 当 :id 匹配
        .route(
            "/auto-generate",
            post(collection_task_handler::auto_generate),
        )
        .route(
            "/",
            get(collection_task_handler::list_tasks).post(collection_task_handler::create_task),
        )
        .route("/:id", get(collection_task_handler::get_task))
        .route(
            "/:id/contact",
            post(collection_task_handler::record_contact),
        )
        .route("/:id/reassign", post(collection_task_handler::reassign))
        .route("/:id/cancel", post(collection_task_handler::cancel_task))
}
