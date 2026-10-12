//! AP 付款申请汇率条件必填契约锁
//!
//! 契约面：`backend/src/services/ap_payment_request_service.rs`
//! （`create` 入口 → `resolve_currency_and_rate` 服务端权威解析 →
//!   `build_payment_request_active_model` 真实落库路径）
//!
//! 锁定动机：落库汇率绝不允许 `exchange_rate.unwrap_or(Decimal::new(1, 0))` 这类
//! 静默兜底——外币（USD/EUR）付款申请未录汇率时被伪造成 1，会直接污染
//! 折算/核销/汇兑损益，故外币缺汇率必须显式拒绝。
//!
//! 本文件锁定的四条契约（全部真 PostgreSQL 活库执行，无 #[ignore]；
//! 表结构唯一来源 = backend/migration）
//! 1) 外币缺汇率 → 400 VALIDATION_ERROR（字段校验族，非状态门 BUSINESS 族），
//!    出参 message 外显真实文案「外币付款请填写汇率」，非脱敏常量「请求参数验证失败」，
//!    且主表零写入（校验先行于事务/取号/明细动作）；
//! 2) 外币带汇率 → 经服务端解析 + 真实 builder 落库，真库回读断言汇率为请求真实值
//!    （不是只看 HTTP status）；
//! 3) 本位币（CNY / 币种缺省）即使携带汇率 → 落库汇率恒 1（服务端权威短路，
//!    当前口径为「忽略而非拒绝」）；
//! 4) 源码扫描防回潮锁：`exchange_rate` 落库行 unwrap_or 命中数恒 0（绝对锁），
//!    全文件 `.unwrap_or(` 计数为只减不增 ratchet（基线 = 当前剩余合法值计数）。
//!
//! 覆盖边界声明：create 的**单号生成**走
//! pg_advisory_xact_lock——真 PG 下取号可全程真跑，HTTP 端到端不再停在拒绝点；
//! 1) 仍只到校验先行即返回（外币缺汇率不触库），2)/3) 的落库值契约以
//! 「服务端权威解析 + 生产 builder + 真实 INSERT/回读」锁定，不 mock、不复制构建逻辑。
//! 种子供应商引用迁移播种的参照表 suppliers（m0015 演示供应商，supplier_id=1 稳定可依赖；
//! suppliers 不在业务表清空名单内，不会被用例清掉）。

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
use bingxi_backend::handlers::ap_payment_request_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::ap_payment_request;
use bingxi_backend::services::ap_payment_request_service::{
    ApPaymentRequestService, CreateApPaymentRequest,
};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("测试基准汇率/金额必须可解码为 Decimal")
}

/// 表头合法的基础请求体（与前端 createAPPaymentRequest 真实形状一致：从不发 items）。
/// supplier_id=1 引用迁移播种的演示供应商（m0015；suppliers 为不被清空的参照表，
/// 真库下 fk_ap_payment_request_supplier 外键可满足）
fn base_body() -> Value {
    json!({
        "supplier_id": 1,
        "request_date": "2026-01-15",
        "payment_type": "NORMAL",
        "payment_method": "BANK_TRANSFER",
        "request_amount": "1000.50"
    })
}

// =========================================================
// 真库基建：表结构唯一来源 = backend/migration（不自建同构表）
// ap_payment_request 的 DECIMAL/DEFAULT 口径由迁移提供（m0012 + v15/mod.rs 时区列转换），
// 不再由测试自建 sqlite 同构表（currency/exchange_rate 的 DEFAULT 见
// models/ap_payment_request.rs:49-54 与 m0012:65-66，测试只引用、不重述）
// =========================================================

/// 与 entity 列一一对应的真实表已由迁移建好；此处仅取已迁移并清空业务表的连接
async fn live_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

// =========================================================
// HTTP 端到端基建（真库：校验拒绝点与取号全链路都可真跑）
// =========================================================

fn make_auth(user_id: i32, scope: Option<&str>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: scope.map(|s| s.to_string()),
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
    let value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => json!({ "raw_body": String::from_utf8_lossy(&bytes).to_string() }),
    };
    (status, value)
}

fn build_app(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/ap/payment-requests",
            post(ap_payment_request_handler::create_request),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100, Some("all")), inject_auth))
}

async fn seeded_http_app() -> (Router, DatabaseConnection) {
    let db = live_db().await;
    let read_db = db.clone();
    (build_app(db), read_db)
}

// =========================================================
// 契约 1：外币缺汇率 → 400 VALIDATION_ERROR + 外显文案 + 零写入
// =========================================================

#[tokio::test]
async fn foreign_currency_without_rate_rejected_400_validation_displayable() {
    let (app, db) = seeded_http_app().await;
    let mut body = base_body();
    body["currency"] = json!("USD");
    // 不发 exchange_rate 键——该形态必须被显式拒绝，绝不允许以汇率 1 静默落库

    let (status, v) = call(&app, Method::POST, "/ap/payment-requests", Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "外币缺汇率必须 400: {v}");
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "字段跨字段条件必填属字段校验族（状态门才走 BUSINESS），实际: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert_eq!(
        msg, "外币付款请填写汇率",
        "纯公开规则文案必须 displayable 外显，逐字锁定，实际: {v}"
    );
    assert_ne!(
        msg, "请求参数验证失败",
        "不得被脱敏成 VALIDATION 族固定脱敏常量（那是兜底掩盖真实原因）"
    );

    let rows = ap_payment_request::Entity::find().all(&db).await.unwrap();
    assert!(
        rows.is_empty(),
        "拒绝必须先于事务/取号/写入动作发生，主表必须零写入（修复前此处会落一条汇率=1 的 USD 假单）"
    );
}

/// 反方向防误杀：外币带汇率的请求体在**汇率门**不得被误拒（拒绝仅由"缺汇率"触发）。
/// 真库下该请求会越过汇率门、经 pg_advisory_xact_lock 取号并真实落库（成功 2xx），
/// 无论如何都不得再回到 VALIDATION 汇率文案。
#[tokio::test]
async fn foreign_currency_with_rate_passes_fx_gate() {
    let (app, _db) = seeded_http_app().await;
    let mut body = base_body();
    body["currency"] = json!("USD");
    body["exchange_rate"] = json!("7.234500");

    let (_status, v) = call(&app, Method::POST, "/ap/payment-requests", Some(body)).await;
    assert_ne!(
        v["message"].as_str().unwrap_or_default(),
        "外币付款请填写汇率",
        "已携汇率的外币请求绝不能再落入缺汇率拒绝分支: {v}"
    );
    // 越门后的任何失败（真库下不应发生）都不得伪装成字段校验族的汇率文案
    assert!(
        v["code"] != "VALIDATION_ERROR",
        "越门后的失败不得再伪装成字段校验族: {v}"
    );
}

// =========================================================
// 契约 2：外币带汇率 → 真实 builder 落库回读 = 请求真实值
// =========================================================

async fn resolve_and_insert(json_body: Value) -> ap_payment_request::Model {
    let req: CreateApPaymentRequest = serde_json::from_value(json_body).unwrap();
    let (currency, rate) =
        ApPaymentRequestService::resolve_currency_and_rate(&req).expect("服务端权威解析不得失败");
    let db = live_db().await;
    let am = ApPaymentRequestService::build_payment_request_active_model(
        &req,
        format!("PRQ20260115{:03}", rand_suffix()),
        100,
        currency,
        rate,
    );
    am.insert(&db).await.expect("真实构建路径落库必须成功");
    ap_payment_request::Entity::find()
        .one(&db)
        .await
        .unwrap()
        .expect("落库后必须可回读")
}

fn rand_suffix() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        % 1000
}

#[tokio::test]
async fn foreign_currency_rate_persisted_verbatim_readback() {
    let mut body = base_body();
    body["currency"] = json!("USD");
    body["exchange_rate"] = json!("7.234500");

    let row = resolve_and_insert(body).await;
    assert_eq!(row.currency, "USD");
    assert_eq!(
        row.exchange_rate,
        dec("7.2345"),
        "外币落库汇率必须逐值等于请求真实值（回读断言，不许只看 status）——\
         修复前此值恒为兜底假值 1"
    );
    assert_ne!(row.exchange_rate, Decimal::ONE, "绝不允许再次塌成兜底 1");
}

// =========================================================
// 契约 3：本位币短路——传汇率/缺币种均恒 1
// =========================================================

#[tokio::test]
async fn base_currency_ignores_client_rate_and_forces_one() {
    let mut body = base_body();
    body["currency"] = json!("CNY");
    // 前端（或旁路调用）在 CNY 下仍传汇率：服务端权威忽略、恒落 1
    body["exchange_rate"] = json!("7.990000");

    let row = resolve_and_insert(body).await;
    assert_eq!(row.currency, "CNY");
    assert_eq!(
        row.exchange_rate,
        Decimal::ONE,
        "本位币落库汇率必须恒等于服务端权威值 1，与前端传值无关"
    );
}

#[test]
fn currency_absent_defaults_to_base_with_rate_one() {
    // 币种缺省：按模型 DEFAULT 'CNY' 同源口径短路为本位币 1（无第二套手写默认值）
    let req: CreateApPaymentRequest = serde_json::from_value(base_body()).unwrap();
    let (currency, rate) = ApPaymentRequestService::resolve_currency_and_rate(&req).unwrap();
    assert_eq!(currency, "CNY");
    assert_eq!(rate, Decimal::ONE);
}

#[test]
fn foreign_currency_missing_rate_rejected_with_validation_family() {
    let mut body = base_body();
    body["currency"] = json!("EUR");
    let req: CreateApPaymentRequest = serde_json::from_value(body).unwrap();
    let err = ApPaymentRequestService::resolve_currency_and_rate(&req)
        .expect_err("外币缺汇率服务层直调同样必须显式拒绝");
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert!(
        err.to_string().contains("外币付款请填写汇率"),
        "实际: {err}"
    );
}

// =========================================================
// 契约 4：源码扫描防回潮锁（include_str!，ratchet 只减不增）
// =========================================================

/// 从源码截取一个 impl 内方法块：anchor 起，到首个 "\n    }"（方法级缩进闭合）。
/// 先剔除 `\r`：Windows 工作树 CRLF 会使跨行 contains 断言漏检（先例 wave2 测试）
fn extract_impl_fn(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("方法块结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// 绝对锁：exchange_rate 落库行不得再出现任何 unwrap_or 兜底；
/// builder 不得再从 DTO 原值取汇率（必须消费服务端解析结果）。
#[test]
fn source_scan_exchange_rate_zero_unwrap_or_absolute_lock() {
    let src = include_str!("../src/services/ap_payment_request_service.rs").replace('\r', "");
    let hits: Vec<&str> = src
        .lines()
        .filter(|l| l.trim_start().starts_with("exchange_rate: Set("))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "主表 exchange_rate 落库构造点必须唯一（发现多于一处即注入口回潮），实际: {hits:?}"
    );
    assert!(
        hits[0].contains("Set(exchange_rate)"),
        "exchange_rate 必须取服务端权威解析入参，实际行: {}",
        hits[0]
    );
    let builder = extract_impl_fn(&src, "pub fn build_payment_request_active_model(");
    assert!(
        !builder.contains("unwrap_or"),
        "builder 内不得出现任何 unwrap_or 兜底，实际块:\n{builder}"
    );
    assert!(
        !builder.contains("req.exchange_rate") && !builder.contains("req.currency"),
        "builder 不得直接从 DTO 原值取币种/汇率（唯一注入口 = create 入口的解析结果），实际块:\n{builder}"
    );
}

/// ratchet（只减不增）：全文件**可执行代码**的 `.unwrap_or(` 命中数锁基线 ≤1——
/// 现存的唯一一处是 items 缺省按空切片处理（create 内既有定案口径），
/// 任何新增兜底（尤其汇率/金额类 NOT NULL 列）都会顶破基线判红。
/// 计数先剔除整行注释：`exchange_rate.unwrap_or(1)` 以反引号形态存在于修复说明
/// 注释里，把它计入命中数是对"兜底计数"锁的误报（测试自身解析缺陷，裁定 R4
/// 直接修；注释行不产生运行期兜底，剔除后锁的对象恰是真实代码路径，不放松）。
#[test]
fn source_scan_unwrap_or_ratchet_shrink_only() {
    let src = include_str!("../src/services/ap_payment_request_service.rs").replace('\r', "");
    let count = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .flat_map(|l| l.match_indices(".unwrap_or("))
        .count();
    const BASELINE: usize = 1;
    assert!(
        count <= BASELINE,
        "`.unwrap_or(` 命中数 {count} 超过基线 {BASELINE}：兜底掩盖只减不增，\
         新缺口一律条件必填/显式拒绝，禁止加默认值把缺键吞掉"
    );
}
