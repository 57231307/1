//! 客户团队协作与数据共享域路由

use crate::container::AppState;
use crate::handlers::customer_team_share_handler;
use axum::{
    routing::{get, post},
    Router,
};

/// 团队成员路由（path 前缀 /customer-team-members）
pub fn team_members() -> Router<AppState> {
    Router::new()
        .route(
            "/customer-team-members",
            post(customer_team_share_handler::add_team_member),
        )
        .route(
            "/customer-team-members/:member_id",
            post(customer_team_share_handler::remove_team_member),
        )
        .route(
            "/customer-team-members/by-customer/:customer_id",
            get(customer_team_share_handler::list_team_members),
        )
        .route(
            "/customer-team-members/by-user",
            get(customer_team_share_handler::list_user_teams),
        )
        .route(
            "/customer-team-members/check",
            get(customer_team_share_handler::is_team_member),
        )
}

/// 客户共享路由（path 前缀 /customer-shares）
pub fn shares() -> Router<AppState> {
    Router::new()
        .route(
            "/customer-shares",
            post(customer_team_share_handler::share_customer),
        )
        .route(
            "/customer-shares/revoke",
            post(customer_team_share_handler::revoke_share),
        )
        .route(
            "/customer-shares/by-customer",
            get(customer_team_share_handler::list_customer_shares),
        )
        .route(
            "/customer-shares/by-user",
            get(customer_team_share_handler::list_user_shares),
        )
        .route(
            "/customer-shares/check",
            get(customer_team_share_handler::check_share_permission),
        )
        .route(
            "/customer-shares/expire-overdue",
            post(customer_team_share_handler::expire_overdue_shares),
        )
}

/// 客户团队共享域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(team_members()).merge(shares())
}
