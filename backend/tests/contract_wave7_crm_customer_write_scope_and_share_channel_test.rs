//! 契约波次 7 · 看板 #223 / #225：
//! - #223 增强客户域写路径（PUT/DELETE `/crm/customers/enhanced/:id`）行级数据权限门——
//!   修复前该两端点**完全不注入 data_scope ctx**，任何过 RBAC 键的角色可按 id 改/删他人行；
//!   收口为与标准入口完全同门：构造 `auth.to_data_scope_context()`，写前走既有
//!   `get_customer(id, Some(&ctx))`（owner=created_by、dept=department_id）/
//!   `get_lead(id, Some(&ctx))`（owner=owner_id）预检，判定源
//!   `utils/data_scope.rs::check_resource_owner` 零新造。任一行不可见即整笔 403
//!   （固定脱敏文案 + FORBIDDEN 码，真实原因只进服务端日志），且**零写入**（先判后写，
//!   不落"部分成功"）。admin 的 `DataScope::All` 既有通道未收紧——是否豁免另线裁定，
//!   本锁只固定"与其它入口同门"这一事实。
//! - #225 `POST /customer-shares` 裸 500（CI run #4669 e2e
//!   `frontend/e2e/crm/05-assign-share-merge.spec.ts:341`，backend.log 真因：
//!   `error occurred while decoding column "id": mismatched types; Rust type
//!   core::option::Option<i32> (as SQL type INT4) is not compatible with SQL type INT8`，
//!   trace_id=7ba6f4e5fa674e788464cdc7750e696e）。根因 = 模型 PK `i32` 与 m0082 DDL
//!   `"id" BIGSERIAL`（PG 回读 INT8）宽度不一致，insert 回读解码必炸并经
//!   `From<DbErr>` 拍平成 500 `DATABASE_ERROR`。修后错误通道：可读业务拒绝走
//!   `BUSINESS_ERROR`(400)/`VALIDATION_ERROR`(400)；唯一约束并发竞态（
//!   `uk_cs_customer_to_user_active`，SQLSTATE 23505）按同语义归类 `BUSINESS_ERROR`，
//!   不再可能落 `DATABASE_ERROR`；非约束类 DbErr 原样上报（不吞不改道）。
//!   本文件同时是源码扫描棘轮：PK 宽度对齐与"共享写路径 insert 不得裸重包"锁死。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL
//! （customer_shares 的 BIGSERIAL 主键、唯一约束 uk_cs_customer_to_user_active、
//! customers/crm_lead 的触发器与 FK 全部取真表）。users/customers/crm_lead
//! 按裁定 R1 自种子；roles 为迁移种子参照表不再插。跨 owner 写守卫
//! （crm_write_guard / check_resource_write_owner）的断言逐条保持原文，只换连接。

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
use bingxi_backend::handlers::crm_customer_handler::{delete_customer, update_customer};
use bingxi_backend::handlers::customer_team_share_handler::share_customer;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::utils::messages::err_msg;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER: i32 = 50;
const OTHER: i32 = 60;
const ADMIN: i32 = 70;
const SHARED_TO: i32 = 51;

// ---------------------------------------------------------------------------
// 夹具（真 PostgreSQL 真发 HTTP；表结构唯一来源 = backend/migration。
// customers 种子显式落 created_by=OWNER——行级归属判定源字段）
// ---------------------------------------------------------------------------

fn make_auth(user_id: i32, username: &str, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
        role_id,
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
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

/// 回读辅助：v1 对齐真表 INT4 列（customers.id/crm_lead.id/shared_to_user_id），
/// v2 对齐文本列；断言只消费行数，列宽以真表为准不做隐式放宽。
async fn query_names(db: &sea_orm::DatabaseConnection, sql: &str) -> Vec<Value> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("回读失败: {e}\nSQL: {sql}"));
    rows.iter()
        .map(|r| {
            json!({
                "v1": r.try_get::<Option<i32>>("", "v1").ok().flatten(),
                "v2": r.try_get::<Option<String>>("", "v2").ok().flatten(),
            })
        })
        .collect()
}

async fn base_state() -> AppState {
    let db = test_common::setup_test_db().await;
    // roles 不插：迁移种子参照表（id=1 code='admin'、id=2 为非 admin 角色）。
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (51,'shared_to_wangwu','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_user','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // customers 行：归属判定源字段 created_by=OWNER（get_customer 的 owner 列），
    // owner_id=OWNER（共享/团队入口 validate_share_permission 的 owner 列，两列语义各表各的既有口径）；
    // department_id 由 trg_customers_dept 按 owner 的 users.department_id 回填。
    exec(
        &db,
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             contact_phone,contact_email,address,credit_limit,payment_terms,status,
             customer_type,owner_id,created_by,created_at,updated_at) VALUES
             (1,'CUS-0001','甲客户','张三','13812348888','alice@example.com',
              '河北省邢台市某某路 1 号',0,30,'active','retail',50,50,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // crm_lead 行：增强页 DELETE 的落点（owner_id=OWNER；department_id 由
    // trg_crm_lead_dept 回填）
    exec(
        &db,
        "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,owner_id,owner_name,
             created_at,updated_at) VALUES
             (1,'LD001','website','new','甲公司','张三','13812348888',
              50,'销售甲','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    let mut state = AppState::default();
    state.db = Arc::new(db);
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    state
}

/// 按真实挂载路径（routes/crm.rs::crm_customers 与 routes/customer_team_share.rs）建路由
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/erp/crm/customers/enhanced/{id}",
            put(update_customer).delete(delete_customer),
        )
        .route("/erp/customer-shares", post(share_customer))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
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
            panic!(
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

/// 失败信封唯一形状锁：AppError{code,message,trace_id,timestamp}，无第五键
fn assert_error_envelope_shape(v: &Value) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("失败信封必须是 JSON object: {v}"));
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["code", "message", "timestamp", "trace_id"],
        "失败信封键集合必须恰为 code/message/trace_id/timestamp，实际: {keys:?}"
    );
}

// ---------------------------------------------------------------------------
// #223-A · PUT /crm/customers/enhanced/:id 行级门
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enhanced_update_self_scope_other_owner_row_is_403_with_zero_write() {
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(OTHER, "sales_b", Some(2), "self"));

    let (status, v) = send(
        &app,
        Method::PUT,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "越权改名"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "越权改他人客户行必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "机器码必须是 FORBIDDEN: {v}");
    assert_error_envelope_shape(&v);
    // 权限拒绝文案永久脱敏：外显 message 恒等于固定脱敏常量（真实原因含 ID/判定依据，
    // 只进服务端日志；本锁只断 status+code+固定常量，不断真实文案）
    assert_eq!(
        v["message"],
        err_msg::PERMISSION_PUBLIC,
        "403 外显文案必须是固定脱敏常量，不得外显真实拒绝原因"
    );
    // 整笔拒绝 = 零写入（先判后写，不存在"跳过不可见行继续写"的降级）
    let rows = query_names(
        &db,
        "SELECT customer_name AS v2 FROM customers WHERE id=1 AND customer_name='甲客户'",
    )
    .await;
    assert_eq!(rows.len(), 1, "403 后原行必须未被改动: {rows:?}");
}

#[tokio::test]
async fn enhanced_update_self_scope_owner_row_passes_same_gate() {
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "self"));
    let (status, v) = send(
        &app,
        Method::PUT,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "归属人改名"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "归属人过同门应 200: {v}");
    let rows = query_names(
        &db,
        "SELECT customer_name AS v2 FROM customers WHERE id=1 AND customer_name='归属人改名'",
    )
    .await;
    assert_eq!(rows.len(), 1, "更新须真实生效: {rows:?}");
}

#[tokio::test]
async fn enhanced_update_admin_all_scope_channel_not_tightened() {
    // admin 的 DataScope::All 既有通道原样通过（check_resource_owner All=true）——
    // 本项只把"完全没有门"变成"与其它入口同门"，是否豁免 All 另线裁定，此处锁"未收紧"
    let state = base_state().await;
    let app = build_app(state, make_auth(ADMIN, "admin_user", Some(1), "all"));
    let (status, v) = send(
        &app,
        Method::PUT,
        "/erp/crm/customers/enhanced/1",
        json!({"customer_name": "admin改名"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "All 通道不得被本门收紧: {v}");
}

// ---------------------------------------------------------------------------
// #223-B · DELETE /crm/customers/enhanced/:id 行级门（落点 crm_lead 行）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn enhanced_delete_self_scope_other_owner_lead_is_403_row_survives() {
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(OTHER, "sales_b", Some(2), "self"));
    let (status, v) = send(
        &app,
        Method::DELETE,
        "/erp/crm/customers/enhanced/1",
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "越权删他人线索行必须 403: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN", "机器码必须是 FORBIDDEN: {v}");
    assert_error_envelope_shape(&v);
    assert_eq!(
        v["message"],
        err_msg::PERMISSION_PUBLIC,
        "删除 403 外显文案恒为固定脱敏常量"
    );
    let rows = query_names(&db, "SELECT id AS v1 FROM crm_lead WHERE id=1").await;
    assert_eq!(
        rows.len(),
        1,
        "403 后行必须仍在（整笔拒绝，零删除）: {rows:?}"
    );
}

#[tokio::test]
async fn enhanced_delete_owner_self_scope_and_admin_all_scope_both_pass() {
    // 归属人 self 过门 200；admin All 通道原样通过（各自新建内存库，互不影响）
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "self"));
    let (status, v) = send(
        &app,
        Method::DELETE,
        "/erp/crm/customers/enhanced/1",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "归属人删除应成功: {v}");
    let rows = query_names(&db, "SELECT id AS v1 FROM crm_lead WHERE id=1").await;
    assert!(rows.is_empty(), "删除须真实生效: {rows:?}");

    let state2 = base_state().await;
    let app2 = build_app(state2, make_auth(ADMIN, "admin_user", Some(1), "all"));
    let (status2, v2) = send(
        &app2,
        Method::DELETE,
        "/erp/crm/customers/enhanced/1",
        json!({}),
    )
    .await;
    assert_eq!(status2, StatusCode::OK, "admin All 通道不得被收紧: {v2}");
}

// ---------------------------------------------------------------------------
// #225 · POST /customer-shares 错误通道锁（真 PostgreSQL 发 HTTP）
// ---------------------------------------------------------------------------

fn share_body(shared_to: i32) -> Value {
    json!({
        "customer_id": 1,
        "shared_to_user_id": shared_to,
        "permission": "view",
        "duration_days": 30,
        "share_reason": "E2E 协作"
    })
}

#[tokio::test]
async fn share_customer_success_decodes_bigint_pk() {
    // 真因回归锁：修复前 insert 回读 id 列（PG INT8 / sqlite INTEGER）经 i32 模型
    // 解码必炸 → 500 DATABASE_ERROR；修复后（模型 PK i64）同一路径真实成功并回读
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "all"));
    let (status, v) = send(
        &app,
        Method::POST,
        "/erp/customer-shares",
        share_body(SHARED_TO),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "共享写入在 PK 宽度对齐后必须成功（此前即 #225 的 500）: {v}"
    );
    assert_ne!(
        v["code"],
        json!("DATABASE_ERROR"),
        "成功响应不得带错误码: {v}"
    );
    assert!(v["data"]["id"].is_number(), "落库回读主键必须成功解码: {v}");
    let rows = query_names(
        &db,
        "SELECT shared_to_user_id AS v1 FROM customer_shares WHERE customer_id=1 AND status='active'",
    )
    .await;
    assert_eq!(rows.len(), 1, "共享行必须真实落库: {rows:?}");
}

#[tokio::test]
async fn duplicate_active_share_is_business_error_not_database_error() {
    let state = base_state().await;
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "all"));
    let (s1, v1) = send(
        &app,
        Method::POST,
        "/erp/customer-shares",
        share_body(SHARED_TO),
    )
    .await;
    assert_eq!(s1, StatusCode::OK, "首次共享应成功: {v1}");
    let (status, v) = send(
        &app,
        Method::POST,
        "/erp/customer-shares",
        share_body(SHARED_TO),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "重复共享必须是 400 可读业务拒绝: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR", "错误码通道: {v}");
    assert_ne!(
        v["code"],
        json!("DATABASE_ERROR"),
        "业务前置冲突绝不得经 From<DbErr> 拍平成 500"
    );
    assert_eq!(
        v["message"],
        err_msg::BUSINESS_PUBLIC,
        "BUSINESS_ERROR 外显恒为固定脱敏常量（真实文案含 ID 只进日志）"
    );
    assert_error_envelope_shape(&v);
}

#[tokio::test]
async fn share_to_missing_user_is_validation_error_not_database_error() {
    let state = base_state().await;
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "all"));
    let (status, v) = send(
        &app,
        Method::POST,
        "/erp/customer-shares",
        share_body(99999),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "不存在用户应 400: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码通道: {v}");
    assert_ne!(v["code"], json!("DATABASE_ERROR"));
    assert_eq!(
        v["message"],
        err_msg::VALIDATION_PUBLIC,
        "VALIDATION_ERROR 外显恒为固定脱敏常量（真实文案含用户 ID 只进日志）"
    );
    assert_error_envelope_shape(&v);
}

#[tokio::test]
async fn share_to_inactive_user_is_business_error_not_database_error() {
    let state = base_state().await;
    let db = state.db.clone();
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at)
         VALUES (98,'frozen_user','x',FALSE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let app = build_app(state, make_auth(OWNER, "sales_a", Some(2), "all"));
    let (status, v) = send(&app, Method::POST, "/erp/customer-shares", share_body(98)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "停用用户应 400: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR", "前置状态门归业务族: {v}");
    assert_error_envelope_shape(&v);
}

#[tokio::test]
async fn share_operator_not_owner_is_forbidden_masked() {
    // 操作人既非 owner/primary/full 共享 → permission_denied：status+code 两键断言，
    // 文案永久脱敏（真实原因含 ID，只进服务端日志）
    let state = base_state().await;
    let app = build_app(state, make_auth(OTHER, "sales_b", Some(2), "all"));
    let (status, v) = send(
        &app,
        Method::POST,
        "/erp/customer-shares",
        share_body(SHARED_TO),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "无归属关系共享应 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], err_msg::PERMISSION_PUBLIC);
    assert_error_envelope_shape(&v);
}

// ---------------------------------------------------------------------------
// #225 棘轮 · 源码扫描（防该出口再退回裸重包 / 防模型宽度与 DDL 再度漂移）
// ---------------------------------------------------------------------------

fn read_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
}

/// DDL 权威源（`customer_shares.id` / `customer_team_members.id` 均为 BIGSERIAL）
/// 与模型 PK 宽度（i64）必须**成对**锁死：任一侧单独漂移都会在生产（PG）把
/// 写路径拍成 500 DATABASE_ERROR（CI #4669 e2e 05-assign-share-merge:341 实证）。
#[test]
fn source_scan_share_tables_pk_width_locked_to_bigint_ddl() {
    let ddl = read_src("migration/src/domain/v15/mod.rs");
    for table in ["customer_shares", "customer_team_members"] {
        let anchor = format!(r#"CREATE TABLE IF NOT EXISTS "{table}""#);
        let start = ddl
            .find(&anchor)
            .unwrap_or_else(|| panic!("迁移 DDL 必须仍有 {table} 建表段"));
        let seg = &ddl[start..start + 400];
        assert!(
            seg.contains(r#""id" BIGSERIAL PRIMARY KEY"#),
            "{table}.id 的 DDL 权威宽度是 BIGSERIAL；如需变更必须与模型成对改并过决策，不得单边漂移"
        );
    }
    for model in [
        "src/models/customer_share.rs",
        "src/models/customer_team_member.rs",
    ] {
        let src = read_src(model);
        assert!(
            src.contains("pub id: i64"),
            "{model} 主键必须与 BIGSERIAL DDL 同宽（i64），退回 i32 会在 PG 回读解码时 500"
        );
        assert!(
            !src.contains("pub id: i32"),
            "{model} 不得残留 i32 主键声明"
        );
    }
}

/// `share_customer` 写路径棘轮：insert 落库调用点必须带唯一约束冲突分类
/// （`SqlErr::UniqueConstraintViolation` → `AppError::business`，400 BUSINESS_ERROR），
/// 不得退回 `From<DbErr>` 裸重包（`.insert(&*self.db).await?;`）——那正是把
/// 23505/解码失败拍平成 500 DATABASE_ERROR 的通道。非约束类 DbErr 仍原样上报，
/// 本棘轮不鼓励也不放过"catch 后返回默认值"式绕过。
#[test]
fn source_scan_share_customer_insert_no_bare_dberr_rewrap() {
    let src = read_src("src/services/crm/customer_team_share_service.rs");
    let start = src
        .find("pub async fn share_customer")
        .expect("share_customer 必须仍存在");
    let rest = &src[start..];
    let end = rest[1..]
        .find("pub async fn ")
        .map(|i| i + 1)
        .expect("share_customer 后必须还有同 impl 其它方法");
    let body = &rest[..end];

    let ins = body
        .find(".insert(&*self.db)")
        .expect("share_customer 落库调用点必须存在");
    let tail = &body[ins..];
    assert!(
        tail.contains(".map_err("),
        "share_customer 的 insert 调用点必须显式分类约束冲突，不得裸 `?` 经 From<DbErr> 重包"
    );
    assert!(
        !tail.contains(".await?;"),
        "share_customer 的 insert 调用点不得残留 `.await?;` 裸重包形态"
    );
    assert!(
        body.contains("SqlErr::UniqueConstraintViolation"),
        "唯一约束 23505 必须归类为 BUSINESS_ERROR(400) 而非 DATABASE_ERROR(500)"
    );
    assert!(
        body.contains("AppError::business("),
        "约束冲突走脱敏业务族构造器（文案含 ID，不得外显）"
    );
    assert!(
        !body.contains("AppError::database(") && !body.contains("DatabaseError("),
        "该出口不得自造/改道 DATABASE_ERROR——500 只留给真数据库故障"
    );
}
