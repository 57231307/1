//! customers.customer_type 渠道词表——**真库活体契约锁**（CHECK 与应用词表同源、端点/库层
//! 双层拒写、缺省真落 other）
//!
//! ## 被测对象（三处真源，任一漂移本文件即红）
//! 1. 迁移原文：`migration/src/domain/business/m0078_finalize_customer_type_domain.rs`
//!    的 `const ALLOWED_SQL`（CHECK 取值唯一种子，经 `allowed = ALLOWED_SQL` 填进
//!    `CHECK ("customer_type" IN ({allowed}))`）与 `SET DEFAULT 'other'` 行；
//!    前置只读点名迁移 `m0077_report_customer_type_dirty_values.rs` 携带同一份
//!    `ALLOWED_SQL`。两支文件均注册在 business up 链（`business/mod.rs`）。
//! 2. 应用权威词表：`src/constants/customer_type.rs` 的 `ALLOWED`/`OTHER`
//!    （全仓唯一出现点；`validate(None) => OTHER` 是标准创建入口的缺省补值唯一路径）。
//! 3. 活库目录：`pg_constraint`（contype='c'、convalidated、pg_get_constraintdef）与
//!    `information_schema.columns`（is_nullable、column_default）——收口后的真实生效形态。
//!
//! ## 为什么必须真库（sqlite/纸面比对测不到的形态）
//! - `is_nullable`、`column_default`、`convalidated`、`pg_get_constraintdef` 是 PostgreSQL
//!   目录里的状态，sqlite 夹具既没有这些目录视图也没有 NOT VALID 语义；
//! - "CHECK 是否真的在拒"只能靠真库写入取证（旁路裸 SQL 直插，看驱动报回的 SQLSTATE
//!   与约束名）；把迁移文本与应用常量做字符串比对只能证纸面同源，证不了库里那条约束
//!   存在、已校验、且值域逐元素一致——本文件把文本解析结果与活库目录读数**闭环对齐**。
//! - 夹具 `setup_test_db()` 连已迁移的真实 PostgreSQL（缺 `TEST_DATABASE_URL` 直接 panic，
//!   不静默降级），nextest 下所有真库用例串行执行。
//!
//! ## 断言分界："端点层拒"与"库层 CHECK 兜底"各证一次，互不掩盖
//! - 端点层：应用校验先于任何触库（validator 通道 + 模块通道，见
//!   `src/handlers/customer_handler.rs`），拒绝必须是 400 + VALIDATION_ERROR 且**零落库**；
//!   若摘掉入口校验，越界值将带着 DATABASE_ERROR 族别触库冒 500——本用例的 400+
//!   VALIDATION_ERROR+零落库三条同时红，掩盖不了。
//! - 库层：旁路裸 SQL 绕开全部应用校验直插同一批越界值，必须由
//!   `chk_customers_customer_type` 报 23514、约束名逐字归因、零落库；若库里约束被撤
//!   （down 半态/迁移未注册），旁路写入成功即红。
//!   两层缺一即回归，所以两条路径分开钉，拒绝只断 HTTP 码 + 机器 code + 约束名/SQLSTATE，
//!   **不断任何用户可见文案**（业务拒绝文案永久脱敏，文案不是契约面）。
//!
//! ## 期望值来源（防恒真）
//! 合法值集合与缺省 token 一律取**运行时** `constants::customer_type::ALLOWED/OTHER` 与
//! **迁移原文解析**、**活库目录读数**三方对撞，测试文件内不手抄第二套取值清单；
//! 越界探针是历史脏值形态与大小写漂移样本（`VIP`/`Other`/分层词/空白混入），
//! 每个必失败用例都配同模板合法值对照，防夹具本身写坏导致拒绝集体假绿。
//!
//! ## 写入通道选型
//! 合法值"真能写"选**端点通道**（POST /customers，真实业务写全链：DTO validate →
//! service → ORM → DB），因为要钉的是"业务写入口接受五值且原文落库不被改写"；
//! ActiveModel 直插只覆盖 DB+ORM 两点、证不了 validator 通道。回读一律裸 SQL 直查列值，
//! 不依赖响应 JSON——响应经过两层字段权限掩码（`apply_customer_field_config_mask` +
//! `apply_customer_field_permission`），键可能被动过，落库真值才是本锁的断言对象。
//!
//! ## id 带与种子
//! `customers`/`users` 属逐用例 TRUNCATE 的业务表（不在封存参照表清单），本文件独占
//! 990_6xx 段（990_1xx/990_3xx、991_0xx、992_0xx 已被其它锁占用，不碰）；
//! `roles` id=1(code='admin')、`departments` id=1 为迁移种子参照行，只引用不自建。
//! 端点创建行走自增序列、按唯一 customer_code 回读，不占显式 id。

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
use bingxi_backend::constants::customer_type::{ALLOWED, OTHER, RETAIL};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::customer_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, RuntimeErr, Statement};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// m0078 添加的 CHECK 约束名（`constants/customer_type.rs` 头注释成文约定同名）
const CHK_NAME: &str = "chk_customers_customer_type";
const M0077_REL: &str = "migration/src/domain/business/m0077_report_customer_type_dirty_values.rs";
const M0078_REL: &str = "migration/src/domain/business/m0078_finalize_customer_type_domain.rs";
const HANDLER_REL: &str = "src/handlers/customer_handler.rs";

/// 本文件专属 id 带（990_6xx，见文件头「id 带与种子」）
const OPERATOR_ID: i32 = 990_601;
const PROBE_BASE: i32 = 990_620; // 旁路越界探针（每 token 一行，全部应当插不进去）
const PROBE_LEGAL_ID: i32 = 990_640; // 旁路合法值对照（同模板，必须插得进）
const DEFAULT_PROBE_ID: i32 = 990_650; // 裸 SQL 省略该列的 DEFAULT 生效探针
const PUT_TARGET_ID: i32 = 990_660; // PUT 端点拒绝用例的目标行（应零变化）

/// PG 对 varchar 列默认值的两种等价文本渲染（`pg_get_expr` 是否补写类型标注由 PG 决定，
/// 语义同一个表达式；取值 token 恰 1 个——NULL 与其它 token 一律拒绝，"默认值真的求值为
/// other"由用例 4 的行为回读证明，目录断言只负责"存在默认值且字面量形态正确"）
const DEFAULT_RENDERINGS: [&str; 2] = ["'other'", "'other'::character varying"];

/// 端点层越界探针：分层词大写/缺省桶大写漂移形态（本仓踩过的中英/大小写混维样本）、
/// 分层词、CLV segment 词、空串、前导空白混入（校验不 trim ⇒ 必须拒）。
const ENDPOINT_BANNED_TOKENS: &[&str] = &["VIP", "Other", "normal", "POTENTIAL", "", " retail"];

/// 库层旁路探针：在上面基础上追加小写分层词与尾随空白混入（CHECK 逐字符敏感）。
const BYPASS_BANNED_TOKENS: &[&str] = &[
    "VIP",
    "Other",
    "vip",
    "normal",
    "POTENTIAL",
    "",
    " retail",
    "manufacturer ",
];

// ---------------------------------------------------------------------------
// 助手（裸 SQL 一律走 execute_raw / query_one_raw，不引入第二套连接方式）
// ---------------------------------------------------------------------------

fn read_repo_file(rel: &str) -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败: {e}"))
}

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<sea_orm::Value>::new(),
    )
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

/// 目录/计数类单列文本读取；查询 0 行返回 `None`（调用方据此断"对象不存在"负形态）。
/// 列值为真 NULL 时显式抛错而不是塌成空串——分不清"对象不存在"与"存在但值为 NULL"。
/// 需要判定"缺省不存在"形态的调用点自行 `COALESCE(..., '<NULL>')` 取哨兵值。
async fn one_text(db: &DatabaseConnection, sql: &str, col: &str) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("单值查询失败: {e}\nSQL: {sql}"));
    row.map(|r| {
        r.try_get::<Option<String>>("", col)
            .unwrap_or_else(|e| panic!("列 {col} 解码为文本失败: {e}\nSQL: {sql}"))
            .unwrap_or_else(|| {
                panic!("列 {col} 是真 NULL（本助手不塌缩，需缺省判定请自行 COALESCE）\nSQL: {sql}")
            })
    })
}

async fn one_i64(db: &DatabaseConnection, sql: &str, col: &str) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("计数查询失败: {e}\nSQL: {sql}"));
    let r = row.unwrap_or_else(|| panic!("计数查询没有返回行: SQL: {sql}"));
    r.try_get::<i64>("", col)
        .unwrap_or_else(|e| panic!("列 {col} 解码为 i64 失败: {e}\nSQL: {sql}"))
}

/// 驱动上报的 SQLSTATE：从 `DbErr::Exec/Query(RuntimeErr::SqlxError)` 取驱动字段，
/// 不做 `to_string().contains(...)` 的含混匹配（任何 FK/类型/长度错误都会让
/// 只判 is_err 的断言"看起来通过"）。非驱动错误直接 panic。
fn sqlstate_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::code(de).map(|c| c.into_owned())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 驱动上报的约束名（PG 专用）：把 23514 归因到**本列这一条 CHECK**，
/// 而不是表上任意一条 CHECK 或任意一种失败。
fn constraint_of(err: &DbErr) -> Option<String> {
    match err {
        DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e
            .as_database_error()
            .and_then(|de| sqlx::error::DatabaseError::constraint(de).map(|s| s.to_string())),
        other => panic!("期望的是执行期数据库错误，实际是非驱动错误: {other}"),
    }
}

/// 提取文本中全部单引号 token（`'retail'::character varying` ⇒ retail）。
/// 引号不闭合直接 panic——解析面失守必须炸，不许静默产出残缺集合。
fn quoted_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut cursor = text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor
            .find('\'')
            .unwrap_or_else(|| panic!("文本里的单引号未闭合: {text}"));
        tokens.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    tokens
}

/// 从迁移原文解析 `const ALLOWED_SQL` 字面量段的取值集合（锚点缺失即 panic：
/// 迁移改写导致形态漂移同样判红，禁止静默返回空集让"相等"退化成"两边都空"）。
fn parse_allowed_sql_tokens(src: &str, file: &str) -> BTreeSet<String> {
    let anchor = "const ALLOWED_SQL: &str = \"";
    let start = src
        .find(anchor)
        .unwrap_or_else(|| panic!("{file} 缺少锚点 {anchor:?}（CHECK 取值种子形态已改动）"));
    let rest = &src[start + anchor.len()..];
    let end = rest
        .find("\";")
        .unwrap_or_else(|| panic!("{file} 的 ALLOWED_SQL 字面量未闭合"));
    let tokens = quoted_tokens(&rest[..end]);
    assert!(
        !tokens.is_empty(),
        "{file} 解析出空取值集合——解析失守，拒绝继续比对"
    );
    tokens
}

/// 从 m0078 原文解析 `SET DEFAULT` 行的取值 token（钉缺省字面量，不是只 contains）。
fn parse_set_default_token(src: &str) -> String {
    let anchor = "ALTER COLUMN \"customer_type\" SET DEFAULT ";
    let start = src
        .find(anchor)
        .unwrap_or_else(|| panic!("m0078 缺少锚点 {anchor:?}（列缺省形态已改动）"));
    let rest = &src[start + anchor.len()..];
    let open = rest
        .find('\'')
        .unwrap_or_else(|| panic!("SET DEFAULT 之后没有字面量: {rest}"));
    let after = &rest[open + 1..];
    let close = after
        .find('\'')
        .unwrap_or_else(|| panic!("SET DEFAULT 字面量引号未闭合: {after}"));
    after[..close].to_string()
}

/// users 自建（`customers.created_by` 的外键父行；users 不在迁移种子里）。
/// 列清单照已在 CI 跑通的既有形态；department_id=1 引用迁移种子部门行。
async fn seed_users(db: &DatabaseConnection) {
    exec(db, &format!("DELETE FROM users WHERE id={OPERATOR_ID}")).await;
    exec(
        db,
        &format!(
            "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at)
             VALUES ({OPERATOR_ID},'ct_w11_lock_op','test-only-not-a-real-hash',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
}

/// 带显式 `customer_type` 的旁路直插（返回 DbErr 供 SQLSTATE 级断言；绕开全部应用层
/// 校验，模拟"脚本/旁路写入"这一 CHECK 要拦的形态）。
async fn insert_customer_typed(
    db: &DatabaseConnection,
    id: i32,
    customer_type: &str,
) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("CT-LCK-{id}").into(),
        OPERATOR_ID.into(),
        OPERATOR_ID.into(),
        customer_type.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,customer_type,created_at,updated_at)
         VALUES ($1,$2,'渠道词表活体锁探针',0,30,'active',$3,$4,$5,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

/// **省略** `customer_type` 列的旁路直插——列 DEFAULT 唯一能生效的形态
/// （显式 NULL 走 NOT NULL，两条分支不同，各由各用例取证）。
async fn insert_customer_omitting_type(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
    let values: Vec<sea_orm::Value> = vec![
        id.into(),
        format!("CT-LCK-DEF-{id}").into(),
        OPERATOR_ID.into(),
        OPERATOR_ID.into(),
    ];
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,status,owner_id,created_by,created_at,updated_at)
         VALUES ($1,$2,'缺省活体探针',0,30,'active',$3,$4,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
        values,
    ))
    .await
    .map(|_| ())
}

async fn customer_type_of_id(db: &DatabaseConnection, id: i32) -> Option<String> {
    let row = db
        .query_one_raw(pg_stmt(&format!(
            "SELECT customer_type FROM customers WHERE id={id}"
        )))
        .await
        .unwrap_or_else(|e| panic!("回读 customers.customer_type(id={id}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("customers 里没有 id={id} 的行（探针行未落库？）"));
    r.try_get::<Option<String>>("", "customer_type")
        .unwrap_or_else(|e| panic!("id={id} 的 customer_type 解码失败: {e}"))
}

async fn customer_type_of_code(db: &DatabaseConnection, code: &str) -> Option<String> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT customer_type FROM customers WHERE customer_code=$1".to_string(),
            vec![code.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("按编码回读 customer_type({code}) 失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("customers 里没有 customer_code={code} 的行"));
    r.try_get::<Option<String>>("", "customer_type")
        .unwrap_or_else(|e| panic!("customer_code={code} 的 customer_type 解码失败: {e}"))
}

async fn count_by_code(db: &DatabaseConnection, code: &str) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM customers WHERE customer_code='{code}'"),
        "n",
    )
    .await
}

async fn count_by_id(db: &DatabaseConnection, id: i32) -> i64 {
    one_i64(
        db,
        &format!("SELECT COUNT(*) AS n FROM customers WHERE id={id}"),
        "n",
    )
    .await
}

// ---------------------------------------------------------------------------
// HTTP 形态夹具（路由形状与真实挂载一致：routes/crm.rs 的 /customers、/customers/{id}）
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: OPERATOR_ID,
        username: "ct_w11_lock_op".to_string(),
        role_id: Some(1),
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

/// 真库 AppState（照既有 CI 跑通形态：default + 覆盖 db 与 data_permission_service
/// 为真实连接；admin 判定 `is_admin_role` 会查 roles 种子行 id=1）
async fn app_with_db() -> (Arc<DatabaseConnection>, Router) {
    let db = Arc::new(setup_test_db().await);
    seed_users(&db).await;
    let mut state = AppState::default();
    state.db = db.clone();
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    let app = Router::new()
        .route("/customers", post(customer_handler::create_customer))
        .route("/customers/{id}", put(customer_handler::update_customer))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth));
    (db, app)
}

async fn send_json(app: &Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
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
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

// ---------------------------------------------------------------------------
// 1. 同源三向对撞：m0077/m0078 迁移原文解析集 == 运行时 ALLOWED == 活库 CHECK 取值集，
//    逐元素**集合相等**（不是包含）；并钉 CHECK/DEFAULT 的原文接线形态与目录生效形态。
//    改坏什么必红：
//    · m0078 ALLOWED_SQL 增/删/改任一 token ⇒ 与运行时 ALLOWED 集合相等断言红；
//    · constants ALLOWED 单边改动 ⇒ 同一断言红（含 m0077 与 m0078 两份原文互等，
//      只改一支也红）；
//    · m0078 不再经 `allowed = ALLOWED_SQL` 填 CHECK、或约束名/SET NOT NULL/SET DEFAULT
//      行被改写 ⇒ 锚点 contains/解析 panic 红；
//    · 活库 CHECK 值域与前两者任一不等（迁移未注册、down 半态、旁路又加第二条 CHECK、
//      NOT VALID）⇒ 目录断言红；
//    · SET DEFAULT 行 token 漂移（如改成 'retail'）⇒ 与 OTHER 相等断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn check_vocab_in_migrations_equals_app_allowed_equals_live_db_check() {
    let db = setup_test_db().await;
    let m0077_src = read_repo_file(M0077_REL);
    let m0078_src = read_repo_file(M0078_REL);

    let from_m0077 = parse_allowed_sql_tokens(&m0077_src, M0077_REL);
    let from_m0078 = parse_allowed_sql_tokens(&m0078_src, M0078_REL);
    let runtime: BTreeSet<String> = ALLOWED.iter().map(|t| t.to_string()).collect();

    assert_eq!(
        from_m0078, runtime,
        "m0078 ALLOWED_SQL 解析集与运行时 constants::customer_type::ALLOWED 不是同一集合\
         （集合相等判据，包含关系不算）：\n迁移原文={from_m0078:?}\n应用词表={runtime:?}"
    );
    assert_eq!(
        from_m0077, from_m0078,
        "m0077 点名口径与 m0078 收口口径分叉（两支迁移必须共用同一份五值清单）：\n\
         m0077={from_m0077:?}\nm0078={from_m0078:?}"
    );

    // 接线形态：CHECK 取值只能来自 ALLOWED_SQL 这一份种子，约束名与收口动作逐字钉住
    assert!(
        m0078_src.contains("ADD CONSTRAINT \"chk_customers_customer_type\""),
        "m0078 必须以带引号约束名 {CHK_NAME} 添加 CHECK（本锁与错误归因按该名），\
         实际原文未出现该字面量"
    );
    assert!(
        m0078_src.contains("CHECK (\"customer_type\" IN ({allowed}))"),
        "m0078 的 CHECK 必须是 `\"customer_type\" IN ({{allowed}})` 占位形态——\
         值域唯一由 ALLOWED_SQL 注入；改成内联第二套清单即漂移"
    );
    assert!(
        m0078_src.contains("allowed = ALLOWED_SQL,"),
        "m0078 的 format 实参必须把 {{allowed}} 绑定到 ALLOWED_SQL（同一份清单），\
         换绑其它来源=库里词表与 constants 脱钩"
    );
    assert!(
        m0078_src.contains("ALTER COLUMN \"customer_type\" SET NOT NULL"),
        "m0078 必须包含 SET NOT NULL 收口（NULL 不再是合法存储形态的定案）"
    );
    let parsed_default = parse_set_default_token(&m0078_src);
    assert_eq!(
        parsed_default, OTHER,
        "m0078 原文的列缺省 token 必须等于运行时 constants::customer_type::OTHER\
         （缺省桶字面量两处同源）：迁移原文='{parsed_default}' 应用缺省='{OTHER}'"
    );

    // —— 活库目录：约束真实存在、类型/校验位正确、值域与前两向逐元素相等 ——
    let contype = one_text(
        &db,
        &format!(
            "SELECT c.contype::text FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "contype",
    )
    .await;
    assert_eq!(
        contype.as_deref(),
        Some("c"),
        "pg_constraint 里必须存在名为 {CHK_NAME} 且 contype='c'（CHECK）的约束；\n\
         None=约束不存在（m0078 未注册/被 down 撤掉），其它值=约束类型漂移；实际: {contype:?}"
    );

    let validated = one_text(
        &db,
        &format!(
            "SELECT c.convalidated::text FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "convalidated",
    )
    .await;
    assert_eq!(
        validated.as_deref(),
        Some("true"),
        "convalidated 必须为 true：NOT VALID 的 CHECK 放行存量脏行、只对后续写入生效，\
         不算收口；实际: {validated:?}"
    );

    let other_type_checks = one_i64(
        &db,
        &format!(
            "SELECT COUNT(*) AS n FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.contype='c' \
                AND c.conname <> '{CHK_NAME}' \
                AND pg_get_constraintdef(c.oid) LIKE '%customer_type%'"
        ),
        "n",
    )
    .await;
    assert_eq!(
        other_type_checks, 0,
        "customers 上除 {CHK_NAME} 外还另有 {other_type_checks} 条涉及 customer_type 的 \
         CHECK——同列多 CHECK 即第二套词表，本锁的\"集合相等\"判据失去唯一真源，必须先归一"
    );

    let def = one_text(
        &db,
        &format!(
            "SELECT pg_get_constraintdef(c.oid) AS def FROM pg_constraint c \
              WHERE c.conrelid='customers'::regclass AND c.conname='{CHK_NAME}'"
        ),
        "def",
    )
    .await
    .unwrap_or_else(|| panic!("读不到 {CHK_NAME} 的定义（与上一句 contype 查询自相矛盾）"));
    assert!(
        def.contains("customer_type"),
        "{CHK_NAME} 必须作用在 customer_type 列上，实际定义: {def}"
    );
    let live = quoted_tokens(&def);
    assert_eq!(
        live, runtime,
        "活库 CHECK 取值集 != 应用词表运行时集合（逐元素相等，不是包含；\n\
         m0078 原文解析集已与 runtime 逐元素相等钉过，故活库==原文==应用三向闭环由此+上条成立）：\n\
         活库={live:?}\n应用={runtime:?}\n\
         活库多于应用=旁路可写入业务永不产生的脏 token；活库少于应用=某个合法渠道值\
         会在写入时撞 23514 冒裸 500——两侧任一侧单独增删值都是契约漂移"
    );

    // —— 分层词永不并入值域（混维回潮在此列上就是脏值，见 m0077 点名口径）。
    //    前两条集合相等断言成立时这几条看似恒真——它们防的是**两侧一起改错**
    //    （若有人把 vip/normal 同时塞进 ALLOWED 与 ALLOWED_SQL，相等断言仍绿，
    //    本条与 m0077 点名语义、渠道列定案冲突，在这里红）。——
    for tier in ["vip", "normal"] {
        assert!(
            !live.contains(tier),
            "分层词 '{tier}' 混进了活库 CHECK 值域——本列语义是渠道，分层词永不并入"
        );
    }

    // —— 目录形态：NOT NULL 与默认值字面量（行为级证明见用例 4） ——
    let nullable = one_text(
        &db,
        "SELECT is_nullable FROM information_schema.columns \
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "is_nullable",
    )
    .await;
    assert_eq!(
        nullable.as_deref(),
        Some("NO"),
        "customers.customer_type 的 is_nullable 必须是 'NO'；实际: {nullable:?}"
    );
    let default_raw = one_text(
        &db,
        "SELECT COALESCE(column_default, '<NULL>') AS column_default \
          FROM information_schema.columns \
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "column_default",
    )
    .await
    .unwrap_or_else(|| panic!("information_schema 里查不到 customers.customer_type（建表漂移）"));
    assert!(
        DEFAULT_RENDERINGS.contains(&default_raw.as_str()),
        "活库 column_default 必须是 other 的字面量渲染（带/不带 varchar 类型标注两种等价 \
         PG 渲染，取值 token 恰 1 个，理由见文件头，不是放宽）：\n\
         期望恰为 {DEFAULT_RENDERINGS:?} 之一，实际: {default_raw:?}\n\
         '<NULL>'=该列根本没有 DEFAULT（m0078 的 SET DEFAULT 未生效，缺省只剩应用层一条腿）"
    );
}

// ---------------------------------------------------------------------------
// 2. 合法值真能写：运行时 ALLOWED 每个取值经真实业务写入口（POST /customers 端点全链）
//    落库成功并按原文回读（回读走裸 SQL，绕开响应侧字段权限掩码，断的是库内真值）。
//    改坏什么必红：
//    · 词表成员被 CHECK 或入口校验漏掉（如只改 constants 忘了迁移）⇒ 该 token 请求 400
//      或旁路 23514，200/落库断言红；
//    · 写链上任一环开始归一大小写/trim（原文语义被破坏）⇒ 逐字符回读断言红；
//    · 取值清单来源=运行时 ALLOWED——为通过而增删词表成员，本用例会同步改测量对象，
//      但用例 1 的三方集合相等会立刻红（用例间互锁）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn endpoint_accepts_each_allowed_value_and_stores_verbatim() {
    let (db, app) = app_with_db().await;

    for token in ALLOWED {
        let code = format!("CT-LCK-OK-{token}");
        let (status, v) = send_json(
            &app,
            Method::POST,
            "/customers",
            json!({
                "customer_code": code,
                "customer_name": "渠道词表活体锁合法值行",
                "customer_type": token,
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "合法渠道值 '{token}' 经标准创建端点必须 200（入口校验或 CHECK 漏掉该值即红），\
             实得信封: {v}"
        );
        assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
        assert_eq!(
            count_by_code(&db, &code).await,
            1,
            "合法值 '{token}' 必须真实落库恰 1 行（端点声称成功但库里没行=假成功）"
        );
        let landed = customer_type_of_code(&db, &code).await;
        assert_eq!(
            landed.as_deref(),
            Some(*token),
            "落库值必须逐字符等于写入值（写链任何 trim/大小写归一都会在这里红）：\
             期望 '{token}' 实际 {landed:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. 越界值真被拒——两条路径分开钉，互不掩盖：
//    3a 端点层：应用校验先于任何触库，POST/PUT 越界值必须 400 + VALIDATION_ERROR
//       （VALIDATION_ERROR 这一族别等值即同时排除 DATABASE_ERROR 族——带着越界值
//       触库撞 CHECK 回来的错误是 500 族，说明入口校验被摘），拒绝必须零落库/目标行零变化；
//    3b 库层兜底：旁路裸 SQL（绕开全部应用校验）写同一批越界形态，必须被
//       {CHK_NAME} 报 23514、约束名逐字归因、零落库；
//    反假绿对照：与 3b 完全同模板的合法值直插必须成功（夹具自身写坏列清单时，
//    一整排"必失败"会集体假绿，此处即对照）。
//    改坏什么必红：handler/模块校验被摘 ⇒ 3a 的 400+VALIDATION_ERROR+零落库红（变
//    500/DATABASE_ERROR 或落库）；DB CHECK 被撤或 NOT VALID ⇒ 3b 直插成功红；
//    归因到别的约束（表上多出第二套词表）⇒ 约束名断言红；
//    越界值被静默吞成 NULL ⇒ 3a 零落库断言红（行会以 NULL/缺省形态存在），3b 同红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn out_of_vocab_rejected_by_endpoint_layer_and_db_check_backstops_bypass_writes() {
    let (db, app) = app_with_db().await;
    // PUT 目标行：owner=操作人本人（排除跨 owner 写门干扰），起点值 'retail'
    insert_customer_typed(&db, PUT_TARGET_ID, "retail")
        .await
        .unwrap_or_else(|e| panic!("前置：种 PUT 目标行失败: {e}"));

    // —— 3a. 端点层拒绝（含大小写漂移样本 VIP/Other） ——
    for (idx, token) in ENDPOINT_BANNED_TOKENS.iter().enumerate() {
        let code = format!("CT-LCK-BAN-EP-{idx}");
        let (status, v) = send_json(
            &app,
            Method::POST,
            "/customers",
            json!({
                "customer_code": code,
                "customer_name": "渠道词表活体锁越界行",
                "customer_type": token,
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "端点对词表外值 '{token}' 必须 400（不得 500/透传落库），实得信封: {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "端点层拒绝必须归 VALIDATION_ERROR 信封（字段校验族）；实得: {v}\n\
             这条等值断言同时排除 DATABASE_ERROR 族：若入口校验被摘，越界值会带着库层\
             错误回来（500/DATABASE_ERROR），本断言与上面的 400 一起红——钉的是\
             **端点层先拒**这条路径，不是库层拒后包装"
        );
        assert_eq!(
            count_by_code(&db, &code).await,
            0,
            "被端点拒绝的 '{token}' 必须零落库（校验置于任何触库之前）"
        );
    }

    // PUT 通道同口径：越界值 400 + VALIDATION_ERROR，目标行逐列保持起点值
    let (status, v) = send_json(
        &app,
        Method::PUT,
        &format!("/customers/{PUT_TARGET_ID}"),
        json!({ "customer_type": "VIP" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "标准更新端点对 'VIP' 必须 400，实得信封: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "PUT 拒绝族别红: {v}");
    let kept = customer_type_of_id(&db, PUT_TARGET_ID).await;
    assert_eq!(
        kept.as_deref(),
        Some("retail"),
        "被拒的 PUT 不得改动目标行（起点 'retail'），实际: {kept:?}"
    );

    // —— 3b. 库层 CHECK 兜底：旁路裸 SQL 绕开所有应用校验 ——
    for (idx, token) in BYPASS_BANNED_TOKENS.iter().enumerate() {
        let id = PROBE_BASE + idx as i32;
        let err = insert_customer_typed(&db, id, token)
            .await
            .err()
            .unwrap_or_else(|| panic!("旁路直插 customer_type='{token}' 竟成功——库层兜底失效"));
        assert_eq!(
            sqlstate_of(&err).as_deref(),
            Some("23514"),
            "旁路写 '{token}' 必须撞 CHECK 成 check_violation（SQLSTATE 23514）；\n\
             期望 23514 实际 SQLSTATE={:?}\n原始错误: {err}\n\
             只断 is_err 是假绿：FK/类型/长度错误都会\"看起来通过\"",
            sqlstate_of(&err)
        );
        assert_eq!(
            constraint_of(&err).as_deref(),
            Some(CHK_NAME),
            "23514 必须归因到 {CHK_NAME} 本约束（只断状态码会把表上别的 CHECK 在挡\
             误当本锁生效）；实际约束名={:?}\n原始错误: {err}",
            constraint_of(&err)
        );
        assert_eq!(
            count_by_id(&db, id).await,
            0,
            "被 CHECK 拒掉的 '{token}' 写入必须零落库（单语句原子失败，不留半行）"
        );
    }

    // —— 反假绿对照：与上面完全同模板的合法值必须插得进 ——
    insert_customer_typed(&db, PROBE_LEGAL_ID, OTHER)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "合法值 '{OTHER}' 同模板直插必须成功（失败说明夹具/列清单写坏，\
                 上面一排\"必失败\"就失去对照意义）: {e}"
            )
        });
    let landed = customer_type_of_id(&db, PROBE_LEGAL_ID).await;
    assert_eq!(
        landed.as_deref(),
        Some(OTHER),
        "对照行必须原样落 '{OTHER}'，实际: {landed:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. 缺省真落 other，且钉"缺省不是 handler 硬塞的兜底"：
//    4a 行为（服务缺省腿）：POST 不带 customer_type ⇒ 落库值 == 运行时 OTHER
//       且 != RETAIL（未知渠道不猜零售）；
//    4b 行为（列缺省腿，独立于任何应用代码）：旁路裸 SQL **省略该列**直插必须成功且
//       落库值 == OTHER——若缺省只活在 handler/service 里而列上没有 DEFAULT，
//       这一行会直接撞 NOT NULL（23502）红在这里；
//    4c 目录：column_default 的字面量 token 与 m0078 原文解析、运行时 OTHER 三向一致
//       （在用例 1 已钉渲染形态，这里补行为回读后的一致性收口）；
//    4d 源码扫描：标准创建入口的缺省补值唯一经 `constants::customer_type::validate`
//       （None→OTHER 的模块路径），handler 文件内不得出现内联 `"other"` 字面量兜底——
//       缺省值全仓只允许出现在词表模块常量与列 DEFAULT 两处。
//    改坏什么必红：validate(None) 改成 RETAIL ⇒ 4a 红；撤列 DEFAULT ⇒ 4b 红；
//    handler 回潮 unwrap_or("other") 式硬塞 ⇒ 4d 红（同时 4b 仍独立证明列缺省存在，
//    不允许用 handler 兜底顶替列缺省，也不允许反过来）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn missing_customer_type_lands_other_via_service_and_column_default() {
    let (db, app) = app_with_db().await;

    // 4a. 端点缺省腿：不带 customer_type 建客户
    let code = "CT-LCK-DEF-EP";
    let (status, v) = send_json(
        &app,
        Method::POST,
        "/customers",
        json!({ "customer_code": code, "customer_name": "渠道缺省活体锁行" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "不带 customer_type 的创建必须成功（缺省腿兜住），实得信封: {v}"
    );
    let landed = customer_type_of_code(&db, code).await;
    assert_eq!(
        landed.as_deref(),
        Some(OTHER),
        "缺省落库值必须等于运行时 constants::customer_type::OTHER（缺省桶=渠道未知），\
         期望 '{OTHER}' 实际 {landed:?}"
    );
    assert_ne!(
        OTHER, RETAIL,
        "缺省桶常量与零售 token 必须是两个不同的值——若有人把 OTHER 常量改成 \"{RETAIL}\"，\
         上面的等值回读会跟着一起绿（测的是漂移后的口径本身），这里独立拦截：\
         缺省不得猜零售，把未知当已知写死会污染渠道维度的全部下游筛选与统计"
    );

    // 4b. 列缺省腿：旁路省略该列（不经任何 handler/service/ORM 代码）
    insert_customer_omitting_type(&db, DEFAULT_PROBE_ID)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "省略 customer_type 的旁路直插必须成功且由**列 DEFAULT** 顶上（失败=缺省\
                 只存在于应用代码、列上没有兜底；若是 NOT NULL 报错说明 DEFAULT 被撤）: {e}"
            )
        });
    let column_landed = customer_type_of_id(&db, DEFAULT_PROBE_ID).await;
    assert_eq!(
        column_landed.as_deref(),
        Some(OTHER),
        "列 DEFAULT 求值必须落 '{OTHER}'（与词表模块 OTHER 同源同值），实际: {column_landed:?}"
    );

    // 4c. 目录字面量与迁移原文、运行时三向一致
    let m0078_src = read_repo_file(M0078_REL);
    let parsed_default = parse_set_default_token(&m0078_src);
    assert_eq!(
        parsed_default, OTHER,
        "m0078 原文 SET DEFAULT token 与运行时 OTHER 不等（迁移原文={parsed_default:?}）"
    );
    let default_raw = one_text(
        &db,
        "SELECT COALESCE(column_default, '<NULL>') AS column_default \
          FROM information_schema.columns \
          WHERE table_schema='public' AND table_name='customers' AND column_name='customer_type'",
        "column_default",
    )
    .await
    .unwrap_or_else(|| panic!("information_schema 里查不到 customers.customer_type（建表漂移）"));
    let default_token = quoted_tokens(&default_raw);
    if default_raw != "<NULL>" {
        assert_eq!(
            default_token,
            [OTHER.to_string()]
                .into_iter()
                .collect::<BTreeSet<String>>(),
            "活库 column_default 表达式里出现的取值字面量必须恰好是 {{'{OTHER}'}} 一个 \
             （NULL 与任何其它 token 一律拒绝）；实际渲染: {default_raw:?}"
        );
    } else {
        // 4b 已经用行为证伪过"列没有 DEFAULT"；走到这里说明目录与行为矛盾，直接红
        panic!(
            "行为回读显示列 DEFAULT 真实顶上（4b 已落行），但 information_schema 报 \
             column_default=NULL——目录与行为矛盾，须人工核查"
        );
    }

    // 4d. 缺省不来自 handler 硬塞（源码扫描：值只允许出现在词表模块与列 DEFAULT 两处）
    let handler_src = read_repo_file(HANDLER_REL);
    assert!(
        handler_src.contains(
            "crate::constants::customer_type::validate(payload.customer_type.as_deref())?"
        ),
        "标准创建入口的缺省补值必须经唯一词表模块 `customer_type::validate` \
         （None→OTHER 的模块路径是缺省的唯一来源）"
    );
    assert!(
        !handler_src.contains("\"other\""),
        "customer_handler.rs 出现内联 \"other\" 字面量=缺省口径第二套（硬塞兜底），\
         缺省值必须只来自 constants::customer_type::OTHER 与列 DEFAULT"
    );
}
