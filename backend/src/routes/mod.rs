use axum::{
    routing::{delete, get, post, put},
    Router,
};
use crate::utils::app_state::AppState;

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
pub fn create_router(state: AppState) -> Router {
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
        .route("/:id", put(batch_new_handler::update