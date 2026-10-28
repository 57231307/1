//! 纺织品安全合规域路由（送检单、第三方报告、主数据三张资源面）
//!
//! 挂载风格与出口商检同型：seg3 即权限资源名（直接资源），登记于
//! utils/path_utils.rs 直接资源白名单与 init_service 的 PERMISSION_RESOURCES；
//! 未注册段会被权限中间件 fail-closed 403（含 admin）。
//! 状态推进、判定与完整性门的判据在 service 层，路由面不承载业务校验。
use crate::container::AppState;
use crate::handlers::{
    textile_safety_handler, textile_safety_master_handler, textile_safety_report_handler,
};
use axum::{
    Router,
    routing::{delete, get, post, put},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/textile-submissions", submission_routes())
        .nest("/textile-reports", report_routes())
        .nest("/textile-test-items", test_item_routes())
        .nest("/textile-limit-rules", limit_rule_routes())
        .nest("/textile-export-requirements", export_requirement_routes())
}

/// 送检单面：建单、列表、详情、要素修改、草稿删除、状态机、清单推导、童装核查、整单判定。
fn submission_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(textile_safety_handler::list_submissions))
        .route("/", post(textile_safety_handler::create_submission))
        .route("/{id}", get(textile_safety_handler::get_submission))
        .route("/{id}", put(textile_safety_handler::update_submission))
        .route("/{id}", delete(textile_safety_handler::delete_submission))
        // 状态机白名单推进（撤回也走本面）
        .route(
            "/{id}/status",
            put(textile_safety_handler::set_submission_status),
        )
        // 项目清单推导（POST 建集，重生成幂等）
        .route(
            "/{id}/items",
            post(textile_safety_handler::generate_items),
        )
        // 童装加考核查登记（upsert 语义走 PUT 集合面）
        .route(
            "/{id}/child-checks",
            put(textile_safety_handler::upsert_child_checks),
        )
        // 整单判定（judged 态唯一产出口）
        .route(
            "/{id}/judgement",
            put(textile_safety_handler::judge_submission),
        )
}

/// 第三方报告面：登记、修正、完整性门、结果录入、报告级判定。
fn report_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(textile_safety_report_handler::list_reports))
        .route("/", post(textile_safety_report_handler::create_report))
        .route("/{id}", get(textile_safety_report_handler::get_report))
        .route("/{id}", put(textile_safety_report_handler::update_report))
        // 完整性门：accepted 与 rejected 只能由本面产出
        .route(
            "/{id}/acceptance",
            put(textile_safety_report_handler::accept_report),
        )
        // 逐项目实测结果 upsert（结构化承载值、单位、方法）
        .route(
            "/{id}/results",
            post(textile_safety_report_handler::upsert_results),
        )
        .route(
            "/{id}/judgement",
            put(textile_safety_report_handler::judge_report),
        )
}

/// 检测项目主数据面（判定链第一维）。
fn test_item_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(textile_safety_master_handler::list_test_items))
        .route("/", post(textile_safety_master_handler::create_test_item))
        .route(
            "/{id}",
            put(textile_safety_master_handler::update_test_item),
        )
        .route(
            "/{id}",
            delete(textile_safety_master_handler::delete_test_item),
        )
}

/// 限值判定表面（四维矩阵的可维护通道）。
fn limit_rule_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(textile_safety_master_handler::list_limit_rules))
        .route("/", post(textile_safety_master_handler::create_limit_rule))
        .route(
            "/{id}",
            put(textile_safety_master_handler::update_limit_rule),
        )
        .route(
            "/{id}",
            delete(textile_safety_master_handler::delete_limit_rule),
        )
}

/// 出口市场要求登记面（报价与订单侧只读消费 GET，写权归质量岗）。
fn export_requirement_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(textile_safety_master_handler::list_export_requirements),
        )
        .route(
            "/",
            post(textile_safety_master_handler::create_export_requirement),
        )
        .route(
            "/{id}",
            put(textile_safety_master_handler::update_export_requirement),
        )
        .route(
            "/{id}",
            delete(textile_safety_master_handler::delete_export_requirement),
        )
}
