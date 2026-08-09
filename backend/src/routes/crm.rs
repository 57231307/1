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

use crate::container::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    crm_assignment_handler, crm_customer_handler, crm_pool_handler, customer_address_handler,
    customer_credit_handler, customer_handler, customer_transfer_approval_handler,
    five_dimension_handler, missing_handlers, sales_analysis_handler,
};

/// 客户管理路由（path 前缀 /customers）
pub fn customers() -> Router<AppState> {
    Router::new()
        .route("/customers", get(customer_handler::list_customers))
        .route("/customers", post(customer_handler::create_customer))
        .route("/customers/select", get(customer_handler::list_customers))
        // V15 P0-S12 + P0-S15 新增（Batch 474）：客户列表带水印导出
        // 路由顺序：静态路径 /customers/export 必须在 /customers/:id 之前注册，
        // 避免 axum 把 "export" 当作 :id 参数匹配（axum matchit 静态优先，但仍按注册顺序保险）
        .route(
            "/customers/export",
            get(customer_handler::export_customers),
        )
        .route("/customers/:id", get(customer_handler::get_customer))
        .route("/customers/:id", put(customer_handler::update_customer))
        .route("/customers/:id", delete(customer_handler::delete_customer))
        .route(
            "/customers/:id/credit",
            get(customer_credit_handler::get_credit),
        )
        // batch-13 P3：客户多地址支持
        .route(
            "/customers/:id/addresses",
            get(customer_address_handler::list_customer_addresses),
        )
        .route(
            "/customers/:id/addresses",
            post(customer_address_handler::create_customer_address),
        )
        .route(
            "/customers/:customer_id/addresses/:address_id",
            put(customer_address_handler::update_customer_address),
        )
        .route(
            "/customers/:customer_id/addresses/:address_id",
            delete(customer_address_handler::delete_customer_address),
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
        .route(
            "/customer-credits/:id/print",
            get(crate::handlers::print_handler::customer_credit_print_docx),
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

/// CRM 客户增强路由（/customers/enhanced、/:id/tags、/:id/contacts，基础 CRUD 由 customers 提供）
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

/// CRM 标签路由（/crm/tags，与前端 crm-enhanced.ts 调用路径匹配）
/// 批次 122 v8 复审 P1 修复：原 `/crm-tags` 与前端不一致导致 404，已统一
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
        // V15 P0-S08 修复：公海规则 CRUD（保护期/领取上限/最大持有数）
        .route(
            "/pool/rules",
            get(crm_pool_handler::list_pool_rules).post(crm_pool_handler::create_pool_rule),
        )
        .route(
            "/pool/rules/:id",
            put(crm_pool_handler::update_pool_rule)
                .delete(crm_pool_handler::delete_pool_rule),
        )
}

/// V15 P0-S08 修复：客户转移审批路由（/transfer-approvals，多级审批流）
/// 流程：申请→经理审批（普通客户完成）/→总监审批（大客户二次）→可取消
pub fn crm_transfer_approvals() -> Router<AppState> {
    Router::new()
        .route(
            "/transfer-approvals",
            post(customer_transfer_approval_handler::create_approval)
                .get(customer_transfer_approval_handler::list_approvals),
        )
        .route(
            "/transfer-approvals/:id",
            get(customer_transfer_approval_handler::get_approval),
        )
        .route(
            "/transfer-approvals/:id/cancel",
            post(customer_transfer_approval_handler::cancel_approval),
        )
        .route(
            "/transfer-approvals/:id/manager-approve",
            post(customer_transfer_approval_handler::manager_approve),
        )
        .route(
            "/transfer-approvals/:id/director-approve",
            post(customer_transfer_approval_handler::director_approve),
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

/// CRM 线索路由（/leads，含 CRUD/导入导出/状态流转/转化/关联查询）
fn crm_lead_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/leads",
            post(crate::handlers::crm_handler::create_lead)
                .get(crate::handlers::crm_handler::list_leads),
        )
        // v11 批次 141：导出线索为 xlsx（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/leads/export",
            get(crate::handlers::crm_handler::export_leads),
        )
        // v11 批次 157d-4：批量导入线索（xlsx），注册在 /:id 之前避免路径参数匹配
        .route(
            "/leads/import",
            post(crate::handlers::crm_handler::import_leads),
        )
        // V15 P2 18.1-D4: 渠道 ROI 分析（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/leads/channel-roi",
            get(crate::handlers::crm_handler::get_channel_roi_report),
        )
        .route(
            "/leads/calculate-channel-roi",
            post(crate::handlers::crm_handler::calculate_channel_roi),
        )
        // V15 P2 18.1-D5: 线索分配规则（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/leads/allocation-rules",
            get(crate::handlers::crm_handler::list_allocation_rules)
                .post(crate::handlers::crm_handler::create_allocation_rule),
        )
        // V15 P2 18.1-D6: 线索培育计划（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/leads/nurture-plans",
            get(crate::handlers::crm_handler::list_nurture_plans)
                .post(crate::handlers::crm_handler::create_nurture_plan),
        )
        .route(
            "/leads/nurture-plans/:id/execute",
            post(crate::handlers::crm_handler::execute_nurture_plan),
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
        // V15 P2 18.1-D5: 自动分配线索
        .route(
            "/leads/:id/auto-assign",
            post(crate::handlers::crm_handler::auto_assign_lead),
        )
}

/// CRM 商机路由（/opportunities，含 CRUD/导出/转化/关单）
fn crm_opportunity_routes() -> Router<AppState> {
    Router::new()
        // V15 P2 18.2-D5: 阶段停留时长分析（注册在 /:id 之前避免路径参数匹配）
        .route(
            "/opportunities/stage-duration",
            get(crate::handlers::crm_handler::get_stage_duration_analysis),
        )
        .route(
            "/opportunities",
            post(crate::handlers::crm_handler::create_opportunity)
                .get(crate::handlers::crm_handler::list_opportunities),
        )
        // v11 批次 141：导出商机为 xlsx（注册在 /:id 之前避免路径参数匹配）
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
        // V15 P0-B09（Batch 482）：商机关单（输单流程），必须填写流失原因
        .route(
            "/opportunities/:id/close-lost",
            post(crate::handlers::crm_handler::close_opportunity_as_lost),
        )
        // V15 P2 18.2-D6: 商机竞争对手管理
        .route(
            "/opportunities/:id/competitors",
            get(crate::handlers::crm_handler::list_opportunity_competitors)
                .post(crate::handlers::crm_handler::add_opportunity_competitor),
        )
        // V15 P2 18.2-D7: 商机跟进记录
        .route(
            "/opportunities/:id/follow-ups",
            get(crate::handlers::crm_handler::list_opportunity_follow_ups)
                .post(crate::handlers::crm_handler::create_opportunity_follow_up),
        )
        // V15 P2 18.2-D4/D5: 商机分析与预测
        .route(
            "/opportunities/forecast-accuracy",
            get(crate::handlers::crm_handler::get_forecast_accuracy),
        )
        .route(
            "/opportunities/weighted-forecast",
            get(crate::handlers::crm_handler::get_weighted_forecast),
        )
        .route(
            "/opportunities/conversion-rate",
            get(crate::handlers::crm_handler::get_conversion_rate),
        )
        .route(
            "/opportunities/sales-funnel",
            get(crate::handlers::crm_handler::get_sales_funnel),
        )
        // V15 P2 18.2-D5: 商机阶段变更记录
        .route(
            "/opportunities/:id/stage-change",
            post(crate::handlers::crm_handler::record_opportunity_stage_change),
        )
}

/// CRM 客户增强路由（/customers/:id/{summary,360,follow-ups,rfm} + /rfm/distribution）
fn crm_customer_enhancement_routes() -> Router<AppState> {
    Router::new()
        // V15 P2 18.4-D5: 客户字段权限（静态路径注册在 /:id 之前）
        .route(
            "/customers/field-permissions",
            post(crate::handlers::crm_handler::set_customer_field_permission),
        )
        .route(
            "/customers/field-permissions/:role_id",
            get(crate::handlers::crm_handler::get_customer_field_permissions),
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
        // V15 P2 18.4-D6: 客户操作日志
        .route(
            "/customers/:id/audit-logs",
            get(crate::handlers::crm_handler::list_customer_audit_logs)
                .post(crate::handlers::crm_handler::create_customer_audit_log),
        )
        // V15 P2 18.5-D5: 客户 CLV
        .route(
            "/customers/:id/clv",
            get(crate::handlers::crm_handler::get_customer_clv),
        )
        .route(
            "/customers/:id/clv/calculate",
            post(crate::handlers::crm_handler::calculate_customer_clv),
        )
        .route(
            "/rfm/distribution",
            get(crate::handlers::crm_handler::get_rfm_distribution),
        )
}

/// CRM 业务路由（合并线索/商机/客户增强，子前缀互不重叠）
pub fn crm_business() -> Router<AppState> {
    // `/customers/enhanced/:id` 的 CRUD 已经在 [`crm_customers`] 中提供，
    // 这里不再重复注册，避免 path+method 冲突。
    Router::new()
        .merge(crm_lead_routes())
        .merge(crm_opportunity_routes())
        .merge(crm_customer_enhancement_routes())
}

/// CRM 竞争对手路由（/competitors）
fn crm_competitors() -> Router<AppState> {
    Router::new()
        .route(
            "/competitors",
            get(crate::handlers::crm_handler::list_competitors)
                .post(crate::handlers::crm_handler::create_competitor),
        )
}

/// CRM 域统一入口（子 router path 已加独立前缀，merge 安全）
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(customers())
        .merge(customer_credits())
        .merge(five_dimension())
        .merge(sales_analysis())
        .merge(crm_customers())
        .merge(crm_tags())
        .merge(crm_pool())
        // V15 P0-S08 修复：客户转移审批流
        .merge(crm_transfer_approvals())
        .merge(crm_assignments())
        .merge(crm_sales_users())
        .merge(crm_recycle_rules())
        .merge(crm_business())
        // V15 P2 18.2-D6: 竞争对手管理
        .merge(crm_competitors())
}
