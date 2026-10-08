//! 契约测试：客户合并撞信用评级唯一约束 fail-visible（拒绝合并 + 零部分写入）
//!
//! ## 判据与拒绝口径
//! `handlers/customer_merge_handler.rs` 把源客户信用评级行的 `customer_id` 改写为目标客户
//! id。评级表对 `customer_id` 有全表唯一约束（索引 `uq_customer_credit_ratings_customer`，
//! 读写契约 = 每客户一行），源、目标各已有一行时该改写撞唯一约束——撞约束在 SeaORM 层以
//! `DbErr` 冒泡经 `?` 映射为 `DATABASE_ERROR`(500)，不可据以纠正，且若发生在事务中途会留
//! 半写状态。据此，端点在开事务、动任何数据之前先判「两侧是否都有评级行」，命中即以
//! `AppError::business` 拒绝（出参固定脱敏文案 + `BUSINESS_ERROR`，真实原因只进日志，
//! 文案不含记录 ID），保证数据库零写入、两侧评级行都原样保留（既不静默挪源覆盖目标，
//! 也不静默保留源）。
//!
//! ## 锁的三条（缺一不可）
//! - 撞评级 → HTTP 400 + `code=BUSINESS_ERROR`（**不是** 500/DATABASE_ERROR，也不是裸拒绝）；
//! - 拒绝后**两侧评级行都还在且内容未被改写**（customer_id、credit_limit 逐值回读比对，
//!   行数各恰 1）——这条是关键，防「拒绝但已把目标覆盖掉/把源挪走」的半写状态；
//! - 正向对照：仅源有评级、目标无评级 → 合并不被本判据拒绝（2xx），源评级行按既有语义
//!   改指目标（客户单行唯一未被破坏）——证明拒绝分支是**条件触发**而非「合并一律拒」回潮。
//!
//! ## 通道与真库
//! 经真实 `POST /erp/customers/merge` 装配（`test_common::setup_test_db()` 真 PostgreSQL +
//! 真 HTTP，表结构唯一来源 = `backend/migration`，禁 sqlite/mock）。合并端点写侧过
//! `crm_write_guard::ensure_cross_owner_write_allowed`（owner=created_by），本锁让操作人即
//! 两客户创建人（本人行直接放行），聚焦唯一约束分支，不在此重复验证归属门。夹具
//! `setup_test_db()` 缺 `TEST_DATABASE_URL` 或指向 sqlite 直接 panic——行为断言只有 CI 活库有结果。
//!
//! ## id 带
//! 本文件独占 997xxx 带（users 997001、customers 997011/997012、评级行 997101/997102），
//! 每用例先删后插保幂等；`departments.id=1` 属密封参照表只引用。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::customer_merge_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const USER_OP: i32 = 997_001;
const CUSTOMER_SOURCE: i32 = 997_011;
const CUSTOMER_TARGET: i32 = 997_012;
const RATING_SOURCE_ID: i32 = 997_101;
const RATING_TARGET_ID: i32 = 997_102;
/// 两侧评级行的可区分额度（断言"内容未被改写"用）
const SRC_LIMIT: i64 = 5_000_000;
const TGT_LIMIT: i64 = 8_000_000;

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: USER_OP,
        username: "merge_unique_lock".to_string(),
        role_id: None,
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

async fn exec(db: &Arc<sea_orm::DatabaseConnection>, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

async fn insert_rating(
    db: &Arc<sea_orm::DatabaseConnection>,
    row_id: i32,
    customer_id: i32,
    limit_lo: i64,
) {
    exec(
        db,
        &format!(
            "INSERT INTO customer_credit_ratings (id,customer_id,credit_limit,status) VALUES ({row_id},{customer_id},{limit_lo}.00,'active')"
        ),
    )
    .await;
}

async fn build_state() -> Arc<sea_orm::DatabaseConnection> {
    let db = Arc::new(test_common::setup_test_db().await);
    exec(
        &db,
        &format!(
            "DELETE FROM customer_credit_ratings WHERE id IN ({RATING_SOURCE_ID},{RATING_TARGET_ID})"
        ),
    )
    .await;
    exec(
        &db,
        &format!("DELETE FROM customers WHERE id IN ({CUSTOMER_SOURCE},{CUSTOMER_TARGET})"),
    )
    .await;
    exec(&db, &format!("DELETE FROM users WHERE id = {USER_OP}")).await;
    exec(
        &db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES ({USER_OP},'merge_op','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    exec(
        &db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at) VALUES
             ({CUSTOMER_SOURCE},'CUS-SRC','合并源客户',0,30,'active',{USER_OP},{USER_OP},'retail','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             ({CUSTOMER_TARGET},'CUS-TGT','合并目标客户',0,30,'active',{USER_OP},{USER_OP},'retail','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    db
}

fn app(db: &Arc<sea_orm::DatabaseConnection>) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/customers/merge",
            post(customer_merge_handler::merge_customers),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
}

async fn merge(app: &Router, source: i32, target: i32) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/erp/customers/merge")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"source_customer_id": source, "target_customer_id": target}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!("响应非 JSON: {e}; body={}", String::from_utf8_lossy(&bytes))
        }),
    )
}

/// 该评级行的 (customer_id, credit_limit) 回读；行不存在返回 None（供"内容未被改写"逐值比对）。
async fn rating_row(db: &Arc<sea_orm::DatabaseConnection>, row_id: i32) -> Option<(i32, i64)> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("SELECT customer_id, credit_limit::bigint AS lim FROM customer_credit_ratings WHERE id={row_id}"),
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("回读评级行 {row_id} 失败: {e}"));
    let row = match row {
        Some(r) => r,
        None => return None,
    };
    Some((
        row.try_get::<i32>("", "customer_id")
            .expect("customer_id 解码失败"),
        row.try_get::<i64>("", "lim")
            .expect("credit_limit 解码失败"),
    ))
}

/// 某客户名下评级行数（COUNT 回读）。
async fn rating_count(db: &Arc<sea_orm::DatabaseConnection>, customer_id: i32) -> i64 {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "SELECT COUNT(*) AS cnt FROM customer_credit_ratings WHERE customer_id={customer_id}"
            ),
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("回读评级行数失败: {e}"))
        .expect("COUNT 恒有行");
    row.try_get::<i64>("", "cnt").expect("行数解码失败")
}

/// 撞评级 → 400 BUSINESS_ERROR + 两侧评级行原样保留、内容未改写、零部分写入。
#[tokio::test]
async fn merge_blocked_when_both_customers_have_ratings() {
    let db = build_state().await;
    insert_rating(&db, RATING_SOURCE_ID, CUSTOMER_SOURCE, SRC_LIMIT).await;
    insert_rating(&db, RATING_TARGET_ID, CUSTOMER_TARGET, TGT_LIMIT).await;

    let (status, body) = merge(&app(&db), CUSTOMER_SOURCE, CUSTOMER_TARGET).await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "两侧都有评级行的合并必须 400（撞唯一约束须映射业务拒绝、不得冒 500），实际 {status} body={body}"
    );
    assert_eq!(
        body["code"], "BUSINESS_ERROR",
        "拒绝必须归 BUSINESS 机器码（状态/业务拒绝归 BUSINESS_ERROR，非 VALIDATION/INTERNAL），实际 {body}"
    );
    let message = body["message"].as_str().unwrap_or_default();
    assert!(
        !message.contains(&CUSTOMER_SOURCE.to_string())
            && !message.contains(&CUSTOMER_TARGET.to_string()),
        "用户可见文案禁止含记录 ID，实际 message={message:?}"
    );

    // 关键：拒绝即整笔回滚，两侧评级行都还在且内容未被改写
    assert_eq!(
        rating_row(&db, RATING_SOURCE_ID).await,
        Some((CUSTOMER_SOURCE, SRC_LIMIT)),
        "源评级行必须原样保留（未被挪去覆盖目标）"
    );
    assert_eq!(
        rating_row(&db, RATING_TARGET_ID).await,
        Some((CUSTOMER_TARGET, TGT_LIMIT)),
        "目标评级行必须原样保留（内容未被改写）"
    );
    assert_eq!(
        rating_count(&db, CUSTOMER_SOURCE).await,
        1,
        "源客户评级行数必须仍为 1（无部分写入）"
    );
    assert_eq!(
        rating_count(&db, CUSTOMER_TARGET).await,
        1,
        "目标客户评级行数必须仍为 1（无第二行/覆盖产生）"
    );
    // 合并数据零写入：源客户不得被置 merged（判据在开事务前，任何转移/标记均未发生）
    let src_status: String = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("SELECT status FROM customers WHERE id={CUSTOMER_SOURCE}"),
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读源客户状态失败")
        .expect("源客户行应存在")
        .try_get("", "status")
        .expect("status 解码失败");
    assert_ne!(
        src_status, "merged",
        "撞评级拒绝后源客户不得被置 merged（拒绝先于任何写入，零部分写入），实际 status={src_status}"
    );
}

/// 正向对照：仅源有评级、目标无评级 → 合并不被本判据拒绝，源评级行改指目标（单行唯一不破）。
#[tokio::test]
async fn merge_transfers_rating_when_only_source_has_rating() {
    let db = build_state().await;
    insert_rating(&db, RATING_SOURCE_ID, CUSTOMER_SOURCE, SRC_LIMIT).await;
    // 目标客户刻意不插评级行

    let (status, body) = merge(&app(&db), CUSTOMER_SOURCE, CUSTOMER_TARGET).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "目标无评级行时合并不应被唯一约束判据拒绝（判据只在两侧都有评级时触发），实际 {status} body={body}"
    );
    // 既有语义：源评级行 customer_id 改指目标；源侧留 0 行、目标侧恰 1 行（单行唯一未被破坏）
    assert_eq!(
        rating_row(&db, RATING_SOURCE_ID).await,
        Some((CUSTOMER_TARGET, SRC_LIMIT)),
        "无碰撞时源评级行应被改指目标客户（内容/额度不变）"
    );
    assert_eq!(
        rating_count(&db, CUSTOMER_SOURCE).await,
        0,
        "评级行已从源客户转移，源侧不应残留"
    );
    assert_eq!(
        rating_count(&db, CUSTOMER_TARGET).await,
        1,
        "目标客户合并后恰一行评级（未破单行唯一约束）"
    );
}
