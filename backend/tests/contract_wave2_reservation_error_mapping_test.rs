//! 库存预留四端点错误映射契约锁（本波修复"业务拒绝/404/403 被压成 500"）
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
//! 覆盖策略（先例：contract_wave1_ap_payment_request_items_test.rs /
//! contract_wave1_data_scope_idor_test.rs，全部真实行为无 mock）：
//! - sqlite::memory: 自建 inventory_reservations 表 + 真实 delete handler 端到端
//!   （tower oneshot）：delete 的 IDOR 预检走 get_reservation（无 lock_exclusive），
//!   404/403 透传可在 sqlite 实跑——修复前这两形态均被强转 500
//!   注：lock/release/delete 的状态门位于服务侧 lock_exclusive 之后，sqlite 方言
//!   不支持（先例注释见 contract_wave1_data_scope_idor_test.rs:24 与
//!   contract_wave1_ap_payment_request_items_test.rs:19），400 外显形态以
//!   #[ignore] 活库用例 + 源码扫描锁双保险固化
//! - 源码扫描防回潮锁

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
use bingxi_backend::models::inventory_reservation;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, Set, Statement};
use serde_json::Value;

// =========================================================
// 1) 真实 delete handler 端到端（sqlite::memory:，透传 404/403）
// =========================================================

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

/// 与 `models/inventory_reservation.rs::Model` 逐列对应的 sqlite 最小 DDL
/// （Decimal 列用 TEXT，DateTime 用 ISO 字符串）
async fn create_reservation_table(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE inventory_reservations (
            id INTEGER PRIMARY KEY,
            order_id INTEGER, product_id INTEGER, warehouse_id INTEGER,
            quantity TEXT, status TEXT,
            reserved_at TEXT, released_at TEXT, notes TEXT, created_by INTEGER,
            created_at TEXT, updated_at TEXT
        )"#,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}"));
}

async fn seed_reservation(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    status: &str,
    created_by: i32,
) {
    let now = chrono::Utc::now();
    let active = inventory_reservation::ActiveModel {
        id: Set(id),
        order_id: Set(1),
        product_id: Set(1),
        warehouse_id: Set(1),
        quantity: Set(rust_decimal::Decimal::ONE),
        status: Set(status.to_string()),
        reserved_at: Set(now),
        released_at: Set(None),
        notes: Set(None),
        created_by: Set(Some(created_by)),
        created_at: Set(now),
        updated_at: Set(now),
    };
    active.insert(db).await.expect("seed 预留行失败");
}

async fn seeded_app() -> Router {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_reservation_table(&db).await;
    seed_reservation(&db, 7, "pending", 200).await; // 他人（200）创建的预留
    let mut state = AppState::default();
    state.db = std::sync::Arc::new(db.clone());
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
// 2) 活库（PostgreSQL）用例：#[ignore]（状态门在服务侧 lock_exclusive
//    之后，sqlite 方言不支持；先例：contract_wave1_ap_payment_request_items_test.rs）
// =========================================================

/// 活库：锁定已释放预留 → 400 BUSINESS_ERROR 且 message 外显真实拒绝文案
/// （构造点契约：business_displayable，不得脱敏成"业务处理失败"、不得 500）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL + TEST_SEED_RELEASED_RESERVATION_ID（状态门在 lock_exclusive 之后，sqlite 方言不支持）"]
async fn lock_released_reservation_service_error_is_displayable_400() {
    use axum::response::IntoResponse;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            std::sync::Arc::new(db),
        );
    let released_id: i32 = std::env::var("TEST_SEED_RELEASED_RESERVATION_ID")
        .expect("活库用例须提供 TEST_SEED_RELEASED_RESERVATION_ID")
        .parse()
        .unwrap();
    let err = svc
        .lock_reservation(released_id)
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

/// 活库：删除已释放预留 → 400 外显"释放的预留不可删除"语义原文
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL + TEST_SEED_RELEASED_RESERVATION_ID（状态门在 lock_exclusive 之后，sqlite 方言不支持）"]
async fn delete_released_reservation_service_error_is_displayable_400() {
    use axum::response::IntoResponse;
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            std::sync::Arc::new(db),
        );
    let released_id: i32 = std::env::var("TEST_SEED_RELEASED_RESERVATION_ID")
        .expect("活库用例须提供 TEST_SEED_RELEASED_RESERVATION_ID")
        .parse()
        .unwrap();
    let err = svc
        .delete_reservation(released_id, 1)
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

/// 活库：释放已使用预留 → 400 外显
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL + TEST_SEED_USED_RESERVATION_ID（状态门在 lock_exclusive 之后，sqlite 方言不支持）"]
async fn release_used_reservation_service_error_is_displayable_400() {
    use bingxi_backend::utils::error::AppError;

    let db = test_common::setup_test_db().await;
    let svc =
        bingxi_backend::services::inventory_reservation_service::InventoryReservationService::new(
            std::sync::Arc::new(db),
        );
    let used_id: i32 = std::env::var("TEST_SEED_USED_RESERVATION_ID")
        .expect("活库用例须提供 TEST_SEED_USED_RESERVATION_ID")
        .parse()
        .unwrap();
    let err = svc
        .release_reservation(used_id)
        .await
        .expect_err("已使用预留不可释放，必须被状态门拒绝");
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
