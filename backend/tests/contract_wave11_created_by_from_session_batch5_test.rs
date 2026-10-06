//! 后端安全「建单人只认服务端会话」行为级活体证明 —— 第 5 批（库存盘点 / 存货跌价准备）
//!
//! 复核结论（先取证后动手，源码零改动）：本组两个被点名的落库点，其身份值
//! **已经**只能来自服务端会话，请求体根本没有承载身份的字段：
//! - 盘点：`handlers/inventory_count_handler.rs:247` 组装 `CreateCountRequest`
//!   时写 `created_by: Some(auth.user_id)`，服务层 `inventory_count_service.rs:188`
//!   `created_by: Set(req.created_by)` 只是转发该内部结构体的字段；建单入参
//!   `CreateCountPayload`（`handlers/inventory_count_handler.rs:37`）无身份字段，
//!   请求体里伪造的 `created_by` 会被 serde 直接丢弃。
//! - 跌价：`handlers/inventory_write_down_handler.rs:140` 写 `created_by: auth.user_id`，
//!   `CreateWriteDownPayload`（`handlers/inventory_write_down_handler.rs:35`）无身份字段，
//!   服务层 `inventory_write_down_service.rs:87` 同样是转发。
//!   注：`inventory_write_down.created_by` 在 DDL 里是 `INTEGER NOT NULL`
//!   （`backend/migration/src/domain/v15/mod.rs:2976`）⇒ 模型为 `i32`
//!   （`backend/src/models/inventory_write_down.rs:26`），落库形态必须是 `Set(user_id)`
//!   而非 `Set(Some(user_id))`；`inventory_counts.created_by` 则是可空列
//!   （`backend/migration/src/domain/system/m0001_initial_schema.rs:577`）⇒ `Option<i32>`。
//!
//! 因此本批**不改业务源码**，改为把上述事实钉成回归锁（一旦有人把身份退回请求体
//! 或在 handler 里换成 body 取值，当场判红）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的 `created_by`/`operator_id`/`user_id`；
//! ③ 动作必须成功（身份键非必填、伪造键被 serde 忽略）；
//! ④ 直读实体行断言建单人列归会话用户 A、绝不归伪造用户 B（可空列断 `Some(A)`，
//!    NOT NULL 列断 `A`），并断业务列不受牵连（盘点状态 pending / 明细数、跌价状态 draft / 跌价额）。
//! ⑤ 形态锁：两域建单入参 DTO 结构体不存在任何身份入参字段。
//! ⑥ 会话来源锁：两域 handler 建单函数体内 `created_by` 的赋值右侧只能是 `auth.user_id`。
//!
//! 检测力（改坏源码即红的点位）：
//! - handler 退回从 body 取身份（如 `created_by: payload.created_by`）→ ①④ 直读断言红 + ⑥ 红；
//! - DTO 重新加回 `created_by` 字段 → ⑤ 当场红；
//! - 形态锁/会话来源锁解析不到目标结构体或函数体 → 地板断言当场红（禁止空集合=绿）。

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
use bingxi_backend::handlers::{inventory_count_handler, inventory_write_down_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::inventory_count as count_status;
use bingxi_backend::models::{
    inventory_count, inventory_stock, inventory_write_down, product, warehouse,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法建单人（与同族批 1 同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何 created_by 列
const FORGED_B: i32 = 9472;

/// FK 前置链固定行号（inventory_stocks.warehouse_id→warehouses、
/// inventory_stocks.product_id→products，见 m0001_initial_schema.rs:630-631；
/// inventory_count_items.stock_id→inventory_stocks，见 m0044:1135）
const WH_ID: i32 = 7742;
const PRODUCT_ID: i32 = 7742;
const STOCK_ID: i32 = 7742;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_createdby_b5_{user_id}"),
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

/// 请求体统一掺入伪造身份键（handler 不认的键 serde 直接忽略——正是要证明的「发了也无效」）
fn with_forged_identity(mut obj: serde_json::Map<String, Value>) -> String {
    obj.insert("created_by".to_string(), Value::from(FORGED_B));
    obj.insert("operator_id".to_string(), Value::from(FORGED_B));
    obj.insert("user_id".to_string(), Value::from(FORGED_B));
    serde_json::to_string(&Value::Object(obj)).expect("伪造身份体必须是合法 JSON（夹具自证）")
}

async fn call_post(app: &Router, path: &str, body: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
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

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按占位符注册，同真实 routes 形态）
fn layered_app(
    state: AppState,
    routes: Vec<(&'static str, axum::routing::MethodRouter<AppState>)>,
) -> Router {
    let mut router: Router<AppState> = Router::new();
    for (path, method_route) in routes {
        router = router.route(path, method_route);
    }
    router
        .with_state(state)
        .layer(from_fn_with_state(make_auth(SESSION_A), inject_auth))
}

/// 成功信封取新建行 id，并断动作真实成功（缺身份键合法、伪造键被忽略）
fn created_id(status: StatusCode, v: &Value, what: &str) -> i32 {
    assert_eq!(
        status,
        StatusCode::OK,
        "{what} 必须成功（created_by 不是入参字段），实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "{what} 成功信封 code 必须为 200, 实际: {v}");
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("{what} 出参必须携带新行 id, 实际: {v}")) as i32
}

/// 断言可空建单人列（inventory_counts.created_by，m0001:577 可空）归会话用户 A、绝归伪造 B
fn assert_created_by_is_session(row_created_by: Option<i32>, what: &str) {
    assert_eq!(
        row_created_by,
        Some(SESSION_A),
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {row_created_by:?}"
    );
    assert_ne!(
        row_created_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

/// 断言 NOT NULL 建单人列（inventory_write_down.created_by，v15/mod.rs:2976 NOT NULL）
fn assert_created_by_column_is_session(row_created_by: i32, what: &str) {
    assert_eq!(
        row_created_by, SESSION_A,
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {row_created_by}"
    );
    assert_ne!(
        row_created_by, FORGED_B,
        "直读实体：body 伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

macro_rules! read_row {
    ($mod:ident, $id:expr, $db:expr, $what:literal) => {
        $mod::Entity::find_by_id($id)
            .one($db)
            .await
            .unwrap_or_else(|e| panic!(concat!($what, " 直读失败: {}"), e))
            .unwrap_or_else(|| panic!(concat!($what, " 必须存在")))
    };
}

/// 盘点建单的 FK 前置链：products → warehouses → inventory_stocks
/// （`create_count` 要求仓库下有库存快照，否则按业务门控拒绝，`inventory_count_service.rs:125`）
async fn seed_count_prereq(db: &sea_orm::DatabaseConnection) {
    product::ActiveModel {
        id: Set(PRODUCT_ID),
        name: Set("盘点身份锁面料".to_string()),
        code: Set("PRD-CB5-0001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    warehouse::ActiveModel {
        id: Set(WH_ID),
        warehouse_code: Set("WH-CB5-0001".to_string()),
        name: Set("盘点身份锁仓库".to_string()),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    inventory_stock::ActiveModel {
        id: Set(STOCK_ID),
        warehouse_id: Set(WH_ID),
        product_id: Set(PRODUCT_ID),
        quantity_on_hand: Set(Decimal::new(12050, 2)),
        quantity_available: Set(Decimal::new(12050, 2)),
        quantity_reserved: Set(Decimal::ZERO),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(Decimal::ZERO),
        max_stock_point: Set(Decimal::ZERO),
        reorder_quantity: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        batch_no: Set("B-CB5-0001".to_string()),
        color_no: Set("RED-CB5".to_string()),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(Decimal::new(12050, 2)),
        quantity_kg: Set(Decimal::new(100, 2)),
        stock_status: Set("正常".to_string()),
        quality_status: Set("合格".to_string()),
        version: Set(0),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

// =========================================================
// 站点 1：库存盘点建单（POST /inventory/counts）
// =========================================================

#[tokio::test]
async fn inventory_count_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    seed_count_prereq(&db).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/inventory/counts",
            post(inventory_count_handler::create_count),
        )],
    );

    let mut body = serde_json::Map::new();
    body.insert("warehouse_id".to_string(), json!(WH_ID));
    body.insert("count_date".to_string(), json!("2026-10-01T00:00:00Z"));
    body.insert("notes".to_string(), json!("_createdby_b5_lock"));
    let body = with_forged_identity(body);

    let (status, v) = call_post(&app, "/inventory/counts", &body).await;
    let id = created_id(status, &v, "盘点单建单");

    let row = read_row!(inventory_count, id, &read_db, "盘点单");
    assert_created_by_is_session(row.created_by, "盘点单");
    // 业务列不受身份改造牵连（状态词表取权威常量，与写入方同源）
    assert_eq!(
        row.status,
        count_status::PENDING,
        "盘点单初始状态必须由服务层写 pending（inventory_count_service.rs:183）"
    );
    assert_eq!(row.total_items, 1, "盘点明细数必须按快照行数真实落库");
    assert_eq!(row.warehouse_id, WH_ID, "业务列 warehouse_id 不受牵连");
    assert!(
        row.count_no.starts_with("IC"),
        "单号仍由后端生成器产出，实际: {}",
        row.count_no
    );
}

// =========================================================
// 站点 2：存货跌价准备建单（POST /inventory/write-downs）
// =========================================================

#[tokio::test]
async fn inventory_write_down_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/inventory/write-downs",
            post(inventory_write_down_handler::create_write_down),
        )],
    );

    let mut body = serde_json::Map::new();
    body.insert("product_id".to_string(), json!(PRODUCT_ID));
    body.insert("write_down_type".to_string(), json!("seasonal"));
    body.insert("original_cost".to_string(), json!(100));
    body.insert("net_realizable_value".to_string(), json!(80));
    body.insert("reason".to_string(), json!("_createdby_b5_lock"));
    body.insert("period".to_string(), json!("2026-09-30"));
    let body = with_forged_identity(body);

    let (status, v) = call_post(&app, "/inventory/write-downs", &body).await;
    let id = created_id(status, &v, "跌价准备建单");

    let row = read_row!(inventory_write_down, id, &read_db, "跌价准备记录");
    assert_created_by_column_is_session(row.created_by, "跌价准备记录");
    // 业务列不受身份改造牵连（跌价准备无状态常量载体，取值即服务写入字面量
    // inventory_write_down_service.rs:86 `status: Set("draft".to_string())`）
    assert_eq!(row.status, "draft", "跌价准备初始状态必须由服务层写 draft");
    assert_eq!(
        row.write_down_amount,
        Decimal::new(2000, 2),
        "跌价额=成本-可变现净值，必须真实算出"
    );
    assert_eq!(row.product_id, PRODUCT_ID, "业务列 product_id 不受牵连");
}

// =========================================================
// 形态锁：两域建单入参 DTO 不再承载任何身份入参字段
// =========================================================

/// 在源码文本中定位 `pub struct NAME { ... }` 的结构体体文本。
/// 找不到即返回 None——由调用方按地板判红，禁止静默跳过。
fn struct_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub struct {name} {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

/// 在源码文本中定位 `pub async fn NAME( ... ` 起的函数体文本（到首个顶格 `}` 为止）。
fn fn_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub async fn {name}(");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

#[test]
fn batch5_request_dtos_have_no_identity_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // (handler 源码相对路径, 建单入参 DTO 名) 与上方两条行为锁一一对应
    let targets: [(&str, &str); 2] = [
        (
            "src/handlers/inventory_count_handler.rs",
            "CreateCountPayload",
        ),
        (
            "src/handlers/inventory_write_down_handler.rs",
            "CreateWriteDownPayload",
        ),
    ];
    let mut parsed = 0usize;
    for (file, dto) in targets.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（检测力地板，禁止空集合=绿）"));
        parsed += 1;
        for forbidden in ["created_by", "operator_id", "user_id"] {
            assert!(
                !body.contains(forbidden),
                "建单入参 DTO {dto}（{file}）不应承载身份入参字段 {forbidden}，实际体: {body}"
            );
        }
    }
    assert_eq!(
        parsed,
        targets.len(),
        "两个建单 DTO 必须全部被解析到（检测力地板），实际 {parsed}"
    );
}

/// 会话来源锁：handler 建单函数体内 `created_by` 的赋值右侧只能是会话 `auth.user_id`
/// ——若有人改回从 payload 取身份（`payload.created_by` / 反序列化字段），当场判红。
#[test]
fn batch5_handlers_source_created_by_from_session() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let checks: [(&str, &str, &str); 2] = [
        (
            "src/handlers/inventory_count_handler.rs",
            "create_count",
            "created_by: Some(auth.user_id)",
        ),
        (
            "src/handlers/inventory_write_down_handler.rs",
            "create_write_down",
            "created_by: auth.user_id",
        ),
    ];
    let mut parsed = 0usize;
    for (file, func, expected) in checks.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("会话来源锁取不到源码 {}（检测力地板）: {e}", path.display())
        });
        let body = fn_body(&src, func)
            .unwrap_or_else(|| panic!("会话来源锁取不到函数 {func}（检测力地板）"));
        parsed += 1;
        assert!(
            body.contains(expected),
            "{file} 的 {func} 必须以 {expected} 落身份（建单人只能来自服务端会话），实际体: {body}"
        );
        assert!(
            !body.contains("payload.created_by"),
            "{file} 的 {func} 禁止从请求体 payload 取 created_by，实际体: {body}"
        );
    }
    assert_eq!(
        parsed,
        checks.len(),
        "两个建单 handler 函数必须全部被解析到（检测力地板），实际 {parsed}"
    );
}
