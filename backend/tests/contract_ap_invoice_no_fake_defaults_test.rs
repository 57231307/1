//! 手工应付票建单（ap_invoice_ops/crud.rs::create_manual）"缺键被静默折算"拆除契约锁
//!
//! 锁定的契约（file:line 以现行工作树为准）：
//! - `backend/src/services/ap_invoice_ops/crud.rs::create_manual`
//!   六要素入参（supplier_id / invoice_type / invoice_date / due_date / amount / tax_amount）
//!   一律**缺键即点名拒绝**，不再有任何代填：
//!   - invoice_type 原兜 `unwrap_or_else(|| "PURCHASE")`（ap_invoice.invoice_type 为
//!     NOT NULL 无默认列，见 m0012:22）；代填把费用类/其他类手工票冒充成采购应付；
//!   - invoice_date / due_date 原兜"今天"（m0012:25-26 NOT NULL）；代填造出"建单即到期"
//!     的应付行，账龄分析与到期提醒从建单那一刻失真；
//!   - amount 原兜 `unwrap_or(Decimal::ZERO)`（m0012:28 NOT NULL 无默认）；缺键洗 0
//!     等于凭空造一张零元应付票；
//!   - tax_amount 原兜 `unwrap_or(Decimal::ZERO)`。拒绝理由不是 DDL（m0012:35 是
//!     NOT NULL DEFAULT 0，列本身承认 0 是存量合法态），而是：本表既无 tax_percent
//!     也无免税标志，缺键洗 0 之后"没录"与"免税 / 法定 0% 税率 / 取得普票不得抵扣 /
//!     无票暂估"四种互斥来源在库里彻底不可分。**0 只能由建单方显式提交**。
//! - `unpaid_amount` 不是入参：由已校验的 amount 减去 paid_amount 派生，
//!   本入口不接收付款（付款走付款申请与付款单），手工建单时 paid_amount 恒 0。
//! - 前端同批口径（views/ap/tabs/InvoiceTab.vue）：税额初值不预置 0、应付类型与到期日期
//!   为必填项、提交面税额有值才下发（不做后端代填的前端等价物）。
//!
//! 违例形态回潮时必然红的机理（回归锁判据）：
//! - 任一缺键被改回兜底值 → 该用例的 400 断言直接变 200 而失配；
//! - 缺键拒绝被改成笼统文案（不带字段名）→ 文案逐字断言失配；
//! - 拒绝分支写在校验之后、插入之后 → 零落库断言（按表 COUNT）变红；
//! - unpaid_amount 被改回读入参或写 0 → 派生断言失配；
//! - 源码扫描：crud.rs 出现被禁的兜底表达式、前端把 undefined 预置回 0，任一命中即红。
//!
//! 覆盖策略（无 mock）：表结构唯一来源 = backend/migration ——
//! `test_common::setup_test_db()`（已迁移 PostgreSQL，TRUNCATE 业务表 + RESTART IDENTITY）
//! + 真实 handler 端到端（tower oneshot，同 contract_wave4 先例）。
//! 活体用例随全仓真库契约锁一起按 `--test-threads=1` 串行执行（共享真库夹具，并行会互相 TRUNCATE）。
//! 供应商为 ap_invoice.supplier_id 的 FK 父行，用例自种子，不依赖外部库状态。

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
use bingxi_backend::handlers::ap_invoice_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::common as common_status;
use bingxi_backend::models::{ap_invoice, supplier};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|e| panic!("夹具十进制字面量非法 {s}: {e}"))
}

fn invoice_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 4, 10).expect("夹具日期非法")
}

fn due_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 5, 10).expect("夹具日期非法")
}

// =========================================================
// HTTP 夹具（真实 handler，无 mock）
// =========================================================

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("contract_ap_user_{user_id}"),
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

fn build_app(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route("/ap/invoices", post(ap_invoice_handler::create_ap_invoice))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

async fn call_create(app: &Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/ap/invoices")
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
// 真库夹具
// =========================================================

async fn fresh_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

/// ap_invoice.supplier_id 的 FK 父行（列形态按 models/supplier.rs 逐列核对；
/// suppliers 属迁移播种的参照表、夹具不清空，故 supplier_code 用时间戳保证 UNIQUE）
async fn seed_supplier(db: &DatabaseConnection) -> i32 {
    let now = Utc::now();
    let tag = now
        .timestamp_nanos_opt()
        .expect("Utc::now 纳秒必落在 chrono 可表示区间，None 不可达");
    supplier::ActiveModel {
        supplier_code: Set(format!("APND{tag}")),
        supplier_name: Set("应付缺键契约测试供应商".to_string()),
        supplier_short_name: Set("缺键供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000APND00001X1".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期非法")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000001".to_string()),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("供应商 FK 父行插入失败: {e}"))
    .id
}

/// 六要素齐备的合法建单载荷（amount/tax_amount 按本仓十进制串口径下发）
fn base_body(supplier_id: i32) -> Value {
    json!({
        "supplier_id": supplier_id,
        "invoice_type": "PURCHASE",
        "invoice_date": invoice_date().to_string(),
        "due_date": due_date().to_string(),
        "amount": "1000.00",
        "tax_amount": "130.00",
    })
}

/// 缺一个必键 → 400 + 文案点名该字段 + 整表零落库
async fn assert_missing_key_rejected(key: &str, expected_message: &str) {
    let db = fresh_db().await;
    let supplier_id = seed_supplier(&db).await;
    let read_db = db.clone();
    let app = build_app(db);

    let mut body = base_body(supplier_id);
    let removed = body
        .as_object_mut()
        .expect("基础载荷必须是对象")
        .remove(key);
    assert!(removed.is_some(), "基础夹具应包含必键 {key}");

    let (status, v) = call_create(&app, body).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺 {key} 必须点名拒绝（400），不得静默代填后建单，实际: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "实际响应: {v}");
    assert_eq!(
        v["message"], expected_message,
        "拒绝文案必须点名缺的是哪个字段（并给出补法），实际: {v}"
    );

    let rows = ap_invoice::Entity::find().count(&read_db).await.unwrap();
    assert_eq!(
        rows, 0,
        "被拒绝的建单必须零落库（修复前该形状会静默写入一行兜底值票）"
    );
}

/// 按主键回读库里终态行
async fn reload(db: &DatabaseConnection, id: i32) -> ap_invoice::Model {
    ap_invoice::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("建单成功但按主键 {id} 查不到落库行"))
}

fn created_id(v: &Value) -> i32 {
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("成功响应应回显新行 id，实际: {v}")) as i32
}

// =========================================================
// 1) 缺键点名拒绝（四组入参 + 零落库）
// =========================================================

#[tokio::test]
async fn missing_invoice_type_rejected_not_backfilled_purchase() {
    assert_missing_key_rejected("invoice_type", "应付单缺少应付类型").await;
}

#[tokio::test]
async fn missing_invoice_date_rejected_not_backfilled_today() {
    assert_missing_key_rejected("invoice_date", "应付单缺少发票日期").await;
}

#[tokio::test]
async fn missing_due_date_rejected_not_backfilled_today() {
    assert_missing_key_rejected("due_date", "应付单缺少到期日期").await;
}

#[tokio::test]
async fn missing_amount_rejected_not_washed_to_zero() {
    assert_missing_key_rejected("amount", "应付单缺少发票金额").await;
}

#[tokio::test]
async fn missing_tax_amount_rejected_and_tells_how_to_declare_no_input_tax() {
    assert_missing_key_rejected("tax_amount", "应付单缺少税额，无进项税请显式提交 0").await;
}

// =========================================================
// 2) 显式 0 是合法声明值：可建成且逐列回读相等
// =========================================================

#[tokio::test]
async fn explicitly_declared_zero_tax_persists_every_column_as_submitted() {
    let db = fresh_db().await;
    let supplier_id = seed_supplier(&db).await;
    let read_db = db.clone();
    let app = build_app(db);

    let mut body = base_body(supplier_id);
    body["tax_amount"] = json!("0.00");
    let (status, v) = call_create(&app, body).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "显式提交 0 税额（建单方声明本票无进项税）必须放行，实际: {v}"
    );

    let row = reload(&read_db, created_id(&v)).await;
    assert_eq!(row.supplier_id, supplier_id);
    assert_eq!(
        row.invoice_type, "PURCHASE",
        "应付类型取提交值，不是兜底字面量"
    );
    assert_eq!(row.invoice_date, invoice_date());
    assert_eq!(row.due_date, due_date());
    assert_eq!(row.amount, dec("1000.00"));
    assert_eq!(
        row.tax_amount,
        Decimal::ZERO,
        "显式声明的 0 必须原样落库，既不被改写也不与缺键混淆"
    );
    assert_eq!(row.paid_amount, Decimal::ZERO, "手工建单入口不带付款");
    assert_eq!(row.unpaid_amount, dec("1000.00"));
    assert_eq!(row.source_type.as_deref(), Some("MANUAL"));
    assert_eq!(row.invoice_status, common_status::STATUS_DRAFT);
}

#[tokio::test]
async fn submitted_nonzero_tax_persists_exact_value_and_echoes_row() {
    let db = fresh_db().await;
    let supplier_id = seed_supplier(&db).await;
    let read_db = db.clone();
    let app = build_app(db);

    let (status, v) = call_create(&app, base_body(supplier_id)).await;
    assert_eq!(status, StatusCode::OK, "六要素齐备应建单成功，实际: {v}");
    assert_eq!(
        v["data"]["invoice_status"],
        common_status::STATUS_DRAFT,
        "出参应回显落库行，实际: {v}"
    );

    let row = reload(&read_db, created_id(&v)).await;
    assert_eq!(row.tax_amount, dec("130.00"), "非零税额必须逐分回读相等");
    assert_eq!(row.amount, dec("1000.00"));
}

// =========================================================
// 3) unpaid_amount 由 amount 派生，不是入参
// =========================================================

#[tokio::test]
async fn unpaid_amount_is_derived_from_amount_and_ignores_request_key() {
    let db = fresh_db().await;
    let supplier_id = seed_supplier(&db).await;
    let read_db = db.clone();
    let app = build_app(db);

    let mut body = base_body(supplier_id);
    body["amount"] = json!("100.00");
    // 造一个同名键：CreateApInvoiceRequest 从不声明 unpaid_amount，
    // 库里 unpaid 必须仍等于 amount，不受请求体这个键影响
    body["unpaid_amount"] = json!("0.00");
    let (status, v) = call_create(&app, body).await;
    assert_eq!(status, StatusCode::OK, "合法载荷应建单成功，实际: {v}");

    let row = reload(&read_db, created_id(&v)).await;
    assert_eq!(row.amount, dec("100.00"));
    assert_eq!(
        row.unpaid_amount,
        dec("100.00"),
        "unpaid_amount 由 amount 减 paid_amount（本入口恒 0）派生，不是入参"
    );
    assert_eq!(row.paid_amount, Decimal::ZERO);
}

// =========================================================
// 4) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层函数块：anchor 起，到首个 "\n}"。
/// 先剔除 `\r`：Windows 工作树 CRLF 会使跨行 contains 断言漏检（先例 wave4 测试）
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
fn source_scan_create_manual_has_no_silent_backfill() {
    let src = include_str!("../src/services/ap_invoice_ops/crud.rs");
    let block = extract_block(src, "pub async fn create_manual");

    for token in [
        "unwrap_or_else(|| \"PURCHASE\"",
        "unwrap_or(Decimal::ZERO)",
        "unwrap_or_else(|| chrono::Utc::now().date_naive())",
    ] {
        assert!(
            !block.contains(token),
            "create_manual 禁止再出现静默代填表达式 {token}，实际块:\n{block}"
        );
    }
    for message in [
        "应付单缺少应付类型",
        "应付单缺少发票日期",
        "应付单缺少到期日期",
        "应付单缺少发票金额",
        "应付单缺少税额",
    ] {
        assert!(
            block.contains(message),
            "create_manual 必须对每个必键点名拒绝（缺 {message}），实际块:\n{block}"
        );
    }
    assert!(
        block.contains("无进项税请显式提交 0"),
        "税额拒绝文案必须给出补法（显式提交 0），否则前端与调用方无从知道 0 是合法声明值，实际块:\n{block}"
    );
    assert!(
        block.contains("unpaid_amount: Set(amount - paid_amount)"),
        "unpaid_amount 必须由已校验 amount 派生，不得回潮为独立入参或兜底 0，实际块:\n{block}"
    );
}

#[test]
fn source_scan_manual_invoice_form_declares_keys_instead_of_prefilling_zero() {
    let vue = include_str!("../../frontend/src/views/ap/tabs/InvoiceTab.vue");
    let flat = vue.replace('\r', "");

    assert!(
        !flat.contains("tax_amount: 0"),
        "税额初值不得预置 0：预置等于替建单人声明\"本票无进项税\""
    );
    assert!(
        flat.contains("tax_amount: undefined"),
        "税额初值必须留空（undefined），0 只能由人工显式敲出"
    );
    assert!(
        flat.contains("prop=\"tax_amount\""),
        "税额必须挂在带 prop 的表单项上，必填规则才会真的拦"
    );
    for key in [
        "apModule.invoice.invoiceTypeRequired",
        "apModule.invoice.dueDateRequired",
        "apModule.invoice.taxAmountRequired",
    ] {
        assert!(
            flat.contains(key),
            "必填规则文案键缺失（后端已缺键即拒，前端不得继续造缺键载荷）: {key}"
        );
    }
    assert!(
        !flat.contains("tax_amount: invoiceForm.tax_amount.toFixed(2)"),
        "税额不得无条件 toFixed：对未填写取值调 toFixed 会送出 \"NaN\"，等价于另一种代填"
    );
    assert!(
        flat.contains("invoice_type: invoiceForm.invoice_type"),
        "应付类型必须由表单采集并下发，不得再靠后端兜 \"PURCHASE\""
    );
}
