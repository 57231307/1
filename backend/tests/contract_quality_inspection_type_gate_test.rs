//! 质量检验记录写入口「检验类型取值域门」契约锁
//!
//! 锁定的契约（`quality_inspection_records.inspection_type` 列，DB VARCHAR 无 CHECK，
//! 取值域唯一来源 = 写入方词表 `models::status::quality_inspection_type::ALL`，
//! 判定函数 `handlers::quality_inspection_handler::validate_inspection_type`，
//! 列表/导出查询侧与创建/更新写入侧共用这一个函数，不引入第二套手写常量）：
//! - POST /records 创建：词表外值在任何事务/落库动作之前拒绝——HTTP 400、
//!   code=VALIDATION_ERROR、message 逐字符点名 ALL.join("/") 允许清单，
//!   且拒绝路径库里零新增（防"门立在写入之后"的假关门）；
//! - PUT /records/{id} 更新：请求携带覆盖值时同样拒绝且原行类型不漂移；
//!   键缺席=保持原值（NOT NULL 列的"不传"是合法语义，缺省不判错）；
//! - 词表内全部 token 创建放行并逐字符落库（防判错过严把合法写入拦成 400）。
//! - 本列与 `ai_quality_predictions.inspection_type` 的 CHECK 词表
//!   （all/incoming/inprocess/final/outgoing）分属两套，拒绝组专门包含后者，
//!   任何一侧借用另一侧词表都当场红。
//!
//! 夹具范式同 contract_wave11_created_by_from_session_batch1：
//! 真 PostgreSQL（test_common::setup_test_db，连接已迁移库并 TRUNCATE 业务表，
//! 缺 TEST_DATABASE_URL 直接 panic，禁止 sqlite 回退假绿）+ 占位 Router 注入会话，
//! 非 #[ignore]，随 CI 的 Rust 测试分片 job（该 job 注入 TEST_DATABASE_URL）执行。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::quality_inspection_handler::{create_record, update_record};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::quality_inspection_record;
use bingxi_backend::models::status::quality_inspection_type;
use chrono::Utc;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话注入用户（私有 id 段，与同族 created_by 锁的占位会话同一形态）
const SESSION_USER: i32 = 9641;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("qi_type_gate_{user_id}"),
        role_id: Some(2),
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

/// 按真实挂载形态注册创建/更新两个写口的占位 Router
fn layered_app(state: AppState) -> Router {
    Router::new()
        .route("/records", post(create_record))
        .route("/records/{id}", put(update_record))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(SESSION_USER), inject_auth))
}

async fn call(app: &Router, method: Method, path: &str, body: &Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

/// 创建请求体：其余字段保持合法，只变化被测的 inspection_type；
/// 结论走本域中文词表，避免两个门的拒绝原因互相掩盖
fn create_body(type_token: &str, prefix: &str) -> Value {
    json!({
        "inspection_no": format!("{prefix}-QITG-{}", unique_suffix()),
        "inspection_type": type_token,
        "product_id": 1,
        "inspection_date": "2026-01-01",
        "total_qty": "100",
        "inspected_qty": "100",
        "qualified_qty": "100",
        "unqualified_qty": "0",
        "qualification_rate": "100",
        "inspection_result": "合格"
    })
}

/// 越界拒绝的期望文案：由权威词表 join 生成，禁止手写第二套清单
fn expected_allow_list_message(raw: &str) -> String {
    format!(
        "检验类型 {raw} 不是合法取值，允许值：{}",
        quality_inspection_type::ALL.join("/")
    )
}

async fn record_count(db: &DatabaseConnection) -> u64 {
    quality_inspection_record::Entity::find()
        .count(db)
        .await
        .expect("统计 quality_inspection_records 行数失败")
}

async fn read_row(db: &DatabaseConnection, id: i32) -> quality_inspection_record::Model {
    quality_inspection_record::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("直读质检记录 {id} 失败: {e}"))
        .unwrap_or_else(|| panic!("质检记录 {id} 必须存在"))
}

// =========================================================
// ① 创建写口：越界类型 400 + message 点名允许清单 + 库里零新增
// =========================================================

#[tokio::test]
async fn create_rejects_outside_domain_type_and_leaves_no_rows() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let app = layered_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });

    // 拒绝组：ai_quality_predictions 表 CHECK 词表整组（all/inprocess/final 对
    // 本列均是词表外近邻码）、别套词表自造变体、中文界面标签、空串——一律拒绝
    for bad in ["incoming_check", "inprocess", "final", "all", "成品", ""] {
        let (status, body) = call(&app, Method::POST, "/records", &create_body(bad, "REJ")).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "词表外类型 {bad:?} 建单必须 400，实际: {status} {body}"
        );
        assert_eq!(
            body["code"], "VALIDATION_ERROR",
            "越界类型机器码必须 VALIDATION_ERROR，实际: {body}"
        );
        assert_eq!(
            body["message"].as_str(),
            Some(expected_allow_list_message(bad).as_str()),
            "越界类型必须逐字符点名 ALL 允许清单，实际: {body}"
        );
    }
    assert_eq!(
        record_count(&read_db).await,
        0,
        "门必须立在写入之前：被拒绝的建单不得留下任何质检行"
    );
}

// =========================================================
// ② 创建写口：词表内全部 token 放行并逐字符落库（防判错过严）
// =========================================================

#[tokio::test]
async fn create_accepts_every_domain_token_verbatim() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let app = layered_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });

    for token in quality_inspection_type::ALL {
        let (status, body) = call(&app, Method::POST, "/records", &create_body(token, "OK")).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "词表内类型 {token:?} 建单必须成功（判错过严即缺陷），实际: {status} {body}"
        );
        let id = body["data"]["id"]
            .as_i64()
            .unwrap_or_else(|| panic!("建单出参必须携带新行 id，实际: {body}"))
            as i32;
        let row = read_row(&read_db, id).await;
        assert_eq!(
            row.inspection_type, *token,
            "检验类型必须逐字符落库，禁止归一/转写"
        );
    }
    assert_eq!(
        record_count(&read_db).await,
        quality_inspection_type::ALL.len() as u64,
        "合法建单逐条真实落库"
    );
}

// =========================================================
// ③ 更新写口：缺省放行（键缺席=保持原值）、越界拒绝且零漂移、合法覆盖放行
// =========================================================

#[tokio::test]
async fn update_default_keeps_type_and_outside_domain_rejected_without_drift() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let app = layered_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });

    let (status, body) = call(
        &app,
        Method::POST,
        "/records",
        &create_body(quality_inspection_type::PROCESS, "UPD"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "前置建单必须成功，实际: {status} {body}"
    );
    let id = body["data"]["id"].as_i64().expect("前置建单行 id") as i32;

    // 缺省值放行：请求体不带 inspection_type 键（"不传"= 保持原值，合法语义）
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("/records/{id}"),
        &json!({ "remark": "类型键缺席仅改备注" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "更新缺 inspection_type 键必须放行，实际: {status} {body}"
    );
    let row = read_row(&read_db, id).await;
    assert_eq!(
        row.inspection_type,
        quality_inspection_type::PROCESS,
        "键缺席必须保持原类型"
    );
    assert_eq!(
        row.remark.as_deref(),
        Some("类型键缺席仅改备注"),
        "同请求携带的备注必须真实落库"
    );

    // 越界覆盖值拒绝且原行零漂移
    for bad in ["incoming_check", "final", ""] {
        let (status, body) = call(
            &app,
            Method::PUT,
            &format!("/records/{id}"),
            &json!({ "inspection_type": bad }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "词表外类型 {bad:?} 更新必须 400，实际: {status} {body}"
        );
        assert_eq!(
            body["code"], "VALIDATION_ERROR",
            "越界更新机器码必须 VALIDATION_ERROR，实际: {body}"
        );
        assert_eq!(
            body["message"].as_str(),
            Some(expected_allow_list_message(bad).as_str()),
            "越界更新必须逐字符点名 ALL 允许清单，实际: {body}"
        );
        let row = read_row(&read_db, id).await;
        assert_eq!(
            row.inspection_type,
            quality_inspection_type::PROCESS,
            "被拒绝的更新不得改动原行类型（拒绝零副作用）"
        );
    }

    // 合法覆盖值放行并逐字符落库
    let (status, body) = call(
        &app,
        Method::PUT,
        &format!("/records/{id}"),
        &json!({ "inspection_type": quality_inspection_type::OUTGOING }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "词表内类型覆盖更新必须成功（判错过严即缺陷），实际: {status} {body}"
    );
    let row = read_row(&read_db, id).await;
    assert_eq!(
        row.inspection_type,
        quality_inspection_type::OUTGOING,
        "合法覆盖必须逐字符落库"
    );
    assert_eq!(
        record_count(&read_db).await,
        1,
        "更新链路全程不得新增质检行"
    );
}
