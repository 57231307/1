//! AR 收款端点 business/4xx 错误映射锁（本波契约：去掉 map_err(internal) 重包后原码原样传播）
//!
//! 锁定的 file:line 契约：
//! - `backend/src/handlers/ar_payment_handler.rs:93-105`（get_payment：service 的
//!   `AppError` 直接 `?` 传播，不再被 `.map_err(|e| AppError::internal(..))` 强转 500）
//! - `backend/src/handlers/ar_payment_handler.rs:109-131`（create_payment：DTO `Validate`
//!   先行，校验错 400 VALIDATION_ERROR，不带病触库）
//! - `backend/src/handlers/ar_payment_handler.rs:58-89`（list_payments 出参 data 键
//!   {list,total,page,page_size} —— 注意其列表键是 `list` 而非 items，形状在此锁死，防止误改）
//! - `backend/src/utils/error.rs:166-183,468-504`（失败信封：BusinessError→400/BUSINESS_ERROR
//!   且 message 脱敏为固定常量；InternalError→500/INTERNAL_ERROR；NotFound→404；
//!   PermissionDenied→403/FORBIDDEN；business_displayable→400 且真实文案外显）
//! - `backend/src/services/ar_ops/collection.rs:84-106`（get_payment 的 404/403 语义源）
//!
//! 对任务书"message 不含『业务处理失败』强转"的落实口径：强转缺陷指旧代码把 4xx 重包成
//! InternalError(500/"服务器内部错误")；因此断言 **code 非 INTERNAL_ERROR 且 message 非
//! "服务器内部错误"**。`AppError::business`（脱敏族）出参 message 恰应为"业务处理失败"，
//! 该行为同样在此锁死，两者不可混。
//!
//! 覆盖策略：
//! - 失败信封矩阵：纯 `IntoResponse`，无需任何 DB
//! - 源码扫描锁：ar_payment_handler.rs 全文件 0 处 `map_err` / `AppError::internal`
//!   （本波修复的静态形态锁，防止重包回潮；先例：quotation_status_word_list_test 的源码扫描）
//! - sqlite::memory 自建 ar_collections 表走真实 handler：404 / 403 / 200（**无需活 PG**）
//! - POST 校验 400（validator 先于 DB，AppState::default 即够）

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::ar_payment_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::ar_collection;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::ConnectionTrait;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DbBackend, Statement};
use serde_json::Value;
use std::str::FromStr;
use tower::ServiceExt;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
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

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/ar/payments/{id}", get(ar_payment_handler::get_payment))
        .route("/ar/payments", post(ar_payment_handler::create_payment))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn request_json(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

/// 与 `models/ar_collection.rs::Model`（表 ar_collections）逐列对应
async fn create_ar_collections_table(db: &sea_orm::DatabaseConnection) {
    exec(
        db,
        r#"CREATE TABLE ar_collections (
            id INTEGER PRIMARY KEY,
            collection_no TEXT, collection_date TEXT,
            customer_id INTEGER, customer_name TEXT,
            collection_amount TEXT, collection_method TEXT, bank_account TEXT, check_no TEXT,
            request_id INTEGER, request_no TEXT,
            status TEXT, confirmed_by INTEGER, confirmed_at TEXT,
            created_by INTEGER, created_at TEXT, updated_at TEXT
        )"#,
    )
    .await;
}

async fn seed_collection_created_by(db: &sea_orm::DatabaseConnection, created_by: i32) {
    ar_collection::ActiveModel {
        collection_no: Set("COL-TEST-0007".to_string()),
        collection_date: Set(NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()),
        customer_id: Set(1),
        collection_amount: Set(Decimal::from_str("88.80").unwrap()),
        status: Set("PENDING".to_string()),
        created_by: Set(created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seeded_app(viewer_user_id: i32) -> Router {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_ar_collections_table(&db).await;
    seed_collection_created_by(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(viewer_user_id))
}

// =========================================================
// 1) 失败信封矩阵（纯 IntoResponse，无 DB）
// =========================================================

async fn envelope_of(err: AppError) -> (StatusCode, Value) {
    let resp = err.into_response();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn app_error_envelope_matrix_exact_status_code_message() {
    // business（脱敏族）：400 + BUSINESS_ERROR + 固定脱敏常量"业务处理失败"，
    // 绝不能是 500/"服务器内部错误"（本波修复前的强转形态）
    let (status, v) = envelope_of(AppError::business("库存不足 12.5")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "业务处理失败");
    assert_ne!(v["message"], "服务器内部错误");
    assert!(!v.to_string().contains("12.5"), "真实文案不得外泄到出参");

    // business_displayable：同 code，真实文案外显（与脱敏族仅 message 之差）
    let (status, v) = envelope_of(AppError::business_displayable("该线索已转商机，不可删除")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "该线索已转商机，不可删除");

    // 其余信封逐一钉死（error.rs:166-183 映射表）
    let (status, v) = envelope_of(AppError::not_found("收款单 999999 不存在")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "NOT_FOUND");
    assert_eq!(v["message"], "资源未找到");

    let (status, v) = envelope_of(AppError::permission_denied("越权")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], "无权限");

    let (status, v) = envelope_of(AppError::validation("金额必须为正且不超过10亿")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], "请求参数验证失败");

    let (status, v) = envelope_of(AppError::internal("业务错误：库存不足")).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(v["code"], "INTERNAL_ERROR");
    assert_eq!(v["message"], "服务器内部错误");

    // 失败信封固定四键（code/message/trace_id/timestamp），不允许漂移
    let (_, v) = envelope_of(AppError::business("x")).await;
    let mut keys: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, vec!["code", "message", "timestamp", "trace_id"]);
}

// =========================================================
// 2) 源码扫描锁：本波去除的 map_err 重包不得回潮
// =========================================================

#[test]
fn ar_payment_handler_source_has_no_error_rewrapping() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/handlers/ar_payment_handler.rs");
    let src =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e));
    assert!(
        !src.contains("map_err"),
        "ar_payment_handler.rs 出现 map_err：service 已返回 AppError，任何重包（尤其强转 internal/500）都违反本波契约"
    );
    assert!(
        !src.contains("AppError::internal"),
        "ar_payment_handler.rs 出现 AppError::internal：会把 4xx 业务/权限错强转成 500（历史 AR 收款 500 根因）"
    );
}

// =========================================================
// 3) HTTP 真实映射（sqlite 自建表，无需活 PG）
// =========================================================

/// 收款单不存在 → 404 NOT_FOUND（修复前被强转 500 INTERNAL_ERROR，即本波红案的直接回归锁）
#[tokio::test]
async fn get_missing_payment_returns_404_not_500() {
    let app = seeded_app(100).await;
    let (status, v) = request_json(
        &app,
        Request::builder()
            .uri("/ar/payments/999999")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(v["code"], "INTERNAL_ERROR");
    assert_ne!(v["message"], "服务器内部错误");
    assert_ne!(
        v["message"], "业务处理失败",
        "NotFound 不得被降级/重包成业务错"
    );
}

/// 存在的收款单但非本人（data_scope=self）→ 403 FORBIDDEN（修复前 403 被伪装成 404/400/500）
#[tokio::test]
async fn get_payment_cross_owner_returns_403_forbidden() {
    let app = seeded_app(200).await; // 查看人 200，归属人 100
    let (status, v) = request_json(
        &app,
        Request::builder()
            .uri("/ar/payments/1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], "无权限");
}

/// owner 本人 → 200 且出参为收款单 JSON（双键 payment_no/collection_no 同源值、金额字符串）
#[tokio::test]
async fn get_payment_owner_returns_200_with_expected_keys() {
    let app = seeded_app(100).await;
    let (status, v) = request_json(
        &app,
        Request::builder()
            .uri("/ar/payments/1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["payment_no"], "COL-TEST-0007");
    assert_eq!(v["data"]["collection_no"], "COL-TEST-0007");
    assert!(v["data"]["amount"].is_string(), "Decimal 出参必须是字符串");
    assert_eq!(v["data"]["collection_amount"], "88.80");
}

/// POST 金额为 0 → validator 先行 400 VALIDATION_ERROR（不带病触库、非 500）
#[tokio::test]
async fn create_payment_zero_amount_rejected_400_before_db() {
    let app = build_app(AppState::default(), make_auth(100));
    let (status, v) = request_json(
        &app,
        Request::builder()
            .method("POST")
            .uri("/ar/payments")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"customer_id":1,"amount":"0.00","payment_method":"bank_transfer","payment_date":"2026-01-15"}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_ne!(v["message"], "服务器内部错误");
}

/// POST 金额精度超 2 位小数 → 同为 400 VALIDATION_ERROR（validate_amount_range round_dp(2)）
#[tokio::test]
async fn create_payment_precision_amount_rejected_400() {
    let app = build_app(AppState::default(), make_auth(100));
    let (status, v) = request_json(
        &app,
        Request::builder()
            .method("POST")
            .uri("/ar/payments")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"customer_id":1,"amount":"12.3456","payment_method":"bank_transfer","payment_date":"2026-01-15"}"#,
            ))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
}
