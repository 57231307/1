//! 契约波次 6 · 面料行业版销售订单出口不得整行原文回传联系方式
//!
//! 根因：面料订单复用 `sales_order` Entity（`services/so/fabric_order.rs`），
//! `contact_phone`/`contact_person` 为真实列（`models/sales_order.rs:28-30`）。
//! 标准入口 GET /sales/orders* 已按权威列集合打码
//! （`sales_order_handler.rs:98/:184` 调 `utils/field_mask::mask_contact_fields_for_role`），
//! 而面料入口四个出口（列表 :66、详情、更新写响应、审核写响应）整行原文直出——
//! **同一张表同一行**换入口即拿到 contact_phone 原文，属"同一份数据、不同入口不一致"
//! 的打码旁路（update/approve 按 id 操作 sales_orders 任意行，不区分建档来源，
//! 标准渠道建的单带真实联系方式）。
//! 收口：四个出口复用同一份权威掩码实现
//! `utils/field_mask::mask_contact_fields_for_role`（本仓 sales_order 形状出参的
//! 既有唯一打码实现，与标准读出口同一调用、同一列集合；不新造判定分支、
//! 不新增权限键）。create_fabric_order 同样挂同一掩码实现：面料建档行当前
//! contact_person/contact_phone 由服务层恒置 None（fabric_order.rs:382-383
//! `Set(None)`），出参暂无原文可掩，但出参形状与 list/detail 同一张表同一批列——
//! 统一掩码防"列一旦接入真实值即成旁路"，且与其余出口口径一致。
//!
//! 掩码契约=掩码**保留键**（`138****8888`），非整键移除（与标准读出口既有语义一致）。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL。
//! sales_orders 的 DECIMAL 金额列、TIMESTAMPTZ 日期列取真表类型；
//! users/customers 按裁定 R1 自种子（sales_orders.customer_id 有真 FK，
//! roles 为迁移种子参照表不再插）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_fabric_order_handler::{
    approve_fabric_order, get_fabric_order, list_fabric_orders, update_fabric_order,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const SALES: i32 = 50;
const SALES_LOGIN: &str = "sales_a";
const ADMIN: i32 = 70;
const ADMIN_LOGIN: &str = "admin_user";

const O_PHONE: &str = "13812348888";
const O_MASKED_PHONE: &str = "138****8888";
const O_PERSON: &str = "张三";

fn make_auth(user_id: i32, username: &str, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id,
        department_id: Some(1),
        data_scope: Some(if role_id == Some(1) { "all" } else { "self" }.to_string()),
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

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

async fn base_state() -> AppState {
    let db = test_common::setup_test_db().await;
    // roles 不插：迁移种子参照表（id=1 code='admin' = is_admin_role 判定源）。
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // customers 父行（裁定 R1）：sales_orders.customer_id NOT NULL + 真 FK
    // fk_sales_orders_customer；status/customer_type 为迁移既有词表合法值。
    exec(
        &db,
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,contact_phone,
         credit_limit,payment_terms,status,customer_type,owner_id,created_at,updated_at)
         VALUES (1,'CUS-0001','甲客户','张三','13711112222',0,30,'active','retail',50,
         '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // 一条**标准渠道建档**的销售订单（带真实联系方式）——面料入口按 id 操作同一行。
    // 金额列为真表 DECIMAL，直接给数值字面量；subtotal+tax-discount+shipping 口径
    // 与 total_amount 自洽（1000+0-0+0=1000，已付 0、余款=总额）。
    exec(
        &db,
        &format!(
            "INSERT INTO sales_orders (id,order_no,customer_id,order_date,required_date,
             status,subtotal,tax_amount,discount_amount,shipping_cost,total_amount,
             paid_amount,balance_amount,contact_person,contact_phone,
             created_at,updated_at) VALUES
             (1,'SO001',1,'2026-01-01T00:00:00Z','2026-02-01T00:00:00Z',
              'pending',1000,0,0,0,1000,0,1000,
              '{O_PERSON}','{O_PHONE}','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;

    AppState {
        db: Arc::new(db),
        ..Default::default()
    }
}

/// 按真实挂载路径（routes/sales.rs）建路由
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/sales/fabric-orders", get(list_fabric_orders))
        .route(
            "/erp/sales/fabric-orders/{id}",
            get(get_fabric_order).put(update_fabric_order),
        )
        .route(
            "/erp/sales/fabric-orders/{id}/approve",
            post(approve_fabric_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn request_json(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(v) => builder.body(Body::from(v.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("响应非 JSON（{uri}）: {e}")),
    )
}

fn assert_order_masked(order: &Value, where_label: &str) {
    assert_eq!(
        order["contact_phone"],
        json!(O_MASKED_PHONE),
        "{where_label}：contact_phone 应为掩码值（掩码保留键，与标准读出口同一实现契约）"
    );
    let raw = order.to_string();
    assert!(
        !raw.contains(O_PHONE),
        "{where_label}：响应体含联系方式原文 {O_PHONE}\n出参: {raw}"
    );
}

fn assert_order_raw(order: &Value, where_label: &str) {
    assert_eq!(
        order["contact_phone"],
        json!(O_PHONE),
        "{where_label}：admin 原值契约"
    );
}

#[tokio::test]
async fn non_admin_fabric_list_and_detail_are_masked() {
    let app = build_app(base_state().await, make_auth(SALES, SALES_LOGIN, Some(2)));
    let (status, v) = get_and_check(&app, "/erp/sales/fabric-orders?page=1&page_size=10").await;
    assert_eq!(status, StatusCode::OK, "列表应成功: {v}");
    // 面料列表出参是 PaginatedResponse（items 键）；兼容取数组断言首行
    let items = v["data"]["items"]
        .as_array()
        .or_else(|| v["data"]["list"].as_array())
        .expect("列表数组缺失");
    assert!(!items.is_empty(), "订单行应返回");
    assert_order_masked(&items[0], "GET /sales/fabric-orders 列表行");

    let (status, v) = get_and_check(&app, "/erp/sales/fabric-orders/1").await;
    assert_eq!(status, StatusCode::OK, "详情应成功: {v}");
    assert_order_masked(&v["data"], "GET /sales/fabric-orders/:id 详情");
}

#[tokio::test]
async fn non_admin_fabric_update_and_approve_responses_are_masked() {
    let app = build_app(base_state().await, make_auth(SALES, SALES_LOGIN, Some(2)));
    let (status, v) = request_json(
        &app,
        Method::PUT,
        "/erp/sales/fabric-orders/1",
        Some(json!({"status": "pending"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "更新应成功: {v}");
    assert_order_masked(&v["data"], "PUT /sales/fabric-orders/:id 写响应");

    let (status, v) = request_json(
        &app,
        Method::POST,
        "/erp/sales/fabric-orders/1/approve",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "审核应成功: {v}");
    assert_eq!(v["data"]["status"], json!("approved"), "审核须真实生效");
    assert_order_masked(&v["data"], "POST /sales/fabric-orders/:id/approve 写响应");
}

#[tokio::test]
async fn admin_fabric_exits_keep_raw_contact_phone() {
    let app = build_app(base_state().await, make_auth(ADMIN, ADMIN_LOGIN, Some(1)));
    let (status, v) = get_and_check(&app, "/erp/sales/fabric-orders/1").await;
    assert_eq!(status, StatusCode::OK, "admin 详情应成功: {v}");
    assert_order_raw(&v["data"], "admin GET /sales/fabric-orders/:id");

    let (status, v) = request_json(
        &app,
        Method::POST,
        "/erp/sales/fabric-orders/1/approve",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin 审核应成功: {v}");
    assert_order_raw(&v["data"], "admin approve 写响应");
}

async fn get_and_check(app: &Router, uri: &str) -> (StatusCode, Value) {
    request_json(app, Method::GET, uri, None).await
}

/// 源码棘轮：面料四出口必须调权威掩码实现；create 出口显式登记"不改+原因"
#[test]
fn fabric_exits_ratchet_mask_via_single_authoritative_impl() {
    let handler = include_str!("../src/handlers/sales_fabric_order_handler.rs");
    let split_body = |name: &str| -> String {
        handler
            .split(&format!("pub async fn {name}"))
            .nth(1)
            .unwrap_or_else(|| panic!("{name} 定义缺失"))
            .split("pub async fn ")
            .next()
            .unwrap_or_else(|| panic!("{name} 函数体边界缺失"))
            .to_string()
    };
    for name in [
        "list_fabric_orders",
        "get_fabric_order",
        "create_fabric_order",
        "update_fabric_order",
        "approve_fabric_order",
    ] {
        let body = split_body(name);
        assert!(
            body.contains("mask_contact_fields_for_role("),
            "回潮棘轮：{name} 又整行原文直出（未复用 utils/field_mask 权威掩码实现）"
        );
    }
    // 数据层前提锁（非出参侧兜底替代）：面料建档行的联系方式列由服务层恒置 None，
    // 出参掩码是"列一旦接入真实值即有旁路"的形状防御，两者同时成立。
    let fabric_service = include_str!("../src/services/so/fabric_order.rs");
    assert!(
        fabric_service.contains("contact_person: Set(None)"),
        "面料建档 contact_person 恒 None 的前提已被破坏——确认出参掩码仍覆盖新列"
    );
    assert!(
        fabric_service.contains("contact_phone: Set(None)"),
        "面料建档 contact_phone 恒 None 的前提已被破坏——确认出参掩码仍覆盖新列"
    );
}
