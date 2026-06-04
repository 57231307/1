//! 产品目录域路由
//!
//! 处理产品、产品类别、仓库、BOM 物料清单等目录相关接口。

use crate::utils::app_state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::handlers::{
    bom_handler, bulk_product_handler, product_category_handler, product_handler, warehouse_handler,
};

/// 产品路由（nest 到 /api/v1/erp/products）
pub fn products() -> Router<AppState> {
    Router::new()
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
        .route("/export", get(product_handler::export_products))
        .route("/import", post(product_handler::import_products))
        .route(
            "/import-template",
            get(product_handler::get_product_import_template),
        )
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
        )
}

/// 产品类别路由（nest 到 /api/v1/erp/product-categories）
pub fn product_categories() -> Router<AppState> {
    Router::new()
        .route("/", get(product_category_handler::list))
        .route("/", post(product_category_handler::create))
        .route("/:id", get(product_category_handler::get))
        .route("/:id", put(product_category_handler::update))
        .route("/:id", delete(product_category_handler::delete))
        .route(
            "/tree",
            get(product_category_handler::get_product_category_tree),
        )
}

/// 仓库路由（nest 到 /api/v1/erp/warehouses）
pub fn warehouses() -> Router<AppState> {
    Router::new()
        .route("/", get(warehouse_handler::list))
        .route("/", post(warehouse_handler::create))
        .route("/select", get(warehouse_handler::list))
        .route("/:id", get(warehouse_handler::get))
        .route("/:id", put(warehouse_handler::update))
        .route("/:id", delete(warehouse_handler::delete))
        .route("/locations", get(warehouse_handler::list_locations))
        .route("/locations", post(warehouse_handler::create_location))
        .route("/locations/:id", get(warehouse_handler::get_location))
        .route("/locations/:id", put(warehouse_handler::update_location))
        .route("/locations/:id", delete(warehouse_handler::delete_location))
}

/// BOM 物料清单路由（nest 到 /api/v1/erp/boms）
pub fn boms() -> Router<AppState> {
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
        .route("/versions/:product_id", get(bom_handler::get_bom_versions))
}

/// 目录域统一入口
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(products())
        .merge(product_categories())
        .merge(warehouses())
        .merge(boms())
}
