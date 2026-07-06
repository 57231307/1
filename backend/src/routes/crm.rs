//! CRM 客户关系管理域路由
//!
//! 处理客户、客户信用、五维分析、销售分析、CRM 客户/标签/公海池/分配/回收规则、
//! CRM 线索/商机/客户 360/跟进/RFM 等客户关系管理相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/customers`、`/customer-credits`、`/five-dimension`、`/sales-analysis`、
//!  `/customers/enhanced`、`/crm/tags`、`/pool`、`/assignments`、
//!  `/sales-users`、`/recycle-rules`、`/leads`、`/opportunities` 等），
//!  这样 `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//!  避免 axum 0.7 `Overlapping method route` panic。
//!
//! 重要：原本 `crm_customers()` 内部注册了 `GET /customers`、`GET /customers/:id`
//! 等基础路径，与 `customers()` 完全冲突。本次重构把 `crm_customers()` 改为
//! 只暴露 CRM 增强版特有的子路径（`/customers/enhanced`、`/customers/:id/tags` 等），
//! 基础 CRUD 由 `customers()` 统一提供。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    crm_assignment_handler, crm_customer_handler, crm_pool_handler, customer_credit_handler,
    customer_handler, five_dimension_handler, missing_handlers, sales_analysis_handler,
};

/// 客户管理路由（path 前缀 /customers）
pub fn customers() -> Router<AppState> {
    Router::new()
        .route("/customers", get(customer_handler::list_customers))
        .route("/customers", post(customer_handler::create_customer))
        .route("/customers/select", get(customer_handler::list_customers))
        .route("/customers/:id", get(customer_handler::get_customer))
        .route("/customers/:id", put(customer_handler::update_customer))
        .route("/customers/:id", delete(customer_handler::delete_customer))
        .route(
            "/customers/:id/credit",
            get(customer_credit_handler::get_credit),
        )
}

/// 客户信用路由（path 前缀 /customer-credits）
pub fn customer_credits() -> Router<AppState> {
    Router::new()
        .route(
            "/customer-credits",
            get(customer_credit_handler::list_credits),
        )
        .route(
            "/customer-credits",
            post(customer_credit_handler::create_credit),
        )
        .route(
            "/customer-credits/:id",
            get(customer_credit_handler::get_credit),
        )
        .route(
            "/customer-credits/:id",
            put(customer_credit_handler::update_credit),
        )
        .route(
            "/customer-credits/:id",
            delete(customer_credit_handler::delete_credit),
        )
        .route(
            "/customer-credits/:id/rating",
            post(customer_credit_handler::set_credit_rating),
        )
        .route(
            "/customer-credits/:id/occupy",
            post(customer_credit_handler::occupy_credit),
        )
        .route(
            "/customer-credits/:id/release",
            post(customer_credit_handler::release_credit),
        )
        .route(
            "/customer-credits/:id/adjust",
            post(customer_credit_handler::adjust_credit_limit),
        )
        .route(
            "/customer-credits/:id/deactivate",
            post(customer_credit_handler::deactivate_credit),
        )
        .route(
            "/customer-credits/evaluate",
            post(customer_credit_handler::evaluate_credit),
        )
}

/// 五维管理路由（path 前缀 /five-dimension）
pub fn five_dimension() -> Router<AppState> {
    Router::new()
        .route(
            "/five-dimension/stats",
            get(five_dimension_handler::get_five_dimension_stats),
        )
        .route(
            "/five-dimension/list",
            get(five_dimension_handler::list_five_dimension_stats),
        )
        .route(
            "/five-dimension/search",
            get(five_dimension_handler::search_five_dimension),
        )
        .route(
            "/five-dimension/:five_dimension_id",
            get(five_dimension_handler::get_stats_by_five_dimension_id),
        )
        .route(
            "/five-dimension/parse",
            post(five_dimension_handler::parse_five_dimension_id),
        )
        .route(
            "/five-dimension/summary",
            get(five_dimension_handler::get_five_dimension_summary),
        )
}

/// 销售分析路由（path 前缀 /sales-analysis）
pub fn sales_analysis() -> Router<AppState> {
    Router::new()
        .route(
            "/sales-analysis/statistics",
            get(sales_analysis_handler::list_statistics),
        )
        .route(
            "/sales-analysis/trends",
            get(sales_analysis_handler::get_trends),
        )
        .route(
            "/sales-analysis/rankings",
            get(sales_analysis_handler::get_rankings),
        )
        .route(
            "/sales-analysis/stats",
            get(sales_analysis_handler::get_stats),
        )
        .route(
            "/sales-analysis/product-ranking",
            get(sales_analysis_handler::get_product_ranking),
        )
        .route(
            "/sales-analysis/customer-ranking",
            get(sales_analysis_handler::get_customer_ranking),
        )
        .route(
            "/sales-analysis/trend",
            get(sales_analysis_handler::get_trends),
        )
        .route(
            "/sales-analysis/export",
            get(sales_analysis_handler::export_analysis),
        )
        .route(
            "/sales-analysis/targets",
            get(sales_analysis_handler::get_targets),
        )
        .route(
            "/sales-analysis/targets",
            post(sales_analysis_handler::create_target),
        )
        .route(
            "/sales-analysis/targets/:period",
            put(sales_analysis_handler::update_sales_target),
        )
}

/// CRM 客户增强路由
///
/// 仅暴露 CRM 增强版特有子路径（`/customers/enhanced`、`/customers/:id/tags`、
/// `/customers/:id/contacts`），基础 CRUD 由 [`customers`] 提供。
pub fn crm_customers() -> Router<AppState> {
    Router::new()
        .route(
            "/customers/enhanced",
            get(crm_customer_handler::list_customers).post(crm_customer_handler::create_customer),
        )
        .route(
            "/customers/enhanced/:id",
            get(crm_customer_handler::get_customer)
                .put(crm_customer_handler::update_customer)
                .delete(crm_customer_handler::delete_customer),
        )
        .route("/customers/:id/tags", post(crm_customer_handler::add_tags))
        // 批次 90b P2-12：联系人 CRUD（GET 既有，新增 POST/PUT/DELETE）
        .route(
            "/customers/:id/contacts",
            get(crm_customer_handler::list_contacts).post(crm_customer_handler::create_contact),
        )
        .route(
            "/customers/:id/contacts/:contact_id",
            put(crm_customer_handler::update_contact).delete(crm_customer_handler::delete_contact),
        )
}

/// CRM 标签路由（path 前缀 /crm/tags）
///
/// 批次 122 v8 复审 P1 修复：原路径 `/crm-tags` 与前端调用 `/crm/tags` 不一致导致 404。
/// 现统一为 `/crm/tags` 和 `/crm/tags/:id`，与前端 crm-enhanced.ts 调用路径匹配。
pub fn crm_tags() -> Router<AppState> {
    Router::new()
        .route(
            "/crm/tags",
            get(crm_customer_handler::list_tags).post(crm_customer_handler::create_tag),
        )
        .route("/crm/tags/:id", delete(crm_customer_handler::delete_tag))
}

/// CRM 公海池路由（path 前缀 /pool）
pub fn crm_pool() -> Router<AppState> {
    Router::new()
        .route("/pool", get(crm_pool_handler::list_pool))
        .route("/pool/claim", post(crm_pool_handler::claim_from_pool))
        .route("/pool/recycle", post(crm_pool_handler::recycle_to_pool))
        .route("/pool/batch-claim", post(crm_pool_handler::batch_claim))
        .route(
            "/pool/:customer_id/claim",
            post(crm_pool_handler::claim_specific),
        )
}

/// CRM 分配路由（path 前缀 /assignments）
pub fn crm_assignments() -> Router<AppState> {
    Router::new()
        .route(
            "/assignments",
            get(crm_assignment_handler::list_assignments)
                .post(crm_assignment_handler::assign_customer),
        )
        .route(
            "/assignments/batch",
            post(crm_assignment_handler::batch_assign),
        )
        .route(
            "/assignments/history",
            get(crm_assignment_handler::list_assignment_history),
        )
        // v10 P1 批次 140：assign 模块"保留扩展空间"功能真实接入
        .route(
            "/assignments/auto-assign",
            post(crm_assignment_handler::auto_assign),
        )
        .route(
            "/assignments/transfer",
            post(crm_assignment_handler::transfer_lead),
        )
        .route(
            "/assignments/claim",
            post(crm_assignment_handler::claim_lead),
        )
        .route(
            "/assignments/workload",
            get(crm_assignment_handler::list_workload),
        )
}

/// CRM 销售用户路由（path 前缀 /sales-users）
pub fn crm_sales_users() -> Router<AppState> {
    Router::new().route("/sales-users", get(missing_handlers::get_sales_users))
}

/// CRM 回收规则路由（path 前缀 /recycle-rules）
pub fn crm_recycle_rules() -> Router<AppState> {
    Router::new()
        .route(
            "/recycle-rules",
            get(missing_handlers::get_recycle_rules).post(missing_handlers::create_recycle_rule),
        )
        .route(
            "/recycle-rules/:id",
            put(missing_handlers::update_recycle_rule)
                .delete(missing_handlers::delete_recycle_rule),
        )
}

/// CRM 业务路由（线索/商机/客户 360/跟进/RFM）
///
/// 所有 path 已加 `/leads`、`/opportunities` 等子前缀。
/// 客户相关 summary/360/follow-ups/rfm 等增强接口已放在 `/customers/:id/...`
/// 路径下，由本函数与 [`customers`]、[`crm_customers`] 联合提供，避免重复。
pub fn crm_business() -> Router<AppState> {
    Router::new()
        .route(
            "/leads",
            post(crate::handlers::crm_handler::create_lead)
                .get(crate::handlers::crm_handler::list_leads),
        )
        // v11 批次 141：导出线索为 CSV（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/leads/export",
            get(crate::handlers::crm_handler::export_leads),
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
        // v11 批次 141：导出商机为 CSV（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/opportunities/export",
            get(crate::handlers::crm_handler::export_opportunities),
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
    // `/customers/enhanced/:id` 的 CRUD 已经在 [`crm_customers`] 中提供，
    // 这里不再重复注册，避免 path+method 冲突。
}

/// CRM 域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
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
