//! CRM 客户关系管理域路由
//!
//! 处理客户、客户信用、五维分析、销售分析、CRM 客户/标签/公海池/分配/回收规则、
//! CRM 线索/商机/客户 360/跟进/RFM 等客户关系管理相关接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    crm_assignment_handler, crm_customer_handler, crm_pool_handler, customer_credit_handler,
    customer_handler, five_dimension_handler, missing_handlers, sales_analysis_handler,
};

/// 客户管理路由（nest 到 /api/v1/erp/customers）
pub fn customers() -> Router<AppState> {
    Router::new()
        .route("/", get(customer_handler::list_customers))
        .route("/", post(customer_handler::create_customer))
        .route("/select", get(customer_handler::list_customers))
        .route("/:id", get(customer_handler::get_customer))
        .route("/:id", put(customer_handler::update_customer))
        .route("/:id", delete(customer_handler::delete_customer))
        .route("/:id/credit", get(customer_credit_handler::get_credit))
}

/// 客户信用路由（nest 到 /api/v1/erp/customer-credits）
pub fn customer_credits() -> Router<AppState> {
    Router::new()
        .route("/", get(customer_credit_handler::list_credits))
        .route("/", post(customer_credit_handler::create_credit))
        .route("/:id", get(customer_credit_handler::get_credit))
        .route("/:id", put(customer_credit_handler::update_credit))
        .route("/:id", delete(customer_credit_handler::delete_credit))
        .route(
            "/:id/rating",
            post(customer_credit_handler::set_credit_rating),
        )
        .route("/:id/occupy", post(customer_credit_handler::occupy_credit))
        .route(
            "/:id/release",
            post(customer_credit_handler::release_credit),
        )
        .route(
            "/:id/adjust",
            post(customer_credit_handler::adjust_credit_limit),
        )
        .route(
            "/:id/deactivate",
            post(customer_credit_handler::deactivate_credit),
        )
        .route("/evaluate", post(customer_credit_handler::evaluate_credit))
}

/// 五维管理路由（nest 到 /api/v1/erp/five-dimension）
pub fn five_dimension() -> Router<AppState> {
    Router::new()
        .route(
            "/stats",
            get(five_dimension_handler::get_five_dimension_stats),
        )
        .route(
            "/list",
            get(five_dimension_handler::list_five_dimension_stats),
        )
        .route(
            "/search",
            get(five_dimension_handler::search_five_dimension),
        )
        .route(
            "/:five_dimension_id",
            get(five_dimension_handler::get_stats_by_five_dimension_id),
        )
        .route(
            "/parse",
            post(five_dimension_handler::parse_five_dimension_id),
        )
        .route(
            "/summary",
            get(five_dimension_handler::get_five_dimension_summary),
        )
}

/// 销售分析路由（nest 到 /api/v1/erp/sales-analysis）
pub fn sales_analysis() -> Router<AppState> {
    Router::new()
        .route("/statistics", get(sales_analysis_handler::list_statistics))
        .route("/trends", get(sales_analysis_handler::get_trends))
        .route("/rankings", get(sales_analysis_handler::get_rankings))
        .route("/stats", get(sales_analysis_handler::get_stats))
        .route(
            "/product-ranking",
            get(sales_analysis_handler::get_product_ranking),
        )
        .route(
            "/customer-ranking",
            get(sales_analysis_handler::get_customer_ranking),
        )
        .route("/trend", get(sales_analysis_handler::get_trends))
        .route("/export", get(sales_analysis_handler::export_analysis))
        .route("/targets", get(sales_analysis_handler::get_targets))
        .route("/targets", post(sales_analysis_handler::create_target))
        .route(
            "/targets/:period",
            put(sales_analysis_handler::update_sales_target),
        )
}

/// CRM 客户增强路由（nest 到 /api/v1/erp/crm/customers）
pub fn crm_customers() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(crm_customer_handler::list_customers).post(crm_customer_handler::create_customer),
        )
        .route(
            "/enhanced",
            get(crm_customer_handler::list_customers).post(crm_customer_handler::create_customer),
        )
        .route(
            "/:id",
            get(crm_customer_handler::get_customer)
                .put(crm_customer_handler::update_customer)
                .delete(crm_customer_handler::delete_customer),
        )
        .route("/:id/tags", post(crm_customer_handler::add_tags))
        .route("/:id/contacts", get(crm_customer_handler::list_contacts))
}

/// CRM 标签路由（nest 到 /api/v1/erp/crm/tags）
pub fn crm_tags() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(crm_customer_handler::list_tags).post(crm_customer_handler::create_tag),
        )
        .route("/:id", delete(crm_customer_handler::delete_tag))
}

/// CRM 公海池路由（nest 到 /api/v1/erp/crm/pool）
pub fn crm_pool() -> Router<AppState> {
    Router::new()
        .route("/", get(crm_pool_handler::list_pool))
        .route("/claim", post(crm_pool_handler::claim_from_pool))
        .route("/recycle", post(crm_pool_handler::recycle_to_pool))
        .route("/batch-claim", post(crm_pool_handler::batch_claim))
        .route(
            "/:customer_id/claim",
            post(crm_pool_handler::claim_specific),
        )
}

/// CRM 分配路由（nest 到 /api/v1/erp/crm/assignments）
pub fn crm_assignments() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(crm_assignment_handler::list_assignments)
                .post(crm_assignment_handler::assign_customer),
        )
        .route("/batch", post(crm_assignment_handler::batch_assign))
        .route(
            "/history",
            get(crm_assignment_handler::list_assignment_history),
        )
}

/// CRM 销售用户路由（/api/v1/erp/crm/sales-users）
pub fn crm_sales_users() -> Router<AppState> {
    Router::new().route("/", get(missing_handlers::get_sales_users))
}

/// CRM 回收规则路由（nest 到 /api/v1/erp/crm/recycle-rules）
pub fn crm_recycle_rules() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(missing_handlers::get_recycle_rules).post(missing_handlers::create_recycle_rule),
        )
        .route(
            "/:id",
            put(missing_handlers::update_recycle_rule)
                .delete(missing_handlers::delete_recycle_rule),
        )
}

/// CRM 业务路由（线索/商机/客户 360/跟进/RFM，nest 到 /api/v1/erp/crm）
pub fn crm_business() -> Router<AppState> {
    Router::new()
        .route(
            "/leads",
            post(crate::handlers::crm_handler::create_lead)
                .get(crate::handlers::crm_handler::list_leads),
        )
        .route(
            "/leads/:id",
            get(crate::handlers::crm_handler::get_lead)
                .put(crate::handlers::crm_handler::update_lead)
                .delete(crate::handlers::crm_handler::delete_lead),
        )
        .route(
            "/leads/:id/status",
            put(crate::handlers::crm_handler::update_lead_status),
        )
        .route(
            "/leads/:id/convert",
            post(crate::handlers::crm_handler::convert_lead),
        )
        .route(
            "/leads/:id/relations",
            get(crate::handlers::crm_handler::get_lead_relation),
        )
        .route(
            "/opportunities",
            post(crate::handlers::crm_handler::create_opportunity)
                .get(crate::handlers::crm_handler::list_opportunities),
        )
        .route(
            "/opportunities/:id",
            get(crate::handlers::crm_handler::get_opportunity)
                .put(crate::handlers::crm_handler::update_opportunity)
                .delete(crate::handlers::crm_handler::delete_opportunity),
        )
        .route(
            "/opportunities/:id/convert",
            post(crate::handlers::crm_handler::convert_opportunity_to_order),
        )
        .route(
            "/customers/:id/summary",
            get(crate::handlers::crm_handler::get_customer_relation_summary),
        )
        .route(
            "/customers/:id/360",
            get(crate::handlers::crm_handler::get_customer_360),
        )
        .route(
            "/customers/:id/follow-ups",
            get(crate::handlers::crm_handler::list_follow_ups)
                .post(crate::handlers::crm_handler::create_follow_up),
        )
        .route(
            "/customers/:id/rfm",
            get(crate::handlers::crm_handler::get_rfm_score),
        )
        .route(
            "/rfm/distribution",
            get(crate::handlers::crm_handler::get_rfm_distribution),
        )
        .route(
            "/customers/enhanced/:id",
            get(crate::handlers::crm_handler::get_customer_enhanced_detail)
                .put(crate::handlers::crm_handler::update_customer_enhanced)
                .delete(crate::handlers::crm_handler::delete_customer_enhanced),
        )
}

/// CRM 域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(customers())
        .merge(customer_credits())
        .merge(five_dimension())
        .merge(sales_analysis())
        .merge(crm_customers())
        .merge(crm_tags())
        .merge(crm_pool())
        .merge(crm_assignments())
        .merge(crm_sales_users())
        .merge(crm_recycle_rules())
        .merge(crm_business())
}
