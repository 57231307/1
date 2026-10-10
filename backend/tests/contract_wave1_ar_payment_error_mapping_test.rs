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
//! - `backend/src/services/ar_ops/collection.rs:409-486`（update_payment 备注/支票号分列：
//!   remark 三态落 ar_collections.remark 列、check_no 只接受支票号入参，互不覆盖；
//!   三态=键缺席保持、显式 null 清空、字符串值覆盖）
//! - `backend/src/services/ar_ops/json_helpers.rs:11-34`（collection_to_json 出参键
//!   与 models/ar_collection.rs 字段对齐：含 remark 与 check_no，写了必须读得回）
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
//! - 真 PostgreSQL（TEST_DATABASE_URL + 迁移建表，夹具清空业务表）走真实 handler：
//!   404 / 403 / 200
//! - POST 校验 400（validator 先于 DB，AppState::default 即够）
//! - PUT remark 分列锁 + GET 回读（真 PG 真实 handler）；三态回环（真实 service，
//!   绕开 update DTO 单层 Option 把"键缺席"物化成 null 的 handler 层塌陷）

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::ar_payment_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::ar_collection;
use bingxi_backend::services::ar_service::ArService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::Value;
use std::str::FromStr;
use tower::ServiceExt;
use validator::Validate;

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
        .route("/ar/payments/{id}", put(ar_payment_handler::update_payment))
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
    // 真 PG：夹具清空业务表后 ar_collections 由迁移建表，自增 id 从 1 起，
    // 用例内 URI 直引 /ar/payments/1 稳定可断言。
    let db = test_common::setup_test_db().await;
    seed_collection_created_by(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(viewer_user_id))
}

/// 播一条 pending 状态、已带真实支票号的收款单（状态值与写入方常量逐字符相同，
/// 见 models/status/finance.rs COLLECTION_PENDING）
async fn seed_pending_collection(db: &sea_orm::DatabaseConnection, created_by: i32) {
    ar_collection::ActiveModel {
        collection_no: Set("COL-TEST-0008".to_string()),
        collection_date: Set(NaiveDate::from_ymd_opt(2026, 2, 20).unwrap()),
        customer_id: Set(1),
        collection_amount: Set(Decimal::from_str("66.60").unwrap()),
        check_no: Set(Some("CHQ-2026-001".to_string())),
        status: Set(bingxi_backend::models::status::ar::COLLECTION_PENDING.to_string()),
        created_by: Set(created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
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
// 3) HTTP 真实映射（真 PostgreSQL，迁移建表）
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

// =========================================================
// 4) AR 收款备注/支票号分列锁（真 PG）：传 remark 后 GET 能回读该键，且 check_no 未被覆盖
// =========================================================

#[tokio::test]
async fn update_payment_remark_lands_remark_column_and_keeps_check_no() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = request_json(
        &app,
        Request::builder()
            .method("PUT")
            .uri("/ar/payments/1")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"remark":"收款备注-契约锁"}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["remark"], "收款备注-契约锁");
    assert_eq!(
        v["data"]["check_no"], "CHQ-2026-001",
        "remark 写入后 check_no 绝不可被备注覆盖"
    );

    let (status, g) = request_json(
        &app,
        Request::builder()
            .uri("/ar/payments/1")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        g["data"]["remark"], "收款备注-契约锁",
        "GET 出参必须含 remark 键且与提交值一致"
    );

    // 真库行回读：备注落 remark 列，check_no 保持原支票号原文
    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.remark.as_deref(), Some("收款备注-契约锁"));
    assert_eq!(row.check_no.as_deref(), Some("CHQ-2026-001"));
}

/// 三态回环（真实 service 层）：有值=覆盖、键缺席=保持原值、显式 null=清空为 NULL。
/// 经 HTTP 走 update_payment 时 handler DTO 是单层 Option，会把"键缺席"重新序列化成
/// 显式 null（保持分支在 HTTP 层不可证真），故该语义在 service 层锁死；DTO 收口为
/// 双层 Option + double_option 适配器属 handler 侧待办。
#[tokio::test]
async fn update_payment_remark_three_state_keep_and_clear() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection(&db, 100).await;
    let svc = ArService::new(std::sync::Arc::new(db.clone()));

    let r = svc
        .update_payment(1, serde_json::json!({"remark": "三态-覆盖"}), 100)
        .await
        .unwrap();
    assert_eq!(r["remark"], "三态-覆盖", "有值=覆盖写入 remark 列");

    let r = svc
        .update_payment(1, serde_json::json!({"payment_method": "cash"}), 100)
        .await
        .unwrap();
    assert_eq!(r["remark"], "三态-覆盖", "remark 键缺席=保持原值");
    assert_eq!(r["payment_method"], "cash");

    let r = svc
        .update_payment(1, serde_json::json!({"remark": null}), 100)
        .await
        .unwrap();
    assert!(r["remark"].is_null(), "显式传 null=清空为 NULL");
    assert_eq!(
        r["check_no"], "CHQ-2026-001",
        "清空 remark 不得牵连 check_no"
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.remark, None);
    assert_eq!(row.check_no.as_deref(), Some("CHQ-2026-001"));
}

// =========================================================
// 5) update DTO 三态收口后的端到端锁（DTO 已收口为双层 Option + double_option 适配器）：
//    键缺席=保持、显式 null=清空、有值=覆盖，经真实 handler + 真 PG 全链路证真。
//    此前"三态回环"只能在 service 层锁（单层 Option DTO 会把缺席物化成 null），
//    本节在 HTTP 层补齐缺席保持的可证真性。
// =========================================================

/// 播一条 pending 状态、四个可空业务列均带真实值的收款单
async fn seed_pending_collection_full_nullable(db: &sea_orm::DatabaseConnection, created_by: i32) {
    ar_collection::ActiveModel {
        collection_no: Set("COL-TEST-0009".to_string()),
        collection_date: Set(NaiveDate::from_ymd_opt(2026, 3, 5).unwrap()),
        customer_id: Set(1),
        collection_amount: Set(Decimal::from_str("77.70").unwrap()),
        collection_method: Set(Some("bank_transfer".to_string())),
        bank_account: Set(Some("6222000011112222".to_string())),
        check_no: Set(Some("CHQ-FULL-001".to_string())),
        remark: Set(Some("原始备注".to_string())),
        status: Set(bingxi_backend::models::status::ar::COLLECTION_PENDING.to_string()),
        created_by: Set(created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn put_payment_body(app: &Router, body: &str) -> (StatusCode, Value) {
    request_json(
        app,
        Request::builder()
            .method("PUT")
            .uri("/ar/payments/1")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
}

async fn get_payment_body(app: &Router) -> (StatusCode, Value) {
    request_json(
        app,
        Request::builder()
            .uri("/ar/payments/1")
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

/// DTO 三态形状锁（纯 serde，无 DB）：缺席=None、显式 null=Some(None)、有值=Some(Some(v))；
/// 序列化回 JSON 时缺席省略键、显式 null 落 null 键（handler 以 to_value 传递 service）；
/// 清空路径 Some(None) 不被长度校验阻挡，校验只在实际覆盖时生效。
#[test]
fn ar_update_dto_distinguishes_absent_null_and_value() {
    let dto: ar_payment_handler::UpdateArPaymentRequest =
        serde_json::from_value(serde_json::json!({
            "remark": null,
            "check_no": null,
            "payment_method": "cash",
        }))
        .expect("AR update DTO 三态反序列化失败");
    assert!(
        matches!(dto.remark, Some(None)),
        "显式 null 的 remark 必须反序列化为 Some(None)（清空），不得与缺席塌同"
    );
    assert!(matches!(dto.check_no, Some(None)));
    assert!(matches!(&dto.payment_method, Some(Some(v)) if v == "cash"));
    assert!(dto.bank_account.is_none(), "缺席键必须是 None（保持原值）");

    let wire = serde_json::to_value(&dto).expect("AR update DTO 三态序列化失败");
    assert!(
        wire.get("bank_account").is_none(),
        "缺席键序列化必须省略，不得物化成 null 把保持塌成清空"
    );
    assert!(
        wire.get("remark").map(|v| v.is_null()).unwrap_or(false),
        "显式 null 序列化必须落 null 键"
    );
    assert_eq!(wire["payment_method"], "cash");

    assert!(
        dto.validate().is_ok(),
        "清空路径 Some(None) 不得被必填/长度校验阻挡"
    );

    let over_len: ar_payment_handler::UpdateArPaymentRequest =
        serde_json::from_value(serde_json::json!({ "remark": "r".repeat(501) })).unwrap();
    assert!(
        over_len.validate().is_err(),
        "实际覆盖（Some(Some(v))）时长度校验必须生效"
    );
}

/// ① 只携带部分键的有值更新：未发键的可空列经 HTTP 全链路必须保持原值
#[tokio::test]
async fn update_payment_partial_keys_keep_absent_nullable_values() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_full_nullable(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"payment_method":"cash"}"#).await;
    assert_eq!(status, StatusCode::OK, "部分键更新失败: {v}");
    assert_eq!(v["data"]["payment_method"], "cash");

    let (status, g) = get_payment_body(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        g["data"]["remark"], "原始备注",
        "remark 键缺席经真实 handler 必须保持原值（双层 Option DTO 收口前此处会把缺席物化成 null）"
    );
    assert_eq!(
        g["data"]["check_no"], "CHQ-FULL-001",
        "check_no 键缺席必须保持原值"
    );
    assert_eq!(
        g["data"]["bank_account"], "6222000011112222",
        "bank_account 键缺席必须保持原值"
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.remark.as_deref(), Some("原始备注"));
    assert_eq!(row.check_no.as_deref(), Some("CHQ-FULL-001"));
    assert_eq!(row.collection_method.as_deref(), Some("cash"));
}

/// ② 显式 null 清空：发 null 的可空列 GET 回读与 DB 回读均为 null，缺席键不受牵连
#[tokio::test]
async fn update_payment_explicit_null_clears_nullable_columns_via_http() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_full_nullable(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(
        &app,
        r#"{"remark":null,"check_no":null,"bank_account":null}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "显式 null 三态更新失败: {v}");
    assert!(v["data"]["remark"].is_null(), "响应体 remark 必须为 null");

    let (status, g) = get_payment_body(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        g["data"]["remark"].is_null(),
        "发 null 后 GET 回读 remark 必须为 null"
    );
    assert!(
        g["data"]["check_no"].is_null(),
        "发 null 后 GET 回读 check_no 必须为 null"
    );
    assert!(
        g["data"]["bank_account"].is_null(),
        "发 null 后 GET 回读 bank_account 必须为 null"
    );
    assert_eq!(
        g["data"]["payment_method"], "bank_transfer",
        "缺席的 payment_method 不得被牵连清空"
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        row.remark.is_none(),
        "发 null 后 remark 列 DB 回读必须为 NULL"
    );
    assert!(
        row.check_no.is_none(),
        "发 null 后 check_no 列 DB 回读必须为 NULL"
    );
    assert!(
        row.bank_account.is_none(),
        "发 null 后 bank_account 列 DB 回读必须为 NULL"
    );
    assert_eq!(row.collection_method.as_deref(), Some("bank_transfer"));
}

/// ③ 覆盖为新值：PUT 提交值与响应体、GET 回读、DB 行回读四处一致
#[tokio::test]
async fn update_payment_overwrite_new_values_readback_matches() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_full_nullable(&db, 100).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(
        &app,
        r#"{"remark":"新备注-三态锁","check_no":"CHQ-NEW-002"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "覆盖更新失败: {v}");
    assert_eq!(v["data"]["remark"], "新备注-三态锁");
    assert_eq!(v["data"]["check_no"], "CHQ-NEW-002");

    let (status, g) = get_payment_body(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        g["data"]["remark"], "新备注-三态锁",
        "覆盖后 GET 回读必须与提交值一致"
    );
    assert_eq!(
        g["data"]["check_no"], "CHQ-NEW-002",
        "覆盖后 GET 回读 check_no 必须与提交值一致"
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.remark.as_deref(), Some("新备注-三态锁"));
    assert_eq!(row.check_no.as_deref(), Some("CHQ-NEW-002"));
    assert_eq!(
        row.bank_account.as_deref(),
        Some("6222000011112222"),
        "本次未发键的 bank_account 必须保持原值"
    );
}

// =========================================================
// 6) AR 收款金额/收款日期真实编辑锁（真 PG + 真实 handler）：
//    NOT NULL 列三态（键缺席=保持、有值=覆盖、显式 null=业务拒绝不落默认值）、
//    "新金额 < 已核销分配金额" 一致性门（拒绝即事务回滚，库内无部分写入）、
//    关账期间日期变更拒绝（复用 check_payment_period_locked 同源判据）、
//    出参 amount/payment_date 经 GET 以 Decimal→字符串形态读回。
//    夹具说明：收款单维度的分配账本 = ar_reconciliation_items
//    （item_type=RECEIPT、document_id=收款单ID、金额按记账方向存负），
//    与 check_payment_available_balance / load_auto_verify_data 同源；
//    pending 收款单业务上尚不可被核销（核销门控要求 confirmed），故该账本行
//    以夹具直插构造历史分配形态，锁的是服务层一致性门本身。
// =========================================================

/// 播一条 pending 收款单：COL-EDIT-0001，2026-04-10，金额 100.00
async fn seed_pending_collection_for_amount_edit(db: &sea_orm::DatabaseConnection) {
    ar_collection::ActiveModel {
        collection_no: Set("COL-EDIT-0001".to_string()),
        collection_date: Set(NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()),
        customer_id: Set(1),
        collection_amount: Set(Decimal::from_str("100.00").unwrap()),
        status: Set(bingxi_backend::models::status::ar::COLLECTION_PENDING.to_string()),
        created_by: Set(100),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 为该收款单（id=1）插一张核销单 + RECEIPT 明细，已分配金额 60.00。
/// 主单 reconciliation_status 必须为词表常量 closed：收款单级"已核销分配额"读数
/// （collection.rs::active_receipt_ledger_items）只计 closed 核销单挂的
/// AR_COLLECTION 明细，与本夹具的分配形态同真实核销创建路径逐字段一致。
async fn seed_receipt_verified_allocation_60(db: &sea_orm::DatabaseConnection) {
    let today = Utc::now().date_naive();
    bingxi_backend::models::ar_reconciliation::ActiveModel {
        reconciliation_no: Set("VER-EDIT-0001".to_string()),
        reconciliation_date: Set(today),
        period_start: Set(today),
        period_end: Set(today),
        customer_id: Set(1),
        opening_balance: Set(Decimal::ZERO),
        total_invoices: Set(Decimal::from_str("60.00").unwrap()),
        total_collections: Set(Decimal::from_str("60.00").unwrap()),
        closing_balance: Set(Decimal::ZERO),
        reconciliation_status: Set(Some(
            bingxi_backend::models::status::ar::RECONCILIATION_CLOSED.to_string(),
        )),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    bingxi_backend::models::ar_reconciliation_item::ActiveModel {
        reconciliation_id: Set(1),
        item_type: Set("RECEIPT".to_string()),
        document_type: Set(Some("AR_COLLECTION".to_string())),
        document_id: Set(Some(1)),
        document_no: Set(Some("COL-EDIT-0001".to_string())),
        document_date: Set(Some(NaiveDate::from_ymd_opt(2026, 4, 10).unwrap())),
        // RECEIPT 明细金额按记账方向存负（manual.rs::create_reconciliation_items 形态）
        amount: Set(-Decimal::from_str("60.00").unwrap()),
        matched_amount: Set(Some(Decimal::from_str("60.00").unwrap())),
        match_status: Set(bingxi_backend::models::status::ar::MATCH_MATCHED.to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 播 2026-05 会计期间且已关账（CLOSED 词表常量与写入方逐字符相同）
async fn seed_closed_period_2026_05(db: &sea_orm::DatabaseConnection) {
    let start = NaiveDate::from_ymd_opt(2026, 5, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 5, 31)
        .unwrap()
        .and_hms_opt(23, 59, 59)
        .unwrap();
    bingxi_backend::models::accounting_period::ActiveModel {
        year: Set(2026),
        period: Set(5),
        period_name: Set("2026-05".to_string()),
        start_date: Set(Utc.from_utc_datetime(&start)),
        end_date: Set(Utc.from_utc_datetime(&end)),
        status: Set(bingxi_backend::models::status::accounting_period::CLOSED.to_string()),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 金额上调：响应体与 GET 回读一致、amount 以字符串形态出参、
/// 收款单已核销分配明细未被破坏
#[tokio::test]
async fn update_payment_amount_increase_readback_and_verified_allocation_intact() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_for_amount_edit(&db).await;
    seed_receipt_verified_allocation_60(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"amount":"150.00"}"#).await;
    assert_eq!(status, StatusCode::OK, "金额上调失败: {v}");
    assert!(v["data"]["amount"].is_string(), "Decimal 出参必须是字符串");
    assert_eq!(v["data"]["amount"], "150.00");

    let (status, g) = get_payment_body(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        g["data"]["amount"], "150.00",
        "改后 GET 回读必须得到新金额（有写必须有读）"
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.collection_amount, Decimal::from_str("150.00").unwrap());

    // 已核销分配明细原样保留（上调不动分配账本，发票侧自洽关系不被破坏）
    let item = bingxi_backend::models::ar_reconciliation_item::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.amount, -Decimal::from_str("60.00").unwrap());
    assert_eq!(item.document_id, Some(1));
    assert_eq!(
        item.match_status,
        bingxi_backend::models::status::ar::MATCH_MATCHED
    );
}

/// 新金额小于已核销分配金额 → 业务拒绝（文案不含记录 ID/内部结构），
/// 且同载荷携带的 remark 修改一并回滚，库内原值零写入
#[tokio::test]
async fn update_payment_amount_below_verified_total_rejected_and_no_partial_write() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_for_amount_edit(&db).await;
    seed_receipt_verified_allocation_60(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"amount":"30.00","remark":"不应部分写入"}"#).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "低于已分配额的下调必须被拒: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"],
        "新收款金额小于该收款单已核销分配金额，不可下调"
    );
    assert!(
        !v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("COL-EDIT")
            && !v["message"]
                .as_str()
                .unwrap_or_default()
                .contains("收款单 1"),
        "拒绝文案不得外泄记录标识: {}",
        v["message"]
    );

    // 事务回滚证明：金额与同载荷 remark 均未落库
    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.collection_amount, Decimal::from_str("100.00").unwrap());
    assert!(row.remark.is_none(), "被拒请求的其余字段不得部分写入");
}

/// 收款日期改为已关账期间 → 复用关账判据拒绝，库内日期保持原值
#[tokio::test]
async fn update_payment_date_into_closed_period_rejected() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_for_amount_edit(&db).await;
    seed_closed_period_2026_05(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"payment_date":"2026-05-15"}"#).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "关账期间日期变更必须被拒: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("该期间的数据已被锁定"),
        "拒绝文案必须来自 check_date_locked_txn 同源判据: {}",
        v["message"]
    );

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.collection_date,
        NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
    );
}

/// NOT NULL 列显式 null → 业务拒绝而不是静默落默认值（amount 与 payment_date 双向锁）
#[tokio::test]
async fn update_payment_not_null_columns_explicit_null_rejected() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_for_amount_edit(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"amount":null}"#).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "amount 显式 null 必须被拒: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "收款金额不能清空：该字段为必填项");

    let (status, v) = put_payment_body(&app, r#"{"payment_date":null}"#).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "payment_date 显式 null 必须被拒: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "收款日期不能清空：该字段为必填项");

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.collection_amount, Decimal::from_str("100.00").unwrap());
    assert_eq!(
        row.collection_date,
        NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
    );
}

/// 键缺席保持：只发改收款方式时，金额与收款日期经响应体、GET、DB 三处均为原值
#[tokio::test]
async fn update_payment_amount_and_date_absent_keep_original_values() {
    let db = test_common::setup_test_db().await;
    seed_pending_collection_for_amount_edit(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = build_app(state, make_auth(100));

    let (status, v) = put_payment_body(&app, r#"{"payment_method":"cash"}"#).await;
    assert_eq!(status, StatusCode::OK, "缺席键更新失败: {v}");
    assert_eq!(
        v["data"]["amount"], "100.00",
        "amount 键缺席必须保持原值（响应体）"
    );
    assert_eq!(
        v["data"]["payment_date"], "2026-04-10",
        "payment_date 键缺席必须保持原值（响应体）"
    );

    let (status, g) = get_payment_body(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(g["data"]["amount"], "100.00");
    assert_eq!(g["data"]["payment_date"], "2026-04-10");

    let row = ar_collection::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.collection_amount, Decimal::from_str("100.00").unwrap());
    assert_eq!(
        row.collection_date,
        NaiveDate::from_ymd_opt(2026, 4, 10).unwrap()
    );
    assert_eq!(row.collection_method.as_deref(), Some("cash"));
}

/// DTO 三态形状锁（纯 serde，无 DB）：amount/payment_date 的
/// 缺席=None（保持）、显式 null=Some(None)（由 service 业务拒绝）、有值=Some(Some(v))；
/// 序列化回 JSON 时缺席省略键、显式 null 落 null 键（handler 以 to_value 传递 service）。
#[test]
fn ar_update_dto_amount_and_date_distinguish_absent_null_and_value() {
    let dto: ar_payment_handler::UpdateArPaymentRequest =
        serde_json::from_value(serde_json::json!({
            "amount": "150.00",
            "payment_date": "2026-06-01",
            "remark": null,
        }))
        .expect("AR update DTO amount/payment_date 三态反序列化失败");
    assert!(matches!(
        &dto.amount,
        Some(Some(v)) if *v == Decimal::from_str("150.00").unwrap()
    ));
    assert!(matches!(
        &dto.payment_date,
        Some(Some(d)) if *d == NaiveDate::from_ymd_opt(2026, 6, 1).unwrap()
    ));
    assert!(matches!(dto.remark, Some(None)));

    let wire = serde_json::to_value(&dto).expect("AR update DTO amount/payment_date 序列化失败");
    assert!(
        wire["amount"].is_string(),
        "Decimal 上送 service 载荷必须保持字符串形态"
    );
    assert_eq!(wire["amount"], "150.00");
    assert_eq!(wire["payment_date"], "2026-06-01");
    assert!(wire.get("payment_method").is_none(), "缺席键序列化必须省略");

    let absent: ar_payment_handler::UpdateArPaymentRequest =
        serde_json::from_value(serde_json::json!({"remark": "x"})).unwrap();
    assert!(
        absent.amount.is_none(),
        "amount 键缺席必须是 None（保持原值）"
    );
    assert!(
        absent.payment_date.is_none(),
        "payment_date 键缺席必须是 None（保持原值）"
    );

    let nulled: ar_payment_handler::UpdateArPaymentRequest =
        serde_json::from_value(serde_json::json!({"amount": null, "payment_date": null})).unwrap();
    assert!(matches!(nulled.amount, Some(None)));
    assert!(matches!(nulled.payment_date, Some(None)));
    assert!(
        nulled.validate().is_ok(),
        "NOT NULL 列的显式 null 不被 DTO 校验阻挡，由 service 入口业务拒绝"
    );
}
