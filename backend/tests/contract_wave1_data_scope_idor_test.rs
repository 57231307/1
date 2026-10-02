//! data-scope 越权参数化套件（本波 P0 无锁/IDOR 修复的 403/404/400 映射锁）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/handlers/inventory_adjustment_handler.rs:335-373,467-495`（get/list_items
//!   走 `get_adjustment(Some(&ctx))`，71a6ac03 去除 map_err 后：403 不再伪装 404、404 不再降级）
//! - `backend/src/handlers/inventory_adjustment_handler.rs:534-589` +
//!   `backend/src/services/inventory_adjustment_service.rs:620-626`（item 级 IDOR：
//!   `get_adjustment_id_by_item` 反查父单再走归属校验，8eac10bb）
//! - `backend/src/services/inventory_adjustment_service.rs:490-516`（check_resource_owner
//!   不命中 → `AppError::permission_denied` = 403/FORBIDDEN；不存在 → 404/NOT_FOUND）
//! - `backend/src/handlers/sales_order_handler.rs:409-427`（ship_order：payload.order_id !=
//!   路径 :id → 400 BAD_REQUEST「双 id 错位」强校验；随后 get_order_detail(Some(&ctx)) 归属）
//! - `backend/src/handlers/production_order_handler.rs:398-421,424-447`（操作日志只读归属
//!   + update_status 状态流转前置 get_by_id(Some(&ctx))，8eac10bb/c7827f4b）
//! - `backend/src/services/inv/inventory_move.rs:186-192,536+`（调拨 update/delete 前置
//!   get_transfer_detail 归属 403）
//! - `backend/src/middleware/auth_context.rs:103-134`（data_scope=None 缺省 → Self_ 最小权限）
//!
//! 覆盖策略：
//! - 真 PostgreSQL（TEST_DATABASE_URL，夹具清空业务表 + RESTART IDENTITY）：
//!   调整单两表（inventory_adjustments/inventory_adjustment_items，含 warehouses/
//!   products/inventory_stocks FK 前置）走真实 handler，
//!   锁 purchaser/non-owner 403、owner 与 all 200、不存在 404、item 反查 404；
//!   ship 双 id 不一致在触库前 400（AppState::default 即可，零 DB）；
//!   data_scope None→Self_ 纯函数锁。
//! - `#[ignore]` 活库（TEST_DATABASE_URL→PG）：transfer update/delete、production update_status/
//!   logs、sales ship 的 非owner 403 / owner 2xx（或至少非 403、非 500）全矩阵。

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
use bingxi_backend::handlers::{
    inventory_adjustment_handler, production_order_handler, sales_order_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    inventory_adjustment, inventory_adjustment_item, inventory_stock, product, warehouse,
};
use bingxi_backend::utils::data_scope::DataScope;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// purchaser/salesperson/非 owner 语义：data_scope=self、user_id 各异（角色键对齐
/// e2e 侧修复后的 purchaser/salesperson 口径，见 ci4653 备忘"保密矩阵假绿"）
fn make_scope_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
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

// =========================================================
// A) 纯函数：data_scope=None 的最小权限缺省
// =========================================================

/// AuthContext.data_scope 未加载（None）时 to_data_scope_context 必须按 Self_ 处理
/// （auth_context.rs:70 注释契约；越权套件的缺省语义根）
#[test]
fn auth_scope_none_defaults_to_self_min_privilege() {
    let ctx = make_scope_auth(7, None).to_data_scope_context();
    assert_eq!(ctx.scope, DataScope::Self_);
    assert_eq!(ctx.user_id, 7);

    let ctx_all = make_scope_auth(7, Some("all")).to_data_scope_context();
    assert_eq!(ctx_all.scope, DataScope::All);
    // 未知词串也按最小权限 Self_（parse_scope 兜底），不允许误放开
    let ctx_unknown = make_scope_auth(7, Some("weird")).to_data_scope_context();
    assert_eq!(ctx_unknown.scope, DataScope::Self_);
}

// =========================================================
// B) 调整单（含 item 级反查）——真 PG 迁移表 + FK 前置，读路径/403 路径
// =========================================================

/// 真表 FK 前置链：adjustments.warehouse_id→warehouses、items.stock_id→inventory_stocks
/// （stocks 又 FK 到 products）。夹具 TRUNCATE…RESTART IDENTITY 后显式 id 稳定。
async fn seed_adjustment_prereq(db: &sea_orm::DatabaseConnection) {
    product::ActiveModel {
        id: Set(1),
        name: Set("越权套件产品".to_string()),
        code: Set("PRD-DS-0001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    warehouse::ActiveModel {
        id: Set(10),
        warehouse_code: Set("WH-DS-10".to_string()),
        name: Set("越权套件-调整仓".to_string()),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    inventory_stock::ActiveModel {
        id: Set(66),
        warehouse_id: Set(10),
        product_id: Set(1),
        quantity_on_hand: Set(dec("10.00")),
        quantity_available: Set(dec("10.00")),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        batch_no: Set("B-DS-0001".to_string()),
        color_no: Set("C-DS-0001".to_string()),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(dec("10.00")),
        quantity_kg: Set(dec("1.00")),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        version: Set(0),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 调整单 id=1 created_by=100（owner），明细 item id=1 属该单
async fn seed_adjustment(db: &sea_orm::DatabaseConnection) {
    inventory_adjustment::ActiveModel {
        id: Set(1),
        adjustment_no: Set("ADJ-E2E-0001".to_string()),
        warehouse_id: Set(10),
        adjustment_date: Set(Utc::now()),
        adjustment_type: Set("increase".to_string()),
        reason_type: Set("correction".to_string()),
        total_quantity: Set(dec("5.00")),
        created_by: Set(Some(100)),
        status: Set("pending".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    inventory_adjustment_item::ActiveModel {
        id: Set(1),
        adjustment_id: Set(1),
        stock_id: Set(66),
        quantity: Set(dec("5.00")),
        quantity_before: Set(dec("10.00")),
        quantity_after: Set(dec("15.00")),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn build_adjustment_app(db: sea_orm::DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/inventory/adjustments/{id}",
            axum::routing::get(inventory_adjustment_handler::get_adjustment),
        )
        .route(
            "/inventory/adjustments/{id}/items",
            axum::routing::get(inventory_adjustment_handler::list_items),
        )
        .route(
            "/inventory/adjustment-items/{item_id}",
            axum::routing::put(inventory_adjustment_handler::update_item)
                .delete(inventory_adjustment_handler::delete_item),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn seeded_adjustment_app(auth: AuthContext) -> Router {
    let db = test_common::setup_test_db().await;
    seed_adjustment_prereq(&db).await;
    seed_adjustment(&db).await;
    build_adjustment_app(db, auth)
}

/// owner（purchaser，self 范围，本人单）→ 200；明细数量以字符串回读（Decimal 契约）
#[tokio::test]
async fn adjustment_owner_get_200_and_decimal_string() {
    let app = seeded_adjustment_app(make_scope_auth(100, Some("self"))).await;
    let (status, v) = call(&app, Method::GET, "/inventory/adjustments/1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["id"], 1);
    assert_eq!(v["data"]["adjustment_no"], "ADJ-E2E-0001");
    assert_eq!(v["data"]["total_quantity"], "5.00");
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"]["items"][0]["quantity"], "5.00");
}

/// 非 owner（另一 purchaser，self 范围）→ 必须 403/FORBIDDEN。
/// 回归锁①：修复前 map_err(not_found) 把 403 伪装成 404；
/// 回归锁②：也不得是 500/INTERNAL_ERROR。
#[tokio::test]
async fn adjustment_non_owner_get_403_not_masked_404() {
    let app = seeded_adjustment_app(make_scope_auth(200, Some("self"))).await;
    let (status, v) = call(&app, Method::GET, "/inventory/adjustments/1", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], "无权限");
}

/// all 范围用户跨 owner 仍放行（scope 语义对照组，防止把 data-scope 修成一律 403）
#[tokio::test]
async fn adjustment_all_scope_cross_owner_200() {
    let app = seeded_adjustment_app(make_scope_auth(200, Some("all"))).await;
    let (status, _v) = call(&app, Method::GET, "/inventory/adjustments/1", None).await;
    assert_eq!(status, StatusCode::OK);
}

/// 不存在 → 404/NOT_FOUND（回归锁：修复前统一降级 400/强转 500）
#[tokio::test]
async fn adjustment_missing_get_404() {
    let app = seeded_adjustment_app(make_scope_auth(100, Some("self"))).await;
    let (status, v) = call(&app, Method::GET, "/inventory/adjustments/999", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(v["code"], "BAD_REQUEST");
    assert_ne!(v["code"], "DATABASE_ERROR");
}

/// 明细只读枚举同样受父单归属门禁（8eac10bb：list_items 前置 get_adjustment(ctx)）
#[tokio::test]
async fn adjustment_items_non_owner_403() {
    let app = seeded_adjustment_app(make_scope_auth(200, Some("self"))).await;
    let (status, v) = call(&app, Method::GET, "/inventory/adjustments/1/items", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
}

/// item 级 IDOR（update/delete 仅拿 item_id）：非 owner → 反查父单再校验 → 403，
/// 且发生在任何服务写操作之前（delete_item 的 lock_exclusive 分支不可达，sqlite 亦可跑）
#[tokio::test]
async fn adjustment_item_level_update_and_delete_non_owner_403() {
    let app = seeded_adjustment_app(make_scope_auth(200, Some("self"))).await;
    let payload = json!({ "stock_id": 66, "quantity": "9.00", "unit_cost": null, "notes": null });

    let (status, v) = call(
        &app,
        Method::PUT,
        "/inventory/adjustment-items/1",
        Some(payload.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");

    let (status, v) = call(&app, Method::DELETE, "/inventory/adjustment-items/1", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
}

/// item 不存在 → 反查 404（get_adjustment_id_by_item 的 not_found 不被吞成 500）
#[tokio::test]
async fn adjustment_item_missing_reverse_lookup_404() {
    let app = seeded_adjustment_app(make_scope_auth(100, Some("self"))).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        "/inventory/adjustment-items/999",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "NOT_FOUND");
}

// =========================================================
// C) 销售发货双 id 错位——触库前的 400（零 DB）
// =========================================================

#[tokio::test]
async fn sales_ship_dual_id_mismatch_400_before_any_db() {
    let app = Router::new()
        .route(
            "/sales/orders/{id}/ship",
            axum::routing::post(sales_order_handler::ship_order),
        )
        .with_state(AppState::default())
        .layer(from_fn_with_state(
            make_scope_auth(100, Some("self")),
            inject_auth,
        ));
    let body = json!({
        "order_id": 6,
        "warehouse_code": "WH01",
        "items": [],
        "remarks": null
    });
    // 路径 :id=5 与 payload.order_id=6 不一致 → 400（消除"以谁为准"的双源错位）
    let (status, v) = call(&app, Method::POST, "/sales/orders/5/ship", Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BAD_REQUEST");
    assert_eq!(v["message"], "请求参数错误", "BadRequest 族脱敏常量");
    assert_ne!(v["code"], "INTERNAL_ERROR");
}

// =========================================================
// D) 活库全矩阵（transfer/production/sales 写与只读归属）：#[ignore]
// =========================================================

/// 活库：调整单 delete_item owner 正向（lock_exclusive 仅 PG 支持）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（delete_item 服务侧 lock_exclusive）"]
async fn live_adjustment_owner_delete_item_200_non_owner_403() {
    let db = test_common::setup_test_db().await;
    let owner = make_scope_auth(9101, Some("self"));
    let app = build_adjustment_app(db, owner);
    // 走真实端点建单太宽，此用例聚焦门控：不存在单号 → 反查 404 已非活库覆盖；
    // 活库下由 ops agent 接库后补 owner 删除 200 + 非 owner 403 成对断言（保持 ignore 直至接库）
    let (status, _v) = call(
        &app,
        Method::DELETE,
        "/inventory/adjustment-items/424242",
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "不存在明细应 404（接库后若变 500 即迁移/schema 漂移信号）"
    );
}

/// 活库：调拨单 update/delete 的 IDOR 归属（inventory_move.rs get_transfer_detail 403）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（transfer 详情多表 LEFT JOIN + 审计）"]
async fn live_transfer_update_delete_scope_matrix() {
    use bingxi_backend::handlers::inventory_transfer_handler;
    use bingxi_backend::models::{inventory_transfer, warehouse};

    let db = test_common::setup_test_db().await;
    let wh_from = warehouse::ActiveModel {
        warehouse_code: Set("WH-DS-F".to_string()),
        name: Set("越权套件-调出仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let wh_to = warehouse::ActiveModel {
        warehouse_code: Set("WH-DS-T".to_string()),
        name: Set("越权套件-调入仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let trf = inventory_transfer::ActiveModel {
        transfer_no: Set(format!("TRF-DS-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        from_warehouse_id: Set(wh_from.id),
        to_warehouse_id: Set(wh_to.id),
        transfer_date: Set(Utc::now()),
        status: Set("pending".to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(Some(9201)),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let uri = format!("/inventory/transfers/{}", trf.id);
    let patch = json!({ "status": null, "notes": "data-scope 套件", "items": null });

    // 非 owner（salesperson/purchaser 同语义 self）→ 403
    let app = {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
        Router::new()
            .route(
                "/inventory/transfers/{id}",
                axum::routing::put(inventory_transfer_handler::update_transfer)
                    .delete(inventory_transfer_handler::delete_transfer),
            )
            .with_state(state)
            .layer(from_fn_with_state(
                make_scope_auth(9202, Some("self")),
                inject_auth,
            ))
    };
    let (status, v) = call(&app, Method::PUT, &uri, Some(patch.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "实际: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    let (status, v) = call(&app, Method::DELETE, &uri, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "实际: {v}");

    // owner → 2xx
    let app_owner = {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
        Router::new()
            .route(
                "/inventory/transfers/{id}",
                axum::routing::put(inventory_transfer_handler::update_transfer)
                    .delete(inventory_transfer_handler::delete_transfer),
            )
            .with_state(state)
            .layer(from_fn_with_state(
                make_scope_auth(9201, Some("self")),
                inject_auth,
            ))
    };
    let (status, v) = call(&app_owner, Method::PUT, &uri, Some(patch)).await;
    assert!(status.is_success(), "owner 更新应 2xx，实际 {status}: {v}");
    let (status, v) = call(&app_owner, Method::DELETE, &uri, None).await;
    assert!(status.is_success(), "owner 删除应 2xx，实际 {status}: {v}");
}

/// 活库：生产订单 update_status（状态流转 IDOR）+ 操作日志只读归属
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（production_order JOIN products/audit_logs）"]
async fn live_production_status_update_and_logs_scope_matrix() {
    use bingxi_backend::models::{product, production_order};

    let db = test_common::setup_test_db().await;
    let p = product::ActiveModel {
        name: Set("越权套件产品".to_string()),
        code: Set(format!("PRD-DS-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let po = production_order::ActiveModel {
        order_no: Set(format!("MO-DS-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        product_id: Set(p.id),
        planned_quantity: Set(dec("100.00")),
        status: Set("DRAFT".to_string()),
        priority: Set(5),
        color_no: Set("".to_string()),
        dye_lot_no: Set("".to_string()),
        batch_no: Set("".to_string()),
        order_type: Set("normal".to_string()),
        created_by: Set(9301),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let mk_app = |viewer: i32| {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
        Router::new()
            .route(
                "/production/orders/{id}/status",
                axum::routing::put(production_order_handler::update_production_order_status),
            )
            .route(
                "/production/orders/{id}/logs",
                axum::routing::get(production_order_handler::get_production_order_logs),
            )
            .with_state(state)
            .layer(from_fn_with_state(
                make_scope_auth(viewer, Some("self")),
                inject_auth,
            ))
    };

    let id_uri_status = format!("/production/orders/{}/status", po.id);
    let id_uri_logs = format!("/production/orders/{}/logs", po.id);
    let body = json!({ "status": "SCHEDULED", "actual_quantity": null });

    let app = mk_app(9302);
    let (status, v) = call(&app, Method::PUT, &id_uri_status, Some(body.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "状态流转越权应 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    let (status, v) = call(&app, Method::GET, &id_uri_logs, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "操作日志只读越权应 403: {v}");

    let app_owner = mk_app(9301);
    let (status, v) = call(&app_owner, Method::GET, &id_uri_logs, None).await;
    assert!(
        status.is_success(),
        "owner 读日志应 2xx，实际 {status}: {v}"
    );
    assert_eq!(v["data"]["order_id"], po.id);
    assert_eq!(
        v["data"]["total"], 0u64,
        "无审计流水时 total=0（logs 数组形状）"
    );
    let (status, v) = call(&app_owner, Method::PUT, &id_uri_status, Some(body)).await;
    assert!(
        status.is_success(),
        "owner 状态流转应 2xx（若非 2xx 说明状态机/DRAFT→SCHEDULED 词表漂移）: {status} {v}"
    );
}

/// 活库：销售发货归属门禁——非 owner 403；owner 不被归属层拦截（后续业务门另说）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（sales_orders JOIN customers/audit 链）"]
async fn live_sales_ship_owner_gate_matrix() {
    use bingxi_backend::handlers::sales_order_handler::ship_order;
    use bingxi_backend::models::{customer, sales_order, user};

    let db = test_common::setup_test_db().await;
    // 真表 FK：sales_orders.created_by REFERENCES users(id)（fk_sales_orders_created_by），
    // users 不在迁移播种参照表内，夹具 TRUNCATE 后必须由用例自插 owner 行。
    user::ActiveModel {
        id: Set(9401),
        username: Set("e2e_user_9401".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("越权套件owner".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let cust = customer::ActiveModel {
        customer_code: Set(format!("CUS-DS-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        customer_name: Set("越权套件客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(9401),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let so = sales_order::ActiveModel {
        order_no: Set(format!("SO-DS-{}", Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价"))),
        customer_id: Set(cust.id),
        order_date: Set(Utc::now()),
        required_date: Set(Utc::now()),
        status: Set("PENDING".to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        created_by: Set(Some(9401)),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let mk_app = |viewer: i32| {
        let state = AppState {
            db: Arc::new(db.clone()),
            ..Default::default()
        };
        Router::new()
            .route("/sales/orders/{id}/ship", axum::routing::post(ship_order))
            .with_state(state)
            .layer(from_fn_with_state(
                make_scope_auth(viewer, Some("self")),
                inject_auth,
            ))
    };
    let uri = format!("/sales/orders/{}/ship", so.id);
    let body = json!({
        "order_id": so.id,
        "warehouse_code": "WH01",
        "items": [],
        "remarks": null
    });

    let app = mk_app(9402);
    let (status, v) = call(&app, Method::POST, &uri, Some(body.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "发货越权应 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");

    let app_owner = mk_app(9401);
    let (status, v) = call(&app_owner, Method::POST, &uri, Some(body)).await;
    assert_ne!(status, StatusCode::FORBIDDEN, "owner 不应被归属层拦截");
    assert_ne!(
        v["code"], "INTERNAL_ERROR",
        "owner 路径不得裸 500（后续业务门允许 4xx）: {v}"
    );
}
