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
    capacity_handler, cost_collection_handler, dye_batch_handler, dye_batch_state_machine_handler,
    dye_recipe_handler, fabric_inspection_handler, flow_card_handler, greige_fabric_handler,
    lab_dip_handler, missing_handlers, mrp_handler, outsourcing_handler,
    production_order_handler, production_recipe_handler, quality_inspection_handler, wage_handler,
    energy_handler, business_mode_handler,
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

/// 验布打卷路由（path 前缀 /fabric-inspections 和 /fabric-defects）
///
/// v14 批次 426：验布打卷流程贯通
/// 真实业务流程：验布机对接码表/电子称 → 疵点采集 → 生成验布报告
///   → 卷唛标签打印 → PDA 扫描卷唛条码 → 自动入库
/// 评分制式：四分制（AATCC/ASTM D5430）/ 十分制（梭织布）
pub fn fabric_inspections() -> Router<AppState> {
    Router::new()
        // ===== 验布记录 CRUD =====
        .route("/fabric-inspections", get(fabric_inspection_handler::list_inspections))
        .route("/fabric-inspections", post(fabric_inspection_handler::create_inspection))
        .route("/fabric-inspections/by-no/:no", get(fabric_inspection_handler::get_by_no))
        .route("/fabric-inspections/:id", get(fabric_inspection_handler::get_inspection))
        .route("/fabric-inspections/:id", put(fabric_inspection_handler::update_inspection))
        .route("/fabric-inspections/:id", delete(fabric_inspection_handler::delete_inspection))
        // ===== 验布记录状态机流转 =====
        .route("/fabric-inspections/:id/start", post(fabric_inspection_handler::start_inspection))
        .route("/fabric-inspections/:id/grade", post(fabric_inspection_handler::grade_inspection))
        .route("/fabric-inspections/:id/roll", post(fabric_inspection_handler::roll_fabric))
        .route("/fabric-inspections/:id/close", post(fabric_inspection_handler::close_inspection))
        // ===== 疵点明细 =====
        .route("/fabric-inspections/:inspection_id/defects", get(fabric_inspection_handler::list_defects_by_inspection))
        .route("/fabric-defects", post(fabric_inspection_handler::create_defect))
        .route("/fabric-defects/:id", get(fabric_inspection_handler::get_defect))
        .route("/fabric-defects/:id", delete(fabric_inspection_handler::delete_defect))
}

/// 产量工资路由（path 前缀 /wage-rates、/wage-records、/wage-details）
///
/// v14 批次 427：产量工资核算贯通
/// 真实业务流程：工序流转扫码 → 工价方案定义 → 工资计算 → 班组汇总 → 进入财务工资核算
/// 三维度产量统计：工序产量 + 设备产量 + 工人产量工资
/// 等级系数：A 级全额/B 级 8 折/C 级不计
pub fn wages() -> Router<AppState> {
    Router::new()
        // ===== 工序工价 CRUD =====
        .route("/wage-rates", get(wage_handler::list_wage_rates))
        .route("/wage-rates", post(wage_handler::create_wage_rate))
        .route("/wage-rates/by-no/:no", get(wage_handler::get_wage_rate_by_no))
        .route("/wage-rates/:id", get(wage_handler::get_wage_rate))
        .route("/wage-rates/:id", put(wage_handler::update_wage_rate))
        .route("/wage-rates/:id", delete(wage_handler::delete_wage_rate))
        // 工价状态机流转
        .route("/wage-rates/:id/activate", post(wage_handler::activate_wage_rate))
        .route("/wage-rates/:id/disable", post(wage_handler::disable_wage_rate))
        // 查询工序当前生效的工价
        .route("/wage-rates/effective/:route_id", get(wage_handler::get_effective_wage_rate))
        // ===== 工资记录 CRUD =====
        .route("/wage-records", get(wage_handler::list_wage_records))
        .route("/wage-records", post(wage_handler::create_wage_record))
        .route("/wage-records/by-no/:no", get(wage_handler::get_wage_record_by_no))
        .route("/wage-records/:id", get(wage_handler::get_wage_record))
        .route("/wage-records/:id", put(wage_handler::update_wage_record))
        .route("/wage-records/:id", delete(wage_handler::delete_wage_record))
        // 工资记录状态机流转
        .route("/wage-records/:id/calculate", post(wage_handler::calculate_wage))
        .route("/wage-records/:id/confirm", post(wage_handler::confirm_wage_record))
        .route("/wage-records/:id/pay", post(wage_handler::pay_wage_record))
        .route("/wage-records/:id/cancel", post(wage_handler::cancel_wage_record))
        // ===== 工资明细 =====
        .route("/wage-records/:id/details", get(wage_handler::list_wage_details))
        .route("/wage-details/by-worker/:worker_id", get(wage_handler::list_wage_details_by_worker))
}

/// 能耗管理路由（v14 批次 428：能耗管理贯通）
///
/// 业务来源：面料行业真实业务调研文档 §12.6 能耗管理
/// 路由分组：
/// - /energy-meters：能源计量设备 CRUD
/// - /energy-consumptions：能耗记录 CRUD + 状态机
/// - /energy-rules：能耗分摊规则 CRUD + 状态机 + 查询生效规则
/// - /energy-allocations：能耗分摊记录 CRUD + 状态机 + 月末自动分摊
pub fn energy() -> Router<AppState> {
    Router::new()
        // ===== 能源计量设备 CRUD =====
        .route("/energy-meters", get(energy_handler::list_energy_meters))
        .route("/energy-meters", post(energy_handler::create_energy_meter))
        .route("/energy-meters/by-no/:no", get(energy_handler::get_energy_meter_by_no))
        .route("/energy-meters/:id", get(energy_handler::get_energy_meter))
        .route("/energy-meters/:id", put(energy_handler::update_energy_meter))
        .route("/energy-meters/:id", delete(energy_handler::delete_energy_meter))
        // ===== 能耗记录 CRUD + 状态机 =====
        .route("/energy-consumptions", get(energy_handler::list_energy_consumptions))
        .route("/energy-consumptions", post(energy_handler::create_energy_consumption))
        .route("/energy-consumptions/by-no/:no", get(energy_handler::get_energy_consumption_by_no))
        .route("/energy-consumptions/:id", get(energy_handler::get_energy_consumption))
        .route("/energy-consumptions/:id", put(energy_handler::update_energy_consumption))
        .route("/energy-consumptions/:id", delete(energy_handler::delete_energy_consumption))
        // 能耗记录状态机
        .route("/energy-consumptions/:id/confirm", post(energy_handler::confirm_energy_consumption))
        .route("/energy-consumptions/:id/cancel", post(energy_handler::cancel_energy_consumption))
        // ===== 能耗分摊规则 CRUD + 状态机 =====
        .route("/energy-rules", get(energy_handler::list_energy_rules))
        .route("/energy-rules", post(energy_handler::create_energy_rule))
        .route("/energy-rules/by-no/:no", get(energy_handler::get_energy_rule_by_no))
        .route("/energy-rules/:id", get(energy_handler::get_energy_rule))
        .route("/energy-rules/:id", put(energy_handler::update_energy_rule))
        .route("/energy-rules/:id", delete(energy_handler::delete_energy_rule))
        // 分摊规则状态机
        .route("/energy-rules/:id/activate", post(energy_handler::activate_energy_rule))
        .route("/energy-rules/:id/disable", post(energy_handler::disable_energy_rule))
        .route("/energy-rules/effective", get(energy_handler::get_effective_energy_rule))
        // ===== 能耗分摊记录 CRUD + 状态机 =====
        .route("/energy-allocations", get(energy_handler::list_energy_allocations))
        .route("/energy-allocations", post(energy_handler::create_energy_allocation))
        .route("/energy-allocations/by-no/:no", get(energy_handler::get_energy_allocation_by_no))
        .route("/energy-allocations/:id", get(energy_handler::get_energy_allocation))
        .route("/energy-allocations/:id", put(energy_handler::update_energy_allocation))
        .route("/energy-allocations/:id", delete(energy_handler::delete_energy_allocation))
        // 分摊记录状态机
        .route("/energy-allocations/:id/confirm", post(energy_handler::confirm_energy_allocation))
        .route("/energy-allocations/:id/cancel", post(energy_handler::cancel_energy_allocation))
        // 月末按工时自动分摊
        .route("/energy-allocations/monthly", post(energy_handler::monthly_allocation))
}

/// 委外加工管理路由（path 前缀 /outsourcing-orders、/outsourcing-receipts、/outsourcing-vouchers）
///
/// v14 批次 430：委托加工物资贯通
/// 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算三步分录 + §5.5 委外织布场景
///       + §5.7 损耗率标准 + §6.5 委托加工模式
/// 真实业务流程：
///   委外订单（draft→issued→processing→received→settled→closed→cancelled）
///   发料明细（按面料四维标识追溯，发料分录：借 委托加工物资/贷 自制半成品-胚布）
///   收回入库单（draft→confirmed，含损耗分类；入库分录：借 库存商品-成品布/贷 委托加工物资）
///   会计凭证（issue 发料 / fee 加工费 / receipt 入库 / loss 损耗处理）
/// 三步分录（§5.4）：
///   1. 发料：借 委托加工物资 / 贷 自制半成品-胚布
///   2. 加工费：借 委托加工物资 + 应交税费-进项税额 / 贷 银行存款
///   3. 入库：借 库存商品-成品布 / 贷 委托加工物资
pub fn outsourcing() -> Router<AppState> {
    Router::new()
        // ===== 委外订单 CRUD =====
        .route("/outsourcing-orders", get(outsourcing_handler::list_outsourcing_orders))
        .route("/outsourcing-orders", post(outsourcing_handler::create_outsourcing_order))
        .route("/outsourcing-orders/by-no/:no", get(outsourcing_handler::get_outsourcing_order_by_no))
        .route("/outsourcing-orders/:id", get(outsourcing_handler::get_outsourcing_order))
        .route("/outsourcing-orders/:id", put(outsourcing_handler::update_outsourcing_order))
        .route("/outsourcing-orders/:id", delete(outsourcing_handler::delete_outsourcing_order))
        // 委外订单状态机流转
        .route("/outsourcing-orders/:id/issue", post(outsourcing_handler::issue_outsourcing_order))
        .route("/outsourcing-orders/:id/processing", post(outsourcing_handler::record_processing))
        .route("/outsourcing-orders/:id/settle", post(outsourcing_handler::settle_outsourcing_order))
        .route("/outsourcing-orders/:id/close", post(outsourcing_handler::close_outsourcing_order))
        .route("/outsourcing-orders/:id/cancel", post(outsourcing_handler::cancel_outsourcing_order))
        // ===== 委外发料明细 =====
        .route("/outsourcing-orders/items/by-order/:order_id", get(outsourcing_handler::list_outsourcing_items))
        .route("/outsourcing-orders/items", post(outsourcing_handler::create_outsourcing_item))
        .route("/outsourcing-orders/items/:id", put(outsourcing_handler::update_outsourcing_item))
        .route("/outsourcing-orders/items/:id", delete(outsourcing_handler::delete_outsourcing_item))
        // ===== 委外收回入库单 CRUD + 状态机 =====
        .route("/outsourcing-receipts", get(outsourcing_handler::list_outsourcing_receipts))
        .route("/outsourcing-receipts", post(outsourcing_handler::create_outsourcing_receipt))
        .route("/outsourcing-receipts/by-no/:no", get(outsourcing_handler::get_outsourcing_receipt_by_no))
        .route("/outsourcing-receipts/:id/confirm", post(outsourcing_handler::confirm_outsourcing_receipt))
        .route("/outsourcing-receipts/:id", put(outsourcing_handler::update_outsourcing_receipt))
        .route("/outsourcing-receipts/:id", delete(outsourcing_handler::delete_outsourcing_receipt))
        // ===== 委外会计分录凭证 =====
        .route("/outsourcing-vouchers", get(outsourcing_handler::list_outsourcing_vouchers))
        .route("/outsourcing-vouchers", post(outsourcing_handler::create_outsourcing_voucher))
        .route("/outsourcing-vouchers/by-no/:no", get(outsourcing_handler::get_outsourcing_voucher_by_no))
        .route("/outsourcing-vouchers/:id/post", post(outsourcing_handler::post_outsourcing_voucher))
        .route("/outsourcing-vouchers/:id", delete(outsourcing_handler::delete_outsourcing_voucher))
}

/// 多业务模式支持路由（path 前缀 /business-modes、/business-mode-links）
///
/// v14 批次 431：多业务模式支持
/// 依据：面料行业真实业务调研文档 §6 业务模式 6 种
/// 真实业务：6 种典型业务模式（坯布经销/成品经销/染整加工/自织自染/委托加工/来料加工）
///   贯穿采购/库存/生产/委外/销售/结算全链路
/// 路由分组：
/// - /business-modes：业务模式配置 CRUD + 按代码查询 + 默认模式 + 完整详情
/// - /business-modes/flow-steps：流程节点 CRUD + 按模式查询
/// - /business-modes/rules：业务规则 CRUD + 按模式查询
/// - /business-mode-links：单据-业务模式关联 CRUD + 按单据查询
pub fn business_mode() -> Router<AppState> {
    Router::new()
        // ===== 业务模式配置 CRUD =====
        .route("/business-modes", get(business_mode_handler::list_business_modes).post(business_mode_handler::create_business_mode))
        // 静态路径必须在动态路径 /:id 之前，避免 axum 0.7 Overlapping method route panic
        .route("/business-modes/default", get(business_mode_handler::get_default_business_mode))
        .route("/business-modes/by-code/:code", get(business_mode_handler::get_business_mode_by_code))
        .route("/business-modes/:id", get(business_mode_handler::get_business_mode).put(business_mode_handler::update_business_mode).delete(business_mode_handler::delete_business_mode))
        // 业务模式配置状态流转
        .route("/business-modes/:id/set-default", post(business_mode_handler::set_default_business_mode))
        // 业务模式完整详情（含流程节点+规则）
        .route("/business-modes/:id/detail", get(business_mode_handler::get_business_mode_detail))
        // ===== 业务模式流程节点 CRUD =====
        // 静态路径必须在动态路径 /:id 之前
        .route("/business-modes/flow-steps/by-mode/:mode_id", get(business_mode_handler::list_flow_steps_by_mode))
        .route("/business-modes/flow-steps", post(business_mode_handler::create_flow_step))
        .route("/business-modes/flow-steps/:id", put(business_mode_handler::update_flow_step).delete(business_mode_handler::delete_flow_step))
        // ===== 业务模式规则 CRUD =====
        // 静态路径必须在动态路径 /:id 之前
        .route("/business-modes/rules/by-mode/:mode_id", get(business_mode_handler::list_rules_by_mode))
        .route("/business-modes/rules", post(business_mode_handler::create_rule))
        .route("/business-modes/rules/:id", put(business_mode_handler::update_rule).delete(business_mode_handler::delete_rule))
        // ===== 单据-业务模式关联 CRUD =====
        .route("/business-mode-links", get(business_mode_handler::list_order_links).post(business_mode_handler::link_order))
        // 静态路径必须在动态路径 /:id 之前
        .route("/business-mode-links/by-document/:doc_type/:doc_id", get(business_mode_handler::get_order_link_by_document))
        .route("/business-mode-links/:id", put(business_mode_handler::update_order_link).delete(business_mode_handler::delete_order_link))
}

/// 缸号全生命周期状态机路由（path 前缀 /dye-batch-lifecycle-logs、/dye-batch-state-rules、
/// /dye-batch-reworks、/dye-batch-operations）
///
/// v14 批次 432：缸号全生命周期状态机
/// 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
/// 真实业务流程：14 种状态流转（待排缸→已排缸→备布中→进缸染色→皂洗→固色→脱水→烘干→验布→入库→发货）
/// 加上回修流转（验布/入库 → 回修中 → 重新进缸染色）与终态保护（发货/取消/终止不可流转）
/// 路由分组：/dye-batch-lifecycle-logs 生命周期日志 CRUD + 按缸号查询 + 获取最新状态 + 记录流转；
/// /dye-batch-state-rules 状态流转规则 CRUD + 校验流转 + 查询允许的流转；
/// /dye-batch-reworks 回修记录 CRUD + 审批 + 开始/完成/取消回修；
/// /dye-batch-operations 操作记录 CRUD + 按类型查询 + 按缸号查询。
pub fn dye_batch_state_machine() -> Router<AppState> {
    Router::new()
        // ===== 缸号生命周期日志 =====
        // 静态路径必须在动态路径 /:id 之前，避免 axum 0.7 Overlapping method route panic
        .route("/dye-batch-lifecycle-logs/by-batch/:batch_id", get(dye_batch_state_machine_handler::list_lifecycle_logs_by_batch))
        .route("/dye-batch-lifecycle-logs/latest-status/:batch_id", get(dye_batch_state_machine_handler::get_latest_status))
        .route("/dye-batch-lifecycle-logs", get(dye_batch_state_machine_handler::list_lifecycle_logs).post(dye_batch_state_machine_handler::record_transition))
        .route("/dye-batch-lifecycle-logs/:id", get(dye_batch_state_machine_handler::get_lifecycle_log))
        // ===== 缸号状态流转规则 =====
        // 静态路径必须在动态路径 /:id 之前
        .route("/dye-batch-state-rules/allowed-transitions", get(dye_batch_state_machine_handler::list_allowed_transitions))
        .route("/dye-batch-state-rules/check", get(dye_batch_state_machine_handler::check_transition))
        .route("/dye-batch-state-rules", get(dye_batch_state_machine_handler::list_state_rules).post(dye_batch_state_machine_handler::create_state_rule))
        .route("/dye-batch-state-rules/:id", get(dye_batch_state_machine_handler::get_state_rule).put(dye_batch_state_machine_handler::update_state_rule).delete(dye_batch_state_machine_handler::delete_state_rule))
        // ===== 缸号回修记录 =====
        .route("/dye-batch-reworks", get(dye_batch_state_machine_handler::list_reworks).post(dye_batch_state_machine_handler::create_rework))
        .route("/dye-batch-reworks/:id", get(dye_batch_state_machine_handler::get_rework).put(dye_batch_state_machine_handler::update_rework).delete(dye_batch_state_machine_handler::delete_rework))
        // 回修单状态机流转
        .route("/dye-batch-reworks/:id/approve", post(dye_batch_state_machine_handler::approve_rework))
        .route("/dye-batch-reworks/:id/start", post(dye_batch_state_machine_handler::start_rework))
        .route("/dye-batch-reworks/:id/complete", post(dye_batch_state_machine_handler::complete_rework))
        .route("/dye-batch-reworks/:id/cancel", post(dye_batch_state_machine_handler::cancel_rework))
        // ===== 缸号操作记录 =====
        // 静态路径必须在动态路径 /:id 之前
        .route("/dye-batch-operations/by-type/:operation_type", get(dye_batch_state_machine_handler::list_operations_by_type))
        .route("/dye-batch-operations/by-batch/:batch_id", get(dye_batch_state_machine_handler::list_operations_by_batch))
        .route("/dye-batch-operations", get(dye_batch_state_machine_handler::list_operations).post(dye_batch_state_machine_handler::create_operation))
        .route("/dye-batch-operations/:id", get(dye_batch_state_machine_handler::get_operation))
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
        // V15 P0-S12 修复（Batch 475c）：生产订单导出端点（必须在 /:id 之前注册，避免 axum matchit 把 "export" 当 :id 匹配）
        .route(
            "/production-orders/orders/export",
            get(production_order_handler::export_production_orders),
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
        .merge(fabric_inspections())
        .merge(wages())
        .merge(energy())
        .merge(outsourcing())
        .merge(business_mode())
        .merge(dye_batch_state_machine())
        .merge(quality_inspection())
        .merge(cost_collections())
        .merge(production())
        .merge(mrp())
        .merge(mrp_history())
        .merge(capacity())
}
