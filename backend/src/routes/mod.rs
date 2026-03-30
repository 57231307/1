use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::handlers::{
    account_subject_handler,
    ap_invoice_handler,
    ap_payment_handler,
    ap_payment_request_handler,
    ap_reconciliation_handler,
    ap_report_handler,
    ap_verification_handler,
    ar_invoice_handler,
    assist_accounting_handler,
    auth_handler,
    batch_handler,
    batch_new_handler,
    budget_management_handler,
    business_trace_handler,
    cost_collection_handler,
    customer_credit_handler,
    customer_handler,
    dashboard_handler,
    department_handler,
    dual_unit_converter_handler,
    dye_batch_handler,
    dye_recipe_handler,
    finance_invoice_handler,
    finance_payment_handler,
    financial_analysis_handler,
    five_dimension_handler,
    fixed_asset_handler,
    fund_management_handler,
    greige_fabric_handler,
    health_handler,
    init_handler,
    inventory_adjustment_handler,
    inventory_count_handler,
    inventory_stock_handler,
    inventory_transfer_handler,
    product_category_handler,
    product_handler,
    purchase_contract_handler,
    purchase_inspection_handler,
    purchase_order_handler,
    purchase_price_handler,
    purchase_receipt_handler,
    purchase_return_handler,
    quality_inspection_handler,
    quality_standard_handler,
    role_handler,
    sales_analysis_handler,
    sales_contract_handler,
    sales_fabric_order_handler,
    sales_order_handler,
    sales_price_handler,
    supplier_evaluation_handler,
    supplier_handler,
    user_handler,
    system_update_handler,
    voucher_handler,
    warehouse_handler,
};

/// 创建路由配置
/// 所有接口路径统一为 /api/v1/erp/* 格式
pub fn create_router(db: Arc<DatabaseConnection>) -> Router {
    // 认证路由
    let auth_routes = Router::new()
        .route("/login", post(auth_handler::login))
        .route("/logout", post(auth_handler::logout))
        .route("/refresh", post(auth_handler::refresh_token));

    // 用户管理路由
    let user_routes = Router::new()
        .route("/", get(user_handler::list_users))
        .route("/", post(user_handler::create_user))
        .route("/:id", get(user_handler::get_user))
        .route("/:id", put(user_handler::update_user))
        .route("/:id", delete(user_handler::delete_user));

    // 角色管理路由
    let role_routes = Router::new()
        .route("/", get(role_handler::list_roles))
        .route("/", post(role_handler::create_role))
        .route("/:id", get(role_handler::get_role))
        .route("/:id", put(role_handler::update_role))
        .route("/:id", delete(role_handler::delete_role))
        .route("/:id/permissions", get(role_handler::get_role_permissions))
        .route("/:id/permissions", post(role_handler::assign_permission))
        .route("/permissions/:id", delete(role_handler::remove_permission));

    // 产品管理路由
    let product_routes = Router::new()
        .route("/", get(product_handler::list_products))
        .route("/", post(product_handler::create_product))
        .route("/:id", get(product_handler::get_product))
        .route("/:id", put(product_handler::update_product))
        .route("/:id", delete(product_handler::delete_product))
        .route("/batch/create", post(batch_handler::batch_create_products))
        .route("/batch/update", post(batch_handler::batch_update_products))
        .route("/batch/delete", post(batch_handler::batch_delete_products))
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
        .route("/", get(product_category_handler::list_product_categories))
        .route("/", post(product_category_handler::create_product_category))
        .route("/:id", get(product_category_handler::get_product_category))
        .route(
            "/:id",
            put(product_category_handler::update_product_category),
        )
        .route(
            "/:id",
            delete(product_category_handler::delete_product_category),
        )
        .route(
            "/tree",
            get(product_category_handler::get_product_category_tree),
        );

    // 仓库管理路由
    let warehouse_routes = Router::new()
        .route("/", get(warehouse_handler::list_warehouses))
        .route("/", post(warehouse_handler::create_warehouse))
        .route("/:id", get(warehouse_handler::get_warehouse))
        .route("/:id", put(warehouse_handler::update_warehouse))
        .route("/:id", delete(warehouse_handler::delete_warehouse))
        // 库位管理路由
        .route("/locations", get(warehouse_handler::list_locations))
        .route("/locations", post(warehouse_handler::create_location))
        .route("/locations/:id", get(warehouse_handler::get_location))
        .route("/locations/:id", put(warehouse_handler::update_location))
        .route("/locations/:id", delete(warehouse_handler::delete_location));

    // 部门管理路由
    let department_routes = Router::new()
        .route("/", get(department_handler::list_departments))
        .route("/", post(department_handler::create_department))
        .route("/:id", get(department_handler::get_department))
        .route("/:id", put(department_handler::update_department))
        .route("/:id", delete(department_handler::delete_department))
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
        .route("/invoices", get(finance_invoice_handler::list_invoices))
        .route("/invoices", post(finance_invoice_handler::create_invoice))
        .route("/invoices/:id", get(finance_invoice_handler::get_invoice))
        .route(
            "/invoices/:id",
            put(finance_invoice_handler::update_invoice),
        )
        .route(
            "/invoices/:id",
            delete(finance_invoice_handler::delete_invoice),
        )
        .route(
            "/invoices/:id/approve",
            post(finance_invoice_handler::approve_invoice),
        )
        .route(
            "/invoices/:id/verify",
            post(finance_invoice_handler::verify_invoice),
        );

    // 销售管理路由
    let sales_routes = Router::new()
        .route("/orders", get(sales_order_handler::list_orders))
        .route("/orders", post(sales_order_handler::create_order))
        .route("/orders/:id", get(sales_order_handler::get_order))
        .route("/orders/:id", put(sales_order_handler::update_order))
        .route("/orders/:id", delete(sales_order_handler::delete_order))
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
        );

    // 客户管理路由
    let customer_routes = Router::new()
        .route("/", get(customer_handler::list_customers))
        .route("/", post(customer_handler::create_customer))
        .route("/:id", get(customer_handler::get_customer))
        .route("/:id", put(customer_handler::update_customer))
        .route("/:id", delete(customer_handler::delete_customer));

    // 批次管理路由（面料行业核心）
    let batch_routes = Router::new()
        .route("/", get(batch_new_handler::list_batches))
        .route("/", post(batch_new_handler::create_batch))
        .route("/:id", get(batch_new_handler::get_batch))
        .route("/:id", put(batch_new_handler::update_batch))
        .route("/:id", delete(batch_new_handler::delete_batch))
        .route("/:id/transfer", post(batch_new_handler::transfer_batch));

    // 缸号管理路由（染色批次管理）
    let dye_batch_routes = Router::new()
        .route("/", get(dye_batch_handler::list_dye_batches))
        .route("/", post(dye_batch_handler::create_dye_batch))
        .route("/:id", get(dye_batch_handler::get_dye_batch))
        .route("/:id", put(dye_batch_handler::update_dye_batch))
        .route("/:id", delete(dye_batch_handler::delete_dye_batch))
        .route("/:id/complete", post(dye_batch_handler::complete_dye_batch))
        .route("/by-color/:color_code", get(dye_batch_handler::get_dye_batches_by_color));

    // 坯布管理路由（原料布匹管理）
    let greige_fabric_routes = Router::new()
        .route("/", get(greige_fabric_handler::list_greige_fabrics))
        .route("/", post(greige_fabric_handler::create_greige_fabric))
        .route("/:id", get(greige_fabric_handler::get_greige_fabric))
        .route("/:id", put(greige_fabric_handler::update_greige_fabric))
        .route("/:id", delete(greige_fabric_handler::delete_greige_fabric))
        .route("/:id/stock-in", post(greige_fabric_handler::stock_in))
        .route("/:id/stock-out", post(greige_fabric_handler::stock_out))
        .route("/by-supplier/:supplier_id", get(greige_fabric_handler::get_greige_by_supplier));

    // 染色配方管理路由
    let dye_recipe_routes = Router::new()
        .route("/", get(dye_recipe_handler::list_dye_recipes))
        .route("/", post(dye_recipe_handler::create_dye_recipe))
        .route("/:id", get(dye_recipe_handler::get_dye_recipe))
        .route("/:id", put(dye_recipe_handler::update_dye_recipe))
        .route("/:id", delete(dye_recipe_handler::delete_dye_recipe))
        .route("/:id/approve", post(dye_recipe_handler::approve_recipe))
        .route("/:id/version", post(dye_recipe_handler::create_new_version))
        .route("/by-color/:color_code", get(dye_recipe_handler::get_recipes_by_color))
        .route("/:id/versions", get(dye_recipe_handler::get_recipe_versions));

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
        .route("/vouchers/:id/post", post(voucher_handler::post_voucher));

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
        .route("/:id", get(supplier_handler::get_supplier))
        .route("/:id", put(supplier_handler::update_supplier))
        .route("/:id", delete(supplier_handler::delete_supplier))
        .route(
            "/:id/toggle-status",
            post(supplier_handler::toggle_supplier_status),
        )
        // 联系人管理
        .route(
            "/:id/contacts",
            get(supplier_handler::list_supplier_contacts),
        )
        .route(
            "/:id/contacts",
            post(supplier_handler::create_supplier_contact),
        )
        .route(
            "/:id/contacts/:contact_id",
            put(supplier_handler::update_supplier_contact),
        )
        .route(
            "/:id/contacts/:contact_id",
            delete(supplier_handler::delete_supplier_contact),
        )
        // 资质管理
        .route(
            "/:id/qualifications",
            get(supplier_handler::list_supplier_qualifications),
        )
        .route(
            "/:id/qualifications",
            post(supplier_handler::create_supplier_qualification),
        );

    // 采购管理路由
    let purchase_order_routes = Router::new()
        .route("/", get(purchase_order_handler::list_orders))
        .route("/", post(purchase_order_handler::create_order))
        .route("/:id", get(purchase_order_handler::get_order))
        .route("/:id", put(purchase_order_handler::update_order))
        .route("/:id", delete(purchase_order_handler::delete_order))
        .route("/:id/submit", post(purchase_order_handler::submit_order))
        .route("/:id/approve", post(purchase_order_handler::approve_order))
        .route("/:id/reject", post(purchase_order_handler::reject_order))
        .route("/:id/close", post(purchase_order_handler::close_order))
        .route("/:id/items", get(purchase_order_handler::list_order_items))
        .route(
            "/:id/items",
            post(purchase_order_handler::create_order_item),
        )
        .route(
            "/:id/items/:item_id",
            put(purchase_order_handler::update_order_item),
        )
        .route(
            "/:id/items/:item_id",
            delete(purchase_order_handler::delete_order_item),
        );

    let purchase_receipt_routes = Router::new()
        .route("/", get(purchase_receipt_handler::list_receipts))
        .route("/", post(purchase_receipt_handler::create_receipt))
        .route("/:id", get(purchase_receipt_handler::get_receipt))
        .route("/:id", put(purchase_receipt_handler::update_receipt))
        .route(
            "/:id/confirm",
            post(purchase_receipt_handler::confirm_receipt),
        )
        .route(
            "/:id/items",
            get(purchase_receipt_handler::list_receipt_items),
        )
        .route(
            "/:id/items",
            post(purchase_receipt_handler::create_receipt_item),
        )
        .route(
            "/:id/items/:item_id",
            put(purchase_receipt_handler::update_receipt_item),
        )
        .route(
            "/:id/items/:item_id",
            delete(purchase_receipt_handler::delete_receipt_item),
        );

    let purchase_return_routes = Router::new()
        .route("/", get(purchase_return_handler::list_returns))
        .route("/", post(purchase_return_handler::create_return))
        .route("/:id", get(purchase_return_handler::get_return))
        .route("/:id", put(purchase_return_handler::update_return))
        .route("/:id/submit", post(purchase_return_handler::submit_return))
        .route(
            "/:id/approve",
            post(purchase_return_handler::approve_return),
        )
        .route("/:id/reject", post(purchase_return_handler::reject_return));

    let purchase_inspection_routes = Router::new()
        .route("/", get(purchase_inspection_handler::list_inspections))
        .route("/", post(purchase_inspection_handler::create_inspection))
        .route("/:id", get(purchase_inspection_handler::get_inspection))
        .route("/:id", put(purchase_inspection_handler::update_inspection))
        .route(
            "/:id/complete",
            post(purchase_inspection_handler::complete_inspection),
        );

    // 应收账款管理路由
    let ar_invoice_routes = Router::new()
        .route("/", get(ar_invoice_handler::list_invoices))
        .route("/", post(ar_invoice_handler::create_invoice));

    // 成本管理路由
    let cost_collection_routes = Router::new()
        .route("/", get(cost_collection_handler::list_collections))
        .route("/", post(cost_collection_handler::create_collection));

    // P1 模块路由
    // 固定资产管理路由
    let fixed_asset_routes = Router::new()
        .route("/", get(fixed_asset_handler::list_assets))
        .route("/", post(fixed_asset_handler::create_asset))
        .route("/:id", get(fixed_asset_handler::get_asset))
        .route("/:id", delete(fixed_asset_handler::delete_asset))
        .route(
            "/:id/depreciate",
            post(fixed_asset_handler::depreciate_asset),
        )
        .route("/:id/dispose", post(fixed_asset_handler::dispose_asset));

    // 采购合同管理路由
    let purchase_contract_routes = Router::new()
        .route("/", get(purchase_contract_handler::list_contracts))
        .route("/", post(purchase_contract_handler::create_contract))
        .route("/:id", get(purchase_contract_handler::get_contract))
        .route(
            "/:id/approve",
            post(purchase_contract_handler::approve_contract),
        )
        .route(
            "/:id/execute",
            post(purchase_contract_handler::execute_contract),
        )
        .route(
            "/:id/cancel",
            post(purchase_contract_handler::cancel_contract),
        );

    // 销售合同管理路由
    let sales_contract_routes = Router::new()
        .route("/", get(sales_contract_handler::list_contracts))
        .route("/", post(sales_contract_handler::create_contract))
        .route("/:id", get(sales_contract_handler::get_contract))
        .route(
            "/:id/approve",
            post(sales_contract_handler::approve_contract),
        )
        .route(
            "/:id/execute",
            post(sales_contract_handler::execute_contract),
        )
        .route("/:id/cancel", post(sales_contract_handler::cancel_contract));

    // 客户信用管理路由
    let customer_credit_routes = Router::new()
        .route("/", get(customer_credit_handler::list_credits))
        .route("/rating", post(customer_credit_handler::set_credit_rating))
        .route("/:customer_id", get(customer_credit_handler::get_credit))
        .route(
            "/:customer_id/occupy",
            post(customer_credit_handler::occupy_credit),
        )
        .route(
            "/:customer_id/release",
            post(customer_credit_handler::release_credit),
        )
        .route(
            "/:customer_id/adjust",
            post(customer_credit_handler::adjust_credit_limit),
        )
        .route(
            "/:customer_id/deactivate",
            post(customer_credit_handler::deactivate_credit),
        );

    // P1 模块路由
    // 资金管理路由
    let fund_management_routes = Router::new()
        .route("/", get(fund_management_handler::list_accounts))
        .route("/", post(fund_management_handler::create_account))
        .route("/:id", get(fund_management_handler::get_account))
        .route("/:id/deposit", post(fund_management_handler::deposit))
        .route("/:id/withdraw", post(fund_management_handler::withdraw))
        .route("/:id/freeze", post(fund_management_handler::freeze_funds))
        .route(
            "/:id/unfreeze",
            post(fund_management_handler::unfreeze_funds),
        )
        .route("/:id", delete(fund_management_handler::delete_account));

    // 预算管理路由
    let budget_management_routes = Router::new()
        .route("/items", get(budget_management_handler::list_items))
        .route("/items", post(budget_management_handler::create_item))
        .route("/items/:id", get(budget_management_handler::get_item))
        .route("/items/:id", put(budget_management_handler::update_item))
        .route("/items/:id", delete(budget_management_handler::delete_item))
        .route("/plans", get(budget_management_handler::list_plans))
        .route("/plans", post(budget_management_handler::create_plan))
        .route("/plans/:id", get(budget_management_handler::get_plan))
        .route(
            "/plans/:id/execute",
            post(budget_management_handler::execute_plan),
        )
        .route(
            "/plans/:id/executions",
            post(budget_management_handler::create_execution),
        )
        .route("/controls/:id", get(budget_management_handler::get_control))
        .route(
            "/controls/:id/data",
            get(budget_management_handler::get_budget_control_data),
        )
        .route(
            "/controls/:id/executions",
            get(budget_management_handler::get_plan_executions),
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
            "/:id/approve",
            post(quality_standard_handler::approve_standard),
        )
        .route(
            "/:id/publish",
            post(quality_standard_handler::publish_standard),
        );

    // P2 模块路由
    // 财务分析路由
    let financial_analysis_routes = Router::new()
        .route(
            "/indicators",
            get(financial_analysis_handler::list_indicators),
        )
        .route(
            "/indicators",
            post(financial_analysis_handler::create_indicator),
        )
        .route(
            "/results",
            post(financial_analysis_handler::create_analysis_result),
        )
        .route("/trends", get(financial_analysis_handler::get_trends));

    // 供应商评估路由
    let supplier_evaluation_routes = Router::new()
        .route(
            "/indicators",
            get(supplier_evaluation_handler::list_indicators),
        )
        .route(
            "/indicators",
            post(supplier_evaluation_handler::create_indicator),
        )
        .route(
            "/evaluations",
            post(supplier_evaluation_handler::create_evaluation_record),
        )
        .route(
            "/evaluations",
            get(supplier_evaluation_handler::list_evaluation_records),
        )
        .route(
            "/evaluations/:id",
            get(supplier_evaluation_handler::get_evaluation_record),
        )
        .route(
            "/scores/:supplier_id",
            get(supplier_evaluation_handler::get_supplier_score),
        )
        .route("/ratings", get(supplier_evaluation_handler::list_ratings))
        .route("/rankings", get(supplier_evaluation_handler::get_rankings));
    // 待实现：供应商评估管理（评估记录CRUD、评分计算、排名）
    // 路由：GET/POST /evaluations, GET/PUT/DELETE /evaluations/:id, POST /suppliers/:id/calculate, GET /rankings
    // 状态：等待供应商评估数据模型完善后实现

    // 采购价格路由
    let purchase_price_routes = Router::new()
        .route("/", get(purchase_price_handler::list_prices))
        .route("/", post(purchase_price_handler::create_price))
        .route("/:id", get(purchase_price_handler::get_price))
        .route("/:id", put(purchase_price_handler::update_price))
        .route("/:id", delete(purchase_price_handler::delete_price))
        .route("/:id/approve", post(purchase_price_handler::approve_price))
        .route(
            "/history/:material_id",
            get(purchase_price_handler::get_price_history),
        );

    // 销售价格路由
    let sales_price_routes = Router::new()
        .route("/", get(sales_price_handler::list_prices))
        .route("/", post(sales_price_handler::create_price))
        .route("/:id", get(sales_price_handler::get_price))
        .route("/:id/approve", post(sales_price_handler::approve_price))
        .route("/strategies", get(sales_price_handler::list_strategies))
        .route(
            "/history/:product_id",
            get(sales_price_handler::get_price_history),
        );

    // 销售分析路由
    let sales_analysis_routes = Router::new()
        .route("/stats", get(sales_analysis_handler::list_statistics))
        .route("/trends", get(sales_analysis_handler::get_trends))
        .route("/rankings", get(sales_analysis_handler::get_rankings))
        .route("/targets", get(sales_analysis_handler::get_targets))
        .route("/targets", post(sales_analysis_handler::create_target));

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
            "/defects/:id",
            post(quality_inspection_handler::process_defect),
        );

    // 应付管理路由
    let ap_invoice_routes = Router::new()
        .route("/", get(ap_invoice_handler::list_invoices))
        .route("/", post(ap_invoice_handler::create_invoice))
        .route("/:id", get(ap_invoice_handler::get_invoice))
        .route("/:id", put(ap_invoice_handler::update_invoice))
        .route("/:id", delete(ap_invoice_handler::delete_invoice))
        .route("/:id/approve", post(ap_invoice_handler::approve_invoice))
        .route("/:id/cancel", post(ap_invoice_handler::cancel_invoice))
        .route("/auto-generate", post(ap_invoice_handler::auto_generate))
        .route("/aging", get(ap_invoice_handler::get_aging_analysis))
        .route(
            "/balance-summary",
            get(ap_invoice_handler::get_balance_summary),
        )
        .route("/statistics", get(ap_invoice_handler::get_statistics));

    let ap_payment_request_routes = Router::new()
        .route("/", get(ap_payment_request_handler::list_requests))
        .route("/", post(ap_payment_request_handler::create_request))
        .route("/:id", get(ap_payment_request_handler::get_request))
        .route("/:id", put(ap_payment_request_handler::update_request))
        .route("/:id", delete(ap_payment_request_handler::delete_request))
        .route(
            "/:id/submit",
            post(ap_payment_request_handler::submit_request),
        )
        .route(
            "/:id/approve",
            post(ap_payment_request_handler::approve_request),
        )
        .route(
            "/:id/reject",
            post(ap_payment_request_handler::reject_request),
        );

    let ap_payment_routes = Router::new()
        .route("/", get(ap_payment_handler::list_payments))
        .route("/", post(ap_payment_handler::create_payment))
        .route("/:id", get(ap_payment_handler::get_payment))
        .route("/:id", put(ap_payment_handler::update_payment))
        .route("/:id/confirm", post(ap_payment_handler::confirm_payment))
        .route("/schedule", get(ap_payment_handler::get_payment_schedule));

    let ap_verification_routes = Router::new()
        .route("/", get(ap_verification_handler::list_verifications))
        .route("/auto", post(ap_verification_handler::auto_verify))
        .route("/manual", post(ap_verification_handler::manual_verify))
        .route("/:id", get(ap_verification_handler::get_verification))
        .route(
            "/:id/cancel",
            post(ap_verification_handler::cancel_verification),
        )
        .route(
            "/unverified-invoices",
            get(ap_verification_handler::get_unverified_invoices),
        )
        .route(
            "/unverified-payments",
            get(ap_verification_handler::get_unverified_payments),
        );

    // 供应商对账路由
    let ap_reconciliation_routes = Router::new()
        .route("/", get(ap_reconciliation_handler::list_reconciliations))
        .route(
            "/",
            post(ap_reconciliation_handler::generate_reconciliation),
        )
        .route("/:id", get(ap_reconciliation_handler::get_reconciliation))
        .route(
            "/:id/confirm",
            post(ap_reconciliation_handler::confirm_reconciliation),
        )
        .route(
            "/:id/dispute",
            post(ap_reconciliation_handler::dispute_reconciliation),
        )
        .route(
            "/summary",
            get(ap_reconciliation_handler::get_supplier_summary),
        );

    // 应付报表路由
    let ap_report_routes = Router::new()
        .route("/statistics", get(ap_report_handler::get_statistics_report))
        .route("/daily", get(ap_report_handler::get_daily_report))
        .route("/monthly", get(ap_report_handler::get_monthly_report))
        .route("/aging", get(ap_report_handler::get_aging_report));

    // 系统更新路由
    let system_update_routes = Router::new()
        .route("/version", get(system_update_handler::get_version))
        .route("/status", get(system_update_handler::get_update_status))
        .route("/upload", post(system_update_handler::upload_and_update))
        .route("/backups", get(system_update_handler::get_backup_versions))
        .route("/rollback", post(system_update_handler::rollback_version))
        .route("/check", get(system_update_handler::check_for_updates))
        .route("/download", post(system_update_handler::download_and_update))
        .route("/local/check", get(system_update_handler::check_for_local_updates))
        .route("/local/releases", get(system_update_handler::list_local_releases))
        .route("/local/apply", post(system_update_handler::apply_local_update));

    // 健康检查路由
    let health_routes = Router::new()
        .route("/", get(health_handler::health_check))
        .route("/ready", get(health_handler::readiness_check))
        .route("/live", get(health_handler::liveness_check));

    // 初始化路由
    let init_routes = Router::new()
        .route("/status", get(init_handler::get_init_status))
        .route("/test-database", post(init_handler::test_database_connection))
        .route("/initialize", post(init_handler::initialize_system))
        .route("/initialize-with-db", post(init_handler::initialize_system_with_db))
        .route("/reset-password", post(init_handler::reset_admin_password));

    // API v1 版本路由
    let api_v1_routes = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/roles", role_routes)
        .nest("/products", product_routes)
        .nest("/product-categories", product_category_routes)
        .nest("/warehouses", warehouse_routes)
        .nest("/departments", department_routes)
        .nest("/dashboard", dashboard_routes)
        .nest("/finance", finance_routes)
        .nest("/sales", sales_routes)
        .nest("/inventory", inventory_routes)
        .nest("/customers", customer_routes)
        .nest("/batches", batch_routes)
        .nest("/gl", gl_routes)
        .nest("/dual-unit", dual_unit_routes)
        .nest("/five-dimension", five_dimension_routes)
        .nest("/assist-accounting", assist_accounting_routes)
        .nest("/business-trace", business_trace_routes)
        .nest("/suppliers", supplier_routes)
        .nest("/purchases/orders", purchase_order_routes)
        .nest("/purchases/receipts", purchase_receipt_routes)
        .nest("/purchases/returns", purchase_return_routes)
        .nest("/purchases/inspections", purchase_inspection_routes)
        .nest("/ap/invoices", ap_invoice_routes)
        .nest("/ap/payment-requests", ap_payment_request_routes)
        .nest("/ap/payments", ap_payment_routes)
        .nest("/ap/verifications", ap_verification_routes)
        .nest("/ap/reconciliations", ap_reconciliation_routes)
        .nest("/ap/reports", ap_report_routes)
        .nest("/ar/invoices", ar_invoice_routes)
        .nest("/cost/collections", cost_collection_routes)
        // P1 模块路由
        .nest("/fa/assets", fixed_asset_routes)
        .nest("/purchases/contracts", purchase_contract_routes)
        .nest("/sales/contracts", sales_contract_routes)
        .nest("/credits/customers", customer_credit_routes)
        .nest("/fund/accounts", fund_management_routes)
        .nest("/budget/items", budget_management_routes)
        .nest("/quality/standards", quality_standard_routes)
        // P2 模块路由
        .nest("/financial-analysis", financial_analysis_routes)
        .nest("/supplier-evaluation", supplier_evaluation_routes)
        .nest("/purchases/prices", purchase_price_routes)
        .nest("/sales/prices", sales_price_routes)
        .nest("/sales-analysis", sales_analysis_routes)
        .nest("/quality-inspections", quality_inspection_routes)
        // 面料行业核心模块路由
        .nest("/dye-batches", dye_batch_routes)
        .nest("/greige-fabrics", greige_fabric_routes)
        .nest("/dye-recipes", dye_recipe_routes)
        .nest("/system-update", system_update_routes)
        // 初始化和健康检查路由（也支持通过 /api/v1/erp/ 访问）
        .nest("/health", health_routes.clone())
        .nest("/init", init_routes.clone());

    // 公共路由（不需要认证）
    let public_routes = Router::new()
        .nest("/health", health_routes)
        .nest("/init", init_routes);

    // 统一接口前缀：/api/v1/erp/
    Router::new()
        .nest("/api/v1/erp", api_v1_routes)
        .nest("/", public_routes)
        .with_state(db)
}
