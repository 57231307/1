//! API 密钥「描述/有效期真实回显 + 三态清空 + 非法格式拒绝」契约锁（wave4）
//!
//! 锁定的真实契约（对应本轮修复，基准 `backend/src/handlers/api_gateway_handler.rs`）：
//! 1. 出参不再硬写空串：
//!    - `description` = `api_keys.description` 真实可空列（此前 json! 里恒为 `""`，
//!      用户填的描述「存进去却永远显示不出来」）；
//!    - `expires_at` = 列真值，NULL → JSON `null`（= 永不过期），不再 `unwrap_or_default()`
//!      塌成 `""`（前端无法与脏值区分）；
//!    - `created_by_name` = 读侧 `column_as(users.username, "created_by_name")` +
//!      `JoinType::LeftJoin` + `into_model::<ApiKeyWithCreator>` 的真实用户名
//!      （此前恒为 `""`）；用户行缺失（悬挂 created_by）时如实为 `null`，
//!      禁止空串/"未知"/id 冒充（范式来源：提交 13f6bd09 售后工单 customer_name）；
//!    - `last_used_at` 与 expires_at 同口径：NULL → `null`（同源一致性修复）。
//! 2. 入参三态（`double_option` 适配器，本仓权威范式，见 department_handler.rs）：
//!    键缺席=保持原值、显式 `null`=清空（description→NULL / expires_at→永不过期）、有值=覆盖。
//!    修复前单层 `Option<String>` 把显式 null 塌成「键缺席」，service 的
//!    `UpdateApiKeyPayload.expires_at: Option<Option<DateTime>>` 三态能力从 HTTP 发不出来。
//! 3. 安全缺陷锁：`expires_at` 有值但格式非法 → 400 `VALIDATION_ERROR` + 真实外显文案，
//!    且**原 expires_at 一字不改**。修复前 `parse_from_rfc3339(s).ok()` 产出 `Some(None)`，
//!    等于「日期写错就把密钥改成永不过期」且无任何报错。
//! 4. 创建链路补 `description` 真实落库（此前 CreateApiKeyGwRequest 无该字段，传了被静默丢弃）。
//!
//! 覆盖策略（全部真实 handler/service 行为，无 mock、无静默 skip；路线一：
//! 统一真库 PostgreSQL）：
//! - 纯 serde：update DTO 三态形状锁（缺席=None / null=Some(None) / 有值=Some(Some(v))）；
//! - `test_common::setup_test_db()`：必须 `TEST_DATABASE_URL` → 已迁移 PostgreSQL，
//!   api_keys/users 表由 migration（m0005/m0001 + m0039(created_by) + m0044(description)）
//!   产出，用例不自建 DDL；缺变量/指 sqlite 由夹具 panic（sqlite 自建同构表是
//!   CI #4669 方言失真红的根因形态，已彻底移除回退路径）；
//! - 源码扫描防回潮锁：禁空串字面量出参、禁 `unwrap_or_default()` 出参、
//!   禁富化链路旁路（into_tuple / 手工拼装 created_by_name）、禁前端把 null 掩盖成空串。

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
use bingxi_backend::handlers::api_gateway_handler::{
    self, CreateApiKeyGwRequest, UpdateApiKeyGwRequest,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::api_key;
use bingxi_backend::models::user;
use bingxi_backend::services::api_key_service::ApiKeyService;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

/// 修复后 handler 使用的用户可见文案（不含内部机制术语）
const EXPIRES_AT_FORMAT_MSG: &str = "有效期格式不正确，应为 ISO 8601（如 2026-12-31T23:59:59Z）";
/// 悬挂创建者（users 无该行）：LEFT JOIN 必须产出 NULL，不得回退空串/假名
const ORPHAN_CREATOR_ID: i32 = 999_999;

// =========================================================
// 夹具
// =========================================================

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("gw_owner_{user_id}"),
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

/// 夹具：已迁移 PostgreSQL 真库（路线一，无 sqlite 回退）。
/// api_keys/users 由 m0005/m0001 + m0039(created_by) + m0044(description) 建好，
/// 用例不再自建同构 DDL；`created_by` 悬挂值合法（该列无 FK，LEFT JOIN NULL 态
/// 在真库可复现），正是本契约锁第二态的真实形态。
async fn prepare_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

/// 种子创建者用户（username 唯一，避免 PG 上撞唯一索引）
async fn seed_user(db: &DatabaseConnection, username: &str) -> i32 {
    let now = Utc::now();
    let created = user::ActiveModel {
        username: Set(username.to_string()),
        password_hash: Set("not-a-real-hash".to_string()),
        real_name: Set(Some(format!("{username}-姓名"))),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed 用户失败: {e}"));
    created.id
}

fn router(db: DatabaseConnection, user_id: i32) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/api-gateway/keys",
            axum::routing::get(api_gateway_handler::list_api_keys)
                .post(api_gateway_handler::create_api_key),
        )
        .route(
            "/api-gateway/keys/{id}",
            axum::routing::get(api_gateway_handler::get_api_key)
                .put(api_gateway_handler::update_api_key),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(user_id), inject_auth))
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

/// 夹具：已迁移 PostgreSQL 真库 + 种子创建者 + 真实 handler 路由
async fn seeded_app() -> (Router, DatabaseConnection, i32) {
    let db = prepare_db().await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let username = format!("gw_owner_{suffix}");
    let uid = seed_user(&db, &username).await;
    (router(db.clone(), uid), db, uid)
}

/// 出参键存在性 + 值断言（null 也必须是显式键，禁止整键缺失）
fn expect_key(obj: &Value, key: &str, expected: Value, label: &str) {
    assert!(
        obj.get(key).is_some(),
        "{label}：出参必须恒含 {key} 键（null 也要显式给），实际键={:?}",
        obj.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );
    assert_eq!(obj[key], expected, "{label}：键 {key} 期望 {expected}");
}

fn key_id(v: &Value, label: &str) -> i32 {
    v["data"]["id"]
        .as_i64()
        .map(|i| i as i32)
        .unwrap_or_else(|| panic!("{label}：无有效 id，实际体: {v}"))
}

async fn create_key(app: &Router, body: Value, label: &str) -> Value {
    let (status, v) = call(app, Method::POST, "/api-gateway/keys", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{label} 创建失败，实际体: {v}");
    v
}

async fn read_back(db: &DatabaseConnection, id: i32) -> api_key::Model {
    api_key::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("DB 回读不到密钥 id={id}"))
}

fn parse_iso(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .unwrap_or_else(|e| panic!("ISO 8601 解析失败 {s}: {e}"))
        .with_timezone(&Utc)
}

// =========================================================
// ① description 存进去也必须显示得出来（PUT → GET/list 回读等于原值）
// =========================================================

#[tokio::test]
async fn description_round_trips_through_put_and_get() {
    let (app, db, _uid) = seeded_app().await;

    // 创建链路也必须真实落库（修复前 CreateApiKeyGwRequest 根本没有 description 字段）
    let created = create_key(
        &app,
        json!({ "key_name": format!("gw-{}-desc", Utc::now().timestamp_nanos_opt().unwrap()),
            "description": "对账用只读密钥",
            "expires_at": "2026-12-31T23:59:59Z" }),
        "描述回显①",
    )
    .await;
    expect_key(
        &created["data"],
        "description",
        json!("对账用只读密钥"),
        "创建响应",
    );
    let id = key_id(&created, "描述回显①");

    // PUT 覆盖描述 → 响应回显新值
    let (status, updated) = call(
        &app,
        Method::PUT,
        &format!("/api-gateway/keys/{id}"),
        Some(json!({ "description": "改过之后的描述" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "PUT 描述失败，实际体: {updated}");
    expect_key(
        &updated["data"],
        "description",
        json!("改过之后的描述"),
        "更新响应",
    );

    // GET 详情回读 == 原值（这正是修复前恒为 "" 的那条链路）
    let (status, detail) = call(&app, Method::GET, &format!("/api-gateway/keys/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "GET 详情失败，实际体: {detail}");
    expect_key(
        &detail["data"],
        "description",
        json!("改过之后的描述"),
        "详情回读",
    );

    // 列表同口径（列表与详情必须同源，不得一处空串一处真值）
    let (status, list) = call(
        &app,
        Method::GET,
        "/api-gateway/keys?page=1&page_size=200",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "列表失败，实际体: {list}");
    let row = list["data"]
        .as_array()
        .expect("列表 data 应为数组")
        .iter()
        .find(|it| it["id"].as_i64() == Some(id as i64))
        .unwrap_or_else(|| panic!("列表应含 id={id}"));
    expect_key(row, "description", json!("改过之后的描述"), "列表回读");

    // DB 列真值
    assert_eq!(
        read_back(&db, id).await.description.as_deref(),
        Some("改过之后的描述"),
        "DB description 必须真落库"
    );
}

// =========================================================
// ② 显式 null 清空：expires_at → NULL（永不过期）且 status 仍 active；
//    description → NULL；出参为 JSON null 而非 ""
// =========================================================

#[tokio::test]
async fn explicit_null_clears_expiry_and_description_and_outputs_json_null() {
    let (app, db, _uid) = seeded_app().await;
    let created = create_key(
        &app,
        json!({ "key_name": "gw-explicit-null", "description": "待清空描述",
            "expires_at": "2026-12-31T23:59:59Z" }),
        "显式 null 清空②",
    )
    .await;
    let id = key_id(&created, "显式 null 清空②");
    assert!(
        !created["data"]["expires_at"].is_null(),
        "创建时给了有效期，出参必须是真值，实际: {}",
        created["data"]
    );

    let (status, updated) = call(
        &app,
        Method::PUT,
        &format!("/api-gateway/keys/{id}"),
        Some(json!({ "expires_at": null, "description": null })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "显式 null 清空失败，实际体: {updated}"
    );
    // NULL 列 → JSON null（修复前塌成 ""，前端无法与脏值区分）
    expect_key(
        &updated["data"],
        "expires_at",
        Value::Null,
        "清空后更新响应",
    );
    expect_key(
        &updated["data"],
        "description",
        Value::Null,
        "清空后更新响应",
    );
    // 清空有效期不得顺手把密钥停用
    assert_eq!(
        updated["data"]["status"],
        json!("active"),
        "expires_at 清空后 status 必须仍为 active，实际: {}",
        updated["data"]
    );

    let (status, detail) = call(&app, Method::GET, &format!("/api-gateway/keys/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "清空后 GET 失败: {detail}");
    expect_key(&detail["data"], "expires_at", Value::Null, "清空后详情");
    expect_key(&detail["data"], "description", Value::Null, "清空后详情");

    let row = read_back(&db, id).await;
    assert!(
        row.expires_at.is_none(),
        "DB expires_at 必须为 NULL（永不过期）"
    );
    assert!(row.description.is_none(), "DB description 必须为 NULL");
    assert!(row.is_active, "DB is_active 必须仍为 true");
}

// =========================================================
// ③ 非法 expires_at：400 + VALIDATION_ERROR + 真实文案，且原值一字未改
//    （锁死「日期写错＝偷偷把密钥改成永不过期」的安全缺陷）
// =========================================================

#[tokio::test]
async fn invalid_expires_at_is_rejected_and_original_expiry_untouched() {
    let (app, db, _uid) = seeded_app().await;
    let created = create_key(
        &app,
        json!({ "key_name": "gw-bad-expiry", "expires_at": "2026-12-31T23:59:59Z" }),
        "非法有效期③",
    )
    .await;
    let id = key_id(&created, "非法有效期③");
    let original_expiry = created["data"]["expires_at"]
        .as_str()
        .expect("创建出参应含 expires_at 字符串")
        .to_string();
    let original_updated_at = read_back(&db, id).await.updated_at;

    for bad in ["2026-13-45", "2026-12-31 23:59:59", "", "not-a-date"] {
        let (status, v) = call(
            &app,
            Method::PUT,
            &format!("/api-gateway/keys/{id}"),
            Some(json!({ "expires_at": bad })),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "非法 expires_at={bad:?} 必须 400，实际体: {v}"
        );
        assert_eq!(
            v["code"],
            json!("VALIDATION_ERROR"),
            "非法 expires_at={bad:?} 应返回 VALIDATION_ERROR，实际体: {v}"
        );
        assert_eq!(
            v["message"],
            json!(EXPIRES_AT_FORMAT_MSG),
            "validation_displayable 文案必须原样外显（不得被脱敏成通用提示），实际体: {v}"
        );
    }

    // 原 expires_at 未被改动（出参与 DB 双向锁）
    let (status, detail) = call(&app, Method::GET, &format!("/api-gateway/keys/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "拒绝后 GET 失败: {detail}");
    expect_key(
        &detail["data"],
        "expires_at",
        json!(original_expiry),
        "非法请求后的详情",
    );
    let row = read_back(&db, id).await;
    assert_eq!(
        row.expires_at.map(|d| d.timestamp()),
        Some(parse_iso(&original_expiry).timestamp()),
        "非法 expires_at 绝不能把 DB 列改成 NULL 或别的值"
    );
    assert_eq!(
        row.updated_at, original_updated_at,
        "400 拒绝必须发生在任何写库之前，updated_at 不应被推进"
    );
}

// =========================================================
// ④ 键缺席 = 保持原值（含只改 status 的真实局部更新）
// =========================================================

#[tokio::test]
async fn absent_keys_keep_original_description_and_expiry() {
    let (app, db, _uid) = seeded_app().await;
    let created = create_key(
        &app,
        json!({ "key_name": "gw-absent-keys", "description": "缺席不该被动",
            "expires_at": "2026-12-31T23:59:59Z" }),
        "键缺席④",
    )
    .await;
    let id = key_id(&created, "键缺席④");
    let original_expiry = created["data"]["expires_at"].as_str().unwrap().to_string();

    // 只提交 status + rate_limit（前端「启用/停用」按钮的真实载荷形态）
    let (status, updated) = call(
        &app,
        Method::PUT,
        &format!("/api-gateway/keys/{id}"),
        Some(json!({ "status": "inactive", "rate_limit": 50 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "局部更新失败: {updated}");
    assert_eq!(updated["data"]["status"], json!("inactive"));
    assert_eq!(updated["data"]["rate_limit"], json!(50));
    expect_key(
        &updated["data"],
        "description",
        json!("缺席不该被动"),
        "局部更新响应",
    );
    expect_key(
        &updated["data"],
        "expires_at",
        json!(original_expiry),
        "局部更新响应",
    );

    let row = read_back(&db, id).await;
    assert_eq!(row.description.as_deref(), Some("缺席不该被动"));
    assert_eq!(
        row.expires_at.map(|d| d.timestamp()),
        Some(parse_iso(&original_expiry).timestamp())
    );
    assert!(!row.is_active, "status=inactive 应落 is_active=false");
}

// =========================================================
// ⑤ created_by_name 必须是真实用户名（LEFT JOIN 真值），悬挂 created_by → null
// =========================================================

#[tokio::test]
async fn created_by_name_is_real_username_and_null_for_dangling_creator() {
    let (app, db, uid) = seeded_app().await;
    let owner_username = user::Entity::find_by_id(uid)
        .one(&db)
        .await
        .unwrap()
        .expect("种子创建者必须存在")
        .username;

    let created = create_key(&app, json!({ "key_name": "gw-creator-name" }), "创建者名⑤").await;
    let id = key_id(&created, "创建者名⑤");
    expect_key(
        &created["data"],
        "created_by_name",
        json!(owner_username),
        "创建响应",
    );
    assert_ne!(
        created["data"]["created_by_name"],
        json!(""),
        "created_by_name 不得再硬写空串（修复前恒为空白的缺陷）"
    );

    let (status, detail) = call(&app, Method::GET, &format!("/api-gateway/keys/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "详情失败: {detail}");
    expect_key(
        &detail["data"],
        "created_by_name",
        json!(owner_username),
        "详情回读",
    );

    // 悬挂 created_by（users 无该行）：键必须存在且为 null，禁止空串/假名回退
    let service = ApiKeyService::new(Arc::new(db.clone()));
    let (orphan, _plain) = service
        .create_api_key(
            "gw-orphan-creator",
            None,
            None,
            100,
            None,
            ORPHAN_CREATOR_ID,
        )
        .await
        .expect("真实 service 建悬挂创建者密钥失败");
    let (status, list) = call(
        &app,
        Method::GET,
        "/api-gateway/keys?page=1&page_size=200",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "列表失败: {list}");
    let orphan_row = list["data"]
        .as_array()
        .expect("列表 data 应为数组")
        .iter()
        .find(|it| it["id"].as_i64() == Some(orphan.id as i64))
        .unwrap_or_else(|| panic!("列表应含悬挂创建者密钥 id={}", orphan.id));
    expect_key(
        orphan_row,
        "created_by_name",
        Value::Null,
        "悬挂创建者列表行",
    );
    assert_eq!(
        orphan_row["created_by"],
        json!(ORPHAN_CREATOR_ID),
        "created_by 外键值仍须如实透传，只有名称为 null"
    );
}

// =========================================================
// ⑥ DTO 三态形状锁（纯 serde，无 DB）
// =========================================================

#[test]
fn update_dto_distinguishes_absent_null_and_value() {
    let cleared: UpdateApiKeyGwRequest = serde_json::from_value(json!({
        "description": null,
        "expires_at": null,
        "key_name": "n",
    }))
    .expect("密钥 update DTO 三态反序列化失败");
    assert!(
        matches!(cleared.description, Some(None)),
        "显式 null 的 description 必须是 Some(None)（清空），不得与键缺席塌同"
    );
    assert!(
        matches!(cleared.expires_at, Some(None)),
        "显式 null 的 expires_at 必须是 Some(None)（永不过期）"
    );
    assert!(
        cleared.rate_limit.is_none(),
        "缺席键必须是 None（保持原值）"
    );
    assert!(cleared.permissions.is_none());
    assert!(cleared.status.is_none());

    let covered: UpdateApiKeyGwRequest = serde_json::from_value(json!({
        "description": "真实描述",
        "expires_at": "2026-12-31T23:59:59Z",
    }))
    .expect("密钥 update DTO 覆盖态反序列化失败");
    assert!(matches!(&covered.description, Some(Some(v)) if v == "真实描述"));
    assert!(
        matches!(&covered.expires_at, Some(Some(v)) if v == "2026-12-31T23:59:59Z"),
        "有值态必须原样交 handler 严格解析"
    );
    assert!(covered.key_name.is_none(), "缺席的 key_name 不得被当成清空");

    let absent: UpdateApiKeyGwRequest =
        serde_json::from_value(json!({})).expect("密钥 update DTO 全缺席反序列化失败");
    assert!(absent.description.is_none(), "键缺席=保持原值，必须是 None");
    assert!(absent.expires_at.is_none(), "键缺席=保持原值，必须是 None");

    // 创建 DTO：description 真实采集（修复前该字段根本不存在，传了被静默丢弃）
    let create: CreateApiKeyGwRequest =
        serde_json::from_value(json!({ "key_name": "k", "description": "创建即填" }))
            .expect("密钥 create DTO 反序列化失败");
    assert_eq!(create.description.as_deref(), Some("创建即填"));
}

// =========================================================
// ⑦ 源码扫描防回潮锁
// =========================================================

fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n}")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j + 2].to_string()
}

#[test]
fn source_scan_handler_no_fake_empty_string_or_default_output() {
    let src = include_str!("../src/handlers/api_gateway_handler.rs").replace('\r', "");
    let block = extract_block(&src, "fn key_to_json");
    for banned in [
        "\"description\": \"\"",
        "\"created_by_name\": \"\"",
        "expires_at.as_ref().map(|d| d.to_rfc3339()).unwrap_or_default()",
        "last_used_at.as_ref().map(|d| d.to_rfc3339()).unwrap_or_default()",
    ] {
        assert!(
            !block.contains(banned),
            "key_to_json 不得再出现假默认值出参 {banned:?}，实际块:\n{block}"
        );
    }
    for required in [
        "\"description\": &m.description",
        "\"created_by_name\": &m.created_by_name",
        "\"expires_at\": m.expires_at.as_ref().map(|d| d.to_rfc3339())",
    ] {
        assert!(
            block.contains(required),
            "key_to_json 必须原样输出可空列真值，缺锚点 {required}，实际块:\n{block}"
        );
    }
    // 三态适配器与严格解析必须在本 handler 内生效
    assert!(
        src.contains("fn double_option<"),
        "必须使用本仓 double_option 适配器承载三态（否则显式 null 塌成键缺席）"
    );
    for anchor in [
        "pub struct UpdateApiKeyGwRequest",
        "pub struct CreateApiKeyGwRequest",
    ] {
        let dto = extract_block(&src, anchor);
        assert!(
            dto.contains("description"),
            "{anchor} 必须真实接收 description，实际块:\n{dto}"
        );
    }
    let update_dto = extract_block(&src, "pub struct UpdateApiKeyGwRequest");
    assert!(
        update_dto.contains("deserialize_with = \"double_option\""),
        "UpdateApiKeyGwRequest 的可空列必须挂 double_option 适配器，实际块:\n{update_dto}"
    );
    let update_handler = extract_block(&src, "pub async fn update_api_key");
    assert!(
        !update_handler.contains("parse_from_rfc3339(s)") && !update_handler.contains(".ok()"),
        "update 链路禁止再把 expires_at 解析失败吞成 None，实际块:\n{update_handler}"
    );
    for anchor in [
        "pub async fn create_api_key",
        "pub async fn update_api_key",
        "pub async fn regenerate_api_key",
    ] {
        let block = extract_block(&src, anchor);
        assert!(
            block.contains("get_api_key_with_creator"),
            "{anchor} 必须写后经 LEFT JOIN 富化链路回读出参，实际块:\n{block}"
        );
    }
}

#[test]
fn source_scan_service_join_chain_and_tri_state() {
    let src = include_str!("../src/services/api_key_service.rs").replace('\r', "");
    for marker in [
        "column_as(user::Column::Username, \"created_by_name\")",
        "JoinType::LeftJoin",
        "into_model::<ApiKeyWithCreator>",
    ] {
        assert!(
            src.contains(marker),
            "service 读侧富化链路缺锚点 {marker}（created_by_name 只允许来自 LEFT JOIN）"
        );
    }
    assert!(
        !src.contains("into_tuple"),
        "本场景出全列视图，into_tuple 旁路不得出现"
    );
    assert!(
        !src.contains("created_by_name: Some(") && !src.contains("created_by_name: \"\""),
        "service 禁止手工构造 created_by_name（拼装名/空串蒙混）"
    );
    let payload = extract_block(&src, "pub struct UpdateApiKeyPayload");
    assert!(
        payload.contains("pub description: Option<Option<String>>"),
        "payload.description 必须是三态 Option<Option<String>>，实际块:\n{payload}"
    );
    assert!(
        payload.contains("pub expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>"),
        "payload.expires_at 必须保持三态，实际块:\n{payload}"
    );
    let create_fn = extract_block(&src, "pub async fn create_api_key");
    assert!(
        create_fn.contains("description: Set(description.map(|s| s.to_string()))"),
        "create 链路必须真实落库 description，实际块:\n{create_fn}"
    );
    assert!(
        !create_fn.contains("expires_days"),
        "create 链路不得再用 now+days 反算用户选的精确到期时间，实际块:\n{create_fn}"
    );
}

#[test]
fn source_scan_frontend_declares_nullable_and_no_bypass_comments() {
    let api = include_str!("../../frontend/src/api/api-gateway.ts").replace('\r', "");
    let iface = extract_block(&api, "export interface ApiKey {");
    for required in [
        "description: string | null;",
        "expires_at: string | null;",
        "created_by_name: string | null;",
        "last_used_at: string | null;",
    ] {
        assert!(
            iface.contains(required),
            "ApiKey 必须声明可空列真值形态 {required}，实际块:\n{iface}"
        );
    }
    let create_dto = extract_block(&api, "export interface CreateApiKeyRequest {");
    assert!(
        create_dto.contains("description?"),
        "创建 DTO 必须真实带 description，实际块:\n{create_dto}"
    );
    // 用注释记录缺陷（"传了会被静默丢弃""后端按解析失败处理会清掉有效期"）必须消失
    for banned in ["静默丢弃", "会清掉有效期", "后端缺口已登记串行清单"] {
        assert!(
            !api.contains(banned),
            "api-gateway.ts 不得保留绕过说明式注释「{banned}」"
        );
    }

    let composable = include_str!("../../frontend/src/views/api-gateway/composables/useApiKey.ts")
        .replace('\r', "");
    assert!(
        composable.contains("description: toWireDescription(keyForm.description)"),
        "编辑提交必须把清空后的描述显式转成 null 送出（不得省略键或塞空串冒充）"
    );
    assert!(
        composable.contains("expires_at: toWireExpiresAt(keyForm.expires_at)"),
        "编辑提交必须把清空后的有效期显式转成 null 送出（三态中的清空态）"
    );
    assert!(
        !composable.contains("清空白名单语义"),
        "useApiKey.ts 不得保留绕过说明式注释"
    );

    let key_tab =
        include_str!("../../frontend/src/views/api-gateway/tabs/ApiKeyTab.vue").replace('\r', "");
    assert!(
        key_tab.contains("t('apiGateway.keyTab.neverExpires')"),
        "列表须对 expires_at === null 显示「永不过期」i18n 文案"
    );
    assert!(
        key_tab.contains("row.created_by_name"),
        "列表须渲染后端富化的 created_by_name"
    );
    assert!(
        key_tab.contains("'edit-key': [row: ApiKey]"),
        "密钥列表必须有编辑入口，否则回显契约在 UI 上走不到"
    );

    let zh = extract_block(
        &include_str!("../../frontend/src/locales/zh-CN.ts").replace('\r', ""),
        "    keyTab: {",
    );
    let en = extract_block(
        &include_str!("../../frontend/src/locales/en-US.ts").replace('\r', ""),
        "    keyTab: {",
    );
    for key in [
        "neverExpires: '永不过期'",
        "columnCreator: '创建人'",
        "columnDescription: '描述'",
        "edit: '编辑'",
    ] {
        assert!(zh.contains(key), "zh-CN apiGateway.keyTab 缺新键文案 {key}");
    }
    for key in [
        "neverExpires: 'Never expires'",
        "columnCreator: 'Created by'",
        "columnDescription: 'Description'",
        "edit: 'Edit'",
    ] {
        assert!(en.contains(key), "en-US apiGateway.keyTab 缺新键文案 {key}");
    }
}
