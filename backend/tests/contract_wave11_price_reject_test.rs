//! 价格域审批"两动作两理由"真库契约锁（后端线）
//!
//! 已定案口径（本文件逐条钉死，锁口径见各用例头注释）：
//! - 审批 = 两条动作两条理由：approve 端点只处理批准且**通过理由本域必填**；
//!   拒绝走独立 `/reject` 端点、**拒绝理由全域必填**（trim 非空）并落库。
//! - `approved=false` 在 approve 端点继续 400（端点单一职责），拒绝动作不得混入。
//! - 状态门：拒绝只允许 pending 起拒；rejected 为审批结论终态，
//!   不出 rejected→pending/approved 的回退边（再 approve/再 reject 均被门拦，
//!   BUSINESS_ERROR + 400，本文件只断机器码与状态码、不断文案原文——
//!   本仓业务拒绝文案永久脱敏，口径与 `contract_wave8_price_approve_gate_test` 一致）。
//! - 可外显校验文案定性、不含记录 ID（`utils/error.rs` 安全边界）。
//! - 列表筛选白名单同步扩 rejected（sales 分表钉三值、purchase 走 `price_approval::ALL`）。
//! - `approve_price` 服务层真实落 `approval_reason` 列（不再只进日志）。
//!
//! 夹具形态照 `contract_wave8_price_approve_gate_test.rs`：真库 `setup_test_db`
//! （已迁移 PG、nextest 串行 db-integration 组），FK 前提同上——价目种子先种真实
//! 父行（products 逐例 TRUNCATE 可显式固定 id；suppliers 属封存参照表须自增回读）。
//! 路由仅在测试 Router 内注册（src routes 为枢纽文件，注册动作由主智能体落地），
//! 路径形态与将注册的真实路由逐字符一致（`/{id}/reject`、`/{id}/approve`）。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{purchase_price_handler, sales_price_handler};
use bingxi_backend::models::status::price_approval;
use bingxi_backend::models::{product, purchase_price, sales_price, supplier, user};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9441;
/// 逐例种入的真实产品父行主键（products 属业务表、夹具逐例 TRUNCATE 后可显式固定 id）
const SEED_PRODUCT_ID: i32 = 9451;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w11_price_reject".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("价格拒绝契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

/// 种子真实产品父行（形状照 `contract_wave8_price_approve_gate_test::seed_product`）
async fn seed_product(db: &Arc<DatabaseConnection>, id: i32, name: &str, code: &str) {
    product::ActiveModel {
        id: Set(id),
        name: Set(name.to_string()),
        code: Set(code.to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子产品 {id} 插入失败: {e}"));
}

/// 种子真实供应商父行：suppliers 属封存参照表不被清空，显式 id 会跨例撞主键，
/// 自增插入后回读真实主键（形状照 wave8 同名夹具）
async fn seed_supplier(db: &Arc<DatabaseConnection>, code: &str) -> i32 {
    let ts = supplier::ActiveModel {
        supplier_code: Set(code.to_string()),
        supplier_name: Set("价格拒绝契约锁供应商".to_string()),
        supplier_short_name: Set("拒供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(rust_decimal::Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    ts.id
}

/// 直接落一行销售价目（状态夹具可控；列逐一对 models/sales_price.rs 核对，
/// approval_reason/rejected_reason 由 ..Default::default() 落 NULL，行起点为空白痕迹）
async fn seed_sales_price(db: &Arc<DatabaseConnection>, status: &str) -> sales_price::Model {
    sales_price::ActiveModel {
        product_id: Set(SEED_PRODUCT_ID),
        customer_id: Set(None),
        customer_type: Set(Some("standard".to_string())),
        price: Set(rust_decimal::Decimal::new(1234, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(rust_decimal::Decimal::ZERO),
        price_type: Set("standard".to_string()),
        price_level: Set(None),
        effective_date: Set(Utc::now().date_naive()),
        expiry_date: Set(None),
        status: Set(status.to_string()),
        approved_by: Set(None),
        approved_at: Set(None),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售价目（{status}）插入失败: {e}"))
}

/// 直接落一行采购价目（FK 前提：product_id/supplier_id 引用先种好的真实父行）
async fn seed_purchase_price(
    db: &Arc<DatabaseConnection>,
    status: &str,
    supplier_id: i32,
) -> purchase_price::Model {
    purchase_price::ActiveModel {
        product_id: Set(SEED_PRODUCT_ID),
        supplier_id: Set(supplier_id),
        price: Set(rust_decimal::Decimal::new(1234, 2)),
        currency: Set("CNY".to_string()),
        unit: Set("米".to_string()),
        min_order_qty: Set(rust_decimal::Decimal::ZERO),
        price_type: Set("standard".to_string()),
        effective_date: Set(Utc::now().date_naive()),
        expiry_date: Set(None),
        status: Set(status.to_string()),
        approved_by: Set(None),
        approved_at: Set(None),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子采购价目（{status}）插入失败: {e}"))
}

/// 单域某行的 (approve, reject, detail) 端点三元组（路径与 src routes 将注册形态一致）
fn sales_eps(id: i32) -> (String, String, String) {
    (
        format!("/sales/sales-prices/{id}/approve"),
        format!("/sales/sales-prices/{id}/reject"),
        format!("/sales/sales-prices/{id}"),
    )
}

fn purchase_eps(id: i32) -> (String, String, String) {
    (
        format!("/purchase/purchase-prices/{id}/approve"),
        format!("/purchase/purchase-prices/{id}/reject"),
        format!("/purchase/purchase-prices/{id}"),
    )
}

/// 按域名派发端点三元组（跨域循环里行模型类型不同，只传行 id 与域名）
fn eps(domain: &str, id: i32) -> (String, String, String) {
    match domain {
        "sales" => sales_eps(id),
        "purchase" => purchase_eps(id),
        other => panic!("契约锁出现未知域 {other}"),
    }
}

/// HTTP 形态夹具：approve/reject/详情/列表四端点 × 两域；auth 注入与空串剔除
/// 中间件照 wave8 先例（列表 trim 语义需外层 normalize_empty_query_params）。
async fn price_reject_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<bingxi_backend::middleware::auth_context::AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = bingxi_backend::middleware::auth_context::AuthContext {
        user_id: OPERATOR_ID,
        username: "w11_price_reject".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = axum::Router::new()
        .route(
            "/sales/sales-prices/{id}/approve",
            axum::routing::post(sales_price_handler::approve_price),
        )
        .route(
            "/sales/sales-prices/{id}/reject",
            axum::routing::post(sales_price_handler::reject_price),
        )
        .route(
            "/sales/sales-prices/{id}",
            axum::routing::get(sales_price_handler::get_price),
        )
        .route(
            "/sales/sales-prices",
            axum::routing::get(sales_price_handler::list_prices),
        )
        .route(
            "/purchase/purchase-prices/{id}/approve",
            axum::routing::post(purchase_price_handler::approve_price),
        )
        .route(
            "/purchase/purchase-prices/{id}/reject",
            axum::routing::post(purchase_price_handler::reject_price),
        )
        .route(
            "/purchase/purchase-prices/{id}",
            axum::routing::get(purchase_price_handler::get_price),
        )
        .route(
            "/purchase/purchase-prices",
            axum::routing::get(purchase_price_handler::list_prices),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .layer(axum::middleware::from_fn(
            bingxi_backend::utils::query_params::normalize_empty_query_params,
        ));
    (db, app)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(body.to_string()))
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

/// 无 body 的 POST（Option<Json<T>> 形态专用：不带 content-type、零长度 body）
async fn post_empty_body(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .body(axum::body::Body::empty())
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

async fn get_json(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let v: Value = serde_json::from_slice(&bytes).expect("响应必须是 JSON 统一信封");
    (status, v)
}

// ---------------------------------------------------------------------------
// 1. 拒绝理由空串/纯空白 ⇒ 400 + VALIDATION_ERROR，且拒绝路径零副作用（两域）。
//    改坏什么必红：摘掉 handler 的 trim 必填门 ⇒ 空白理由直达服务层落库，本例红；
//    把校验降成日志放行（静默兜底）⇒ 400 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn reject_blank_reason_is_400_validation_and_changes_nothing() {
    let (db, app) = price_reject_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "拒绝锁面料甲", "PRD-W11RJ-T1").await;
    let supplier_id = seed_supplier(&db, "SUP-W11RJ-T1").await;
    let sales = seed_sales_price(&db, price_approval::PENDING).await;
    let purchase = seed_purchase_price(&db, price_approval::PENDING, supplier_id).await;

    let cases = [
        ("sales", sales_eps(sales.id)),
        ("purchase", purchase_eps(purchase.id)),
    ];
    for (domain, (_, reject_uri, detail_uri)) in &cases {
        for blank in ["", "   ", "\t  "] {
            let (status, v) = post_json(&app, reject_uri, json!({ "reason": blank })).await;
            assert_eq!(
                status,
                axum::http::StatusCode::BAD_REQUEST,
                "{domain} 拒绝理由 {blank:?} 必须 400，实得 {v}"
            );
            assert_eq!(
                v["code"], "VALIDATION_ERROR",
                "{domain} 拒绝理由必填必须归 VALIDATION_ERROR 信封，实得 {v}"
            );
        }
        // 拒绝路径零副作用：仍 pending、rejected_reason 保持空白起点
        let (status, detail) = get_json(&app, detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            price_approval::PENDING,
            "{domain} 被拒拒绝不得改状态"
        );
        assert_eq!(
            detail["data"]["rejected_reason"],
            Value::Null,
            "{domain} 被拒拒绝不得落 rejected_reason"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 通过理由缺失（不带 body / approved=true 缺键 / 空串 / 纯空白）⇒ 400 +
//    VALIDATION_ERROR（本域必填档）；approved=false 继续 400（端点单一职责）。
//    改坏什么必红：把入参改回强类型 Json<T> ⇒ 不带 body 调用落到解码层裸 400
//    文本（无 VALIDATION_ERROR 信封），形态①断言红；摘掉必填门 ⇒ 空理由直写库，红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_missing_or_blank_reason_is_400_validation() {
    let (db, app) = price_reject_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "拒绝锁面料甲", "PRD-W11RJ-T2").await;
    let supplier_id = seed_supplier(&db, "SUP-W11RJ-T2").await;
    let sales = seed_sales_price(&db, price_approval::PENDING).await;
    let purchase = seed_purchase_price(&db, price_approval::PENDING, supplier_id).await;

    let cases = [
        ("sales", sales_eps(sales.id)),
        ("purchase", purchase_eps(purchase.id)),
    ];
    for (domain, (approve_uri, _, detail_uri)) in &cases {
        // 形态①：完全不带 body——Option<Json<T>> 必须给出统一 AppError 信封
        let (status, v) = post_empty_body(&app, approve_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} 无 body 批准必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "{domain} 无 body 批准信封必须 VALIDATION_ERROR（解码层裸文本即形态退化），实得 {v}"
        );

        // 形态②：approved=true 但通过理由缺键 / 空串 / 纯空白
        for body in [
            json!({ "approved": true }),
            json!({ "approved": true, "approval_reason": "" }),
            json!({ "approved": true, "approval_reason": "   " }),
        ] {
            let (status, v) = post_json(&app, approve_uri, body).await;
            assert_eq!(
                status,
                axum::http::StatusCode::BAD_REQUEST,
                "{domain} 通过理由缺失形态必须 400，实得 {v}"
            );
            assert_eq!(
                v["code"], "VALIDATION_ERROR",
                "{domain} 通过理由必填信封必须 VALIDATION_ERROR，实得 {v}"
            );
        }

        // 形态③：approved=false——端点单一职责，继续 400（拒绝动作走 /reject）
        let (status, v) = post_json(
            &app,
            approve_uri,
            json!({ "approved": false, "approval_reason": "不通过" }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} approved=false 在 approve 端点必须 400（单一职责），实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "{domain} approved=false 拒绝信封必须 VALIDATION_ERROR，实得 {v}"
        );

        // 全部拒绝形态零副作用：行仍 pending、approval_reason 不动
        let (status, detail) = get_json(&app, detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            price_approval::PENDING,
            "{domain} 被拒批准不得改状态"
        );
        assert_eq!(
            detail["data"]["approval_reason"],
            Value::Null,
            "{domain} 被拒批准不得落 approval_reason"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. 权威流转逐字回读（两域各两行：pending→rejected 与 pending→approved）：
//    - rejected：状态落值、rejected_reason 逐字（trim 后原样）、决策人/决策时间同步，
//      approval_reason 保持 NULL（两动作两理由，列不互写）；
//    - approved：approval_reason 真实落列回读（不再只进日志），rejected_reason NULL。
//    改坏什么必红：拒绝理由只进日志不落库 ⇒ 逐字回读红；approve 未落理由列 ⇒ 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn authoritative_transitions_persist_reasons_verbatim_both_domains() {
    let (db, app) = price_reject_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "拒绝锁面料甲", "PRD-W11RJ-T3").await;
    let supplier_id = seed_supplier(&db, "SUP-W11RJ-T3").await;

    // 带前后空白的理由：落库必须是 trim 后的值（必填门与落库值同源）
    let raw_reject = "  报价高于同期市场基准，且客户资信不足，不予通过  ";
    let raw_approve = "  符合行情与资信，予以通过  ";

    let cases = [
        (
            "sales",
            seed_sales_price(&db, price_approval::PENDING).await.id,
            seed_sales_price(&db, price_approval::PENDING).await.id,
        ),
        (
            "purchase",
            seed_purchase_price(&db, price_approval::PENDING, supplier_id)
                .await
                .id,
            seed_purchase_price(&db, price_approval::PENDING, supplier_id)
                .await
                .id,
        ),
    ];
    for (domain, rejected_id, approved_id) in &cases {
        // —— pending → rejected（拒绝权威路径）——
        let (_, reject_uri, detail_uri) = eps(domain, *rejected_id);
        let (status, v) = post_json(&app, &reject_uri, json!({ "reason": raw_reject })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} pending→rejected 权威路径必须 200，实得 {v}"
        );
        assert_eq!(v["code"], 200, "{domain} 成功信封 code 必须 200，实得 {v}");
        let (status, detail) = get_json(&app, &detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            price_approval::REJECTED,
            "{domain} 拒绝成功必须落 rejected 态"
        );
        assert_eq!(
            detail["data"]["rejected_reason"].as_str(),
            Some(raw_reject.trim()),
            "{domain} rejected_reason 必须逐字回读（trim 后原样，禁止只进日志）"
        );
        assert_eq!(
            detail["data"]["approved_by"].as_i64(),
            Some(OPERATOR_ID as i64),
            "{domain} 拒绝结论必须记录决策人"
        );
        assert_ne!(
            detail["data"]["approved_at"],
            Value::Null,
            "{domain} 拒绝结论必须记录决策时间"
        );
        assert_eq!(
            detail["data"]["approval_reason"],
            Value::Null,
            "{domain} 两动作两理由：拒绝不得写 approval_reason 列"
        );

        // —— pending → approved（批准权威路径，理由真实落列）——
        let (approve_uri, _, detail_uri) = eps(domain, *approved_id);
        let (status, v) = post_json(
            &app,
            &approve_uri,
            json!({ "approved": true, "approval_reason": raw_approve }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} pending→approved 权威路径必须 200，实得 {v}"
        );
        let (status, detail) = get_json(&app, &detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            price_approval::APPROVED,
            "{domain} 批准成功必须落 approved 态"
        );
        assert_eq!(
            detail["data"]["approval_reason"].as_str(),
            Some(raw_approve.trim()),
            "{domain} approval_reason 必须逐字落库回读（不再只进日志）"
        );
        assert_eq!(
            detail["data"]["rejected_reason"],
            Value::Null,
            "{domain} 两动作两理由：批准不得写 rejected_reason 列"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. rejected 终态双向拦截：rejected 行再 approve、再 reject 均被状态门拦
//    （400 + BUSINESS_ERROR，断机器码不断文案），行零变化——不出 rejected→
//    pending/approved 的回退边。
//    改坏什么必红：门只拦 approve ⇒ reject 段红；门回潮放行 ⇒ 400 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rejected_row_is_terminal_for_both_approve_and_reject() {
    let (db, app) = price_reject_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "拒绝锁面料甲", "PRD-W11RJ-T4").await;
    let supplier_id = seed_supplier(&db, "SUP-W11RJ-T4").await;
    let sales = seed_sales_price(&db, price_approval::PENDING).await;
    let purchase = seed_purchase_price(&db, price_approval::PENDING, supplier_id).await;

    let cases = [
        ("sales", sales_eps(sales.id)),
        ("purchase", purchase_eps(purchase.id)),
    ];
    for (domain, (approve_uri, reject_uri, detail_uri)) in &cases {
        // 先走权威拒绝路径进入 rejected 终态
        let (status, v) = post_json(&app, reject_uri, json!({ "reason": "首轮拒绝成立" })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} 前置：pending→rejected 必须成功，实得 {v}"
        );

        // 再批准：状态门拦（理由给满，排除必填档干扰，只测门）
        let (status, v) = post_json(
            &app,
            approve_uri,
            json!({ "approved": true, "approval_reason": "终态后重试批准" }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} rejected 行再批准必须 400（终态无回退边），实得 {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{domain} 批准状态门必须归 BUSINESS_ERROR（只断机器码不断文案），实得 {v}"
        );

        // 再拒绝：同样被门拦
        let (status, v) = post_json(&app, reject_uri, json!({ "reason": "终态后重试拒绝" })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} rejected 行再拒绝必须 400（终态无回退边），实得 {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{domain} 拒绝状态门必须归 BUSINESS_ERROR，实得 {v}"
        );

        // 零副作用：状态与首轮拒绝理由逐字保持
        let (status, detail) = get_json(&app, detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            price_approval::REJECTED,
            "{domain} 被拒的终态流转不得改动作后状态"
        );
        assert_eq!(
            detail["data"]["rejected_reason"].as_str(),
            Some("首轮拒绝成立"),
            "{domain} 被拒的终态流转不得覆写 rejected_reason"
        );
        assert_eq!(
            detail["data"]["approval_reason"],
            Value::Null,
            "{domain} 被拒的再批准不得落 approval_reason"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. 列表按 status=rejected 可筛出（sales 分表钉白名单补 rejected；purchase 走
//    price_approval::ALL 同源放行），命中行状态逐字符一致、他态行不外溢。
//    改坏什么必红：sales 白名单不补 rejected ⇒ 200 放行断言红（假筛选老路回潮）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn list_filter_status_rejected_returns_matching_rows_both_domains() {
    let (db, app) = price_reject_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "拒绝锁面料甲", "PRD-W11RJ-T5").await;
    let supplier_id = seed_supplier(&db, "SUP-W11RJ-T5").await;
    let sales_rejected = seed_sales_price(&db, price_approval::REJECTED).await;
    let _sales_pending = seed_sales_price(&db, price_approval::PENDING).await;
    let purchase_rejected = seed_purchase_price(&db, price_approval::REJECTED, supplier_id).await;
    let _purchase_pending = seed_purchase_price(&db, price_approval::PENDING, supplier_id).await;

    for (uri, expect_id, domain) in [
        (
            "/sales/sales-prices?status=rejected",
            sales_rejected.id,
            "sales",
        ),
        (
            "/purchase/purchase-prices?status=rejected",
            purchase_rejected.id,
            "purchase",
        ),
    ] {
        let (status, v) = get_json(&app, uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} 列表按 rejected 筛选必须放行 200（白名单未同步即 400 回潮），实得 {v}"
        );
        assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
        let rows = v["data"].as_array().expect("列表 data 必须是数组");
        let hit: Vec<&Value> = rows
            .iter()
            .filter(|r| r["id"].as_i64() == Some(expect_id as i64))
            .collect();
        assert_eq!(
            hit.len(),
            1,
            "{domain} rejected 筛选必须精确命中自建行 {expect_id}"
        );
        assert!(
            rows.iter()
                .all(|r| r["status"].as_str() == Some(price_approval::REJECTED)),
            "{domain} rejected 筛选结果不得混入他态行（等值谓词失守即筛选口径漂移），实得 {v}"
        );
    }
}
