//! 产品目录域路由
//!
//! 处理产品、产品类别、仓库、BOM 物料清单等目录相关接口。
//!
//! 路由设计说明：所有子 router 内部 path 都已加上各自独立前缀
//!（`/products`、`/categories`、`/warehouses`、`/boms`），这样
//! `routes()` 入口用 `merge` 组合时不会出现 path+method 重叠，
//! 避免 axum 0.7 `Overlapping method route` panic。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    bom_handler, bulk_product_handler, chemical_handler, product_category_handler, product_handler,
    warehouse_handler,
};

/// 产品路由（path 前缀 /products）
pub fn products() -> Router<AppState> {
    Router::new()
        .route("/products", get(product_handler::list_products))
        .route("/products", post(product_handler::create_product))
        .route("/products/select", get(product_handler::list_products))
        .route("/products/:id", get(product_handler::get_product))
        .route("/products/:id", put(product_handler::update_product))
        .route("/products/:id", delete(product_handler::delete_product))
        .route(
            "/products/batch/create",
            post(bulk_product_handler::batch_create_products),
        )
        .route(
            "/products/batch/update",
            post(bulk_product_handler::batch_update_products),
        )
        .route(
            "/products/batch/delete",
            post(bulk_product_handler::batch_delete_products),
        )
        .route("/products/export", get(product_handler::export_products))
        .route("/products/import", post(product_handler::import_products))
        .route(
            "/products/import-template",
            get(product_handler::get_product_import_template),
        )
        .route(
            "/products/:product_id/colors",
            get(product_handler::list_product_colors),
        )
        .route(
            "/products/:product_id/colors",
            post(product_handler::create_product_color),
        )
        .route(
            "/products/:product_id/colors/:color_id",
            put(product_handler::update_product_color),
        )
        .route(
            "/products/:product_id/colors/:color_id",
            delete(product_handler::delete_product_color),
        )
        .route(
            "/products/:product_id/colors/batch",
            post(product_handler::batch_create_colors),
        )
}

/// 产品类别路由（path 前缀 /categories）
pub fn product_categories() -> Router<AppState> {
    Router::new()
        .route("/categories", get(product_category_handler::list))
        .route("/categories", post(product_category_handler::create))
        .route("/categories/:id", get(product_category_handler::get))
        .route("/categories/:id", put(product_category_handler::update))
        .route("/categories/:id", delete(product_category_handler::delete))
        .route(
            "/categories/tree",
            get(product_category_handler::get_product_category_tree),
        )
}

/// 仓库路由（path 前缀 /warehouses）
pub fn warehouses() -> Router<AppState> {
    Router::new()
        .route("/warehouses", get(warehouse_handler::list))
        .route("/warehouses", post(warehouse_handler::create))
        .route("/warehouses/select", get(warehouse_handler::list))
        .route("/warehouses/:id", get(warehouse_handler::get))
        .route("/warehouses/:id", put(warehouse_handler::update))
        .route("/warehouses/:id", delete(warehouse_handler::delete))
        .route(
            "/warehouses/locations",
            get(warehouse_handler::list_locations),
        )
        .route(
            "/warehouses/locations",
            post(warehouse_handler::create_location),
        )
        .route(
            "/warehouses/locations/:id",
            get(warehouse_handler::get_location),
        )
        .route(
            "/warehouses/locations/:id",
            put(warehouse_handler::update_location),
        )
        .route(
            "/warehouses/locations/:id",
            delete(warehouse_handler::delete_location),
        )
}

/// BOM 物料清单路由（path 前缀 /boms）
pub fn boms() -> Router<AppState> {
    Router::new()
        .route(
            "/boms",
            get(bom_handler::list_boms).post(bom_handler::create_bom),
        )
        .route(
            "/boms/:id",
            get(bom_handler::get_bom)
                .put(bom_handler::update_bom)
                .delete(bom_handler::delete_bom),
        )
        .route("/boms/:id/copy", post(bom_handler::copy_bom))
        .route("/boms/:id/default", put(bom_handler::set_default_bom))
        .route("/boms/:id/submit", put(bom_handler::submit_bom))
        .route("/boms/:id/approve", put(bom_handler::approve_bom))
        .route("/boms/:id/tree", get(bom_handler::get_bom_tree))
        .route(
            "/boms/:id/requirements",
            post(bom_handler::calculate_bom_requirements),
        )
        .route(
            "/boms/versions/:product_id",
            get(bom_handler::get_bom_versions),
        )
}

/// 染化料主数据路由（path 前缀 /chemicals）
pub fn chemicals() -> Router<AppState> {
    Router::new()
        // 染化料主数据
        .route(
            "/chemicals",
            get(chemical_handler::list_chemicals).post(chemical_handler::create_chemical),
        )
        .route(
            "/chemicals/by-code/:code",
            get(chemical_handler::get_chemical_by_code),
        )
        .route(
            "/chemicals/:id",
            get(chemical_handler::get_chemical)
                .put(chemical_handler::update_chemical)
                .delete(chemical_handler::delete_chemical),
        )
        // 染化料分类
        .route(
            "/chemical-categories",
            get(chemical_handler::list_chemical_categories)
                .post(chemical_handler::create_chemical_category),
        )
        .route(
            "/chemical-categories/tree",
            get(chemical_handler::get_chemical_category_tree),
        )
        .route(
            "/chemical-categories/:id",
            get(chemical_handler::get_chemical_category)
                .put(chemical_handler::update_chemical_category)
                .delete(chemical_handler::delete_chemical_category),
        )
        // 染化料批次
        .route(
            "/chemical-lots",
            get(chemical_handler::list_chemical_lots).post(chemical_handler::create_chemical_lot),
        )
        .route(
            "/chemical-lots/by-no/:no",
            get(chemical_handler::get_chemical_lot_by_no),
        )
        .route(
            "/chemical-lots/:id",
            get(chemical_handler::get_chemical_lot)
                .put(chemical_handler::update_chemical_lot)
                .delete(chemical_handler::delete_chemical_lot),
        )
        .route(
            "/chemical-lots/:id/pass-inspection",
            post(chemical_handler::pass_inspection),
        )
        .route(
            "/chemical-lots/:id/fail-inspection",
            post(chemical_handler::fail_inspection),
        )
        .route(
            "/chemical-lots/:id/consume",
            post(chemical_handler::consume_lot),
        )
        .route(
            "/chemical-lots/:id/scrap",
            post(chemical_handler::scrap_lot),
        )
        // 染化料领用单
        .route(
            "/chemical-requisitions",
            get(chemical_handler::list_requisitions)
                .post(chemical_handler::create_requisition),
        )
        .route(
            "/chemical-requisitions/by-no/:no",
            get(chemical_handler::get_requisition_by_no),
        )
        .route(
            "/chemical-requisitions/:id",
            get(chemical_handler::get_requisition)
                .put(chemical_handler::update_requisition)
                .delete(chemical_handler::delete_requisition),
        )
        .route(
            "/chemical-requisitions/:id/approve",
            post(chemical_handler::approve_requisition),
        )
        .route(
            "/chemical-requisitions/:id/issue",
            post(chemical_handler::issue_requisition),
        )
        .route(
            "/chemical-requisitions/:id/close",
            post(chemical_handler::close_requisition),
        )
        .route(
            "/chemical-requisitions/:id/cancel",
            post(chemical_handler::cancel_requisition),
        )
}

/// 目录域统一入口
///
/// 子 router path 已加独立前缀，merge 时 path+method 互不重叠。
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(products())
        .merge(product_categories())
        .merge(warehouses())
        .merge(boms())
        .merge(chemicals())
}
