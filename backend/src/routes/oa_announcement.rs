//! OA 公告路由（P0-D17 / Batch 488）
//!
//! 7 端点（5 基础 CRUD + 2 状态转换）：
//! - GET    /                  列表（支持 status/announcement_type/is_top 过滤）
//! - POST   /                  创建（默认 DRAFT 状态）
//! - GET    /:id               详情
//! - PUT    /:id               更新（DRAFT 全字段，PUBLISHED/ARCHIVED 仅 expiry_date/remarks/is_top）
//! - DELETE /:id               删除（仅 DRAFT 可硬删除）
//! - POST   /:id/publish       发布（DRAFT → PUBLISHED）
//! - POST   /:id/archive       归档（PUBLISHED → ARCHIVED）
//!
//! 路由注册顺序：静态路径（publish/archive）在 /:id 之前会冲突，
//! axum 路由树允许 /:id 与 /:id/publish 共存（前者是单段，后者是两段），
//! 因此无须特殊顺序。

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::oa_announcement_handler;
use crate::utils::app_state::AppState;

/// OA 公告路由（nest 到 /api/v1/erp/oa-announcements）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(oa_announcement_handler::list))
        .route("/", post(oa_announcement_handler::create))
        .route("/:id", get(oa_announcement_handler::get))
        .route("/:id", put(oa_announcement_handler::update))
        .route("/:id", delete(oa_announcement_handler::delete))
        .route("/:id/publish", post(oa_announcement_handler::publish))
        .route("/:id/archive", post(oa_announcement_handler::archive))
}
