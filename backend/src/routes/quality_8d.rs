//! 8D 质量管理流程路由（V15 P0-F20 Batch 480）
//!
//! 7 端点：
//!   GET    /                                 列表
//!   POST   /                                 启动 8D 流程（not_started → d0_plan）
//!   GET    /{id}                              详情
//!   GET    /by-issue/{quality_issue_id}       按质量异常查询 8D 报告
//!   POST   /{id}/advance                      推进下一 D 阶段（8 条合法边）
//!   POST   /{id}/close                        关闭 8D 流程（d8_recognize → closed）
//!
//! 路由注册顺序：静态路径（/ 和 /by-issue/{quality_issue_id}）必须在 /{id} 之前，
//! 避免 axum matchit 把 "by-issue" 当 {id} 匹配。

use axum::{
    Router,
    routing::{get, post},
};

use crate:{container}::AppState;
use crate:{handlers}:{print_handler};
use crate:{handlers}:{quality_}8d_handler;

/// 8D 质量管理流程路由（nest 到 /api/v1/erp/quality-8d-reports）
pub fn routes() -> Router<AppState> {
    Router:{new}()
        .route(
            "/",
            get(quality_8d_handler:{list_}8d).post(quality_8d_handler:{start_}8d),
        )
        // 静态路径必须在 /{id} 之前注册，避免 axum matchit 把 "by-issue" 当 {id} 匹配
        .route(
            "/by-issue/{quality_issue_id}",
            get(quality_8d_handler:{get_by_issue}),
        )
        .route("/{id}", get(quality_8d_handler:{get_}8d))
        .route("/{id}/advance", post(quality_8d_handler:{advance}))
        .route("/{id}/close", post(quality_8d_handler:{close_}8d))
        // 打印路由
        .route("/{id}/print", get(print_handler:{quality_}8d_report_print_docx))
}
