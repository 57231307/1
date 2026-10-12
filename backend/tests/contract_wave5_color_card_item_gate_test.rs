//! 色号维护门控契约锁（建卡→加色号链路真实跑通）
//!
//! 本锁禁绝的形态（门控若比一个写不出来的状态值，API 恒 400）：
//! - `color_card_item_service.rs` create / validate_color_card_for_import 门控比 `master_data::ACTIVE`
//!   （`'active'`）；
//! - 但色卡创建写入的是 `card_status::DRAFT`（`'draft'`，`color_card_crud_service.rs` create），
//!   `'active'` 已被色卡权威词表 `color_card::ALL` 与 DB CHECK `chk_color_card_status`
//!   排除为不可写值（legacy active 由迁移回填为 draft），且没有任何把色卡写成 active 的端点；
//! - ⇒ 比 `'active'` 即比一个业务上写不出来的死值，任何 API 建的色卡加色号 / 批量导入恒 400。
//!
//! 锁定形态（业务语义：允许在草稿态色卡上维护色号）：
//! - 门控比较色卡词表里真实存在且可达的"可编辑态"集合 `EDITABLE_CARD_STATUSES = {draft}`
//!   （与色卡主体 update 门控、前端 `views/color-cards/list.vue` 编辑入口 `status === 'draft'` 同源）；
//! - 拒绝文案按保密分层：'只有草稿态色卡可以维护色号' 是纯公开业务规则（不含内部状态 token /
//!   记录 ID）→ `AppError::business_displayable`（HTTP 400 / code=BUSINESS_ERROR + 真实文案外显）。
//!
//! 覆盖策略（表结构唯一来源 = backend/migration）：
//! - 正向：真实 draft 卡 → POST items 成功、色号回读等值（建卡→加色号链跑通）；批量导入同链成功；
//! - 反向：终态（archived）/ 已发放（issued）→ 400 + code=BUSINESS_ERROR
//!   + 外显文案；被拒时零色号落库；
//! - legacy 死值 active 双层锁：活库层断言"把色卡状态写成 active 必须被
//!   DB CHECK chk_color_card_status 拒绝且零漂移"；应用层由源码扫描锁继续断言
//!   "代码不得把 active 当可编辑态"（真 PG 下该状态根本播种不出来，"插 active 再撞门"
//!   的前置无法构造）；
//! - 源码扫描（include_str!）：门控比较的每个 token 必须来自色卡词表且在 `color_card::ALL` 内，
//!   服务不得再引用 `master_data::ACTIVE` 或裸 "active"。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::color_card::{
    batch_import_items, create_color_card, create_color_item, list_color_items,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::color_card::{Column as CardColumn, Entity as CardEntity};
use bingxi_backend::models::color_card_item::{Column as ItemColumn, Entity as ItemEntity};
use bingxi_backend::models::status::color_card as card_status;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, PaginatorTrait, QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

// =========================================================
// 夹具：真 PostgreSQL（表结构唯一来源 = backend/migration）
// 列与 models/color_card.rs、models/color_card_item.rs 一一对应由迁移保证，
// 本文件不自建 CREATE TABLE（sqlite 方言把 DECIMAL 写成 TEXT 会引发列解码失败）。
// =========================================================

async fn fresh_db() -> sea_orm::DatabaseConnection {
    setup_test_db().await
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave5_user_{user_id}"),
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

fn build_app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route("/color-cards", post(create_color_card))
        // axum 0.8 起路径参数语法为 {id}；写 :id 会在 build 时 panic
        // "Path segments must not start with ':'"
        .route(
            "/color-cards/{id}/items",
            get(list_color_items).post(create_color_item),
        )
        .route("/color-cards/{id}/items/batch", post(batch_import_items))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

async fn send(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
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

/// 建一张真实色卡（走 create 端点，落库状态=词表 draft），返回 (router, raw_db, card_id)
async fn create_real_draft_card() -> (Router, sea_orm::DatabaseConnection, i64) {
    let db = fresh_db().await;
    let raw_db = db.clone();
    let app = build_app(db);
    let (status, v) = send(
        &app,
        Method::POST,
        "/color-cards",
        Some(json!({
            "card_no": "WAVE5-TPX-2026-01",
            "card_name": "wave5 契约色卡",
            "card_type": "PANTONE"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "建卡应 200，实际: {v}");
    // 真实状态 = draft（词表 DRAFT）——本例正是钉在"真实可达的草稿态"上，不造假态
    assert_eq!(
        v["data"]["status"],
        card_status::DRAFT,
        "建卡落库状态应为 draft，实际: {v}"
    );
    let id = v["data"]["id"].as_i64().expect("建卡响应应带 id");
    (app, raw_db, id)
}

fn valid_item_payload(code: &str) -> Value {
    json!({
        "color_code": code,
        "color_name": "wave5 测试色",
        "rgb_r": 63, "rgb_g": 127, "rgb_b": 90,
        "hex_value": "#3F7F5A"
    })
}

/// 直接 UPDATE 色卡状态（真 PG 走 chk_color_card_status 约束）。
/// 返回 DbErr 供 R2 双层锁用例断言"写 legacy 死值必须被数据库拒绝"。
async fn try_set_card_status(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    status: &str,
) -> Result<(), sea_orm::DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE color_cards SET status = $1 WHERE id = $2",
        vec![status.to_string().into(), id.into()],
    ))
    .await
    .map(|_| ())
}

async fn set_card_status(db: &sea_orm::DatabaseConnection, id: i64, status: &str) {
    try_set_card_status(db, id, status)
        .await
        .expect("改色卡状态失败");
}

async fn reload_card_status(db: &sea_orm::DatabaseConnection, id: i64) -> String {
    CardEntity::find()
        .filter(CardColumn::Id.eq(id))
        .one(db)
        .await
        .unwrap()
        .expect("色卡行应存在")
        .status
}

async fn count_items(db: &sea_orm::DatabaseConnection, card_id: i64) -> u64 {
    ItemEntity::find()
        .filter(ItemColumn::ColorCardId.eq(card_id))
        .count(db)
        .await
        .unwrap()
}

// =========================================================
// 1) 正向：真实 draft 卡 → 新增色号成功、色号回读等值（建卡→加色号链首次真跑通）
// =========================================================

#[tokio::test]
async fn create_item_on_real_draft_card_succeeds_and_reads_back() {
    let (app, raw_db, card_id) = create_real_draft_card().await;

    let (status, v) = send(
        &app,
        Method::POST,
        &format!("/color-cards/{card_id}/items"),
        Some(valid_item_payload("W5-C001")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "草稿态色卡新增色号必须成功（修复前比死值 active 恒 400），实际: {v}"
    );
    assert_eq!(v["data"]["color_code"], "W5-C001", "出参色号应回显: {v}");

    // 回读等值：GET 列表能取到刚写入的色号，且字段与提交一致
    let (lstatus, lv) = send(
        &app,
        Method::GET,
        &format!("/color-cards/{card_id}/items"),
        None,
    )
    .await;
    assert_eq!(lstatus, StatusCode::OK, "色号列表查询应 200: {lv}");
    assert_eq!(
        lv["data"]["total"].as_i64(),
        Some(1),
        "应恰好回读到 1 条色号: {lv}"
    );
    let items = lv["data"]["items"].as_array().expect("items 应为数组");
    assert_eq!(items[0]["color_code"], "W5-C001");
    assert_eq!(items[0]["hex_value"], "#3F7F5A");
    assert_eq!(items[0]["rgb_r"].as_i64(), Some(63));

    // 落库核对（不信任出参，直查表）+ 色卡 total_colors 已自增
    assert_eq!(
        count_items(&raw_db, card_id).await,
        1,
        "被成功新增的色号应落库 1 条"
    );
}

// =========================================================
// 2) 正向：批量导入同链在 draft 上成功（覆盖 validate_color_card_for_import 门控）
// =========================================================

#[tokio::test]
async fn batch_import_on_real_draft_card_succeeds() {
    let (app, _raw_db, card_id) = create_real_draft_card().await;

    let (status, v) = send(
        &app,
        Method::POST,
        &format!("/color-cards/{card_id}/items/batch"),
        Some(json!({
            "items": [ valid_item_payload("W5-B001"), valid_item_payload("W5-B002") ]
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "草稿态色卡批量导入必须成功（修复前恒 400），实际: {v}"
    );
    assert_eq!(
        v["data"]["success_count"].as_i64(),
        Some(2),
        "应 2 条成功: {v}"
    );
    assert_eq!(
        v["data"]["failed_count"].as_i64(),
        Some(0),
        "应 0 条失败: {v}"
    );
    assert_eq!(
        v["data"]["total_colors"].as_i64(),
        Some(2),
        "导入后总色号数应为 2: {v}"
    );
}

// =========================================================
// 3) 反向：终态/不可编辑态 → 400 + code=BUSINESS_ERROR + 外显分层文案 + 零落库
// =========================================================

/// 非可编辑态拒绝断言：终态 archived / 已发放 issued（真 PG 下可达的非可编辑态）。
/// legacy 死值 active 不再走本函数——它在活库层就被 CHECK 拒绝（见 R2 双层锁用例
/// create_item_rejected_on_legacy_active_value_not_whitelisted_anymore）。
async fn assert_gate_rejects(app: &Router, card_id: i64, label: &str) {
    let (status, v) = send(
        app,
        Method::POST,
        &format!("/color-cards/{card_id}/items"),
        Some(valid_item_payload("W5-REJ")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "{label} 色卡新增色号必须 400，实际: {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "{label} 状态门必须归 business 族，实际: {v}"
    );
    assert_eq!(
        v["message"], "只有草稿态色卡可以维护色号",
        "{label} 出参必须外显公开业务规则文案，实际: {v}"
    );
    // 不得泄露内部状态 token
    assert!(
        !v["message"].as_str().unwrap_or("").contains("draft")
            && !v["message"].as_str().unwrap_or("").contains("active"),
        "出参文案只描述公开规则，不得回显内部 token: {v}"
    );
}

#[tokio::test]
async fn create_item_rejected_on_terminal_archived_card() {
    let (app, raw_db, card_id) = create_real_draft_card().await;
    set_card_status(&raw_db, card_id, card_status::ARCHIVED).await;
    assert_gate_rejects(&app, card_id, "archived(终态)").await;
    assert_eq!(
        count_items(&raw_db, card_id).await,
        0,
        "被状态门拒绝的色号必须零落库"
    );
}

#[tokio::test]
async fn create_item_rejected_on_issued_card() {
    let (app, raw_db, card_id) = create_real_draft_card().await;
    set_card_status(&raw_db, card_id, card_status::ISSUED).await;
    assert_gate_rejects(&app, card_id, "issued(已发放)").await;
    assert_eq!(count_items(&raw_db, card_id).await, 0, "拒绝后应零落库");
}

#[tokio::test]
async fn create_item_rejected_on_legacy_active_value_not_whitelisted_anymore() {
    // R2 双层锁之"活库层"：'active' 是 legacy 死值，最终 chk_color_card_status 不含它，
    // 真 PG 下把色卡状态写成 active 这一步前置就 23514 被数据库拒绝、整笔不落库（零漂移），
    // 从而证明门控比的那套"可发放/可编辑态"里绝不可能出现 active。
    // 应用层锁见 source_scan_item_gate_tokens_are_all_in_color_card_word_list
    // （继续钉死 EDITABLE_CARD_STATUSES 不含 active、服务不得引用裸 "active"）。
    let (app, raw_db, card_id) = create_real_draft_card().await;

    let err = try_set_card_status(&raw_db, card_id, "active")
        .await
        .expect_err("legacy 死值 active 必须被 chk_color_card_status 数据库拒绝（不得静默落库）");
    let msg = err.to_string();
    assert!(
        msg.contains("chk_color_card_status") || msg.contains("23514"),
        "拒绝原因必须是 CHECK 约束违例（chk_color_card_status / SQLSTATE 23514），实际: {msg}"
    );

    // 回读确认零漂移：状态仍是 draft（写 active 整笔不落库）
    assert_eq!(
        reload_card_status(&raw_db, card_id).await,
        card_status::DRAFT,
        "被数据库拒绝后色卡状态必须保持 draft，不得半落 active"
    );
    // draft 是可编辑态，此时门控不应拒绝——反向证明"死值 active 永不出现在门控面"
    let (status, _v) = send(
        &app,
        Method::POST,
        &format!("/color-cards/{card_id}/items"),
        Some(valid_item_payload("W5-REJ")),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "回读仍为 draft（可编辑态），旧 active 未落库"
    );
}

// =========================================================
// 4) 防回潮源码扫描（无 DB）：门控比较的每个 token 必须出现在 color_card::ALL 里
// =========================================================

fn word_value(ident: &str) -> Option<&'static str> {
    match ident {
        "DRAFT" => Some(card_status::DRAFT),
        "ISSUED" => Some(card_status::ISSUED),
        "RECEIVED" => Some(card_status::RECEIVED),
        "USED" => Some(card_status::USED),
        "EXPIRED" => Some(card_status::EXPIRED),
        "ARCHIVED" => Some(card_status::ARCHIVED),
        "LOST" => Some(card_status::LOST),
        _ => None,
    }
}

#[test]
fn source_scan_item_gate_tokens_are_all_in_color_card_word_list() {
    let src = include_str!("../src/services/color_card_item_service.rs").replace('\r', "");

    // (a) 不得再比较 legacy/master_data 死值
    assert!(
        !src.contains("master_data::ACTIVE"),
        "色号门控不得再引用 master_data::ACTIVE（'active' 死值）"
    );
    assert!(
        !src.contains(r#""active""#),
        "色号门控不得残留裸字面量 \"active\""
    );

    // (b) 门控必须统一经 EDITABLE_CARD_STATUSES.contains(&card.status.as_str())
    assert!(
        src.contains("EDITABLE_CARD_STATUSES.contains(&card.status.as_str())"),
        "色号增删/批量导入必须统一经可编辑态集合门控"
    );

    // (c) 抽取集合初始化式，逐项必须是色卡词表常量引用（不得是字符串字面量）。
    // 锚点必须带 `= &[`（初始化式），不能只锚 `&[`——否则先命中类型注解
    // `const EDITABLE_CARD_STATUSES: &[&str]` 里的 `&[&str]`，把 `&str` 当 token 误报。
    let marker = "const EDITABLE_CARD_STATUSES";
    let i = src.find(marker).expect("可编辑态集合常量缺失");
    let rest = &src[i..];
    let init_marker = "= &[";
    let open = rest
        .find(init_marker)
        .unwrap_or_else(|| panic!("集合初始化式缺失 \"{init_marker}\"，实际片段: {rest}"))
        + init_marker.len();
    let close = open + rest[open..].find(']').expect("集合初始化未闭合 ]");
    let inner = &rest[open..close];

    let mut checked = 0usize;
    for raw in inner.split(',') {
        let tok = raw.trim();
        if tok.is_empty() {
            continue;
        }
        let ident = tok
            .strip_prefix("card_status::")
            .unwrap_or_else(|| panic!("可编辑态集合只能引用色卡词表常量，实际: {tok}"));
        let value = word_value(ident)
            .unwrap_or_else(|| panic!("集合引用了词表外常量 card_status::{ident}"));
        assert!(
            card_status::ALL.contains(&value),
            "门控 token {value} 不在 color_card::ALL 内（防再出现比一个死值）"
        );
        assert_ne!(value, "active", "可编辑态集合不得含 legacy active");
        checked += 1;
    }
    assert!(checked >= 1, "可编辑态集合至少应包含一个词表 token");
}
