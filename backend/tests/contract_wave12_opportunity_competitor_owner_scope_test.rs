//! 功能：商机子竞品面（`/opportunities/{id}/competitors` 的 GET 与 POST）行级归属
//! 门禁的契约锁，钉死"读侧与写侧都先过父商机归属门"，防水平越权（IDOR）回潮。
//! 调用方：`cargo test` 集成层（真已迁移 PostgreSQL，TEST_DATABASE_URL）。
//! 入参：经 `test_common::setup_test_db()` 连接、清空业务表后，按固定 id 播种两条
//! 商机（分属两个 crm_rep）、一条竞品与一条已存在的竞品关联；再以不同
//! `AuthContext`（data_scope=self，user_id= 商机 owner / 非 owner）走真 HTTP 装配。
//! 传给谁：`handlers::crm_handler::list_opportunity_competitors` 与
//! `add_opportunity_competitor`（经 `middleware::auth_context::AuthContext` 注入）。
//! 存什么：写路径成功后把 `opportunity_competitor` 关联行落库；越权拒绝必须零落库。
//! 存哪里：`opportunity_competitor` 表（回读真库比对证明"拒绝即零漂移、放行即真写"）。
//!
//! 判据四条（E3=GET list / E4=POST add / E5,E6=cross owner）：
//! - crm_rep 访问**自己**商机的竞品：GET 2xx 且如实返回真实关联行（非空、非 mock）；
//!   POST 2xx 且关联行确实写入该商机。
//! - crm_rep 访问**不属于自己的**商机：GET 403 + code=FORBIDDEN（不得降级成 2xx 空列表，
//!   那是"看起来正常"的假绿）；POST 403 + code=FORBIDDEN 且 `opportunity_competitor`
//!   对该商机零新增（拒绝必须发生在任何写入之前，不允许先写再拒）。
//! - 失败信封键齐全：code/message/trace_id/timestamp；message 为固定脱敏常量，
//!   不含商机号/记录 ID（本仓对外文案红线）。
//!
//! 反空操作自证：修复前 handler 把会话提取器写成 `_auth`（丢弃），list 仅按
//! `opportunity_id` 过滤、add 盲插任意 `opportunity_id`，均无归属校验；故"cross owner
//! 应 403"两条用例在修复前会拿到 2xx（读：直接返回他人关联行；写：直接落库成功），
//! 断言必红。判红的关键分叉即修复后 list/add 各自新增的
//! `service.get_opportunity(id, Some(&data_scope_ctx)).await?` —— 该 `?` 让归属门失败
//! 时立即向上传 `AppError::permission_denied`，不再触达其后的竞品查询/插入。
//!
//! 夹具防法：`setup_test_db()` 自带 TRUNCATE 台账双向守卫（见 services/test_common.rs
//! 清表前守卫 + 清表后 seaql_migrations 行数自检），本文件不调 Migrator、不依赖台账
//! 内容，故不受"TRUNCATE 抹 seaql_migrations 致真库断言空转"影响。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::{
    add_opportunity_competitor, list_opportunity_competitors,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{competitor, crm_opportunity, customer, opportunity_competitor, user};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// 固定播种 id：owner_a 的商机与竞品关联可访问，owner_b 的商机为越权目标。
const OWNER_A: i32 = 8001;
const OWNER_B: i32 = 8002;
const CUSTOMER_ID: i32 = 8100;
const OPP_OWN: i32 = 8201; // owner_a 名下
const OPP_CROSS: i32 = 8202; // owner_b 名下
const COMPETITOR_ID: i32 = 8300;
const LINK_SEED_ID: i32 = 8401; // 预置：OPP_OWN → COMPETITOR_ID

/// 构造 self 范围会话（role_id=2 非 admin；owner 与跨主都靠行级归属判定，不靠 RBAC）。
fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("competitor_owner_{user_id}"),
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

/// 真库播种：两条商机分属两个 owner，OPP_OWN 预置一条竞品关联，OPP_CROSS 不预置。
async fn seed(db: &sea_orm::DatabaseConnection) {
    use sea_orm::ActiveModelTrait;
    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("competitor_owner_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(1)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    customer::ActiveModel {
        id: Set(CUSTOMER_ID),
        customer_code: Set("CUS-COMP-0001".to_string()),
        customer_name: Set("竞品套件客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OWNER_A),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    for (opp_id, owner) in [(OPP_OWN, OWNER_A), (OPP_CROSS, OWNER_B)] {
        crm_opportunity::ActiveModel {
            id: Set(opp_id),
            opportunity_no: Set(format!("OPP-COMP-{opp_id}")),
            opportunity_name: Set(format!("竞品套件商机-{opp_id}")),
            customer_id: Set(CUSTOMER_ID),
            owner_id: Set(owner),
            owner_name: Set(format!("competitor_owner_{owner}")),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    competitor::ActiveModel {
        id: Set(COMPETITOR_ID),
        name: Set("竞品对手甲".to_string()),
        created_at: Set(Some(Utc::now())),
        updated_at: Set(Some(Utc::now())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    opportunity_competitor::ActiveModel {
        id: Set(LINK_SEED_ID),
        opportunity_id: Set(OPP_OWN),
        competitor_id: Set(COMPETITOR_ID),
        threat_level: Set(Some("high".to_string())),
        notes: Set(Some("既有竞品".to_string())),
        created_at: Set(Some(Utc::now())),
    }
    .insert(db)
    .await
    .unwrap();
}

/// 播种真实库并装配仅含两个竞品子资源端点 + 指定会话的 Router。
/// 返回真库句柄，供用例以**同一条连接**回读落库结果（证明零漂移/真写）。
async fn seeded_db_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/opportunities/{id}/competitors",
            get(list_opportunity_competitors).post(add_opportunity_competitor),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth));
    (app, db)
}

/// OPP 商机下的竞品关联行数（真库回读，拒绝零漂移 / 放行真写的唯一可信证据）。
async fn link_count_for_opp(db: &sea_orm::DatabaseConnection, opp_id: i32) -> i64 {
    opportunity_competitor::Entity::find()
        .filter(opportunity_competitor::Column::OpportunityId.eq(opp_id))
        .all(db)
        .await
        .unwrap()
        .len() as i64
}

// =============================================================
// E3：crm_rep 读自己商机的竞品 → 2xx 且如实返回真实关联行
// =============================================================
#[tokio::test]
async fn crm_rep_list_competitors_of_own_opportunity_is_2xx_and_truthful() {
    let (app, _db) = seeded_db_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/opportunities/{OPP_OWN}/competitors"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 商机竞品读应 2xx: {v}");
    assert_eq!(v["code"], 200);
    let rows = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("data 应为竞品数组: {v}"));
    assert_eq!(
        rows.len(),
        1,
        "应如实返回预置的 1 条关联（非空、非 mock）: {v}"
    );
    assert_eq!(rows[0]["competitor_id"], COMPETITOR_ID);
    assert_eq!(rows[0]["competitor_name"], "竞品对手甲");
    assert_eq!(rows[0]["id"], LINK_SEED_ID);
}

// =============================================================
// E4：crm_rep 写自己商机的竞品 → 2xx 且关联行确实落库
// =============================================================
#[tokio::test]
async fn crm_rep_add_competitor_to_own_opportunity_is_2xx_and_persists() {
    let (app, db) = seeded_db_app(make_auth(OWNER_A)).await;
    let body = json!({
        "competitor_id": COMPETITOR_ID,
        "threat_level": "medium",
        "notes": "own 写入"
    });
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/opportunities/{OPP_OWN}/competitors"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "own 商机竞品写应 2xx: {v}");
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["opportunity_id"], OPP_OWN);
    assert_eq!(v["data"]["competitor_id"], COMPETITOR_ID);

    let count = link_count_for_opp(&db, OPP_OWN).await;
    assert_eq!(
        count, 2,
        "own 写入后该商机的关联行应为预置1+新增1: 实得 {count}"
    );
}

// =============================================================
// E5：crm_rep 读他人商机的竞品 → 403 FORBIDDEN（绝不降级成 2xx 空列表）
// =============================================================
#[tokio::test]
async fn crm_rep_list_competitors_of_others_opportunity_is_403_not_empty_200() {
    let (app, _db) = seeded_db_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/opportunities/{OPP_CROSS}/competitors"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主竞品读必须 403（修复前为 2xx 返回他人关联行）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "失败信封 code 契约: {v}");
    assert_ne!(v["code"], "INTERNAL_ERROR", "越权不得拍平成 500");
    assert!(
        v["message"].is_string() && v["trace_id"].is_string() && v["timestamp"].is_number(),
        "失败信封键齐全（code/message/trace_id/timestamp）: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains(&OPP_CROSS.to_string()),
        "对外文案不得拼商机记录 ID: {v}"
    );
    // 关键反假绿：绝不能把越权静默降级成 200+空数组。
    assert_ne!(status, StatusCode::OK, "越权读严禁伪装成正常 2xx 空列表");
}

// =============================================================
// E6：crm_rep 写他人商机的竞品 → 403 FORBIDDEN 且零落库（先判后写）
// =============================================================
#[tokio::test]
async fn crm_rep_add_competitor_to_others_opportunity_is_403_and_zero_write() {
    let (app, db) = seeded_db_app(make_auth(OWNER_A)).await;
    let before = link_count_for_opp(&db, OPP_CROSS).await;
    assert_eq!(before, 0, "跨主商机不应有预置关联（前提）");

    let body = json!({
        "competitor_id": COMPETITOR_ID,
        "threat_level": "low",
        "notes": "越权写入"
    });
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/opportunities/{OPP_CROSS}/competitors"),
        Some(body),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "跨主竞品写必须 403（修复前为盲插 2xx）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_ne!(v["code"], "INTERNAL_ERROR");

    let after = link_count_for_opp(&db, OPP_CROSS).await;
    assert_eq!(after, 0, "越权写被拒后该商机关联必须零新增（不得先写再拒）");
}
