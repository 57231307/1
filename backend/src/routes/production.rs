//! 生产域路由
//!
//! 处理生产订单、MRP 物料需求计划、生产排程、产能、缺料预警、质量检验/标准、成本归集、
//! 缸号/染色批次/染色配方、坯布等生产与工艺相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/dye-batches`、`/greige-fabrics`、`/dye-recipes`、`/quality-inspection`、
//!  `/quality-standards`、`/cost-collections`、`/production-orders`、`/mrp`、
//!  `/mrp-history`、`/scheduling`、`/capacity`、`/material-shortage` 等），
//!  这样 `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//!  避免 axum 0.7 `Overlapping method route` panic。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    capacity_handler, cost_collection_handler, dye_batch_handler, dye_recipe_handler,
    flow_card_handler, greige_fabric_handler, lab_dip_handler, missing_handlers, mrp_handler,
    production_order_handler, quality_inspection_handler,
};

/// 缸号管理路由（path 前缀 /dye-batches）
pub fn dye_batches() -> Router<AppState> {
    Router::new()
        .route("/dye-batches", get(dye_batch_handler::list_dye_batches))
        .route("/dye-batches", post(dye_batch_handler::create_dye_batch))
        .route("/dye-batches/:id", get(dye_batch_handler::get_dye_batch))
        .route("/dye-batches/:id", put(dye_batch_handler::update_dye_batch))
        .route(
            "/dye-batches/:id",
            delete(dye_batch_handler::delete_dye_batch),
        )
        .route(
            "/dye-batches/:id/complete",
            post(dye_batch_handler::complete_dye_batch),
        )
        .route(
            "/dye-batches/by-color/:color_code",
            get(dye_batch_handler::get_dye_batches_by_color),
        )
        .route(
            "/dye-batches/export",
            get(dye_batch_handler::export_dye_batches),
        )
}

/// 坯布管理路由（path 前缀 /greige-fabrics）
pub fn greige_fabrics() -> Router<AppState> {
    Router::new()
        .route(
            "/greige-fabrics",
            get(greige_fabric_handler::list_greige_fabrics),
        )
        .route(
            "/greige-fabrics",
            post(greige_fabric_handler::create_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            get(greige_fabric_handler::get_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            put(greige_fabric_handler::update_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id",
            delete(greige_fabric_handler::delete_greige_fabric),
        )
        .route(
            "/greige-fabrics/:id/stock-in",
            post(greige_fabric_handler::stock_in),
        )
        .route(
            "/greige-fabrics/:id/stock-out",
            post(greige_fabric_handler::stock_out),
        )
        .route(
            "/greige-fabrics/by-supplier/:supplier_id",
            get(greige_fabric_handler::get_greige_by_supplier),
        )
}

/// 染色配方路由（path 前缀 /dye-recipes）
pub fn dye_recipes() -> Router<AppState> {
    Router::new()
        .route("/dye-recipes", get(dye_recipe_handler::list_dye_recipes))
        .route("/dye-recipes", post(dye_recipe_handler::create_dye_recipe))
        .route("/dye-recipes/:id", get(dye_recipe_handler::get_dye_recipe))
        .route(
            "/dye-recipes/:id",
            put(dye_recipe_handler::update_dye_recipe),
        )
        .route(
            "/dye-recipes/:id",
            delete(dye_recipe_handler::delete_dye_recipe),
        )
        .route(
            "/dye-recipes/:id/approve",
            post(dye_recipe_handler::approve_recipe),
        )
        .route(
            "/dye-recipes/:id/submit",
            post(dye_recipe_handler::submit_dye_recipe),
        )
        .route(
            "/dye-recipes/:id/version",
            post(dye_recipe_handler::create_new_version),
        )
        .route(
            "/dye-recipes/by-color/:color_code",
            get(dye_recipe_handler::get_recipes_by_color),
        )
        .route(
            "/dye-recipes/:id/versions",
            get(dye_recipe_handler::get_recipe_versions),
        )
        .route(
            "/dye-recipes/export",
            get(dye_recipe_handler::export_dye_recipes),
        )
}

/// 化验室打样路由（path 前缀 /lab-dip）
///
/// v14 批次 423B：化验室打样流程贯通
/// 真实业务流程：打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库
pub fn lab_dip() -> Router<AppState> {
    Router::new()
        // ===== 打样通知单 =====
        .route("/lab-dip/requests", get(lab_dip_handler::list_requests))
        .route("/lab-dip/requests", post(lab_dip_handler::create_request))
        .route("/lab-dip/requests/:id", get(lab_dip_handler::get_request))
        .route("/lab-dip/requests/:id", put(lab_dip_handler::update_request))
        .route("/lab-dip/requests/:id", delete(lab_dip_handler::delete_request))
        // 状态流转
        .route("/lab-dip/requests/:id/start-sampling", post(lab_dip_handler::start_sampling))
        .route("/lab-dip/requests/:id/submit", post(lab_dip_handler::submit_to_customer))
        .route("/lab-dip/requests/:id/approve", post(lab_dip_handler::approve_ok_sample))
        .route("/lab-dip/requests/:id/reject", post(lab_dip_handler::reject_and_redo))
        .route("/lab-dip/requests/:id/restart", post(lab_dip_handler::restart_sampling))
        .route("/lab-dip/requests/:id/complete", post(lab_dip_handler::complete_request))
        // ===== 打样小样（ABCD 多版样） =====
        .route("/lab-dip/samples", post(lab_dip_handler::create_sample))
        .route("/lab-dip/samples/:id", get(lab_dip_handler::get_sample))
        .route("/lab-dip/samples/:id", put(lab_dip_handler::update_sample))
        .route("/lab-dip/samples/:id", delete(lab_dip_handler::delete_sample))
        .route("/lab-dip/samples/:id/matching", post(lab_dip_handler::record_matching_result))
        .route("/lab-dip/samples/by-request/:request_id", get(lab_dip_handler::list_samples_by_request))
        // ===== 复样记录 =====
        .route("/lab-dip/resamples", post(lab_dip_handler::create_resample))
        .route("/lab-dip/resamples/:id", get(lab_dip_handler::get_resample))
        .route("/lab-dip/resamples/:id/result", post(lab_dip_handler::record_resample_result))
        .route("/lab-dip/resamples/:id/tech-card", post(lab_dip_handler::issue_tech_card))
        .route("/lab-dip/resamples/by-request/:request_id", get(lab_dip_handler::list_resamples_by_request))
}

/// 大货处方与加料处方路由（path 前缀 /production-recipes）
///
/// v14 批次 424：大货处方与加料处方流程
/// 真实业务流程：
///   大货处方单：扫描流转卡条码 → 依据备布数量 → 加载小样处方/历史大货处方 → 根据浴比/浴量
///              → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据
///   加料处方单：扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
///   关键约束：同一工单号只能开一张大货处方单，追加物料须开加料处方单
pub fn production_recipes() -> Router<AppState> {
    Router::new()
        // ===== 大货处方 CRUD =====
        .route("/production-recipes", get(production_recipe_handler::list))
        .route("/production-recipes", post(production_recipe_handler::create))
        .route("/production-recipes/:id", get(production_recipe_handler::get))
        .route("/production-recipes/:id", put(production_recipe_handler::update))
        .route("/production-recipes/:id", delete(production_recipe_handler::delete))
        // 大货处方状态流转
        .route("/production-recipes/:id/approve", post(production_recipe_handler::approve))
        .route("/production-recipes/:id/close", post(production_recipe_handler::close))
        .route("/production-recipes/:id/cancel", post(production_recipe_handler::cancel))
        // 用量计算（纯函数，无需路径参数）
        .route("/production-recipes/calculate", post(production_recipe_handler::calculate))
        // 按工单查询大货处方（一工单一处方约束）
        .route(
            "/production-recipes/by-work-order/:work_order_id",
            get(production_recipe_handler::get_by_work_order),
        )
        // ===== 加料处方（按大货处方 ID 子资源） =====
        .route(
            "/production-recipes/:id/additions",
            get(production_recipe_handler::list_additions),
        )
        .route(
            "/production-recipes/:id/additions",
            post(production_recipe_handler::create_addition),
        )
        // 加料处方详情与状态流转（按加料处方 ID）
        .route(
            "/production-recipes/additions/:id",
            get(production_recipe_handler::get_addition),
        )
        .route(
            "/production-recipes/additions/:id/approve",
            post(production_recipe_handler::approve_addition),
        )
        .route(
            "/production-recipes/additions/:id/close",
            post(production_recipe_handler::close_addition),
        )
}

/// 流转卡与工序流转路由（path 前缀 /process-routes 和 /flow-cards）
///
/// v14 批次 425：流转卡条码与车间工序流转
/// 真实业务流程：生产计划单 → 备布 → 排缸执行 → 流转卡打印（含条码）
///   扫码应用：白坯出库/染色进度/称料/工序流转/成品入库/发货
pub fn flow_cards() -> Router<AppState> {
    Router::new()
        // ===== 工序路线模板 =====
        .route("/process-routes", get(flow_card_handler::list_process_routes))
        .route("/process-routes", post(flow_card_handler::create_process_route))
        .route("/process-routes/:id", get(flow_card_handler::get_process_route))
        .route("/process-routes/:id", put(flow_card_handler::update_process_route))
        .route("/process-routes/:id", delete(flow_card_handler::delete_process_route))
        // ===== 流转卡 CRUD =====
        .route("/flow-cards", get(flow_card_handler::list_flow_cards))
        .route("/flow-cards", post(flow_card_handler::create_flow_card))
        .route("/flow-cards/by-barcode", get(flow_card_handler::get_by_barcode))
        .route("/flow-cards/:id", get(flow_card_handler::get_flow_card))
        .route("/flow-cards/:id", put(flow_card_handler::update_flow_card))
        .route("/flow-cards/:id", delete(flow_card_handler::delete_flow_card))
        // ===== 流转卡状态机流转 =====
        .route("/flow-cards/:id/schedule", post(flow_card_handler::schedule))
        .route("/flow-cards/:id/start-preparing", post(flow_card_handler::start_preparing))
        .route("/flow-cards/:id/complete-preparing", post(flow_card_handler::complete_preparing))
        .route("/flow-cards/:id/start-dyeing", post(flow_card_handler::start_dyeing))
        .route("/flow-cards/:id/complete-dyeing", post(flow_card_handler::complete_dyeing))
        .route("/flow-cards/:id/start-inspecting", post(flow_card_handler::start_inspecting))
        .route("/flow-cards/:id/complete", post(flow_card_handler::complete_flow_card))
        .route("/flow-cards/:id/ship", post(flow_card_handler::ship_flow_card))
        .route("/flow-cards/:id/terminate", post(flow_card_handler::terminate_flow_card))
        .route("/flow-cards/:id/reactivate", post(flow_card_handler::reactivate_flow_card))
        // ===== 工序流转记录（扫码开始/结束/回修） =====
        .route("/flow-cards/steps/start", post(flow_card_handler::start_step))
        .route("/flow-cards/steps/:id/complete", post(flow_card_handler::complete_step))
        .route("/flow-cards/steps/:id", get(flow_card_handler::get_step))
        .route("/flow-cards/steps/:source_step_id/rework", post(flow_card_handler::create_rework_step))
        .route("/flow-cards/:flow_card_id/steps", get(flow_card_handler::list_steps_by_card))
        // ===== 工序质量反馈单 =====
        .route("/flow-cards/feedbacks", post(flow_card_handler::create_feedback))
        .route("/flow-cards/feedbacks/:id", get(flow_card_handler::get_feedback))
        .route("/flow-cards/feedbacks/:id/handle", post(flow_card_handler::handle_feedback))
        .route("/flow-cards/feedbacks/:id/close", post(flow_card_handler::close_feedback))
        .route("/flow-cards/:flow_card_id/feedbacks", get(flow_card_handler::list_feedbacks_by_card))
}

/// 质量检验路由（path 前缀 /quality-inspection）
///
/// 注意：原代码用 `/standards`、`/records`、`/defects` 等带前缀 path，已天然不冲突。
pub fn quality_inspection() -> Router<AppState> {
    Router::new()
        .route(
            "/quality-inspection/standards",
            get(quality_inspection_handler::list_standards),
        )
        .route(
            "/quality-inspection/standards",
            post(quality_inspection_handler::create_standard),
        )
        .route(
            "/quality-inspection/records",
            get(quality_inspection_handler::list_records),
        )
        .route(
            "/quality-inspection/records",
            post(quality_inspection_handler::create_record),
        )
        .route(
            "/quality-inspection/records/:id",
            get(quality_inspection_handler::get_record),
        )
        .route(
            "/quality-inspection/defects",
            get(quality_inspection_handler::list_defects),
        )
        .route(
            "/quality-inspection/defects/:id/process",
            post(quality_inspection_handler::process_defect),
        )
        .route(
            "/quality-inspection/defects/:id/handle",
            post(quality_inspection_handler::process_defect),
        )
}

/// 成本归集路由（path 前缀 /cost-collections）
pub fn cost_collections() -> Router<AppState> {
    Router::new()
        .route(
            "/cost-collections",
            get(cost_collection_handler::list_collections),
        )
        .route(
            "/cost-collections",
            post(cost_collection_handler::create_collection),
        )
        .route(
            "/cost-collections/:id",
            get(cost_collection_handler::get_collection),
        )
        .route(
            "/cost-collections/:id",
            put(cost_collection_handler::update_collection),
        )
        .route(
            "/cost-collections/:id",
            delete(cost_collection_handler::delete_collection),
        )
        .route(
            "/cost-collections/:id/audit",
            post(cost_collection_handler::audit_collection),
        )
        .route(
            "/cost-collections/analysis/summary",
            get(cost_collection_handler::get_cost_analysis_summary),
        )
        .route(
            "/cost-collections/analysis/by-batch",
            get(cost_collection_handler::get_cost_by_batch),
        )
}

/// 生产订单路由（path 前缀 /production-orders）
pub fn production() -> Router<AppState> {
    Router::new()
        .route(
            "/production-orders/orders",
            get(production_order_handler::list_production_orders)
                .post(production_order_handler::create_production_order),
        )
        .route(
            "/production-orders/orders/:id",
            get(production_order_handler::get_production_order)
                .put(production_order_handler::update_production_order)
                .delete(production_order_handler::delete_production_order),
        )
        .route(
            "/production-orders/orders/:id/status",
            put(production_order_handler::update_production_order_status),
        )
        .route(
            "/production-orders/orders/:id/submit-approval",
            post(production_order_handler::submit_for_approval),
        )
        .route(
            "/production-orders/orders/:id/approve",
            post(production_order_handler::approve_production_order),
        )
        .route(
            "/production-orders/orders/:id/progress",
            post(production_order_handler::update_production_progress),
        )
        .route(
            "/production-orders/orders/:id/logs",
            get(production_order_handler::get_production_order_logs),
        )
}

/// MRP 物料需求计划路由（path 前缀 /mrp）
pub fn mrp() -> Router<AppState> {
    Router::new()
        .route("/mrp/calculate", post(mrp_handler::calculate_mrp))
        .route("/mrp/results", get(mrp_handler::get_mrp_results))
        .route("/mrp/requirements", get(mrp_handler::get_mrp_requirements))
        .route("/mrp/convert-orders", post(mrp_handler::convert_to_orders))
        .route("/mrp/products", get(mrp_handler::list_products_for_mrp))
}

/// MRP 历史记录路由（path 前缀 /mrp-history）
pub fn mrp_history() -> Router<AppState> {
    Router::new()
        .route("/mrp-history", get(missing_handlers::get_mrp_history))
        .route(
            "/mrp-history/:id",
            get(missing_handlers::get_mrp_history_detail),
        )
        .route(
            "/mrp-history/:id/cancel",
            put(mrp_handler::cancel_calculation),
        )
        .route(
            "/mrp-history/:id/export",
            get(mrp_handler::export_calculation),
        )
        .route(
            "/mrp-history/:calculation_id/materials/:material_id",
            get(mrp_handler::get_material_detail),
        )
}

/// 产能分析路由（path 前缀 /capacity）
pub fn capacity() -> Router<AppState> {
    Router::new()
        .route(
            "/capacity/overview",
            get(capacity_handler::get_capacity_overview),
        )
        .route(
            "/capacity/summary",
            get(capacity_handler::get_capacity_overview),
        )
        .route(
            "/capacity/bottlenecks",
            get(capacity_handler::get_load_analysis),
        )
        .route("/capacity/trend", get(capacity_handler::get_load_analysis))
        .route(
            "/capacity/work-centers",
            get(capacity_handler::list_work_centers).post(capacity_handler::create_work_center),
        )
        .route(
            "/capacity/work-centers/:id",
            put(capacity_handler::update_work_center).delete(capacity_handler::delete_work_center),
        )
        .route(
            "/capacity/work-centers/:id/forecast",
            get(capacity_handler::forecast_capacity),
        )
        .route(
            "/capacity/work-centers/:id/available",
            get(capacity_handler::get_available_capacity),
        )
        .route(
            "/capacity/load-analysis",
            get(capacity_handler::get_load_analysis),
        )
        .route(
            "/capacity/overload-check",
            get(capacity_handler::check_capacity_overload),
        )
}

/// 生产域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(dye_batches())
        .merge(greige_fabrics())
        .merge(dye_recipes())
        .merge(lab_dip())
        .merge(production_recipes())
        .merge(flow_cards())
        .merge(quality_inspection())
        .merge(cost_collections())
        .merge(production())
        .merge(mrp())
        .merge(mrp_history())
        .merge(capacity())
}
