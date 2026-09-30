//! 任务 #163（PR #942 续）：染色缸号（dye_batch）新建链路"造假默认值/幽灵字段"拆除契约锁
//!
//! 锁定的契约（修复后形态，file:line 以修复后工作树为准）：
//! - `backend/src/handlers/dye_batch_handler.rs::resolve_dye_color_identity`
//!   白坯/染色身份归一唯一入口，口径与 `services/inv/fabric_class.rs:25-65`（全仓唯一
//!   白坯/染色判定）同型：色号空=白坯 ⇒ color_code/color_name/dye_lot_no 按"NOT NULL 列
//!   空串表达白坯"的真实空值口径落库（`handlers/inventory_stock_handler_dto.rs:19` 同源）；
//!   色号非空=染色布 ⇒ dye_lot_no 必填、color_code/color_name 由色卡档案
//!   （color_card_items，查询形态同 `services/color_card_scan_service.rs:71-82`）反查派生。
//! - 三处造假默认值全部移除：色号缺失不再写 "TEST"/"测试色号"、染色批号不再写 "DEFAULT"。
//! - `frontend/src/views/fabric/tabs/DyeFormDialogTab.vue`：幽灵字段 actual_quantity
//!   （dye_batch 域全仓无此列）移除；新建表单真实采集 dye_lot_no；提交显式构造
//!   Create/UpdateDyeBatchPayload（start_date→dye_date、空值整键省略、status 仅改动才送）。
//!
//! 修复前必然红的机理：
//! - 白坯用例：旧 build_active 把 color_code/color_name 回退成测试假值/写死假色名、
//!   dye_lot_no 回退成占位串，断言"落库=真实空值口径"直接失配；
//! - 未知色号用例：旧实现不查档、静默落库并返回 200，断言 400+零落库必红；
//! - 染色布缺缸号用例：旧实现静默把 dye_lot_no 写成占位串返回 200，断言 400+零落库必红；
//! - 源码扫描：旧源码含三处假值字面量与类型断言/幽灵字段，任何一条都会命中。
//!
//! 覆盖策略（无 mock）：sqlite::memory: 自建 dye_batch + color_card_items 最小列 +
//! 真实 handler 端到端（tower oneshot，同 contract_wave2 先例）。新建一律显式传 batch_no，
//! 避开自动生成路径的 pg advisory lock（sqlite 不支持，先例同左）。

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
use bingxi_backend::handlers::dye_batch_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::dye_batch;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
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

async fn call_create(app: &Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/production/dye-batches")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// =========================================================
// sqlite 自建表（列与 models/dye_batch.rs、models/color_card_item.rs 一一对应）
// =========================================================

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

const DYE_BATCH_DDL: &str = r#"CREATE TABLE dye_batch (
    id INTEGER PRIMARY KEY,
    batch_no TEXT NOT NULL UNIQUE,
    greige_fabric_id INTEGER,
    color_code TEXT NOT NULL,
    color_name TEXT NOT NULL,
    color_no TEXT,
    dye_lot_no TEXT,
    planned_quantity TEXT,
    actual_output_kg TEXT,
    actual_output_m TEXT,
    greige_input_kg TEXT,
    status TEXT,
    started_at TEXT,
    completed_at TEXT,
    remarks TEXT,
    is_deleted INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)"#;

const COLOR_CARD_ITEMS_DDL: &str = r#"CREATE TABLE color_card_items (
    id INTEGER PRIMARY KEY,
    color_card_id INTEGER NOT NULL,
    color_code TEXT NOT NULL,
    color_name TEXT NOT NULL,
    rgb_r INTEGER NOT NULL,
    rgb_g INTEGER NOT NULL,
    rgb_b INTEGER NOT NULL,
    cmyk_c TEXT, cmyk_m TEXT, cmyk_y TEXT, cmyk_k TEXT,
    lab_l TEXT, lab_a TEXT, lab_b TEXT,
    pantone_code TEXT, cncs_code TEXT, custom_code TEXT,
    hex_value TEXT NOT NULL,
    dye_recipe_id INTEGER,
    product_color_price_id INTEGER,
    swatch_image_url TEXT,
    sequence INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)"#;

async fn fresh_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, DYE_BATCH_DDL).await;
    exec(&db, COLOR_CARD_ITEMS_DDL).await;
    db
}

/// 色卡档案播种一条真实色号（id 唯一自增）
async fn seed_master_color(db: &sea_orm::DatabaseConnection, id: i64, code: &str, name: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO color_card_items (id, color_card_id, color_code, color_name, rgb_r, rgb_g, \
         rgb_b, hex_value, sequence, created_at, updated_at) \
         VALUES ($1, 1, $2, $3, 200, 30, 40, '#C81E28', 0, '2026-01-01T00:00:00Z', \
         '2026-01-01T00:00:00Z')",
        vec![id.into(), code.to_string().into(), name.to_string().into()],
    ))
    .await
    .expect("色卡档案种子插入失败");
}

fn build_app(db: sea_orm::DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/production/dye-batches",
            post(dye_batch_handler::create_dye_batch),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

/// 从落库行直读四列（raw SELECT，不整行解码实体）
async fn fetch_identity_row(
    db: &sea_orm::DatabaseConnection,
    batch_no: &str,
) -> (Option<String>, String, String, String) {
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT color_no, color_code, color_name, dye_lot_no FROM dye_batch \
             WHERE batch_no = $1",
            vec![batch_no.to_string().into()],
        ))
        .await
        .unwrap()
        .expect("被断言的落库行应存在");
    (
        row.try_get::<Option<String>>("", "color_no").unwrap(),
        row.try_get::<String>("", "color_code").unwrap(),
        row.try_get::<String>("", "color_name").unwrap(),
        row.try_get::<String>("", "dye_lot_no").unwrap(),
    )
}

/// 出参/落库全链路绝不允许出现的三个假值字面量
fn assert_no_fake_literals(v: &Value) {
    let dump = serde_json::to_string(v).unwrap();
    for token in ["TEST", "测试色号", "DEFAULT"] {
        assert!(
            !dump.contains(token),
            "响应出参禁止出现造假字面量 {token}，实际响应: {dump}"
        );
    }
}

// =========================================================
// 1) 白坯（色号缺失/空白）→ 真实空值口径，绝无假值
// =========================================================

#[tokio::test]
async fn create_without_color_persists_greige_identity_not_fake_values() {
    let db = fresh_db().await;
    let read_db = db.clone();
    let app = build_app(db);
    let (status, v) = call_create(
        &app,
        json!({ "batch_no": "DB-GREIGE-1", "planned_quantity": 30.0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "白坯新建应 200, 实际: {v}");
    assert_eq!(v["data"]["color_code"], "");
    assert_eq!(v["data"]["color_name"], "");
    assert!(v["data"]["color_no"].is_null(), "白坯 color_no 应落 NULL");
    assert_eq!(v["data"]["dye_lot_no"], "");
    assert_no_fake_literals(&v);

    let (color_no, color_code, color_name, dye_lot_no) =
        fetch_identity_row(&read_db, "DB-GREIGE-1").await;
    assert_eq!(color_no, None);
    assert_eq!(
        color_code, "",
        "白坯 color_code 落空串（NOT NULL 列白坯口径），不是占位假值"
    );
    assert_eq!(color_name, "", "白坯 color_name 落空串，不是写死假色名");
    assert_eq!(
        dye_lot_no, "",
        "白坯免缸号（fabric_class:32-33），落空串不是占位串"
    );
}

#[tokio::test]
async fn create_with_blank_color_is_greige_via_trim_normalization() {
    // 纯空白色号按 fabric_class:41 trim 归一 = 白坯（与空串同口径），不得当作有值去反查档案
    let db = fresh_db().await;
    let read_db = db.clone();
    let app = build_app(db);
    let (status, v) = call_create(
        &app,
        json!({ "batch_no": "DB-GREIGE-2", "color_no": "   ", "dye_lot_no": " " }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "空白色号应归一为白坯, 实际: {v}");
    let (_, color_code, color_name, dye_lot_no) = fetch_identity_row(&read_db, "DB-GREIGE-2").await;
    assert_eq!(color_code, "");
    assert_eq!(color_name, "");
    assert_eq!(dye_lot_no, "");
    assert_no_fake_literals(&v);
}

// =========================================================
// 2) 染色布：color_code/color_name 由色卡档案派生、dye_lot_no 取用户提交值
// =========================================================

#[tokio::test]
async fn create_with_master_color_derives_code_and_name_from_archive() {
    let db = fresh_db().await;
    seed_master_color(&db, 1, "C001", "活性红3BS").await;
    let read_db = db.clone();
    let app = build_app(db);
    let (status, v) = call_create(
        &app,
        json!({
            "batch_no": "DB-DYED-1",
            "color_no": "C001",
            "dye_lot_no": "DL-098",
            "planned_quantity": 120.0
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "档案存在的染色布新建应 200, 实际: {v}"
    );
    assert_eq!(v["data"]["color_code"], "C001");
    assert_eq!(
        v["data"]["color_name"], "活性红3BS",
        "色名必须来自主数据反查"
    );
    assert_eq!(v["data"]["dye_lot_no"], "DL-098");
    assert_no_fake_literals(&v);

    let (color_no, color_code, color_name, dye_lot_no) =
        fetch_identity_row(&read_db, "DB-DYED-1").await;
    assert_eq!(color_no.as_deref(), Some("C001"));
    assert_eq!(color_code, "C001");
    assert_eq!(color_name, "活性红3BS");
    assert_eq!(dye_lot_no, "DL-098");
}

// =========================================================
// 3) 色号有值但档案无此色 → 显式 400（不回退假名、零落库）
// =========================================================

#[tokio::test]
async fn create_with_unknown_color_rejected_400_and_nothing_persisted() {
    let db = fresh_db().await;
    seed_master_color(&db, 1, "C001", "活性红3BS").await; // 档案非空，只是没有 ZZZ9
    let read_db = db.clone();
    let app = build_app(db);
    let (status, v) = call_create(
        &app,
        json!({ "batch_no": "DB-UNKNOWN-1", "color_no": "ZZZ9", "dye_lot_no": "DL-1" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "档案无此色号必须 400, 实际: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(
        v["message"], "色号 ZZZ9 在色卡档案中不存在",
        "拒绝文案必须外显且只回显用户提交的色号，不得回退假色名"
    );
    assert_no_fake_literals(&v);
    let rows = dye_batch::Entity::find().count(&read_db).await.unwrap();
    assert_eq!(
        rows, 0,
        "被拒绝的新建必须零落库（修复前会静默写入 TEST/测试色号 假值行）"
    );
}

// =========================================================
// 4) 染色布缺 dye_lot_no → 显式 400（不写占位串）
// =========================================================

#[tokio::test]
async fn create_dyed_without_dye_lot_rejected_400() {
    let db = fresh_db().await;
    seed_master_color(&db, 1, "C001", "活性红3BS").await;
    let read_db = db.clone();
    let app = build_app(db);
    let (status, v) =
        call_create(&app, json!({ "batch_no": "DB-DYED-2", "color_no": "C001" })).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "染色布缺缸号必须 400, 实际: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(
        v["message"], "染色布必须提供缸号（color_no=C001 但 dye_lot_no 为空）",
        "文案与 fabric_class 唯一权威判定同源，只回显用户提交值"
    );
    assert_no_fake_literals(&v);
    let rows = dye_batch::Entity::find().count(&read_db).await.unwrap();
    assert_eq!(
        rows, 0,
        "被拒绝的新建必须零落库（修复前会静默写入占位批号行）"
    );
}

// =========================================================
// 5) helper 级：档案同色号多行歧义 → 显式业务错，不任选不兜底
// =========================================================

#[tokio::test]
async fn ambiguous_master_rows_rejected_not_arbitrarily_picked() {
    let db = fresh_db().await;
    seed_master_color(&db, 1, "C001", "活性红3BS").await;
    seed_master_color(&db, 2, "C001", "活性红3BS-副本卡").await;
    let err = dye_batch_handler::resolve_dye_color_identity(
        &db,
        Some("C001".into()),
        Some("DL-1".into()),
    )
    .await
    .expect_err("多条记录无法唯一定位必须显式报错");
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "实际: {err:?}"
    );
    assert!(
        err.to_string().contains("无法唯一定位"),
        "应指向歧义分支，实际: {err}"
    );
}

// =========================================================
// 6) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层函数块:anchor 起,到首个 "\n}"。
/// 先剔除 `\r`:Windows 工作树 CRLF 会使跨行 contains 断言漏检(先例 wave2 测试)
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
fn source_scan_create_dye_batch_has_no_fake_default_literals() {
    let src = include_str!("../src/handlers/dye_batch_handler.rs");
    // 整文件级：三处造假字面量绝迹（含注释提法也不允许原样回潮）
    for token in ["TEST", "测试色号", "DEFAULT"] {
        assert!(
            !src.contains(token),
            "dye_batch_handler.rs 禁止再出现造假字面量 {token}"
        );
    }
    let block = extract_block(src, "pub async fn create_dye_batch");
    assert!(
        block.contains("resolve_dye_color_identity("),
        "create 必须经白坯/染色身份归一唯一入口取色/批号,实际块:\n{block}"
    );
    assert!(
        block.contains("color_code: Set(identity.color_code.clone())"),
        "color_code 必须来自归一结果,不得回潮 req 字段直接回退,实际块:\n{block}"
    );
    assert!(
        block.contains("dye_lot_no: Set(identity.dye_lot_no.clone())"),
        "dye_lot_no 必须来自归一结果,实际块:\n{block}"
    );
    // 归一实现自身不得用 unwrap_or_else 造默认值
    let helper = extract_block(src, "pub async fn resolve_dye_color_identity");
    assert!(
        !helper.contains("unwrap_or_else(|| \""),
        "归一入口禁止 unwrap_or_else 回退字面量默认值,实际块:\n{helper}"
    );
}

#[test]
fn source_scan_dye_form_dialog_no_ghost_field_no_cast() {
    let vue = include_str!("../../frontend/src/views/fabric/tabs/DyeFormDialogTab.vue");
    assert!(
        !vue.contains("formData.actual_quantity"),
        "actual_quantity 在 dye_batch 域无对应列（models/dye_batch.rs 与两个请求 DTO 均无），\
         幽灵表单项必须保持移除"
    );
    assert!(
        !vue.contains("labelActualQuantity"),
        "幽灵表单项的 i18n 键引用不得回潮"
    );
    assert!(
        !vue.contains(" as Partial<"),
        "提交禁止类型断言绕过真实 payload 契约"
    );
    assert!(
        !vue.contains("Object.assign(formData"),
        "编辑回填必须显式逐字段（出参键/类型与表单不一一对应）"
    );
    assert!(
        vue.contains("dye_lot_no"),
        "新建表单必须真实采集染色批号（后端染色布必填）"
    );
    assert!(
        vue.contains("CreateDyeBatchPayload") && vue.contains("UpdateDyeBatchPayload"),
        "提交必须按真实 payload 类型显式构造"
    );
    assert!(
        vue.contains("loadedStatus"),
        "status 必须原值跟踪、仅改动才携带（后端提交即状态机流转校验）"
    );
}
