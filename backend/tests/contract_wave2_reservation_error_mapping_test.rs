//! 库存预留四端点错误映射契约锁（本波修复"业务拒绝/404/403 被压成 500"）
//!
//! 表结构唯一来源 = backend/migration（路线一，#4669 判责）：本文件不自建 DDL，
//! 全部用例经 `test_common::setup_test_db()` 打已迁移 PostgreSQL。
//!
//! 锁定的 file:line 契约（修复后形态）：
//! - `backend/src/handlers/inventory_reservation_handler.rs`
//!   list/create/delete/lock/release 对 service 返回的 AppError 一律 `?` 透传，
//!   不再有 `.map_err(|e| AppError::internal(...))` 强转（修复前 :75/:108/:139/
//!   :144/:165/:194 六处把 400 BUSINESS_ERROR / 404 NOT_FOUND / 403 FORBIDDEN
//!   全部压成 500，用户看不到"释放的预留不可删除"等拒绝原因）
//! - `backend/src/services/inventory_reservation_service.rs`
//!   三个状态门拒绝（lock/release/delete）用 `AppError::business_displayable`：
//!   文案仅含本预留自身状态 + 公开状态流转规则（对齐任务 #148 aftersales_err
//!   的外显安全边界），出参 message 外显真实拒绝文案；修复前 business 变体
//!   会被脱敏成固定"业务处理失败"
//!
//! 覆盖策略（执行手册第 5 条：TEST_SEED_* 环境变量依赖一律自种子化消除）：
//! - `inventory_reservations` 真表三个 FK（order→sales_orders、product→products、
//!   warehouse→warehouses，m0010_add_inventory_extensions）全部由用例自种子合法父行
//!   （users→customers→sales_orders + products/warehouses，裁定 R1），目标预留行
//!   状态直接写词表终值（released / consumed），拿真实 ID 喂被测服务；
//! - delete 的 IDOR 预检（get_reservation，无 lock_exclusive）与 lock/release/delete
//!   状态门（服务侧 lock_exclusive——真 PG 原生支持）统一在同一通道真跑，
//!   原先因 sqlite 方言拆出的 #[ignore]+TEST_SEED_* 形态已删除，进常跑分片；
//! - 源码扫描防回潮锁（无 DB）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::delete,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::inventory_reservation_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::inventory_reservation as reservation_status;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::status::sales_order as so_status;
use bingxi_backend::models::{
    customer, inventory_reservation, product, sales_order, user, warehouse,
};
use chrono::{TimeZone, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 自种子夹具（FK 父行按真表 NOT NULL 与外键逐一补齐；不依赖任何外部播种）
// =========================================================

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

/// 种子布局（夹具 TRUNCATE…RESTART IDENTITY 后显式 id 稳定）：
/// users 100（本人）/200（他人）→ customers 1（owner=100）→ sales_orders 1 →
/// products 1 / warehouses 1 → inventory_reservations（id 由调用方指定）。
async fn seed_reservation_chain(
    db: &sea_orm::DatabaseConnection,
    reservation_id: i32,
    status: &str,
    created_by: i32,
) {
    for uid in [100i32, 200] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("w2-resv-user-{uid}")),
            password_hash: Set("x".repeat(60)),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            created_at: Set(fixed_time()),
            updated_at: Set(fixed_time()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("种子用户 {uid} 插入失败: {e}"));
    }
    customer::ActiveModel {
        id: Set(1),
        customer_code: Set("C-RESV-0001".to_string()),
        customer_name: Set("预留链路客户".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_by: Set(Some(100)),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子客户插入失败（真表 customers）");

    sales_order::ActiveModel {
        id: Set(1),
        order_no: Set("SO-RESV-0001".to_string()),
        customer_id: Set(1),
        order_date: Set(fixed_time()),
        required_date: Set(Some(fixed_time())),
        status: Set(so_status::PENDING.to_string()),
        subtotal: Set(rust_decimal::Decimal::new(100, 0)),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        shipping_cost: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::new(100, 0)),
        paid_amount: Set(rust_decimal::Decimal::ZERO),
        balance_amount: Set(rust_decimal::Decimal::new(100, 0)),
        created_by: Set(Some(100)),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子销售订单插入失败（真表 sales_orders，customer/created_by 双 FK）");

    product::ActiveModel {
        id: Set(1),
        code: Set("PRD-RESV-0001".to_string()),
        name: Set("预留链路产品".to_string()),
        unit: Set("米".to_string()),
        status: Set(master_data::ACTIVE.to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子产品插入失败（真表 products）");

    warehouse::ActiveModel {
        id: Set(1),
        warehouse_code: Set("WH-RESV-01".to_string()),
        name: Set("预留链路仓库".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子仓库插入失败（真表 warehouses）");

    inventory_reservation::ActiveModel {
        id: Set(reservation_id),
        order_id: Set(1),
        product_id: Set(1),
        warehouse_id: Set(1),
        quantity: Set(rust_decimal::Decimal::ONE),
        status: Set(status.to_string()),
        reserved_at: Set(fixed_time()),
        released_at: Set(None),
        notes: Set(None),
        created_by: Set(Some(created_by)),
        created_at: Set(fixed_time()),
        updated_at: Set(fixed_time()),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed 预留行 {reservation_id}（{status}）失败: {e}"));
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        // self 范围：IDOR 预检 check_resource_owner 要求 created_by 匹配本人
        data_scope: Some("self".to_string()),
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

async fn seeded_app() -> Router {
    let db = test_common::setup_test_db().await;
    // 他人（created_by=200）的 pending 预留：404 用例查 999，403 用例查 7
    seed_reservation_chain(&db, 7, reservation_status::PENDING, 200).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/inventory/reservations/{id}",
            delete(inventory_reservation_handler::delete_reservation),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

async fn call_delete(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 核心回归锁：删除不存在 → 404 NOT_FOUND 原样透传（修复前被
/// map_err(internal) 强转 500 INTERNAL_ERROR，用户看到"服务器内部错误"）
#[tokio::test]
async fn delete_missing_reservation_returns_404_not_500() {
    let app = seeded_app().await;
    let (status, v) = call_delete(&app, "/inventory/reservations/999").await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "不存在的预留必须 404（修复前 500），实际体: {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(v["code"], "INTERNAL_ERROR");
}

/// 核心回归锁：删除他人预留（self 数据范围）→ 403 FORBIDDEN 原样透传
/// （修复前 IDOR 预检的 403 也被压成 500"IDOR 校验失败"）
#[tokio::test]
async fn delete_foreign_reservation_returns_403_not_500() {
    let app = seeded_app().await;
    let (status, v) = call_delete(&app, "/inventory/reservations/7").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "越权删除必须 403（修复前 500），实际体: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_ne!(v["code"], "INTERNAL_ERROR");
}

// =========================================================
// 2) 状态门用例（真 PG 常跑）：lock_exclusive 在 PostgreSQL 真实加锁，
//    目标预留行由用例自种子（状态直接写终值），断言原文与错误码逐字不变。
// =========================================================

/// 锁定已释放预留 → 400 BUSINESS_ERROR 且 message 外显真实拒绝文案
/// （构造点契约：business_displayable，不得脱敏成"业务处理失败"、不得 500）
#[tokio::test]
async fn lock_released_reservation_service_error_is_displayable_400() {
    use axum::response::IntoResponse;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    seed_reservation_chain(&db, 7, reservation_status::RELEASED, 100).await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            Arc::new(db),
        );
    let err = svc
        .lock_reservation(7)
        .await
        .expect_err("已释放预留不可锁定，必须被状态门拒绝");
    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("状态门拒绝必须为 business_displayable，实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    let resp = err.clone().into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"], "预留状态为released，只有待处理状态的预留可以锁定",
        "business_displayable 出参必须外显真实拒绝文案"
    );
}

/// 删除已释放预留 → 400 外显"释放的预留不可删除"语义原文
#[tokio::test]
async fn delete_released_reservation_service_error_is_displayable_400() {
    use axum::response::IntoResponse;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    seed_reservation_chain(&db, 7, reservation_status::RELEASED, 100).await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            Arc::new(db),
        );
    let err = svc
        .delete_reservation(7, 1)
        .await
        .expect_err("已释放预留不可删除，必须被状态门拒绝");
    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("状态门拒绝必须为 business_displayable，实际: {other:?}"),
    }
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"],
        "预留状态为released，只有待处理状态的预留可以删除"
    );
}

/// 释放已终结（consumed，旧 TEST_SEED_USED_RESERVATION_ID 的"used"终态语义，
/// 按权威词表 models/status/purchase_inventory.rs::inventory_reservation 以
/// consumed 落库）预留 → 400 外显
#[tokio::test]
async fn release_used_reservation_service_error_is_displayable_400() {
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    seed_reservation_chain(&db, 7, reservation_status::CONSUMED, 100).await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            Arc::new(db),
        );
    let err = svc
        .release_reservation(7)
        .await
        .expect_err("已终结预留不可释放，必须被状态门拒绝");
    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("状态门拒绝必须为 business_displayable，实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert!(
        err.to_string()
            .contains("只有已锁定或待处理状态的预留可以释放"),
        "拒绝文案须与 service 状态门逐字一致，实际: {err}"
    );
}

// =========================================================
// 3) 源码扫描防回潮锁（无 DB）
// =========================================================

/// handler 全文件不得再出现 AppError::internal 强转与 map_err 闭包强转
/// （本文件唯一合法错误出口：service 的 AppError 经 `?` 透传）
#[test]
fn source_scan_handler_has_no_internal_coercion() {
    let src = include_str!("../src/handlers/inventory_reservation_handler.rs");
    assert!(
        !src.contains("AppError::internal"),
        "预留 handler 禁止再把 service 的 AppError 强转 500 internal"
    );
    assert!(
        !src.contains("map_err(|e|"),
        "预留 handler 禁止残留 map_err 闭包强转，必须 `?` 透传"
    );
}

/// service 三个状态门拒绝必须 business_displayable（拒绝原因对用户可见）
#[test]
fn source_scan_service_status_gates_are_displayable() {
    let src = include_str!("../src/services/inventory_reservation_service.rs").replace('\r', "");
    for gate in [
        "预留状态为{}，只有待处理状态的预留可以锁定",
        "预留状态为{}，只有已锁定或待处理状态的预留可以释放",
        "预留状态为{}，只有待处理状态的预留可以删除",
    ] {
        let i = src
            .find(gate)
            .unwrap_or_else(|| panic!("状态门文案丢失: {gate}"));
        let before = &src[i.saturating_sub(120)..i];
        assert!(
            before.contains("AppError::business_displayable"),
            "状态门拒绝必须 business_displayable（修复前 business 出参被脱敏），文案: {gate}"
        );
        assert!(
            !before.contains("AppError::business(format"),
            "状态门拒绝禁止回潮脱敏的 AppError::business，文案: {gate}"
        );
    }
}
