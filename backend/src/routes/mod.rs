use crate::middleware::rate_limit;
use crate::utils::app_state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::handlers::{
    account_subject_handler, accounting_period_handler, advanced_handler, ai_analysis_handler,
    ap_invoice_handler, ap_payment_handler, ap_payment_request_handler, ap_reconciliation_handler,
    ap_report_handler, ap_verification_handler, api_key_handler, ar_invoice_handler,
    ar_payment_handler, ar_reconciliation_enhanced_handler, ar_reconciliation_handler,
    ar_report_handler, ar_verification_handler, assist_accounting_handler, audit_enhanced_handler,
    auth_handler, barcode_scanner_handler, bom_handler, bpm_definition_handler, bpm_handler,
    budget_management_handler, bulk_product_handler, business_trace_handler, capacity_handler,
    cost_collection_handler, crm_assignment_handler, crm_customer_handler, crm_pool_handler,
    currency_enhanced_handler, currency_handler, customer_credit_handler, customer_handler,
    dashboard_handler, data_permission_handler, department_handler, dual_unit_converter_handler,
    dye_batch_handler, dye_recipe_handler, email_handler, field_permission_handler,
    finance_invoice_handler, finance_payment_handler, finance_report_handler,
    financial_analysis_handler, five_dimension_handler, fixed_asset_handler,
    fund_management_handler, greige_fabric_handler, health_handler, import_export_handler,
    init_handler, inventory_adjustment_handler, inventory_batch_handler, inventory_count_handler,
    inventory_reservation_handler, inventory_stock_handler, inventory_transfer_handler,
    login_security_handler, logistics_handler, material_shortage_handler, missing_handlers,
    mrp_handler, notification_handler, omni_audit_handler, print_handler, product_category_handler,
    product_handler, production_order_handler, purchase_contract_handler,
    purchase_inspection_handler, purchase_order_handler, purchase_price_handler,
    purchase_receipt_handler, purchase_return_handler, quality_inspection_handler,
    quality_standard_handler, report_engine_handler, report_enhanced_handler, role_handler,
    sales_analysis_handler, sales_contract_handler, sales_fabric_order_handler,
    sales_order_handler, sales_price_handler, sales_return_handler, scheduling_handler,
    supplier_evaluation_handler, supplier_handler, system_update_handler, tenant_billing_handler,
    tenant_config_handler, tenant_handler, tracking_handler, user_handler,
    user_notification_setting_handler, voucher_handler, warehouse_handler, webhook_handler,
    webhook_integration_handler,
};

use crate::services::metrics_service::create_metrics_router;

/// 创建路由配置
/// 所有接口路径统一为 /api/v1/erp/* 格式
pub fn create_router(state: AppState) -> Router {
    // 挂载 Prometheus 监控路由
    let metrics_routes = create_metrics_router();

    // 认证路由 - 添加防暴力攻击中间件
    let auth_routes = Router::new()
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler::logout))
        .route("/refresh", post(auth_handler::refresh_token))
        .route("/csrf-token", get(auth_handler::get_csrf_token))
        .route("/totp/setup", get(auth_handler::setup_totp))
        .route("/totp/enable", post(auth_handler::enable_totp))
        .route("/me", get(auth_handler::get_current_user))
        .layer(middleware::from_fn(rate_limit::anti_brute_force));

    // 用户管理路由
    let user_routes = Router::new()
        .route("/", get(user_handler::list_users))
        .route("/", post(user_handler::create_user))
        .route("/:id", get(user_handler::get_user))
        .route("/:id", put(user_handler::update_user))
        .route("/:id", delete(user_handler::delete_user))
        .route("/change-password", post(user_handler::change_password))
        .route("/reset-password", post(init_handler::reset_admin_password));

    // 角色管理路由
    let role_routes = Router::new()
        .route("/", get(role_handler::list_roles))
        .route("/", post(role_handler::create_role))
        .route("/:id", get(role_handler::get_role))
        .route("/:id", put(role_handler::update_role))
        .route("/:id", delete(role_handler::delete_role))
        .route("/:id/permissions", get(role_handler::get_role_permissions))
        .route("/:id/permissions", post(role_handler::assign_permission))
        .route("/permissions/:id", delete(role_handler::remove_permission))
        .route("/permissions", get(role_handler::list_permissions));

    // 产品管理路由
    let product_routes = Router::new()
        .route("/", get(product_handler::list_products))
        .route("/", post(product_handler::create_product))
        .route("/select", get(product_handler::list_products))
        .route("/:id", get(product_handler::get_product))
        .route("/:id", put(product_handler::update_product))
        .route("/:id", delete(product_handler::delete_product))
        .route(
            "/batch/create",
            post(bulk_product_handler::batch_create_products),
        )
        .route(
            "/batch/update",
            post(bulk_product_handler::batch_update_products),
        )
        .route(
            "/batch/delete",
            post(bulk_product_handler::batch_delete_products),
        )
        // 数据导入导出路由
        .route("/export", get(product_handler::export_products))
        .route("/import", post(product_handler::import_products))
        .route(
            "/import-template",
            get(product_handler::get_product_import_template),
        )
        // 色号管理路由
        .route(
            "/:product_id/colors",
            get(product_handler::list_product_colors),
        )
        .route(
            "/:product_id/colors",
            post(product_handler::create_product_color),
        )
        .route(
            "/:product_id/colors/:color_id",
            put(product_handler::update_product_color),
        )
        .route(
            "/:product_id/colors/:color_id",
            delete(product_handler::delete_product_color),
        )
        .route(
            "/:product_id/colors/batch",
            post(product_handler::batch_create_colors),
        );

    // 产品类别管理路由
    let product_category_routes = Router::new()
        .route("/", get(product_category_handler::list))
        .route("/", post(product_category_handler::create))
        .route("/:id", get(product_category_handler::get))
        .route("/:id", put(product_category_handler::update))
        .route("/:id", delete(product_category_handler::delete))
        .route(
            "/tree",
            get(product_category_handler::get_product_category_tree),
        );

    // 仓库管理路由
    let warehouse_routes = Router::new()
        .route("/", get(warehouse_handler::list))
        .route("/", post(warehouse_handler::create))
        .route("/select", get(warehouse_handler::list))
        .route("/:id", get(warehouse_handler::get))
        .route("/:id", put(warehouse_handler::update))
        .route("/:id", delete(warehouse_handler::delete))
        // 库位管理路由
        .route("/locations", get(warehouse_handler::list_locations))
        .route("/locations", post(warehouse_handler::create_location))
        .route("/locations/:id", get(warehouse_handler::get_location))
        .route("/locations/:id", put(warehouse_handler::update_location))
        .route("/locations/:id", delete(warehouse_handler::delete_location));

    // 部门管理路由
    let department_routes = Router::new()
        .route("/", get(department_handler::list))
        .route("/", post(department_handler::create))
        .route("/:id", get(department_handler::get))
        .route("/:id", put(department_handler::update))
        .route("/:id", delete(department_handler::delete))
        .route("/tree", get(department_handler::get_department_tree));

    // 仪表板统计路由
    let dashboard_routes = Router::new()
        .route("/overview", get(dashboard_handler::get_dashboard_overview))
        .route("/sales-stats", get(dashboard_handler::get_sales_statistics))
        .route(
            "/inventory-stats",
            get(dashboard_handler::get_inventory_statistics),
        )
        .route(
            "/low-stock-alerts",
            get(dashboard_handler::get_low_stock_alerts),
        );

    // 财务管理路由
    let finance_routes = Router::new()
        .route("/payments", get(finance_payment_handler::list_payments))
        .route("/payments", post(finance_payment_handler::create_payment))
        .route("/payments/:id", get(finance_payment_handler::get_payment))
        .route(
            "/invoices",
            get(finance_invoice_handler::list_finance_invoices),
        )
        .route(
            "/invoices",
            post(finance_invoice_handler::create_finance_invoice),
        )
        .route(
            "/invoices/:id",
            get(finance_invoice_handler::get_finance_invoice)
                .put(finance_invoice_handler::update_finance_invoice)
                .delete(finance_invoice_handler::delete_finance_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(finance_invoice_handler::approve_finance_invoice),
        )
        .route(
            "/invoices/:id/verify",
            post(finance_invoice_handler::verify_invoice),
        )
        // Accounting Period Routes
        .route(
            "/accounting-periods",
            get(missing_handlers::get_accounting_periods)
                .post(missing_handlers::create_accounting_period),
        )
        .route(
            "/accounting-periods/current",
            get(accounting_period_handler::get_current_period),
        )
        .route(
            "/accounting-periods/init",
            post(accounting_period_handler::init_period),
        )
        .route(
            "/accounting-periods/:id",
            get(missing_handlers::get_accounting_period_detail)
                .put(missing_handlers::update_accounting_period)
                .delete(missing_handlers::delete_accounting_period),
        )
        .route(
            "/accounting-periods/:id/close",
            post(accounting_period_handler::close_period),
        )
        .route(
            "/reports/balance-sheet",
            get(finance_report_handler::get_balance_sheet),
        )
        .route(
            "/reports/income-statement",
            get(finance_report_handler::get_income_statement),
        )
        .route(
            "/reports/cash-flow",
            get(finance_report_handler::get_cash_flow_statement),
        )
        .route(
            "/reports/trial-balance",
            get(finance_report_handler::get_trial_balance),
        )
        .route(
            "/reports/general-ledger/:code",
            get(finance_report_handler::get_general_ledger),
        )
        .route(
            "/reports/subsidiary-ledger",
            get(finance_report_handler::get_subsidiary_ledger),
        )
        // Finance Audit Routes
        .route("/audit/track", post(omni_audit_handler::track_event))
        .route("/audit/stats", get(omni_audit_handler::get_dashboard_stats))
        .route("/audit/search", get(omni_audit_handler::search_logs))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::omni_audit::omni_audit_middleware,
        ));

    // 销售管理路由
    let sales_routes = Router::new()
        .route("/orders", get(sales_order_handler::list_orders))
        .route("/orders", post(sales_order_handler::create_order))
        .route("/orders/:id", get(sales_order_handler::get_order))
        .route("/orders/:id", put(sales_order_handler::update_order))
        .route("/orders/:id", delete(sales_order_handler::delete_order))
        .route(
            "/orders/:id/submit",
            post(sales_order_handler::submit_order),
        )
        .route(
            "/orders/:id/approve",
            post(sales_order_handler::approve_order),
        )
        .route("/orders/:id/ship", post(sales_order_handler::ship_order))
        .route(
            "/orders/:id/complete",
            post(sales_order_handler::complete_order),
        )
        .route(
            "/orders/:id/reject",
            post(sales_order_handler::reject_order),
        )
        .route(
            "/orders/:id/cancel",
            post(sales_order_handler::cancel_order),
        )
        .route(
            "/orders/:id/deliveries",
            get(sales_order_handler::get_order_deliveries)
                .post(sales_order_handler::create_delivery),
        )
        .route(
            "/orders/statistics",
            get(sales_order_handler::get_order_statistics),
        )
        .route(
            "/orders/:id/history",
            get(sales_order_handler::get_order_history),
        )
        .route(
            "/orders/:id/print",
            get(print_handler::sales_order_print_html),
        )
        .route("/orders/export", get(sales_order_handler::export_orders))
        // 面料行业销售订单路由
        .route(
            "/fabric-orders",
            get(sales_fabric_order_handler::list_fabric_orders),
        )
        .route(
            "/fabric-orders",
            post(sales_fabric_order_handler::create_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            get(sales_fabric_order_handler::get_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            put(sales_fabric_order_handler::update_fabric_order),
        )
        .route(
            "/fabric-orders/:id",
            delete(sales_fabric_order_handler::delete_fabric_order),
        )
        .route(
            "/fabric-orders/:id/approve",
            post(sales_fabric_order_handler::approve_fabric_order),
        );

    // 库存管理路由
    let inventory_routes = Router::new()
        .route(
            "/piece-split",
            post(crate::handlers::piece_split_handler::split_fabric_piece),
        )
        .route("/stock", get(inventory_stock_handler::list_stock))
        .route("/stock", post(inventory_stock_handler::create_stock))
        .route("/stock/:id", get(inventory_stock_handler::get_stock))
        .route("/stock/:id", put(inventory_stock_handler::update_stock))
        .route("/stock/:id", delete(inventory_stock_handler::delete_stock))
        // 面料行业库存接口
        .route(
            "/stock/fabric",
            get(inventory_stock_handler::list_stock_fabric),
        )
        .route(
            "/stock/fabric",
            post(inventory_stock_handler::create_stock_fabric),
        )
        .route(
            "/stock/transactions",
            get(inventory_stock_handler::list_transactions),
        )
        .route(
            "/stock/summary",
            get(inventory_stock_handler::get_inventory_summary),
        )
        .route(
            "/stock/low-stock",
            get(inventory_stock_handler::check_low_stock),
        )
        .route(
            "/stock/product/:productId",
            get(inventory_stock_handler::get_stock_by_product),
        )
        .route(
            "/stock/alerts",
            get(inventory_stock_handler::get_stock_alerts),
        )
        .route(
            "/transfers",
            get(inventory_transfer_handler::list_transfers),
        )
        .route(
            "/transfers",
            post(inventory_transfer_handler::create_transfer),
        )
        .route(
            "/transfers/:id",
            get(inventory_transfer_handler::get_transfer),
        )
        .route(
            "/transfers/:id",
            put(inventory_transfer_handler::update_transfer),
        )
        .route(
            "/transfers/:id/approve",
            post(inventory_transfer_handler::approve_transfer),
        )
        .route(
            "/transfers/:id/ship",
            post(inventory_transfer_handler::ship_transfer),
        )
        .route(
            "/transfers/:id/receive",
            post(inventory_transfer_handler::receive_transfer),
        )
        .route(
            "/transfers/:id/print",
            get(print_handler::inventory_transfer_print_html),
        )
        .route("/counts", get(inventory_count_handler::list_counts))
        .route("/counts", post(inventory_count_handler::create_count))
        .route("/counts/:id", get(inventory_count_handler::get_count))
        .route("/counts/:id", put(inventory_count_handler::update_count))
        .route(
            "/counts/:id/approve",
            post(inventory_count_handler::approve_count),
        )
        .route(
            "/counts/:id/complete",
            post(inventory_count_handler::complete_count),
        )
        .route(
            "/counts/:id/print",
            get(print_handler::inventory_count_print_html),
        )
        .route(
            "/adjustments",
            get(inventory_adjustment_handler::list_adjustments),
        )
        .route(
            "/adjustments",
            post(inventory_adjustment_handler::create_adjustment),
        )
        .route(
            "/adjustments/:id",
            get(inventory_adjustment_handler::get_adjustment),
        )
        .route(
            "/adjustments/:id/approve",
            post(inventory_adjustment_handler::approve_adjustment),
        )
        .route(
            "/adjustments/:id/reject",
            post(inventory_adjustment_handler::reject_adjustment),
        )
        // 预留管理路由
        .route(
            "/reservations",
            get(inventory_reservation_handler::list_reservations)
                .post(inventory_reservation_handler::create_reservation),
        )
        .route(
            "/reservations/:id",
            delete(inventory_reservation_handler::delete_reservation),
        );

    // 客户管理路由
    let customer_routes = Router::new()
        .route("/", get(customer_handler::list_customers))
        .route("/", post(customer_handler::create_customer))
        .route("/select", get(customer_handler::list_customers))
        .route("/:id", get(customer_handler::get_customer))
        .route("/:id", put(customer_handler::update_customer))
        .route("/:id", delete(customer_handler::delete_customer))
        .route("/:id/credit", get(customer_credit_handler::get_credit));

    // 批次管理路由（面料行业核心）
    let batch_routes = Router::new()
        .route("/", get(inventory_batch_handler::list_batches))
        .route("/", post(inventory_batch_handler::create_batch))
        .route("/:id", get(inventory_batch_handler::get_batch))
        .route("/:id", put(inventory_batch_handler::update_batch))
        .route("/:id", delete(inventory_batch_handler::delete_batch))
        .route(
            "/:id/transfer",
            post(inventory_batch_handler::transfer_batch),
        );

    // 缸号管理路由（染色批次管理）
    let dye_batch_routes = Router::new()
        .route("/", get(dye_batch_handler::list_dye_batches))
        .route("/", post(dye_batch_handler::create_dye_batch))
        .route("/:id", get(dye_batch_handler::get_dye_batch))
        .route("/:id", put(dye_batch_handler::update_dye_batch))
        .route("/:id", delete(dye_batch_handler::delete_dye_batch))
        .route("/:id/complete", post(dye_batch_handler::complete_dye_batch))
        .route(
            "/by-color/:color_code",
            get(dye_batch_handler::get_dye_batches_by_color),
        )
        .route("/export", get(dye_batch_handler::export_dye_batches));

    // 坯布管理路由（原料布匹管理）
    let greige_fabric_routes = Router::new()
        .route("/", get(greige_fabric_handler::list_greige_fabrics))
        .route("/", post(greige_fabric_handler::create_greige_fabric))
        .route("/:id", get(greige_fabric_handler::get_greige_fabric))
        .route("/:id", put(greige_fabric_handler::update_greige_fabric))
        .route("/:id", delete(greige_fabric_handler::delete_greige_fabric))
        .route("/:id/stock-in", post(greige_fabric_handler::stock_in))
        .route("/:id/stock-out", post(greige_fabric_handler::stock_out))
        .route(
            "/by-supplier/:supplier_id",
            get(greige_fabric_handler::get_greige_by_supplier),
        );

    // 染色配方管理路由
    let dye_recipe_routes = Router::new()
        .route("/", get(dye_recipe_handler::list_dye_recipes))
        .route("/", post(dye_recipe_handler::create_dye_recipe))
        .route("/:id", get(dye_recipe_handler::get_dye_recipe))
        .route("/:id", put(dye_recipe_handler::update_dye_recipe))
        .route("/:id", delete(dye_recipe_handler::delete_dye_recipe))
        .route("/:id/approve", post(dye_recipe_handler::approve_recipe))
        .route("/:id/submit", post(dye_recipe_handler::submit_dye_recipe))
        .route("/:id/version", post(dye_recipe_handler::create_new_version))
        .route(
            "/by-color/:color_code",
            get(dye_recipe_handler::get_recipes_by_color),
        )
        .route(
            "/:id/versions",
            get(dye_recipe_handler::get_recipe_versions),
        )
        .route("/export", get(dye_recipe_handler::export_dye_recipes));

    // 总账管理路由
    let gl_routes = Router::new()
        // 科目管理
        .route("/subjects", get(account_subject_handler::list_subjects))
        .route(
            "/subjects/tree",
            get(account_subject_handler::get_subject_tree),
        )
        .route("/subjects", post(account_subject_handler::create_subject))
        .route("/subjects/:id", get(account_subject_handler::get_subject))
        .route(
            "/subjects/:id",
            put(account_subject_handler::update_subject),
        )
        .route(
            "/subjects/:id",
            delete(account_subject_handler::delete_subject),
        )
        // 凭证管理
        .route("/vouchers/types", get(voucher_handler::get_voucher_types))
        .route("/vouchers", get(voucher_handler::list_vouchers))
        .route("/vouchers/:id", get(voucher_handler::get_voucher))
        .route("/vouchers", post(voucher_handler::create_voucher))
        .route(
            "/vouchers/:id/submit",
            post(voucher_handler::submit_voucher),
        )
        .route(
            "/vouchers/:id/review",
            post(voucher_handler::review_voucher),
        )
        .route("/vouchers/:id/post", post(voucher_handler::post_voucher))
        .route(
            "/vouchers/:id/print",
            get(print_handler::voucher_print_html),
        );

    // 双计量单位换算路由
    let dual_unit_routes = Router::new()
        .route(
            "/convert",
            post(dual_unit_converter_handler::convert_dual_unit),
        )
        .route(
            "/validate",
            post(dual_unit_converter_handler::validate_dual_unit),
        );

    // 五维管理路由
    let five_dimension_routes = Router::new()
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
        );

    // 辅助核算路由
    let assist_accounting_routes = Router::new()
        .route(
            "/dimensions",
            get(assist_accounting_handler::list_assist_dimensions),
        )
        .route(
            "/records",
            get(assist_accounting_handler::query_assist_records),
        )
        .route(
            "/records/business",
            get(assist_accounting_handler::get_assist_records_by_business),
        )
        .route(
            "/records/five-dimension/:five_dimension_id",
            get(assist_accounting_handler::get_assist_records_by_five_dimension),
        )
        .route(
            "/summary",
            get(assist_accounting_handler::get_assist_summary),
        );

    // 业务追溯路由
    let business_trace_routes = Router::new()
        .route(
            "/five-dimension/:five_dimension_id",
            get(business_trace_handler::get_trace_by_five_dimension),
        )
        .route("/forward", get(business_trace_handler::forward_trace))
        .route("/backward", get(business_trace_handler::backward_trace))
        .route(
            "/snapshot/:trace_chain_id",
            post(business_trace_handler::create_trace_snapshot),
        );

    // 供应商管理路由
    let supplier_routes = Router::new()
        .route("/", get(supplier_handler::list_suppliers))
        .route("/", post(supplier_handler::create_supplier))
        .route("/select", get(supplier_handler::list_suppliers))
        .route("/:id", get(supplier_handler::get_supplier))
        .route("/:id", put(supplier_handler::update_supplier))
        .route("/:id", delete(supplier_handler::delete_supplier))
        .route(
            "/:id/status",
            post(supplier_handler::toggle_supplier_status),
        )
        .route(
            "/:id/contacts",
            get(supplier_handler::list_supplier_contacts)
                .post(supplier_handler::create_supplier_contact),
        )
        .route(
            "/:id/contacts/:contact_id",
            put(supplier_handler::update_supplier_contact)
                .delete(supplier_handler::delete_supplier_contact),
        )
        .route(
            "/:id/qualifications",
            get(supplier_handler::list_supplier_qualifications)
                .post(supplier_handler::create_supplier_qualification),
        )
        .route(
            "/:id/evaluate",
            post(supplier_evaluation_handler::create_evaluation_record),
        )
        .route(
            "/:id/evaluations",
            get(supplier_evaluation_handler::list_evaluation_records),
        );

    // 供应商评估路由
    let supplier_evaluation_routes = Router::new()
        .route("/", get(supplier_evaluation_handler::list_evaluations))
        .route("/", post(supplier_evaluation_handler::create_evaluation))
        .route(
            "/suppliers/:supplier_id/score",
            get(supplier_evaluation_handler::get_supplier_score_by_path),
        )
        .route("/:id", get(supplier_evaluation_handler::get_evaluation))
        .route("/:id", put(supplier_evaluation_handler::update_evaluation))
        .route(
            "/:id",
            delete(supplier_evaluation_handler::delete_evaluation),
        )
        .route(
            "/indicators",
            get(supplier_evaluation_handler::list_indicators),
        )
        .route(
            "/indicators",
            post(supplier_evaluation_handler::create_indicator),
        )
        .route("/rankings", get(supplier_evaluation_handler::get_rankings))
        .route(
            "/records",
            get(supplier_evaluation_handler::list_evaluation_records),
        )
        .route(
            "/records",
            post(supplier_evaluation_handler::create_evaluation_record),
        )
        .route(
            "/records/:id",
            get(supplier_evaluation_handler::get_evaluation_record),
        )
        .route(
            "/scores/:supplier_id",
            get(supplier_evaluation_handler::get_supplier_score),
        )
        .route("/ratings", get(supplier_evaluation_handler::list_ratings));

    // 采购管理路由
    let purchase_routes = Router::new()
        .route(
            "/orders/delivery-date",
            post(purchase_order_handler::calculate_delivery_date),
        )
        .route("/orders", get(purchase_order_handler::list_orders))
        .route("/orders", post(purchase_order_handler::create_order))
        .route("/orders/:id", get(purchase_order_handler::get_order))
        .route("/orders/:id", put(purchase_order_handler::update_order))
        .route("/orders/:id", delete(purchase_order_handler::delete_order))
        .route(
            "/orders/:id/approve",
            post(purchase_order_handler::approve_order),
        )
        .route(
            "/orders/:id/submit",
            post(purchase_order_handler::submit_order),
        )
        .route(
            "/orders/:id/reject",
            post(purchase_order_handler::reject_order),
        )
        .route(
            "/orders/:id/close",
            post(purchase_order_handler::close_order),
        )
        .route("/orders/export", get(purchase_order_handler::export_orders))
        .route(
            "/orders/:id/items",
            get(purchase_order_handler::list_order_items)
                .post(purchase_order_handler::create_order_item),
        )
        .route(
            "/orders/:id/items/:item_id",
            put(purchase_order_handler::update_order_item)
                .delete(purchase_order_handler::delete_order_item),
        )
        .route(
            "/orders/:id/print",
            get(print_handler::purchase_order_print_html),
        )
        .route("/receipts", get(purchase_receipt_handler::list_receipts))
        .route(
            "/receipts/:id/print",
            get(print_handler::purchase_receipt_print_html),
        )
        .route("/receipts", post(purchase_receipt_handler::create_receipt))
        .route("/receipts/:id", get(purchase_receipt_handler::get_receipt))
        .route(
            "/receipts/:id",
            put(purchase_receipt_handler::update_receipt),
        )
        .route(
            "/receipts/:id/confirm",
            post(purchase_receipt_handler::confirm_receipt),
        )
        .route(
            "/receipts/:id/items",
            get(purchase_receipt_handler::list_receipt_items)
                .post(purchase_receipt_handler::create_receipt_item),
        )
        .route(
            "/receipts/:id/items/:item_id",
            put(purchase_receipt_handler::update_receipt_item)
                .delete(purchase_receipt_handler::delete_receipt_item),
        )
        .route(
            "/inspections",
            get(purchase_inspection_handler::list_inspections),
        )
        .route(
            "/inspections",
            post(purchase_inspection_handler::create_inspection),
        )
        .route(
            "/inspections/:id",
            get(purchase_inspection_handler::get_inspection),
        )
        .route(
            "/inspections/:id",
            put(purchase_inspection_handler::update_inspection),
        )
        .route(
            "/inspections/:id/complete",
            post(purchase_inspection_handler::complete_inspection),
        )
        .route(
            "/inspections/:id/items",
            get(purchase_inspection_handler::list_inspection_items)
                .post(purchase_inspection_handler::create_inspection_item),
        )
        .route(
            "/inspections/:id/items/:item_id",
            put(purchase_inspection_handler::update_inspection_item)
                .delete(purchase_inspection_handler::delete_inspection_item),
        )
        .route(
            "/returns",
            get(purchase_return_handler::list_purchase_returns),
        )
        .route(
            "/returns",
            post(purchase_return_handler::create_purchase_return),
        )
        .route(
            "/returns/:id",
            get(purchase_return_handler::get_purchase_return),
        )
        .route(
            "/returns/:id",
            put(purchase_return_handler::update_purchase_return),
        )
        .route(
            "/returns/:id",
            delete(purchase_return_handler::delete_purchase_return),
        )
        .route(
            "/returns/:id/submit",
            post(purchase_return_handler::submit_purchase_return),
        )
        .route(
            "/returns/:id/approve",
            post(purchase_return_handler::approve_purchase_return),
        )
        .route(
            "/returns/:id/reject",
            post(purchase_return_handler::reject_purchase_return),
        )
        .route(
            "/returns/:id/items",
            get(purchase_return_handler::list_purchase_return_items),
        )
        .route(
            "/returns/:id/items",
            post(purchase_return_handler::create_purchase_return_item),
        )
        .route(
            "/returns/:id/items/:item_id",
            put(purchase_return_handler::update_purchase_return_item),
        )
        .route(
            "/returns/:id/items/:item_id",
            delete(purchase_return_handler::delete_purchase_return_item),
        );

    // 采购合同路由
    let purchase_contract_routes = Router::new()
        .route("/", get(purchase_contract_handler::list_contracts))
        .route("/", post(purchase_contract_handler::create_contract))
        .route("/:id", get(purchase_contract_handler::get_contract))
        .route("/:id", put(purchase_contract_handler::update_contract))
        .route("/:id", delete(purchase_contract_handler::delete_contract))
        .route(
            "/:id/approve",
            post(purchase_contract_handler::approve_contract),
        )
        .route(
            "/:id/execute",
            put(purchase_contract_handler::execute_contract),
        )
        .route(
            "/:id/cancel",
            put(purchase_contract_handler::cancel_contract),
        );

    // 销售合同路由
    let sales_contract_routes = Router::new()
        .route("/", get(sales_contract_handler::list_contracts))
        .route("/", post(sales_contract_handler::create_contract))
        .route("/:id", get(sales_contract_handler::get_contract))
        .route("/:id", put(sales_contract_handler::update_contract))
        .route("/:id", delete(sales_contract_handler::delete_contract))
        .route(
            "/:id/approve",
            post(sales_contract_handler::approve_contract),
        )
        .route(
            "/:id/execute",
            put(sales_contract_handler::execute_contract),
        )
        .route("/:id/cancel", put(sales_contract_handler::cancel_contract))
        .route("/:id/print", get(print_handler::sales_contract_print_html));

    // 固定资产路由
    let fixed_asset_routes = Router::new()
        .route("/", get(fixed_asset_handler::list_assets))
        .route("/", post(fixed_asset_handler::create_asset))
        .route("/:id", get(fixed_asset_handler::get_asset))
        .route("/:id", put(fixed_asset_handler::update_asset))
        .route("/:id", delete(fixed_asset_handler::delete_asset))
        .route(
            "/:id/depreciate",
            post(fixed_asset_handler::depreciate_asset),
        )
        .route("/:id/dispose", post(fixed_asset_handler::dispose_asset))
        .route(
            "/batch-depreciate",
            post(fixed_asset_handler::batch_depreciate),
        );

    // 预算管理路由
    let budget_management_routes = Router::new()
        .route("/", get(budget_management_handler::list_budgets))
        .route("/", post(budget_management_handler::create_budget))
        .route("/:id", get(budget_management_handler::get_budget))
        .route("/:id", put(budget_management_handler::update_budget))
        .route("/:id", delete(budget_management_handler::delete_budget))
        .route(
            "/:id/approve",
            post(budget_management_handler::approve_budget),
        )
        .route("/adjust", post(budget_management_handler::adjust_budget))
        .route("/items", get(budget_management_handler::list_budget_items))
        .route(
            "/items",
            post(budget_management_handler::create_budget_item),
        )
        .route(
            "/items/:id",
            get(budget_management_handler::get_budget_item),
        )
        .route(
            "/items/:id",
            put(budget_management_handler::update_budget_item),
        )
        .route(
            "/items/:id",
            delete(budget_management_handler::delete_budget_item),
        )
        .route("/plans", get(budget_management_handler::list_plans))
        .route("/plans", post(budget_management_handler::create_plan))
        .route("/plans/:id", get(budget_management_handler::get_plan))
        .route(
            "/plans/:id/approve",
            post(budget_management_handler::approve_plan),
        )
        .route(
            "/plans/:id/execute",
            post(budget_management_handler::execute_plan),
        )
        .route(
            "/plans/:id/executions",
            get(budget_management_handler::get_plan_executions),
        )
        .route(
            "/plans/:id/executions",
            post(budget_management_handler::create_execution),
        )
        .route(
            "/control/:plan_id",
            get(budget_management_handler::get_control),
        )
        .route(
            "/control/:plan_id/data",
            get(budget_management_handler::get_budget_control_data),
        );

    // 客户信用路由
    let customer_credit_routes = Router::new()
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
        .route("/evaluate", post(customer_credit_handler::evaluate_credit));

    // 财务分析路由
    let financial_analysis_routes = Router::new()
        .route("/reports", get(financial_analysis_handler::list_reports))
        .route("/reports", post(financial_analysis_handler::create_report))
        .route("/reports/:id", get(financial_analysis_handler::get_report))
        .route(
            "/reports/:id/execute",
            post(financial_analysis_handler::execute_report),
        )
        .route(
            "/indicators",
            get(financial_analysis_handler::get_indicators)
                .post(financial_analysis_handler::create_indicator),
        )
        .route(
            "/trends",
            get(financial_analysis_handler::get_trends)
                .post(financial_analysis_handler::create_trend),
        );

    // 资金管理路由
    let fund_management_routes = Router::new()
        .route("/accounts", get(fund_management_handler::list_accounts))
        .route("/accounts", post(fund_management_handler::create_account))
        .route("/accounts/:id", get(fund_management_handler::get_account))
        .route(
            "/accounts/:id/deposit",
            post(fund_management_handler::deposit),
        )
        .route(
            "/accounts/:id/withdraw",
            post(fund_management_handler::withdraw),
        )
        .route(
            "/accounts/:id/freeze",
            post(fund_management_handler::freeze_funds),
        )
        .route(
            "/accounts/:id/unfreeze",
            post(fund_management_handler::unfreeze_funds),
        )
        .route(
            "/accounts/:id",
            delete(fund_management_handler::delete_account),
        )
        .route("/transfer", post(fund_management_handler::transfer))
        .route(
            "/transfers",
            get(fund_management_handler::list_transfer_records),
        )
        .route(
            "/transfers/:id",
            get(fund_management_handler::get_transfer_record),
        );

    // 质量检验路由
    let quality_inspection_routes = Router::new()
        .route(
            "/standards",
            get(quality_inspection_handler::list_standards),
        )
        .route(
            "/standards",
            post(quality_inspection_handler::create_standard),
        )
        .route("/records", get(quality_inspection_handler::list_records))
        .route("/records", post(quality_inspection_handler::create_record))
        .route("/records/:id", get(quality_inspection_handler::get_record))
        .route("/defects", get(quality_inspection_handler::list_defects))
        .route(
            "/defects/:id/process",
            post(quality_inspection_handler::process_defect),
        )
        .route(
            "/defects/:id/handle",
            post(quality_inspection_handler::process_defect),
        );

    // 质量标准路由
    let quality_standard_routes = Router::new()
        .route("/", get(quality_standard_handler::list_standards))
        .route("/", post(quality_standard_handler::create_standard))
        .route("/:id", get(quality_standard_handler::get_standard))
        .route("/:id", put(quality_standard_handler::update_standard))
        .route("/:id", delete(quality_standard_handler::delete_standard))
        .route(
            "/:id/versions",
            get(quality_standard_handler::list_versions),
        )
        .route(
            "/:id/versions",
            post(quality_standard_handler::create_version_history),
        )
        .route(
            "/:id/approve",
            post(quality_standard_handler::approve_standard),
        )
        .route(
            "/:id/publish",
            post(quality_standard_handler::publish_standard),
        );

    // 成本归集路由
    let cost_collection_routes = Router::new()
        .route("/", get(cost_collection_handler::list_collections))
        .route("/", post(cost_collection_handler::create_collection))
        .route("/:id", get(cost_collection_handler::get_collection))
        .route("/:id", put(cost_collection_handler::update_collection))
        .route("/:id", delete(cost_collection_handler::delete_collection))
        .route(
            "/:id/audit",
            post(cost_collection_handler::audit_collection),
        )
        .route(
            "/analysis/summary",
            get(cost_collection_handler::get_cost_analysis_summary),
        )
        .route(
            "/analysis/by-batch",
            get(cost_collection_handler::get_cost_by_batch),
        );

    // 销售分析路由
    let sales_analysis_routes = Router::new()
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
        .route(
            "/trend",
            get(sales_analysis_handler::get_trends),
        )
        .route(
            "/export",
            get(sales_analysis_handler::export_analysis),
        )
        .route("/targets", get(sales_analysis_handler::get_targets))
        .route("/targets", post(sales_analysis_handler::create_target))
        .route(
            "/targets/:period",
            put(sales_analysis_handler::update_sales_target),
        );

    // 销售价格路由
    let sales_price_routes = Router::new()
        .route("/", get(sales_price_handler::list_prices))
        .route("/", post(sales_price_handler::create_price))
        .route("/:id", get(sales_price_handler::get_price))
        .route("/:id/approve", post(sales_price_handler::approve_price))
        .route(
            "/history/:product_id",
            get(sales_price_handler::get_price_history),
        )
        .route("/strategies", get(sales_price_handler::list_strategies));

    // 采购价格路由
    let purchase_price_routes = Router::new()
        .route("/", get(purchase_price_handler::list_prices))
        .route("/", post(purchase_price_handler::create_price))
        .route(
            "/history/:product_id",
            get(purchase_price_handler::get_price_history_by_product),
        )
        .route("/:id", get(purchase_price_handler::get_price))
        .route("/:id", put(purchase_price_handler::update_price))
        .route("/:id", delete(purchase_price_handler::delete_price))
        .route("/:id/approve", post(purchase_price_handler::approve_price))
        .route(
            "/:id/history",
            get(purchase_price_handler::get_price_history),
        );

    let sales_return_routes = sales_return_handler::router();

    // 应付账款路由
    let ap_routes = Router::new()
        .route("/invoices", get(ap_invoice_handler::list_ap_invoices))
        .route("/invoices", post(ap_invoice_handler::create_ap_invoice))
        .route("/invoices/:id", get(ap_invoice_handler::get_ap_invoice))
        .route("/invoices/:id", put(ap_invoice_handler::update_ap_invoice))
        .route(
            "/invoices/:id",
            delete(ap_invoice_handler::delete_ap_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(ap_invoice_handler::approve_ap_invoice),
        )
        .route(
            "/invoices/:id/cancel",
            post(ap_invoice_handler::cancel_ap_invoice),
        )
        .route(
            "/invoices/auto-generate",
            post(ap_invoice_handler::auto_generate),
        )
        .route(
            "/invoices/aging",
            get(ap_invoice_handler::get_aging_analysis),
        )
        .route(
            "/invoices/balance",
            get(ap_invoice_handler::get_balance_summary),
        )
        .route(
            "/invoices/statistics",
            get(ap_invoice_handler::get_statistics),
        )
        .route("/payments", get(ap_payment_handler::list_payments))
        .route("/payments", post(ap_payment_handler::create_payment))
        .route("/payments/:id", get(ap_payment_handler::get_payment))
        .route("/payments/:id", put(ap_payment_handler::update_payment))
        .route(
            "/payments/:id/confirm",
            post(ap_payment_handler::confirm_payment),
        )
        .route(
            "/payment-requests",
            get(ap_payment_request_handler::list_requests),
        )
        .route(
            "/payment-requests",
            post(ap_payment_request_handler::create_request),
        )
        .route(
            "/payment-requests/:id",
            get(ap_payment_request_handler::get_request),
        )
        .route(
            "/payment-requests/:id",
            put(ap_payment_request_handler::update_request),
        )
        .route(
            "/payment-requests/:id",
            delete(ap_payment_request_handler::delete_request),
        )
        .route(
            "/payment-requests/:id/submit",
            post(ap_payment_request_handler::submit_request),
        )
        .route(
            "/payment-requests/:id/approve",
            post(ap_payment_request_handler::approve_request),
        )
        .route(
            "/payment-requests/:id/reject",
            post(ap_payment_request_handler::reject_request),
        )
        .route(
            "/verifications",
            get(ap_verification_handler::list_verifications),
        )
        .route(
            "/verifications/:id",
            get(ap_verification_handler::get_verification),
        )
        .route(
            "/verifications/auto",
            post(ap_verification_handler::auto_verify),
        )
        .route(
            "/verifications/manual",
            post(ap_verification_handler::manual_verify),
        )
        .route(
            "/verifications/:id/cancel",
            post(ap_verification_handler::cancel_verification),
        )
        .route(
            "/verifications/unverified/invoices",
            get(ap_verification_handler::get_unverified_invoices),
        )
        .route(
            "/verifications/unverified/payments",
            get(ap_verification_handler::get_unverified_payments),
        )
        .route(
            "/reconciliations",
            get(ap_reconciliation_handler::list_reconciliations),
        )
        .route(
            "/reconciliations/:id",
            get(ap_reconciliation_handler::get_reconciliation),
        )
        .route(
            "/reconciliations/generate",
            post(ap_reconciliation_handler::generate_reconciliation),
        )
        .route(
            "/reconciliations/:id/confirm",
            post(ap_reconciliation_handler::confirm_reconciliation),
        )
        .route(
            "/reconciliations/:id/dispute",
            post(ap_reconciliation_handler::dispute_reconciliation),
        )
        .route(
            "/reconciliations/auto",
            post(ap_reconciliation_handler::auto_reconcile_all),
        )
        .route(
            "/reconciliations/summary",
            get(ap_reconciliation_handler::get_supplier_summary),
        )
        .route(
            "/invoices/:id/relations",
            get(ap_reconciliation_handler::get_invoice_relations),
        )
        .route(
            "/reports/statistics",
            get(ap_report_handler::get_statistics_report),
        )
        .route("/reports/daily", get(ap_report_handler::get_daily_report))
        .route(
            "/reports/monthly",
            get(ap_report_handler::get_monthly_report),
        )
        .route("/reports/aging", get(ap_report_handler::get_aging_report));

    // 应收账款路由
    let ar_routes = Router::new()
        .route("/invoices", get(ar_invoice_handler::list_ar_invoices))
        .route("/invoices", post(ar_invoice_handler::create_ar_invoice))
        .route("/invoices/:id", get(ar_invoice_handler::get_ar_invoice))
        .route("/invoices/:id", put(ar_invoice_handler::update_ar_invoice))
        .route(
            "/invoices/:id",
            delete(ar_invoice_handler::delete_ar_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(ar_invoice_handler::approve_ar_invoice),
        )
        .route(
            "/invoices/:id/cancel",
            post(ar_invoice_handler::cancel_ar_invoice),
        )
        // AR收款路由
        .route("/payments", get(ar_payment_handler::list_payments))
        .route("/payments", post(ar_payment_handler::create_payment))
        .route("/payments/:id", get(ar_payment_handler::get_payment))
        .route("/payments/:id", put(ar_payment_handler::update_payment))
        .route(
            "/payments/:id/confirm",
            post(ar_payment_handler::confirm_payment),
        )
        // AR核销路由
        .route(
            "/verifications",
            get(ar_verification_handler::list_verifications),
        )
        .route(
            "/verifications/:id",
            get(ar_verification_handler::get_verification),
        )
        .route(
            "/verifications/auto",
            post(ar_verification_handler::auto_verify),
        )
        .route(
            "/verifications/manual",
            post(ar_verification_handler::manual_verify),
        )
        .route(
            "/verifications/:id/cancel",
            post(ar_verification_handler::cancel_verification),
        )
        .route(
            "/verifications/unverified/invoices",
            get(ar_verification_handler::get_unverified_invoices),
        )
        .route(
            "/verifications/unverified/payments",
            get(ar_verification_handler::get_unverified_payments),
        )
        // AR报表路由
        .route(
            "/reports/statistics",
            get(ar_report_handler::get_statistics_report),
        )
        .route("/reports/daily", get(ar_report_handler::get_daily_report))
        .route(
            "/reports/monthly",
            get(ar_report_handler::get_monthly_report),
        )
        .route("/reports/aging", get(ar_report_handler::get_aging_report));

    // 系统更新路由
    let system_update_routes = Router::new()
        .route("/check", get(system_update_handler::check_for_updates))
        .route("/update", post(system_update_handler::download_and_update))
        .route("/version", get(system_update_handler::get_version))
        .route("/status", get(system_update_handler::get_update_status))
        .route("/versions", get(system_update_handler::get_backup_versions))
        .route("/rollback", post(system_update_handler::rollback_version))
        .route(
            "/local-releases",
            get(system_update_handler::list_local_releases),
        )
        .route(
            "/local-update",
            post(system_update_handler::apply_local_update),
        )
        .route(
            "/local-check",
            get(system_update_handler::check_for_local_updates),
        )
        .route("/upload", post(system_update_handler::upload_and_update));

    // BPM路由
    let bpm_routes = Router::new()
        .route("/process/start", post(bpm_handler::start_process))
        .route("/tasks/approve", post(bpm_handler::approve_task))
        .route("/tasks", get(bpm_handler::query_tasks))
        .route("/tasks/pending", get(bpm_handler::get_pending_tasks))
        .route("/tasks/completed", get(bpm_handler::get_completed_tasks))
        .route(
            "/business-relation",
            get(bpm_handler::get_business_relation),
        )
        .route(
            "/visualization/:instance_id",
            get(bpm_handler::get_process_visualization),
        )
        // 审批链和监控接口
        .route(
            "/instances/:instance_id/approval-chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/instances/:instance_id/chain",
            get(bpm_handler::get_approval_chain),
        )
        .route(
            "/instances/:instance_id/detail",
            get(bpm_handler::get_instance_detail),
        )
        .route("/monitor/stats", get(bpm_handler::get_monitor_stats))
        .route(
            "/monitor/pending-tasks",
            get(bpm_handler::get_pending_tasks_for_monitor),
        )
        .route(
            "/monitor/instances",
            get(bpm_handler::list_instances_for_monitor),
        )
        .route("/tasks/:task_id/transfer", post(bpm_handler::transfer_task))
        .route("/tasks/:task_id/urge", post(bpm_handler::urge_task))
        .route("/approval/execute", post(bpm_handler::execute_approval))
        // 流程定义路由
        .route(
            "/definitions",
            get(bpm_definition_handler::list_process_definitions)
                .post(bpm_definition_handler::create_process_definition),
        )
        .route(
            "/definitions/:id",
            get(bpm_definition_handler::get_process_definition)
                .put(bpm_definition_handler::update_process_definition)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/definitions/:id/versions",
            get(bpm_definition_handler::list_versions).post(bpm_definition_handler::create_version),
        )
        .route(
            "/definitions/:id/versions/:version/activate",
            post(bpm_definition_handler::activate_version),
        )
        .route(
            "/versions/:version/activate",
            post(bpm_definition_handler::activate_version_by_id),
        )
        // 流程模板路由
        .route("/templates", get(bpm_definition_handler::list_templates))
        .route(
            "/templates/:template_id",
            get(bpm_definition_handler::list_templates)
                .delete(bpm_definition_handler::delete_process_definition),
        )
        .route(
            "/templates/:template_id/create",
            post(bpm_definition_handler::create_from_template),
        );

    // 健康检查路由
    // 扫码出库路由
    let scanner_routes = Router::new().route(
        "/scan-to-ship",
        get(barcode_scanner_handler::scan_to_ship_get)
            .post(barcode_scanner_handler::scan_to_ship_post),
    );

    // 物流管理路由
    let logistics_routes = Router::new()
        .route("/", get(logistics_handler::list_waybills))
        .route("/", post(logistics_handler::create_waybill))
        .route("/:id", get(logistics_handler::get_waybill))
        .route("/:id", put(logistics_handler::update_waybill_status))
        .route("/:id", delete(logistics_handler::delete_waybill));

    let health_routes = Router::new()
        .route("/", get(health_handler::health_check))
        .route("/readiness", get(health_handler::readiness_check))
        .route("/liveness", get(health_handler::liveness_check));

    // 添加 /init/status 路由，用于前端检测系统是否已初始化
    let init_routes = Router::new()
        .route("/status", get(init_handler::get_init_status))
        .route(
            "/test-database",
            post(init_handler::test_database_connection),
        )
        .route("/initialize", post(init_handler::initialize_system))
        .route(
            "/initialize-with-db",
            post(init_handler::initialize_system_with_db),
        );

    Router::new()
        .nest("/api/v1/erp/auth", auth_routes)
        .nest("/api/v1/erp/users", user_routes)
        .nest("/api/v1/erp/logistics", logistics_routes)
        .nest("/api/v1/erp/scanner", scanner_routes)
        .nest("/api/v1/erp/roles", role_routes)
        .nest(
            "/api/v1/erp/permissions",
            Router::new().route("/", get(role_handler::list_permissions)),
        )
        .nest("/api/v1/erp/products", product_routes)
        .nest("/api/v1/erp/product-categories", product_category_routes)
        .nest("/api/v1/erp/warehouses", warehouse_routes)
        .nest("/api/v1/erp/departments", department_routes)
        .nest("/api/v1/erp/dashboard", dashboard_routes)
        .nest("/api/v1/erp/finance", finance_routes)
        .nest("/api/v1/erp/sales", sales_routes)
        .nest("/api/v1/erp/inventory", inventory_routes)
        .nest("/api/v1/erp/customers", customer_routes)
        .nest("/api/v1/erp/batches", batch_routes)
        .nest("/api/v1/erp/dye-batches", dye_batch_routes)
        .nest("/api/v1/erp/greige-fabrics", greige_fabric_routes)
        .nest("/api/v1/erp/dye-recipes", dye_recipe_routes)
        .nest("/api/v1/erp/gl", gl_routes)
        .nest("/api/v1/erp/dual-unit", dual_unit_routes)
        .nest("/api/v1/erp/five-dimension", five_dimension_routes)
        .nest("/api/v1/erp/assist-accounting", assist_accounting_routes)
        .nest("/api/v1/erp/business-trace", business_trace_routes)
        .nest("/api/v1/erp/suppliers", supplier_routes)
        .nest(
            "/api/v1/erp/supplier-evaluation/evaluations",
            supplier_evaluation_routes,
        )
        .nest("/api/v1/erp/purchases", purchase_routes)
        .nest("/api/v1/erp/purchase-contracts", purchase_contract_routes)
        .nest("/api/v1/erp/sales-contracts", sales_contract_routes)
        .nest("/api/v1/erp/fixed-assets", fixed_asset_routes)
        .nest("/api/v1/erp/budgets", budget_management_routes)
        .nest("/api/v1/erp/customer-credits", customer_credit_routes)
        .nest("/api/v1/erp/financial-analysis", financial_analysis_routes)
        .nest("/api/v1/erp/fund-management", fund_management_routes)
        .nest("/api/v1/erp/quality-inspection", quality_inspection_routes)
        .nest("/api/v1/erp/quality-standards", quality_standard_routes)
        .nest("/api/v1/erp/cost-collections", cost_collection_routes)
        .nest("/api/v1/erp/sales-analysis", sales_analysis_routes)
        .nest("/api/v1/erp/sales-prices", sales_price_routes)
        .nest("/api/v1/erp/sales-returns", sales_return_routes)
        .nest("/api/v1/erp/purchase-prices", purchase_price_routes)
        .nest("/api/v1/erp/ap", ap_routes)
        .nest("/api/v1/erp/ar", ar_routes)
        // MRP生产计划路由
        .nest(
            "/api/v1/erp/production",
            Router::new()
                .route(
                    "/orders",
                    get(production_order_handler::list_production_orders)
                        .post(production_order_handler::create_production_order),
                )
                .route(
                    "/orders/:id",
                    get(production_order_handler::get_production_order)
                        .put(production_order_handler::update_production_order)
                        .delete(production_order_handler::delete_production_order),
                )
                .route(
                    "/orders/:id/status",
                    put(production_order_handler::update_production_order_status),
                )
                .route(
                    "/orders/:id/submit-approval",
                    post(production_order_handler::submit_for_approval),
                )
                .route(
                    "/orders/:id/approve",
                    post(production_order_handler::approve_production_order),
                )
                .route(
                    "/orders/:id/progress",
                    post(production_order_handler::update_production_progress),
                )
                .route(
                    "/orders/:id/logs",
                    get(production_order_handler::get_production_order_logs),
                ),
        )
        // BOM物料清单路由
        .nest(
            "/api/v1/erp/boms",
            Router::new()
                .route(
                    "/",
                    get(bom_handler::list_boms).post(bom_handler::create_bom),
                )
                .route(
                    "/:id",
                    get(bom_handler::get_bom)
                        .put(bom_handler::update_bom)
                        .delete(bom_handler::delete_bom),
                )
                .route("/:id/copy", post(bom_handler::copy_bom))
                .route("/:id/default", put(bom_handler::set_default_bom))
                .route("/:id/submit", put(bom_handler::submit_bom))
                .route("/:id/approve", put(bom_handler::approve_bom))
                .route("/:id/tree", get(bom_handler::get_bom_tree))
                .route(
                    "/:id/requirements",
                    post(bom_handler::calculate_bom_requirements),
                )
                .route("/versions/:product_id", get(bom_handler::get_bom_versions)),
        )
        // MRP物料需求计划路由
        .nest(
            "/api/v1/erp/mrp",
            Router::new()
                .route("/calculate", post(mrp_handler::calculate_mrp))
                .route("/results", get(mrp_handler::get_mrp_results))
                .route("/requirements", get(mrp_handler::get_mrp_requirements))
                .route("/convert-orders", post(mrp_handler::convert_to_orders)),
        )
        // MRP历史记录路由
        .nest(
            "/api/v1/erp/mrp/history",
            Router::new()
                .route("/", get(missing_handlers::get_mrp_history))
                .route("/:id", get(missing_handlers::get_mrp_history_detail)),
        )
        // 生产排程路由
        .nest(
            "/api/v1/erp/scheduling",
            Router::new()
                .route("/auto-schedule", post(scheduling_handler::auto_schedule))
                .route("/gantt", get(scheduling_handler::get_gantt_data))
                .route("/conflicts", get(scheduling_handler::detect_conflicts))
                .route("/tasks", get(scheduling_handler::list_scheduled_orders))
                .route(
                    "/tasks/:id/adjust",
                    put(scheduling_handler::adjust_schedule_task),
                )
                .route("/:id", put(scheduling_handler::adjust_schedule))
                .route(
                    "/work-orders",
                    get(scheduling_handler::list_scheduled_orders),
                )
                .route("/history", get(scheduling_handler::get_schedule_history))
                .route("/results/:id", get(scheduling_handler::get_schedule_result))
                .route(
                    "/results/:id/confirm",
                    post(scheduling_handler::confirm_schedule_result),
                ),
        )
        // 产能分析路由
        .nest(
            "/api/v1/erp/capacity",
            Router::new()
                .route("/overview", get(capacity_handler::get_capacity_overview))
                .route("/summary", get(capacity_handler::get_capacity_overview))
                .route("/bottlenecks", get(capacity_handler::get_load_analysis))
                .route("/trend", get(capacity_handler::get_load_analysis))
                .route(
                    "/work-centers",
                    get(capacity_handler::list_work_centers)
                        .post(capacity_handler::create_work_center),
                )
                .route(
                    "/work-centers/:id",
                    put(capacity_handler::update_work_center)
                        .delete(capacity_handler::delete_work_center),
                )
                .route(
                    "/work-centers/:id/forecast",
                    get(capacity_handler::forecast_capacity),
                )
                .route(
                    "/work-centers/:id/available",
                    get(capacity_handler::get_available_capacity),
                )
                .route("/load-analysis", get(capacity_handler::get_load_analysis))
                .route(
                    "/overload-check",
                    get(capacity_handler::check_capacity_overload),
                ),
        )
        // 缺料预警路由
        .nest(
            "/api/v1/erp/material-shortage",
            Router::new()
                .route(
                    "/alerts",
                    get(material_shortage_handler::list_shortage_alerts),
                )
                .route(
                    "/list",
                    get(material_shortage_handler::list_shortage_alerts),
                )
                .route(
                    "/check",
                    post(material_shortage_handler::check_material_shortage),
                )
                .route(
                    "/summary",
                    get(material_shortage_handler::get_shortage_summary),
                )
                .route(
                    "/threshold",
                    get(material_shortage_handler::get_threshold_config)
                        .post(material_shortage_handler::save_threshold_config),
                )
                .route(
                    "/replenishment",
                    get(material_shortage_handler::get_replenishment_suggestions),
                )
                .route(
                    "/:id/status",
                    put(material_shortage_handler::update_shortage_status),
                ),
        )
        // CRM客户管理路由
        .nest(
            "/api/v1/erp/crm/customers",
            Router::new()
                .route(
                    "/",
                    get(crm_customer_handler::list_customers)
                        .post(crm_customer_handler::create_customer),
                )
                .route(
                    "/enhanced",
                    get(crm_customer_handler::list_customers)
                        .post(crm_customer_handler::create_customer),
                )
                .route(
                    "/:id",
                    get(crm_customer_handler::get_customer)
                        .put(crm_customer_handler::update_customer)
                        .delete(crm_customer_handler::delete_customer),
                )
                .route("/:id/tags", post(crm_customer_handler::add_tags))
                .route("/:id/contacts", get(crm_customer_handler::list_contacts)),
        )
        // CRM标签路由
        .nest(
            "/api/v1/erp/crm/tags",
            Router::new()
                .route(
                    "/",
                    get(crm_customer_handler::list_tags).post(crm_customer_handler::create_tag),
                )
                .route("/:id", delete(crm_customer_handler::delete_tag)),
        )
        // CRM公海池路由
        .nest(
            "/api/v1/erp/crm/pool",
            Router::new()
                .route("/", get(crm_pool_handler::list_pool))
                .route("/claim", post(crm_pool_handler::claim_from_pool))
                .route("/recycle", post(crm_pool_handler::recycle_to_pool))
                .route(
                    "/batch-claim",
                    post(crm_pool_handler::batch_claim),
                )
                .route(
                    "/:customer_id/claim",
                    post(crm_pool_handler::claim_specific),
                ),
        )
        // CRM分配路由
        .nest(
            "/api/v1/erp/crm/assignments",
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
                ),
        )
        // CRM销售用户路由
        .route(
            "/api/v1/erp/crm/sales-users",
            get(missing_handlers::get_sales_users),
        )
        // CRM回收规则路由
        .nest(
            "/api/v1/erp/crm/recycle-rules",
            Router::new()
                .route(
                    "/",
                    get(missing_handlers::get_recycle_rules)
                        .post(missing_handlers::create_recycle_rule),
                )
                .route(
                    "/:id",
                    put(missing_handlers::update_recycle_rule)
                        .delete(missing_handlers::delete_recycle_rule),
                ),
        )
        // 应收对账增强路由
        .nest(
            "/api/v1/erp/ar-reconciliations/enhanced",
            Router::new()
                .route(
                    "/auto-match",
                    post(ar_reconciliation_enhanced_handler::auto_match),
                )
                .route(
                    "/aging-report",
                    get(ar_reconciliation_enhanced_handler::aging_report),
                )
                .route(
                    "/:id/details",
                    get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
                )
                .route(
                    "/:id/confirm",
                    post(ar_reconciliation_enhanced_handler::confirm_reconciliation),
                )
                .route(
                    "/:id/dispute",
                    post(ar_reconciliation_enhanced_handler::dispute_reconciliation),
                )
                .route(
                    "/:id/pdf",
                    get(ar_reconciliation_enhanced_handler::export_reconciliation_pdf),
                )
                .route(
                    "/generate",
                    post(ar_reconciliation_enhanced_handler::generate_reconciliation),
                ),
        )
        // 应收对账增强别名路由（与前端 /ar-reconciliation/... 路径对齐）
        .nest(
            "/api/v1/erp/ar-reconciliation",
            Router::new()
                .route(
                    "/auto-reconcile",
                    post(ar_reconciliation_enhanced_handler::auto_match),
                )
                .route(
                    "/auto-reconcile/results",
                    get(ar_reconciliation_enhanced_handler::list_results),
                )
                .route(
                    "/aging-analysis",
                    get(ar_reconciliation_enhanced_handler::aging_report),
                )
                .route(
                    "/:id/details",
                    get(ar_reconciliation_enhanced_handler::get_reconciliation_details),
                )
                .route(
                    "/:id/confirm/send",
                    post(ar_reconciliation_enhanced_handler::send_confirmation),
                )
                .route(
                    "/confirmations",
                    get(ar_reconciliation_enhanced_handler::list_confirmations)
                        .post(ar_reconciliation_enhanced_handler::create_confirmation),
                )
                .route(
                    "/confirmations/:id/status",
                    put(ar_reconciliation_enhanced_handler::update_confirmation_status),
                )
                .route(
                    "/disputes",
                    get(ar_reconciliation_enhanced_handler::list_disputes)
                        .post(ar_reconciliation_enhanced_handler::create_dispute),
                )
                .route(
                    "/disputes/:id",
                    get(ar_reconciliation_enhanced_handler::get_dispute),
                )
                .route(
                    "/disputes/:id/resolve",
                    put(ar_reconciliation_enhanced_handler::resolve_dispute),
                ),
        )
        // 报表增强路由
        .nest(
            "/api/v1/erp/reports/enhanced",
            Router::new()
                .route(
                    "/fields/:template_type",
                    get(report_enhanced_handler::get_available_fields),
                )
                .route(
                    "/templates",
                    get(report_enhanced_handler::list_report_templates)
                        .post(report_enhanced_handler::create_report_template),
                )
                .route(
                    "/templates/:id",
                    get(report_enhanced_handler::get_report_template)
                        .put(report_enhanced_handler::update_report_template)
                        .delete(report_enhanced_handler::delete_report_template),
                )
                .route(
                    "/templates/:id/execute",
                    post(report_enhanced_handler::execute_custom_report),
                )
                .route(
                    "/templates/:id/export",
                    post(report_enhanced_handler::export_template),
                )
                .route(
                    "/templates/:id/preview",
                    get(report_enhanced_handler::preview_template),
                )
                .route("/export/pdf", post(report_enhanced_handler::export_pdf))
                .route("/export/excel", post(report_enhanced_handler::export_excel))
                .route(
                    "/subscriptions",
                    get(report_enhanced_handler::list_subscriptions)
                        .post(report_enhanced_handler::create_subscription),
                )
                .route(
                    "/subscriptions/:id",
                    get(report_enhanced_handler::get_subscription)
                        .put(report_enhanced_handler::update_subscription)
                        .delete(report_enhanced_handler::delete_subscription),
                )
                .route(
                    "/subscriptions/:id/toggle",
                    post(report_enhanced_handler::toggle_subscription),
                )
                .route(
                    "/subscriptions/:id/trigger",
                    post(report_enhanced_handler::trigger_subscription),
                )
                .route(
                    "/subscriptions/:id/send",
                    post(report_enhanced_handler::send_subscription_now),
                ),
        )
        // 导入导出路由
        .nest(
            "/api/v1/erp/import",
            Router::new()
                .route("/csv", post(import_export_handler::import_csv))
                .route("/excel", post(import_export_handler::import_excel))
                .route(
                    "/templates/:import_type",
                    get(import_export_handler::download_template),
                ),
        )
        .nest(
            "/api/v1/erp/export",
            Router::new()
                .route("/csv/:export_type", get(import_export_handler::export_csv))
                .route(
                    "/excel/:export_type",
                    get(import_export_handler::export_excel_type),
                ),
        )
        // 审计日志路由
        .nest(
            "/api/v1/erp/audit",
            Router::new()
                .route("/logs", get(audit_enhanced_handler::list_audit_logs))
                .route(
                    "/logs/export",
                    get(audit_enhanced_handler::export_audit_logs),
                ),
        )
        // 登录安全路由
        .nest(
            "/api/v1/erp/security",
            Router::new()
                .route("/login-logs", get(login_security_handler::list_login_logs))
                .route(
                    "/lock-status",
                    get(login_security_handler::check_lock_status),
                )
                .route("/unlock", post(login_security_handler::unlock_account))
                .route(
                    "/statistics",
                    get(login_security_handler::get_login_statistics),
                )
                .route("/stats", get(login_security_handler::get_login_statistics))
                .route(
                    "/security-alerts",
                    get(login_security_handler::get_security_alerts),
                )
                .route("/alerts", get(login_security_handler::get_security_alerts))
                .route(
                    "/alerts/:id/resolve",
                    post(login_security_handler::resolve_alert),
                )
                .route(
                    "/locked-accounts",
                    get(login_security_handler::get_locked_accounts),
                )
                .route(
                    "/locked-accounts/:id/unlock",
                    post(login_security_handler::unlock_account_by_id),
                )
                .route(
                    "/login-logs/export",
                    get(login_security_handler::export_login_logs),
                ),
        )
        // 邮件路由
        .nest(
            "/api/v1/erp/emails",
            Router::new()
                .route("/send", post(email_handler::send_email))
                .route(
                    "/templates",
                    get(email_handler::list_templates).post(email_handler::create_template),
                )
                .route(
                    "/templates/:id",
                    get(email_handler::get_template)
                        .put(email_handler::update_template)
                        .delete(email_handler::delete_template),
                )
                .route("/records", get(email_handler::get_email_records))
                .route("/statistics", get(email_handler::get_email_statistics)),
        )
        // Webhook集成路由
        .nest(
            "/api/v1/erp/webhooks/integrations",
            Router::new()
                .route(
                    "/",
                    get(webhook_integration_handler::list_integrations)
                        .post(webhook_integration_handler::create_integration),
                )
                .route(
                    "/:id",
                    put(webhook_integration_handler::test_integration)
                        .delete(webhook_integration_handler::delete_integration),
                )
                .route(
                    "/callback",
                    post(webhook_integration_handler::handle_generic_callback),
                )
                .route(
                    "/test/:id",
                    post(webhook_integration_handler::test_integration),
                )
                .route(
                    "/wechat/send",
                    post(webhook_integration_handler::send_wechat_message),
                )
                .route(
                    "/dingtalk/send",
                    post(webhook_integration_handler::send_dingtalk_message),
                ),
        )
        // 租户配置路由
        .nest(
            "/api/v1/erp/tenant/config",
            Router::new()
                .route(
                    "/settings",
                    get(tenant_config_handler::list_configs)
                        .post(tenant_config_handler::set_config),
                )
                .route(
                    "/settings/:key",
                    delete(tenant_config_handler::delete_config),
                )
                .route(
                    "/plans",
                    get(tenant_config_handler::list_plans).post(tenant_config_handler::create_plan),
                )
                .route("/plans/:id", get(tenant_config_handler::get_plan))
                .route("/usage", get(tenant_config_handler::get_usage_statistics)),
        )
        // 租户计费路由
        .nest(
            "/api/v1/erp/tenant/billing",
            Router::new()
                .route("/plan", get(tenant_billing_handler::get_current_plan))
                .route("/upgrade", post(tenant_billing_handler::upgrade_plan))
                .route("/usage", get(tenant_billing_handler::get_usage))
                .route("/invoices", get(tenant_billing_handler::list_invoices))
                .route("/renew", post(tenant_billing_handler::renew_subscription)),
        )
        .nest("/api/v1/erp/bpm", bpm_routes)
        .nest("/api/v1/erp/system-update", system_update_routes)
        .nest("/api/v1/erp/health", health_routes)
        .nest(
            "/api/v1/erp/crm",
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
                ),
        )
        .nest("/api/v1/erp/init", init_routes)
        // 应收对账路由
        .nest(
            "/api/v1/erp/ar-reconciliations",
            Router::new()
                .route(
                    "/",
                    get(ar_reconciliation_handler::list_reconciliations)
                        .post(ar_reconciliation_handler::create_reconciliation),
                )
                .route("/:id", get(ar_reconciliation_handler::get_reconciliation))
                .route(
                    "/:id/status",
                    put(ar_reconciliation_handler::update_reconciliation_status),
                ),
        )
        // 多币种路由
        .nest(
            "/api/v1/erp/currencies",
            Router::new()
                .route("/", get(currency_handler::list_currencies))
                .route("/base", get(currency_handler::get_base_currency))
                .route(
                    "/rates/history",
                    get(currency_enhanced_handler::get_exchange_rate_history),
                )
                .route("/convert", post(currency_enhanced_handler::convert_amount))
                .route("/sync-all", post(currency_enhanced_handler::sync_all_rates))
                .route(
                    "/supported",
                    get(currency_enhanced_handler::get_supported_currencies),
                ),
        )
        .nest(
            "/api/v1/erp/exchange-rates",
            Router::new()
                .route(
                    "/",
                    get(currency_handler::list_exchange_rates)
                        .post(currency_handler::create_exchange_rate),
                )
                .route("/query", get(currency_handler::get_exchange_rate)),
        )
        // AI智能分析路由
        .nest(
            "/api/v1/erp/ai",
            Router::new()
                .route("/forecast-sales", get(ai_analysis_handler::forecast_sales))
                .route(
                    "/optimize-inventory",
                    get(ai_analysis_handler::optimize_inventory),
                )
                .route(
                    "/detect-anomalies",
                    get(ai_analysis_handler::detect_anomalies),
                )
                .route(
                    "/recommendations",
                    get(ai_analysis_handler::get_recommendations),
                ),
        )
        // 报表引擎路由
        .nest(
            "/api/v1/erp/reports",
            Router::new()
                .route("/templates", get(report_engine_handler::list_templates))
                .route("/execute", get(report_engine_handler::execute_report))
                .route("/export", get(report_engine_handler::export_report))
                .route("/aggregate", post(report_engine_handler::aggregate_report))
                .route(
                    "/cache/clear",
                    post(report_engine_handler::clear_report_cache),
                ),
        )
        // 多租户SaaS路由
        .nest(
            "/api/v1/erp/tenants",
            Router::new()
                .route(
                    "/",
                    get(tenant_handler::list_tenants).post(tenant_handler::create_tenant),
                )
                .route("/:id", get(tenant_handler::get_tenant))
                .route("/:id/status", put(tenant_handler::update_tenant_status)),
        )
        // Webhook路由
        .nest(
            "/api/v1/erp/webhooks",
            Router::new()
                .route(
                    "/",
                    get(webhook_handler::list_webhooks).post(webhook_handler::create_webhook),
                )
                .route("/:id", delete(webhook_handler::delete_webhook)),
        )
        // API密钥路由
        .nest(
            "/api/v1/erp/api-keys",
            Router::new()
                .route(
                    "/",
                    get(api_key_handler::list_api_keys).post(api_key_handler::create_api_key),
                )
                .route("/:id/revoke", post(api_key_handler::revoke_api_key)),
        )
        // 数据权限路由
        .nest(
            "/api/v1/erp/data-permissions",
            Router::new()
                .route(
                    "/",
                    get(data_permission_handler::list_data_permissions)
                        .post(data_permission_handler::set_data_permission),
                )
                .route(
                    "/scope-types",
                    get(data_permission_handler::list_scope_types),
                )
                .route(
                    "/roles/:role_id",
                    get(data_permission_handler::list_role_data_permissions),
                )
                .route(
                    "/roles/:role_id/:resource_type",
                    get(data_permission_handler::get_data_permission)
                        .delete(data_permission_handler::delete_data_permission),
                ),
        )
        // 字段权限路由
        .nest(
            "/api/v1/erp/permissions/fields",
            Router::new()
                .route(
                    "/",
                    get(field_permission_handler::list_field_permissions)
                        .post(field_permission_handler::create_field_permission),
                )
                .route(
                    "/:id",
                    get(field_permission_handler::get_field_permission)
                        .put(field_permission_handler::update_field_permission)
                        .delete(field_permission_handler::delete_field_permission),
                ),
        )
        // 消息通知路由
        .nest(
            "/api/v1/erp/notifications",
            Router::new()
                .route("/", get(notification_handler::list_notifications))
                .route("/unread-count", get(notification_handler::get_unread_count))
                .route("/read-all", post(notification_handler::mark_all_as_read))
                .route(
                    "/batch-read",
                    post(notification_handler::batch_mark_as_read),
                )
                .route(
                    "/settings",
                    get(notification_handler::get_settings)
                        .put(notification_handler::update_setting),
                )
                .route("/:id", get(notification_handler::get_notification))
                .route("/:id/read", post(notification_handler::mark_as_read))
                .route("/:id", delete(notification_handler::delete_notification)),
        )
        // 用户通知偏好设置路由
        .nest(
            "/api/v1/erp/user/notification-setting",
            Router::new().route(
                "/",
                get(user_notification_setting_handler::get_setting)
                    .put(user_notification_setting_handler::update_setting),
            ),
        )
        // 交易管理路由
        .nest(
            "/api/v1/erp/trading",
            Router::new()
                // 采购合同
                .route(
                    "/purchase-contracts",
                    get(advanced_handler::list_purchase_contracts)
                        .post(advanced_handler::create_purchase_contract),
                )
                .route(
                    "/purchase-contracts/:id",
                    get(advanced_handler::get_purchase_contract)
                        .put(advanced_handler::update_purchase_contract)
                        .delete(advanced_handler::delete_purchase_contract),
                )
                .route(
                    "/purchase-contracts/:id/approve",
                    post(advanced_handler::approve_purchase_contract),
                )
                .route(
                    "/purchase-contracts/:id/execute",
                    post(advanced_handler::execute_purchase_contract),
                )
                // 采购价格
                .route(
                    "/purchase-prices",
                    get(advanced_handler::list_purchase_prices)
                        .post(advanced_handler::create_purchase_price),
                )
                .route(
                    "/purchase-prices/:id",
                    put(advanced_handler::update_purchase_price)
                        .delete(advanced_handler::delete_purchase_price),
                )
                .route(
                    "/purchase-prices/:id/approve",
                    post(advanced_handler::approve_purchase_price),
                )
                // 销售合同
                .route(
                    "/sales-contracts",
                    get(advanced_handler::list_sales_contracts)
                        .post(advanced_handler::create_sales_contract),
                )
                .route(
                    "/sales-contracts/:id",
                    get(advanced_handler::get_sales_contract)
                        .put(advanced_handler::update_sales_contract)
                        .delete(advanced_handler::delete_sales_contract),
                )
                .route(
                    "/sales-contracts/:id/approve",
                    post(advanced_handler::approve_sales_contract),
                )
                // 销售价格
                .route(
                    "/sales-prices",
                    get(advanced_handler::list_sales_prices)
                        .post(advanced_handler::create_sales_price),
                )
                .route(
                    "/sales-prices/:id",
                    put(advanced_handler::update_sales_price)
                        .delete(advanced_handler::delete_sales_price),
                )
                .route(
                    "/sales-prices/:id/approve",
                    post(advanced_handler::approve_sales_price),
                )
                // 销售退货
                .route(
                    "/sales-returns",
                    get(advanced_handler::list_sales_returns)
                        .post(advanced_handler::create_sales_return),
                )
                .route(
                    "/sales-returns/:id",
                    get(advanced_handler::get_sales_return)
                        .put(advanced_handler::update_sales_return)
                        .delete(advanced_handler::delete_sales_return),
                ),
        )
        // Advanced AI 分析路由
        .nest(
            "/api/v1/erp/advanced",
            Router::new()
                .route("/ai/sales-forecast", post(advanced_handler::sales_forecast))
                .route(
                    "/ai/inventory-optimization",
                    post(advanced_handler::inventory_optimization),
                )
                .route(
                    "/ai/anomaly-detection",
                    post(advanced_handler::anomaly_detection),
                )
                .route(
                    "/ai/recommendations",
                    post(advanced_handler::recommendations),
                )
                // 报表引擎
                .route(
                    "/reports/templates",
                    get(advanced_handler::list_report_templates),
                )
                .route("/reports/execute", post(advanced_handler::execute_report))
                .route("/reports/export", post(advanced_handler::export_report))
                // 多租户
                .route(
                    "/tenants",
                    get(advanced_handler::list_tenants).post(advanced_handler::create_tenant),
                )
                .route(
                    "/tenants/:id",
                    get(advanced_handler::get_tenant).put(advanced_handler::update_tenant),
                ),
        )
        // 页面访问统计路由
        .nest(
            "/api/tracking",
            Router::new().route("/page-view", post(tracking_handler::track_page_view)),
        )
        .nest("/", metrics_routes)
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", crate::docs::ApiDoc::openapi()),
        )
        // 静态文件服务 - CSS样式文件
        .route(
            "/static/*path",
            get({
                move |axum::extract::Path(path): axum::extract::Path<String>| async move {
                    let static_dir = std::env::var("FRONTEND_STATIC_DIR")
                        .unwrap_or_else(|_| "/workspace/frontend/static".to_string());
                    let static_path = format!("{}/{}", static_dir, path);
                    if let Ok(content) = tokio::fs::read(&static_path).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("text/css"),
                        );
                        return Ok::<_, std::convert::Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/static/{}",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string()),
                        path
                    );
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("text/css"),
                        );
                        return Ok(res);
                    }
                    let body = axum::body::Body::from("/* File not found */");
                    Ok(axum::response::Response::builder()
                        .status(axum::http::StatusCode::NOT_FOUND)
                        .body(body)
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to build 404 response: {:?}", e);
                            axum::response::Response::new(axum::body::Body::from("Internal Error"))
                        }))
                }
            }),
        )
        // 前端WASM文件服务
        .route(
            "/bingxi_frontend.js",
            get({
                let wasm_dir = "/workspace/frontend/target/wasm32-unknown-unknown/release";
                move |_req: axum::http::Request<axum::body::Body>| async move {
                    let js_file = format!("{}/bingxi_frontend.js", wasm_dir);
                    if let Ok(content) = tokio::fs::read(&js_file).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("application/javascript"),
                        );
                        res.headers_mut().insert(
                            axum::http::header::CACHE_CONTROL,
                            axum::http::header::HeaderValue::from_static("public, max-age=3600"),
                        );
                        return Ok::<_, std::convert::Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/dist/bingxi_frontend.js",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string())
                    );
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("application/javascript"),
                        );
                        return Ok(res);
                    }
                    let body = axum::body::Body::from("console.log('WASM loader not found')");
                    Ok(axum::response::Response::new(body))
                }
            }),
        )
        .route(
            "/bingxi_frontend_bg.wasm",
            get({
                let wasm_dir = "/workspace/frontend/target/wasm32-unknown-unknown/release";
                move |_req: axum::http::Request<axum::body::Body>| async move {
                    let wasm_file = format!("{}/bingxi_frontend_bg.wasm", wasm_dir);
                    if let Ok(content) = tokio::fs::read(&wasm_file).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("application/wasm"),
                        );
                        res.headers_mut().insert(
                            axum::http::header::CACHE_CONTROL,
                            axum::http::header::HeaderValue::from_static("public, max-age=3600"),
                        );
                        return Ok::<_, std::convert::Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/dist/bingxi_frontend_bg.wasm",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string())
                    );
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = axum::body::Body::from(content);
                        let mut res = axum::response::Response::new(body);
                        res.headers_mut().insert(
                            axum::http::header::CONTENT_TYPE,
                            axum::http::header::HeaderValue::from_static("application/wasm"),
                        );
                        return Ok(res);
                    }
                    let body = axum::body::Body::empty();
                    let mut res = axum::response::Response::new(body);
                    res.headers_mut().insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::HeaderValue::from_static("application/wasm"),
                    );
                    Ok(res)
                }
            }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::rate_limit_by_ip,
        ))
        .with_state(state)
}
