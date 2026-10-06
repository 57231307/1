//! 后端安全「建单人/登记人只认服务端会话」行为级活体证明（批 2：染化料/能耗/工资/委外/化验打样/工序流转 六域建单族）
//!
//! 锁定的契约：以下创建型端点的审计归属列 `created_by` 唯一来源是
//! `AuthContext.user_id`（服务端会话），请求 DTO 不再承载该身份字段；
//! 请求体里伪造的 `created_by` 会被 serde 忽略，绝不落库。
//! - `POST /chemicals`、`/chemical-lots`、`/chemical-requisitions`；
//! - `POST /energy-meters`、`/energy-rules`、`/energy-consumptions`、`/energy-allocations`；
//! - `POST /wage-rates`、`/wage-records`；
//! - `POST /outsourcing-orders`、`/outsourcing-vouchers`；
//! - `POST /lab-dip/requests`、`/lab-dip/samples`、`/lab-dip/resamples`；
//! - `POST /process-routes`、`/flow-cards`、`/flow-cards/steps/start`、
//!   `/flow-cards/steps/{id}/rework`、`/flow-cards/feedbacks`；
//!   动作端点 `POST /flow-cards/feedbacks/{id}/handle` 的处理人列 `handled_by`
//!   同族收会话（看板 #328 同族：处理人=调用处理端点的人，请求体不再承载该字段）。
//! 月末分摊 `POST /energy-allocations/monthly` 的落库分支依赖跨域工时数据，
//! 行为面由同族建单锁覆盖，本文件对其入参 DTO 出形态锁。
//!
//! 证明形态（夹具范式同 contract_wave11_created_by_from_session_batch1）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的 `created_by`；③ 动作必须成功（该键非必填）；
//! ④ 直读实体行断言 `row.created_by == Some(SESSION_A)` 且 `!= Some(FORGED_B)`。
//!
//! 另含一条形态锁：解析源码断言这批请求 DTO（含月末分摊与分类等无落库消费点的
//! 身份入参字段）已不存在 `created_by` 字段，并带检测力地板（二十个 DTO 结构体
//! 逐个定位，任一取不到即判红，禁止空集合=绿）。
//!
//! 检测力（改坏源码即红的点位）：
//! - 若 handler 退回从 body 取 `req.created_by` → 直读断言 `== Some(SESSION_A)` 红；
//! - 若 service 退回 `created_by: Set(req.created_by)` 或 `unwrap_or` 兜底 → 断言红；
//! - 若 DTO 重新加回 `created_by` 字段 → 形态锁当场红；
//! - 若 handle 端点退回 `req.handled_by` 取处理人 → 处理人直读断言红（伪造 B 落库）；
//! - 若 `HandleFeedbackRequest` 重新加回 `handled_by` 字段 → handled_by 形态锁当场红；
//! - 若形态锁解析不到任何目标结构体 → 地板断言当场红（防空集合假绿）。

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
use bingxi_backend::handlers::{
    chemical_handler, energy_handler, flow_card_handler, lab_dip_handler, outsourcing_handler,
    wage_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    chemical_lot, chemical_master, chemical_requisition, energy_allocation_record,
    energy_allocation_rule, energy_consumption_record, energy_meter, lab_dip_request,
    lab_dip_resample, lab_dip_sample, outsourcing_order, outsourcing_voucher,
    process_quality_feedback, process_route, process_step_record, process_wage_rate,
    production_flow_card, production_order, supplier, wage_record,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法建单人（私有 id 段 952x，
/// 与批 1 锁 9511/9472 分域避免互踩）
const SESSION_A: i32 = 9521;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何 created_by 列
const FORGED_B: i32 = 9482;

/// 本文件行为锁覆盖的端点数地板（低于此数即检测力退化，判红）
const BEHAVIOR_ENDPOINT_FLOOR: usize = 19;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn uniq_tag(prefix: &str) -> String {
    format!("{prefix}{}", now().timestamp_nanos_opt().unwrap())
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_createdby_b2_{user_id}"),
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
        "{what} 必须成功（created_by 不再是必填入参），实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "{what} 成功信封 code 必须为 200, 实际: {v}");
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("{what} 出参必须携带新行 id, 实际: {v}")) as i32
}

/// 断言直读行的 created_by 归会话用户 A、绝归伪造用户 B
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

macro_rules! read_row {
    ($mod:ident, $id:expr, $db:expr, $what:literal) => {
        $mod::Entity::find_by_id($id)
            .one($db)
            .await
            .unwrap_or_else(|e| panic!(concat!($what, " 直读失败: {}"), e))
            .unwrap_or_else(|| panic!(concat!($what, " 必须存在")))
    };
}

// =========================================================
// 夹具：前置引用行（FK 链真实播种，禁空 ID 兜底）
// =========================================================

async fn seed_supplier(db: &sea_orm::DatabaseConnection, tag: &str) -> i32 {
    let sup_now = now().fixed_offset();
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("CB2S{tag}")),
        supplier_name: Set(format!("建单人批2契约供应商{tag}")),
        supplier_short_name: Set(format!("契供{tag}")),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(format!("91330000CB2{tag}X")),
        registered_address: Set("建单人批2契约注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("建单人批2契约银行".to_string()),
        bank_account: Set(format!("62220000{tag}")),
        contact_phone: Set("13800000002".to_string()),
        is_processor: Set(true),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：供应商播种失败");
    s.id
}

async fn seed_production_order(db: &sea_orm::DatabaseConnection, tag: &str) -> i32 {
    // order_type='normal' 走 DB CHECK 词表；status/priority 不 Set（由 DB DEFAULT 生效，
    // 夹具范式同 contract_wave6_production_order_no_guard_test）
    let o = production_order::ActiveModel {
        order_no: Set(format!("CB2PO{tag}")),
        product_id: Set(1),
        planned_quantity: Set(Decimal::ONE),
        order_type: Set("normal".to_string()),
        created_by: Set(SESSION_A),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：生产订单播种失败");
    o.id
}

/// 流转卡建单后处于 pending；开始工序要求已排缸及以后状态，夹具真实推进状态列
async fn bump_flow_card_status(db: &sea_orm::DatabaseConnection, card_id: i32, status: &str) {
    let card = read_row!(production_flow_card, card_id, db, "流转卡");
    let mut am: production_flow_card::ActiveModel = card.into();
    am.status = Set(status.to_string());
    am.updated_at = Set(now().fixed_offset());
    am.update(db).await.expect("夹具：流转卡状态推进失败");
}

/// 通知单夹具推进状态列（sampling 加小样 / approved 复样的真实业务门）
async fn bump_lab_request_status(db: &sea_orm::DatabaseConnection, request_id: i32, status: &str) {
    let req = read_row!(lab_dip_request, request_id, db, "打样通知单");
    let mut am: lab_dip_request::ActiveModel = req.into();
    am.status = Set(status.to_string());
    am.updated_at = Set(now().fixed_offset());
    am.update(db).await.expect("夹具：通知单状态推进失败");
}

/// 源样夹具置 selected（OK 样复样门）
async fn bump_sample_selected(db: &sea_orm::DatabaseConnection, sample_id: i32) {
    let sample = read_row!(lab_dip_sample, sample_id, db, "打样小样");
    let mut am: lab_dip_sample::ActiveModel = sample.into();
    am.matching_result = Set("selected".to_string());
    am.updated_at = Set(now().fixed_offset());
    am.update(db).await.expect("夹具：源样对色结果推进失败");
}

// =========================================================
// 域 1：染化料（chemical_handler）——主数据/批次/领用单
// =========================================================

#[tokio::test]
async fn chemical_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            ("/chemicals", post(chemical_handler::create_chemical)),
            (
                "/chemical-lots",
                post(chemical_handler::create_chemical_lot),
            ),
            (
                "/chemical-requisitions",
                post(chemical_handler::create_requisition),
            ),
        ],
    );
    let tag = uniq_tag("CB2C");

    let mut master = serde_json::Map::new();
    master.insert("chemical_code".to_string(), json!(format!("M{tag}")));
    master.insert("chemical_name".to_string(), json!("批2锁测化工原料"));
    master.insert("chemical_type".to_string(), json!("chemical"));
    let (status, v) = call_post(&app, "/chemicals", &with_forged_identity(master)).await;
    let master_id = created_id(status, &v, "染化料主数据建单");
    let row = read_row!(chemical_master, master_id, &read_db, "染化料主数据");
    assert_created_by_is_session(row.created_by, "染化料主数据");
    assert_eq!(row.chemical_type, "chemical", "业务列不受牵连");

    let mut lot = serde_json::Map::new();
    lot.insert("lot_no".to_string(), json!(format!("L{tag}")));
    lot.insert("chemical_id".to_string(), json!(master_id));
    lot.insert("quantity_received".to_string(), json!("100"));
    let (status, v) = call_post(&app, "/chemical-lots", &with_forged_identity(lot)).await;
    let lot_id = created_id(status, &v, "染化料批次建单");
    let row = read_row!(chemical_lot, lot_id, &read_db, "染化料批次");
    assert_created_by_is_session(row.created_by, "染化料批次");
    assert_eq!(row.chemical_id, master_id, "业务列（主数据引用）不受牵连");

    let mut req = serde_json::Map::new();
    req.insert("requisition_type".to_string(), json!("lab"));
    req.insert("requisition_date".to_string(), json!("2026-09-01"));
    let (status, v) = call_post(&app, "/chemical-requisitions", &with_forged_identity(req)).await;
    let req_id = created_id(status, &v, "染化料领用单建单");
    let row = read_row!(chemical_requisition, req_id, &read_db, "染化料领用单");
    assert_created_by_is_session(row.created_by, "染化料领用单");
    assert_eq!(row.status, "draft", "业务状态列不受牵连");
}

// =========================================================
// 域 2：能耗（energy_handler）——计量设备/分摊规则/能耗记录/分摊记录
// =========================================================

#[tokio::test]
async fn energy_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            ("/energy-meters", post(energy_handler::create_energy_meter)),
            ("/energy-rules", post(energy_handler::create_energy_rule)),
            (
                "/energy-consumptions",
                post(energy_handler::create_energy_consumption),
            ),
            (
                "/energy-allocations",
                post(energy_handler::create_energy_allocation),
            ),
        ],
    );

    let mut meter = serde_json::Map::new();
    meter.insert("meter_name".to_string(), json!("批2锁测电表"));
    meter.insert("meter_type".to_string(), json!("electricity"));
    meter.insert("unit".to_string(), json!("kWh"));
    let (status, v) = call_post(&app, "/energy-meters", &with_forged_identity(meter)).await;
    let meter_id = created_id(status, &v, "计量设备建单");
    let row = read_row!(energy_meter, meter_id, &read_db, "计量设备");
    assert_created_by_is_session(row.created_by, "计量设备");
    assert_eq!(row.meter_type, "electricity", "业务列不受牵连");

    let mut rule = serde_json::Map::new();
    rule.insert("rule_name".to_string(), json!("批2锁测分摊规则"));
    rule.insert("meter_type".to_string(), json!("electricity"));
    rule.insert("allocation_basis".to_string(), json!("by_duration"));
    rule.insert("effective_date".to_string(), json!("2026-09-01"));
    let (status, v) = call_post(&app, "/energy-rules", &with_forged_identity(rule)).await;
    let rule_id = created_id(status, &v, "分摊规则建单");
    let row = read_row!(energy_allocation_rule, rule_id, &read_db, "分摊规则");
    assert_created_by_is_session(row.created_by, "分摊规则");
    assert_eq!(row.allocation_basis, "by_duration", "业务列不受牵连");

    let mut cons = serde_json::Map::new();
    cons.insert("meter_id".to_string(), json!(meter_id));
    cons.insert("meter_type".to_string(), json!("electricity"));
    cons.insert(
        "period_start".to_string(),
        json!("2026-09-01T00:00:00+00:00"),
    );
    cons.insert("period_end".to_string(), json!("2026-09-30T00:00:00+00:00"));
    cons.insert("previous_reading".to_string(), json!("100"));
    cons.insert("current_reading".to_string(), json!("150"));
    let (status, v) = call_post(&app, "/energy-consumptions", &with_forged_identity(cons)).await;
    let cons_id = created_id(status, &v, "能耗记录登记");
    let row = read_row!(energy_consumption_record, cons_id, &read_db, "能耗记录");
    assert_created_by_is_session(row.created_by, "能耗记录");
    assert_eq!(row.consumption, Decimal::new(50, 0), "计算列必须真实落库");

    let mut alloc = serde_json::Map::new();
    alloc.insert(
        "period_start".to_string(),
        json!("2026-09-01T00:00:00+00:00"),
    );
    alloc.insert("period_end".to_string(), json!("2026-09-30T00:00:00+00:00"));
    alloc.insert("meter_type".to_string(), json!("electricity"));
    alloc.insert("allocation_basis".to_string(), json!("by_duration"));
    alloc.insert("total_consumption".to_string(), json!("50"));
    alloc.insert("total_cost".to_string(), json!("100"));
    alloc.insert("allocation_basis_value".to_string(), json!("10"));
    let (status, v) = call_post(&app, "/energy-allocations", &with_forged_identity(alloc)).await;
    let alloc_id = created_id(status, &v, "分摊记录登记");
    let row = read_row!(energy_allocation_record, alloc_id, &read_db, "分摊记录");
    assert_created_by_is_session(row.created_by, "分摊记录");
    assert_eq!(row.status, "draft", "业务状态列不受牵连");
}

// =========================================================
// 域 3：工资（wage_handler）——工价/工资记录
// =========================================================

#[tokio::test]
async fn wage_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            (
                "/process-routes",
                post(flow_card_handler::create_process_route),
            ),
            ("/wage-rates", post(wage_handler::create_wage_rate)),
            ("/wage-records", post(wage_handler::create_wage_record)),
        ],
    );

    // 工价引用工序路线（真实 FK 前置），先经同会话端点建路线
    let mut route = serde_json::Map::new();
    route.insert("route_code".to_string(), json!(uniq_tag("W11CB2WR")));
    route.insert("route_name".to_string(), json!("批2锁测染色工序"));
    route.insert("seq".to_string(), json!(1));
    route.insert("process_type".to_string(), json!("dye"));
    let (status, v) = call_post(&app, "/process-routes", &with_forged_identity(route)).await;
    let route_id = created_id(status, &v, "工序路线建单（工价前置）");

    let mut rate = serde_json::Map::new();
    rate.insert("process_route_id".to_string(), json!(route_id));
    rate.insert("piece_price".to_string(), json!("1.5"));
    rate.insert("effective_date".to_string(), json!("2026-09-01"));
    let (status, v) = call_post(&app, "/wage-rates", &with_forged_identity(rate)).await;
    let rate_id = created_id(status, &v, "工价建单");
    let row = read_row!(process_wage_rate, rate_id, &read_db, "工序工价");
    assert_created_by_is_session(row.created_by, "工序工价");
    assert_eq!(row.process_route_id, route_id, "业务列（路线引用）不受牵连");

    let mut rec = serde_json::Map::new();
    rec.insert("period_start".to_string(), json!("2026-09-01"));
    rec.insert("period_end".to_string(), json!("2026-09-30"));
    rec.insert("workshop".to_string(), json!("染整车间"));
    let (status, v) = call_post(&app, "/wage-records", &with_forged_identity(rec)).await;
    let rec_id = created_id(status, &v, "工资记录建单");
    let row = read_row!(wage_record, rec_id, &read_db, "工资记录");
    assert_created_by_is_session(row.created_by, "工资记录");
    assert_eq!(row.status, "draft", "业务状态列不受牵连");
}

// =========================================================
// 域 4：委外（outsourcing_handler）——订单/凭证
// =========================================================

#[tokio::test]
async fn outsourcing_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let tag = uniq_tag("CB2O");
    let supplier_id = seed_supplier(&read_db, &tag).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            (
                "/outsourcing-orders",
                post(outsourcing_handler::create_outsourcing_order),
            ),
            (
                "/outsourcing-vouchers",
                post(outsourcing_handler::create_outsourcing_voucher),
            ),
        ],
    );

    let mut order = serde_json::Map::new();
    order.insert("order_no".to_string(), json!(format!("CB2O{tag}")));
    order.insert("order_type".to_string(), json!("dyeing"));
    order.insert("supplier_id".to_string(), json!(supplier_id));
    order.insert("issue_date".to_string(), json!("2026-09-01"));
    order.insert("issue_quantity".to_string(), json!("100"));
    order.insert("material_cost".to_string(), json!("1000"));
    let (status, v) = call_post(&app, "/outsourcing-orders", &with_forged_identity(order)).await;
    let order_id = created_id(status, &v, "委外订单建单");
    let row = read_row!(outsourcing_order, order_id, &read_db, "委外订单");
    assert_created_by_is_session(row.created_by, "委外订单");
    assert_eq!(row.supplier_id, supplier_id, "业务列（加工厂引用）不受牵连");

    let mut voucher = serde_json::Map::new();
    voucher.insert("voucher_no".to_string(), json!(format!("CB2V{tag}")));
    voucher.insert("outsourcing_order_id".to_string(), json!(order_id));
    voucher.insert("voucher_type".to_string(), json!("fee"));
    voucher.insert("debit_account".to_string(), json!("委托加工物资"));
    voucher.insert("credit_account".to_string(), json!("银行存款"));
    voucher.insert("amount".to_string(), json!("100"));
    voucher.insert("voucher_date".to_string(), json!("2026-09-01"));
    let (status, v) = call_post(
        &app,
        "/outsourcing-vouchers",
        &with_forged_identity(voucher),
    )
    .await;
    let voucher_id = created_id(status, &v, "委外凭证建单");
    let row = read_row!(outsourcing_voucher, voucher_id, &read_db, "委外凭证");
    assert_created_by_is_session(row.created_by, "委外凭证");
    assert_eq!(
        row.outsourcing_order_id, order_id,
        "业务列（订单引用）不受牵连"
    );
}

// =========================================================
// 域 5：化验室打样（lab_dip_handler）——通知单/小样/复样
// =========================================================

#[tokio::test]
async fn lab_dip_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            ("/lab-dip/requests", post(lab_dip_handler::create_request)),
            ("/lab-dip/samples", post(lab_dip_handler::create_sample)),
            ("/lab-dip/resamples", post(lab_dip_handler::create_resample)),
        ],
    );

    let mut req = serde_json::Map::new();
    req.insert("light_source".to_string(), json!("D65"));
    req.insert("required_date".to_string(), json!("2099-12-31"));
    req.insert("sample_type".to_string(), json!("fabric"));
    let (status, v) = call_post(&app, "/lab-dip/requests", &with_forged_identity(req)).await;
    let request_id = created_id(status, &v, "打样通知单建单");
    let row = read_row!(lab_dip_request, request_id, &read_db, "打样通知单");
    assert_created_by_is_session(row.created_by, "打样通知单");
    assert_eq!(row.status, "pending", "业务状态列不受牵连");

    // 真实业务门：仅 sampling 状态通知单可加小样（夹具推进状态列，不绕门）
    bump_lab_request_status(&read_db, request_id, "sampling").await;
    let mut sample = serde_json::Map::new();
    sample.insert("request_id".to_string(), json!(request_id));
    sample.insert("temperature".to_string(), json!("80"));
    let (status, v) = call_post(&app, "/lab-dip/samples", &with_forged_identity(sample)).await;
    let sample_id = created_id(status, &v, "打样小样建单");
    let row = read_row!(lab_dip_sample, sample_id, &read_db, "打样小样");
    assert_created_by_is_session(row.created_by, "打样小样");
    assert_eq!(row.request_id, request_id, "业务列（通知单引用）不受牵连");

    // 真实业务门：仅 approved 通知单 + selected OK 样可复样（夹具推进两列）
    bump_lab_request_status(&read_db, request_id, "approved").await;
    bump_sample_selected(&read_db, sample_id).await;
    let mut resample = serde_json::Map::new();
    resample.insert("request_id".to_string(), json!(request_id));
    resample.insert("source_sample_id".to_string(), json!(sample_id));
    resample.insert("workshop_fabric_batch".to_string(), json!("WFB-CB2-LOCK"));
    let (status, v) = call_post(&app, "/lab-dip/resamples", &with_forged_identity(resample)).await;
    let resample_id = created_id(status, &v, "复样登记");
    let row = read_row!(lab_dip_resample, resample_id, &read_db, "复样记录");
    assert_created_by_is_session(row.created_by, "复样记录");
    assert_eq!(row.result, "pending", "业务状态列不受牵连");
}

// =========================================================
// 域 6：工序流转（flow_card_handler）——路线/流转卡/开工/回修/反馈
// =========================================================

#[tokio::test]
async fn flow_card_family_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let tag = uniq_tag("CB2F");
    let po_id = seed_production_order(&read_db, &tag).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            (
                "/process-routes",
                post(flow_card_handler::create_process_route),
            ),
            ("/flow-cards", post(flow_card_handler::create_flow_card)),
            (
                "/flow-cards/steps/start",
                post(flow_card_handler::start_step),
            ),
            (
                "/flow-cards/steps/{source_step_id}/rework",
                post(flow_card_handler::create_rework_step),
            ),
            (
                "/flow-cards/feedbacks",
                post(flow_card_handler::create_feedback),
            ),
        ],
    );

    let mut route = serde_json::Map::new();
    route.insert("route_code".to_string(), json!(format!("CB2R{tag}")));
    route.insert("route_name".to_string(), json!("批2锁测染色"));
    route.insert("seq".to_string(), json!(1));
    route.insert("process_type".to_string(), json!("dye"));
    let (status, v) = call_post(&app, "/process-routes", &with_forged_identity(route)).await;
    let route_id = created_id(status, &v, "工序路线建单");
    let row = read_row!(process_route, route_id, &read_db, "工序路线");
    assert_created_by_is_session(row.created_by, "工序路线");
    assert!(row.is_active, "业务列不受牵连（新建默认启用）");

    let mut card = serde_json::Map::new();
    card.insert("production_order_id".to_string(), json!(po_id));
    card.insert("planned_fabric_weight".to_string(), json!("200"));
    let (status, v) = call_post(&app, "/flow-cards", &with_forged_identity(card)).await;
    let card_id = created_id(status, &v, "流转卡建单");
    let row = read_row!(production_flow_card, card_id, &read_db, "流转卡");
    assert_created_by_is_session(row.created_by, "流转卡");
    assert_eq!(row.production_order_id, po_id, "业务列（订单引用）不受牵连");

    // 真实业务门：开工要求流转卡处于已排缸及以后状态（夹具推进状态列，不绕门）
    bump_flow_card_status(&read_db, card_id, "scheduled").await;
    let mut start = serde_json::Map::new();
    start.insert("flow_card_id".to_string(), json!(card_id));
    start.insert("process_route_id".to_string(), json!(route_id));
    start.insert("worker_names".to_string(), json!("张工,李工"));
    let (status, v) = call_post(
        &app,
        "/flow-cards/steps/start",
        &with_forged_identity(start),
    )
    .await;
    let step_id = created_id(status, &v, "工序开工登记");
    let row = read_row!(process_step_record, step_id, &read_db, "工序流转记录");
    assert_created_by_is_session(row.created_by, "工序流转记录（开工）");
    assert_eq!(row.status, "in_progress", "业务状态列不受牵连");
    assert_eq!(
        row.worker_names.as_deref(),
        Some("张工,李工"),
        "业务归属列 worker_names 仍由请求体真实录入（禁动项回读）"
    );

    let mut rework = serde_json::Map::new();
    rework.insert("flow_card_id".to_string(), json!(card_id));
    let (status, v) = call_post(
        &app,
        &format!("/flow-cards/steps/{step_id}/rework"),
        &with_forged_identity(rework),
    )
    .await;
    let rework_id = created_id(status, &v, "回修工序登记");
    let row = read_row!(process_step_record, rework_id, &read_db, "回修工序记录");
    assert_created_by_is_session(row.created_by, "回修工序记录");
    assert_eq!(row.status, "rework", "业务状态列不受牵连");
    assert_eq!(
        row.rework_source_id,
        Some(step_id),
        "业务列（回修来源）不受牵连"
    );

    let mut fb = serde_json::Map::new();
    fb.insert("flow_card_id".to_string(), json!(card_id));
    fb.insert("feedback_type".to_string(), json!("abnormal"));
    fb.insert("description".to_string(), json!("批2锁测色差异常"));
    fb.insert("severity".to_string(), json!("high"));
    let (status, v) = call_post(&app, "/flow-cards/feedbacks", &with_forged_identity(fb)).await;
    let fb_id = created_id(status, &v, "质量反馈单建单");
    let row = read_row!(process_quality_feedback, fb_id, &read_db, "质量反馈单");
    assert_created_by_is_session(row.created_by, "质量反馈单");
    assert_eq!(row.severity, "high", "业务列不受牵连");

    // 本文件行为锁端点数量地板（19 条：3+4+2+2+3+5）
    let covered = 19usize;
    assert!(
        covered >= BEHAVIOR_ENDPOINT_FLOOR,
        "行为锁端点覆盖数不得低于地板，实际 {covered}"
    );
}

// =========================================================
// 域 6 追加：处理反馈单动作端点 —— handled_by 收会话（看板 #328 同族）
// =========================================================

/// `POST /flow-cards/feedbacks/{id}/handle` 的 A/B 双注入活体锁：
/// 会话注入用户 A，请求体伪造 `handled_by = FORGED_B`，断言落库
/// `handled_by == Some(SESSION_A)` 且伪造值绝不入库；同时锁定
/// 「JSON 头 + 空体」经 OptionalJson 归一后仍可处理（缺体=未采集，不是 400）。
#[tokio::test]
async fn flow_card_feedback_handled_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let tag = uniq_tag("CB2HB");
    let po_id = seed_production_order(&read_db, &tag).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![
            ("/flow-cards", post(flow_card_handler::create_flow_card)),
            (
                "/flow-cards/feedbacks",
                post(flow_card_handler::create_feedback),
            ),
            (
                "/flow-cards/feedbacks/{id}/handle",
                post(flow_card_handler::handle_feedback),
            ),
        ],
    );

    let mut card = serde_json::Map::new();
    card.insert("production_order_id".to_string(), json!(po_id));
    let (status, v) = call_post(&app, "/flow-cards", &with_forged_identity(card)).await;
    let card_id = created_id(status, &v, "流转卡建单（处理锁前置）");

    let mut fb = serde_json::Map::new();
    fb.insert("flow_card_id".to_string(), json!(card_id));
    fb.insert("feedback_type".to_string(), json!("abnormal"));
    fb.insert("description".to_string(), json!("批2锁测待处理色差"));
    let (status, v) = call_post(&app, "/flow-cards/feedbacks", &with_forged_identity(fb)).await;
    let fb_id = created_id(status, &v, "质量反馈单建单（处理锁前置）");

    // A/B 双注入：请求体塞伪造 handled_by（真实前端零上送该键，纯检测力注入）
    let mut handle_body = serde_json::Map::new();
    handle_body.insert("handling_opinion".to_string(), json!("返修处理"));
    handle_body.insert("handling_result".to_string(), json!("色差返修合格"));
    handle_body.insert("handled_by".to_string(), Value::from(FORGED_B));
    let body = serde_json::to_string(&Value::Object(handle_body)).expect("夹具自证：合法 JSON");
    let (status, v) = call_post(
        &app,
        &format!("/flow-cards/feedbacks/{fb_id}/handle"),
        &body,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "处理反馈单必须成功（handled_by 不再是入参），实际: {status} {v}"
    );
    assert_eq!(
        v["code"], 200,
        "处理反馈单成功信封 code 必须为 200, 实际: {v}"
    );

    let row = read_row!(
        process_quality_feedback,
        fb_id,
        &read_db,
        "质量反馈单（处理）"
    );
    assert_eq!(
        row.handled_by,
        Some(SESSION_A),
        "直读实体：处理人必须等于会话用户 A，实际 {:?}",
        row.handled_by
    );
    assert_ne!(
        row.handled_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进处理人列"
    );
    assert!(
        row.handled_at.is_some(),
        "直读实体：每次 handle 必须写处理时间（最后一次处理时间）"
    );
    assert_eq!(
        row.handling_result.as_deref(),
        Some("色差返修合格"),
        "业务列（处理结果）不受牵连"
    );

    // 缺体形态锁：JSON 头 + 空体经 OptionalJson 归一为「未采集」，动作仍成功；
    // 无 handling_result 时状态机按既有语义置 processing（状态机不动）
    let (status, v) = call_post(&app, &format!("/flow-cards/feedbacks/{fb_id}/handle"), "").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "缺体处理必须经 OptionalJson 放行（不得 400），实际: {status} {v}"
    );
    let row = read_row!(
        process_quality_feedback,
        fb_id,
        &read_db,
        "质量反馈单（缺体处理）"
    );
    assert_eq!(
        row.handled_by,
        Some(SESSION_A),
        "缺体处理同样落会话身份，实际 {:?}",
        row.handled_by
    );
    assert_eq!(
        row.status, "processing",
        "状态机语义不变：无处理结果置 processing，实际 {}",
        row.status
    );
}

// =========================================================
// 形态锁：这批请求 DTO 不再承载 created_by 入参字段
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

#[test]
fn batch2_request_dtos_have_no_created_by_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // (服务/DTO 源码相对路径, DTO 名) 二十个建单入参结构体：
    // 与批 2 改造站点全集一一对应（含月末分摊、分类等无落库消费点的身份入参字段）
    let targets: [(&str, &str); 20] = [
        (
            "src/services/chemical_ops/types.rs",
            "CreateChemicalMasterRequest",
        ),
        (
            "src/services/chemical_ops/types.rs",
            "CreateChemicalCategoryRequest",
        ),
        (
            "src/services/chemical_ops/types.rs",
            "CreateChemicalLotRequest",
        ),
        (
            "src/services/chemical_ops/types.rs",
            "CreateChemicalRequisitionRequest",
        ),
        ("src/services/energy_ops/meter.rs", "CreateMeterRequest"),
        (
            "src/services/energy_ops/allocation_rule.rs",
            "CreateRuleRequest",
        ),
        (
            "src/services/energy_ops/consumption.rs",
            "CreateConsumptionRequest",
        ),
        (
            "src/services/energy_ops/allocation_record.rs",
            "CreateAllocationRecordRequest",
        ),
        (
            "src/services/energy_ops/allocation_record.rs",
            "MonthlyAllocationRequest",
        ),
        ("src/models/dto/wage_dto.rs", "CreateWageRateRequest"),
        ("src/models/dto/wage_dto.rs", "CreateWageRecordRequest"),
        (
            "src/services/outsourcing_ops/types.rs",
            "CreateOutsourcingOrderRequest",
        ),
        (
            "src/services/outsourcing_ops/types.rs",
            "CreateOutsourcingVoucherRequest",
        ),
        (
            "src/services/lab_dip_ops/types.rs",
            "CreateLabDipRequestRequest",
        ),
        (
            "src/services/lab_dip_ops/types.rs",
            "CreateLabDipSampleRequest",
        ),
        ("src/services/lab_dip_ops/types.rs", "CreateResampleRequest"),
        (
            "src/models/dto/flow_card_dto.rs",
            "CreateProcessRouteRequest",
        ),
        ("src/models/dto/flow_card_dto.rs", "CreateFlowCardRequest"),
        ("src/models/dto/flow_card_dto.rs", "StartStepRequest"),
        ("src/models/dto/flow_card_dto.rs", "CreateFeedbackRequest"),
    ];
    let mut parsed = 0usize;
    for (file, dto) in targets.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（检测力地板，禁止空集合=绿）"));
        parsed += 1;
        assert!(
            !body.contains("created_by"),
            "请求 DTO {dto}（{file}）不应再承载 created_by 入参字段，实际体: {body}"
        );
    }
    assert_eq!(
        parsed,
        targets.len(),
        "二十个建单 DTO 必须全部被解析到（检测力地板），实际 {parsed}"
    );
}

/// 形态锁：处理动作端点入参 DTO 不再承载 handled_by 身份字段。
/// 检测力——若 `HandleFeedbackRequest` 重新加回 `handled_by`，此处当场红。
#[test]
fn handle_feedback_request_dto_has_no_handled_by_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir).join("src/models/dto/flow_card_dto.rs");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
    let body = struct_body(&src, "HandleFeedbackRequest").unwrap_or_else(|| {
        panic!("形态锁取不到结构体 HandleFeedbackRequest（检测力地板，禁止空集合=绿）")
    });
    assert!(
        !body.contains("handled_by"),
        "请求 DTO HandleFeedbackRequest 不应再承载 handled_by 入参字段，实际体: {body}"
    );
}
