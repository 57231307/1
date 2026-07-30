//! 角色关系域路由（V15 P1 12.2）
//!
//! 角色继承与互斥校验 RESTful 接口，path 前缀 /role-relations。

use crate::container::AppState;
use crate::handlers::role_relation_handler;
use axum::{
    routing::{delete, get, post},
    Router,
};

/// 角色关系路由（path 前缀 /role-relations）
pub fn role_relation() -> Router<AppState> {
    Router::new()
        .route(
            "/role-relations",
            post(role_relation_handler::create_relation),
        )
        .route(
            "/role-relations",
            get(role_relation_handler::list_relations),
        )
        .route(
            "/role-relations/:relation_id",
            delete(role_relation_handler::delete_relation),
        )
        .route(
            "/role-relations/between/:role_a_code/:role_b_code",
            get(role_relation_handler::get_relation_between),
        )
        .route(
            "/role-relations/inherited/:role_code",
            get(role_relation_handler::get_inherited_role_codes),
        )
        .route(
            "/role-relations/check-mutual-exclusive/:role_code",
            post(role_relation_handler::check_mutual_exclusive),
        )
}

/// 角色关系域统一入口
pub fn routes() -> Router<AppState> {
    Router::new().merge(role_relation())
}
