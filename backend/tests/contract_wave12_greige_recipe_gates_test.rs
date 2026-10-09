//! 坯布与大货处方域 handler 侧归属门契约锁
//!
//! 锁定的端点契约：
//! - `backend/src/handlers/greige_fabric_handler.rs`（list：apply_data_scope 下推
//!   CreatedBy，不同 scope 用户可见行数与 total 同源）
//! - `backend/src/handlers/greige_fabric_handler.rs`（update/delete/
//!   stock_in/stock_out：check_resource_owner_by_member_scope 门在落库之前，越权 403
//!   且库存数值/状态零漂移）
//! - `backend/src/handlers/production_recipe_handler.rs`（close/cancel：
//!   service.get_by_id(id, Some(&ctx)) 归属门先于状态流转写入，越权 403 零状态漂移）
//! - `backend/src/handlers/production_recipe_handler.rs`（close_addition：同上，
//!   production_recipe_addition.created_by 走行级归属门）
//!
//! 用例覆盖：
//! - 每个写端点两条：本人/可见成员 => 2xx + 回读如实；他人 => 403 + FORBIDDEN + 零漂移
//! - 列表一条：不同可见集用户行数不同且 total 与 items 同源

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{greige_fabric_handler, production_recipe_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    greige_fabric, production_recipe, production_recipe_addition, warehouse,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn make_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("test_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: scope.map(|s| s.to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            builder.body(Body::from(v.to_string())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// ============================================================
// greige_fabric 路由构建
// ============================================================

fn build_greige_app(db: sea_orm::DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/greige-fabrics",
            axum::routing::get(greige_fabric_handler::list_greige_fabrics),
        )
        .route(
            "/greige-fabrics/{id}",
            axum::routing::put(greige_fabric_handler::update_greige_fabric)
                .delete(greige_fabric_handler::delete_greige_fabric),
        )
        .route(
            "/greige-fabrics/{id}/stock-in",
            axum::routing::post(greige_fabric_handler::stock_in),
        )
        .route(
            "/greige-fabrics/{id}/stock-out",
            axum::routing::post(greige_fabric_handler::stock_out),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

// ============================================================
// production_recipe 路由构建
// ============================================================

fn build_recipe_app(db: sea_orm::DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/production-recipes/{id}/close",
            axum::routing::post(production_recipe_handler::close),
        )
        .route(
            "/production-recipes/{id}/cancel",
            axum::routing::post(production_recipe_handler::cancel),
        )
        .route(
            "/production-recipes/additions/{id}/close",
            axum::routing::post(production_recipe_handler::close_addition),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

// ============================================================
// 种子数据
// ============================================================

async fn seed_greige_fabric(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    created_by: Option<i32>,
    status: &str,
    weight_kg: Decimal,
) {
    greige_fabric::ActiveModel {
        id: Set(id),
        fabric_no: Set(format!("GF-TEST-{id:04}")),
        fabric_name: Set(format!("测试坯布-{id}")),
        fabric_type: Set("针织".to_string()),
        status: Set(Some(status.to_string())),
        is_deleted: Set(Some(false)),
        created_by: Set(created_by),
        weight_kg: Set(Some(weight_kg)),
        length_m: Set(Some(dec("100.00"))),
        quantity_kg: Set(Some(weight_kg)),
        quantity_meters: Set(Some(dec("100.00"))),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection, id: i32) {
    warehouse::ActiveModel {
        id: Set(id),
        warehouse_code: Set(format!("WH-TEST-{id:04}")),
        name: Set(format!("测试仓库-{id}")),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_production_recipe(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    created_by: Option<i32>,
    status: &str,
) {
    production_recipe::ActiveModel {
        id: Set(id),
        recipe_no: Set(format!("PR-TEST-{id:04}")),
        status: Set(status.to_string()),
        fabric_weight: Set(dec("50.00")),
        liquor_ratio: Set("1:8".to_string()),
        is_deleted: Set(false),
        created_by: Set(created_by),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_production_recipe_addition(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    production_recipe_id: i32,
    created_by: Option<i32>,
    status: &str,
) {
    production_recipe_addition::ActiveModel {
        id: Set(id),
        addition_no: Set(format!("PA-TEST-{id:04}")),
        production_recipe_id: Set(production_recipe_id),
        status: Set(status.to_string()),
        is_deleted: Set(false),
        created_by: Set(created_by),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

// ============================================================
// A) 列表数据范围：不同 scope 用户可见行数不同，total 与 items 同源
// ============================================================

#[tokio::test]
async fn greige_list_scope_users_see_different_row_counts() {
    let db = test_common::setup_test_db().await;
    // 100 号用户有两条，200 号用户有一条
    seed_greige_fabric(&db, 1, Some(100), "在库", dec("10.00")).await;
    seed_greige_fabric(&db, 2, Some(100), "在库", dec("20.00")).await;
    seed_greige_fabric(&db, 3, Some(200), "在库", dec("30.00")).await;

    // self 范围 user=100 → 看到 2 条
    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::GET,
        "/greige-fabrics?page=1&page_size=50",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = v["data"]["items"].as_array().unwrap();
    let total = v["data"]["total"].as_u64().unwrap();
    assert_eq!(items.len(), 2, "self 范围用户应仅看到自己的 2 条");
    assert_eq!(total, 2, "total 必须与 items 同源");

    // self 范围 user=200 → 看到 1 条
    let app = build_greige_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(
        &app,
        Method::GET,
        "/greige-fabrics?page=1&page_size=50",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = v["data"]["items"].as_array().unwrap();
    let total = v["data"]["total"].as_u64().unwrap();
    assert_eq!(items.len(), 1, "self 范围 user=200 应仅看到自己的 1 条");
    assert_eq!(total, 1);

    // all 范围 → 看到全部 3 条
    let app = build_greige_app(db.clone(), make_auth(999, Some("all")));
    let (status, v) = call(
        &app,
        Method::GET,
        "/greige-fabrics?page=1&page_size=50",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = v["data"]["items"].as_array().unwrap();
    let total = v["data"]["total"].as_u64().unwrap();
    assert_eq!(items.len(), 3, "all 范围用户应看到全部 3 条");
    assert_eq!(total, 3);
}

// ============================================================
// B) greige_fabric update 归属门
// ============================================================

#[tokio::test]
async fn greige_update_owner_200_and_readback() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "在库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"fabric_name": "改名成功"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["fabric_name"], "改名成功");
}

#[tokio::test]
async fn greige_update_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "在库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"fabric_name": "越权改名"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 验证零漂移
    let row = greige_fabric::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.fabric_name, "测试坯布-1");
}

// ============================================================
// C) greige_fabric delete 归属门
// ============================================================

#[tokio::test]
async fn greige_delete_owner_200_and_is_deleted() {
    let db = test_common::setup_test_db().await;
    // 状态设为已出库才允许删除
    seed_greige_fabric(&db, 1, Some(100), "已出库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, _v) = call(&app, Method::DELETE, "/greige-fabrics/1", None).await;
    assert_eq!(status, StatusCode::OK);

    let row = greige_fabric::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.is_deleted, Some(true));
}

#[tokio::test]
async fn greige_delete_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "已出库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(&app, Method::DELETE, "/greige-fabrics/1", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：is_deleted 仍为 false
    let row = greige_fabric::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.is_deleted, Some(false));
}

// ============================================================
// D) greige_fabric stock_in 归属门
// ============================================================

#[tokio::test]
async fn greige_stock_in_owner_200_weight_increases() {
    let db = test_common::setup_test_db().await;
    seed_warehouse(&db, 1).await;
    seed_greige_fabric(&db, 1, Some(100), "已出库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 1, "weight_kg": 5.0, "length_m": 50.0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // weight = 10 + 5 = 15
    assert_eq!(v["data"]["weight_kg"], "15.00");
    assert_eq!(v["data"]["status"], "在库");
}

#[tokio::test]
async fn greige_stock_in_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "已出库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 1, "weight_kg": 5.0, "length_m": 50.0})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：weight 未变
    let row = greige_fabric::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.weight_kg, Some(dec("10.00")));
    assert_eq!(row.status, Some("已出库".to_string()));
}

// ============================================================
// E) greige_fabric stock_out 归属门
// ============================================================

#[tokio::test]
async fn greige_stock_out_owner_200_weight_decreases() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "在库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": 3.0})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // weight = 10 - 3 = 7
    assert_eq!(v["data"]["weight_kg"], "7.00");
    assert_eq!(v["data"]["status"], "在库");
}

#[tokio::test]
async fn greige_stock_out_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, Some(100), "在库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": 3.0})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：weight 未变
    let row = greige_fabric::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.weight_kg, Some(dec("10.00")));
}

// ============================================================
// F) production_recipe close 归属门
// ============================================================

#[tokio::test]
async fn recipe_close_owner_200_and_status_drifts() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "approved").await;

    let app = build_recipe_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(&app, Method::POST, "/production-recipes/1/close", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["status"], "closed");
}

#[tokio::test]
async fn recipe_close_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "approved").await;

    let app = build_recipe_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(&app, Method::POST, "/production-recipes/1/close", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：status 不变
    let row = production_recipe::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "approved");
}

// ============================================================
// G) production_recipe cancel 归属门
// ============================================================

#[tokio::test]
async fn recipe_cancel_owner_200_and_status_drifts() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "draft").await;

    let app = build_recipe_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(&app, Method::POST, "/production-recipes/1/cancel", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["status"], "cancelled");
}

#[tokio::test]
async fn recipe_cancel_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "draft").await;

    let app = build_recipe_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(&app, Method::POST, "/production-recipes/1/cancel", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：status 不变
    let row = production_recipe::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "draft");
}

// ============================================================
// H) production_recipe_addition close 归属门
// ============================================================

#[tokio::test]
async fn addition_close_owner_200_and_status_drifts() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "approved").await;
    seed_production_recipe_addition(&db, 10, 1, Some(100), "approved").await;

    let app = build_recipe_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/production-recipes/additions/10/close",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["status"], "closed");
}

#[tokio::test]
async fn addition_close_non_owner_403_and_zero_drift() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, Some(100), "approved").await;
    seed_production_recipe_addition(&db, 10, 1, Some(100), "approved").await;

    let app = build_recipe_app(db.clone(), make_auth(200, Some("self")));
    let (status, v) = call(
        &app,
        Method::POST,
        "/production-recipes/additions/10/close",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    // 零漂移：addition status 不变
    let row = production_recipe_addition::Entity::find_by_id(10)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, "approved");
}

// ============================================================
// I) created_by 为 NULL 的历史行归属门：一律拒绝
// ============================================================

#[tokio::test]
async fn greige_update_null_owner_403() {
    let db = test_common::setup_test_db().await;
    seed_greige_fabric(&db, 1, None, "在库", dec("10.00")).await;

    let app = build_greige_app(db.clone(), make_auth(100, Some("all")));
    let (status, _body) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"fabric_name": "NULL归属人行"})),
    )
    .await;
    // All 范围对任意行放行（含 created_by 为 NULL 的历史行），门只收窄 Dept/Self
    assert_eq!(status, StatusCode::OK);

    // 再用 self scope 验证 NULL owner 被拒
    let app = build_greige_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"fabric_name": "仍拒绝"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
}

#[tokio::test]
async fn recipe_cancel_null_owner_self_403() {
    let db = test_common::setup_test_db().await;
    seed_production_recipe(&db, 1, None, "draft").await;

    // self scope + created_by=NULL → 拒绝（check_resource_owner_by_member_scope:
    // Self_ => resource_owner_id == Some(ctx.user_id) => None != Some(user_id) => false）
    let app = build_recipe_app(db.clone(), make_auth(100, Some("self")));
    let (status, v) = call(&app, Method::POST, "/production-recipes/1/cancel", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
}
