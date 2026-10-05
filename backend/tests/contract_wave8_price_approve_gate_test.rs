//! 价格域批次 C-1/C-2 —— 审批状态门 + 列表筛选 400 化 真库契约锁（后端线）
//!
//! 被测事实（两项回归锁，由本文件断言逐条钉死）：
//! - C-1（D7）：`purchase_price_service::approve_price` 原先**没有**"仅 pending 可审批"
//!   的状态门且不写 `approved_at` ⇒ pending/approved/inactive 都能被反复直批成
//!   approved（旁路 UI 流转），与销售侧 `sales_price_service::approve_price`（:153 有门、
//!   :164 落 approved_at）自相矛盾。
//! - C-2（R3）：两列表端点原先把 `status` 筛选参数原样下推成 SQL 等值条件，越界值
//!   恒零命中并静默返回 200 + 空列表，把拼写错误伪装成"没有数据"，与采购写入侧
//!   （`update_price` 非法 status → 400 + VALIDATION_ERROR）矛盾。
//!
//! 收紧后的口径（本文件逐条钉死）：
//! - 审批门归 **BUSINESS 族**（HTTP 400 + `code=BUSINESS_ERROR`，与销售侧同族；
//!   与 traversal/41-approve-endpoints 矩阵 `REGISTERED_REJECT_CODES` 兼容）。
//!   本仓业务拒绝文案永久脱敏，故本文件**只断 status 与信封机器码，不断文案原文**。
//! - 列表筛选白名单**分表钉、绝不取并集**：sales = {pending, approved, rejected}
//!   （与扩 rejected 后的 `chk_sales_price_status` 同源；rejected 由 m0080 扩集后继
//!   纳入，销售拒绝端点为其唯一写入方），purchase = `price_approval::ALL`
//!   {pending, approved, rejected, inactive}（与 `chk_purchase_price_status` 全等）。
//!   `inactive` 在 purchase 侧必须放行、在 sales 侧必须 400 —— 任何"两侧共用一个
//!   并集白名单"的实现必被第 4/5 例抓住。
//! - 空串/缺省 = 不加筛选（trim 语义，照 greige_fabric 先例）；越界 = 400 +
//!   `VALIDATION_ERROR`。
//! - 真库夹具（`setup_test_db`，已迁移 PostgreSQL），种子自建自清（夹具逐例 TRUNCATE）。
//! - FK 前提（2026-10 适配，非本批断言对象）：`migration/src/domain/price_fk/mod.rs` 已给
//!   `sales_prices.product_id/customer_id`、`purchase_prices.product_id/supplier_id` 施加外键，
//!   故价目种子**必须先种真实父行**（products 属业务表逐例清空、可显式固定 id；suppliers 属
//!   `SEALED_REFERENCE_TABLES` 不被清空、显式 id 会跨例撞主键，须自增回读——形状照
//!   `contract_wave8_price_ref_existence_test.rs::seed_product/seed_supplier`）。

mod test_common;

use bingxi_backend::models::purchase_price;
use bingxi_backend::models::sales_price;
use bingxi_backend::models::status::price_approval;
use bingxi_backend::models::{product, supplier, user};
use bingxi_backend::services::purchase_price_service::PurchasePriceService;
use bingxi_backend::utils::error::AppError;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9431;
/// 审批通过理由（夹具文案）：`approve_price` 现真实落 `approval_reason` 列，
/// 服务层入参为必填 String（必填档在 handler 收口，本锁钉落库回读）。
const APPROVAL_REASON: &str = "通过：价格符合市场行情与客户资信";
/// 逐例种入的**真实**产品父行主键（products 是业务表、夹具逐例 TRUNCATE 后可显式固定 id）
const SEED_PRODUCT_ID: i32 = 9401;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w8_price_gate".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("价格审批门操作人".to_string())),
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

/// 种子真实产品父行（purchase_prices/sales_prices 的 product_id FK 参照对象；
/// 形状照 `contract_wave8_price_ref_existence_test.rs::seed_product`）
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

/// 种子真实供应商父行：`suppliers` 是迁移种子参照表、夹具不清空 ⇒ 不指定显式 id，
/// 自增插入后回读真实主键给采购价目种子使用（形状照
/// `contract_wave8_price_ref_existence_test.rs::seed_supplier`，列对 models/supplier.rs
/// NOT NULL 列逐一核对）
async fn seed_supplier(db: &Arc<DatabaseConnection>, code: &str) -> i32 {
    let ts = supplier::ActiveModel {
        supplier_code: Set(code.to_string()),
        supplier_name: Set("价格审批门契约锁供应商".to_string()),
        supplier_short_name: Set("审供".to_string()),
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
        // is_processor 为 NOT NULL bool 列（models/supplier.rs），显式给值不赌库端默认
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    ts.id
}

/// 直接落一行采购价目（绕开服务口，状态夹具可控；列逐一对 models/purchase_price.rs 核对。
/// FK 前提：product_id/supplier_id 必须引用先种好的**真实**父行，否则撞 23503）
async fn seed_purchase_price(
    db: &Arc<DatabaseConnection>,
    status: &str,
    product_id: i32,
    supplier_id: i32,
) -> purchase_price::Model {
    purchase_price::ActiveModel {
        product_id: Set(product_id),
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

/// 直接落一行销售价目（列逐一对 models/sales_price.rs 核对。FK 前提：product_id 必须引用
/// 先种好的**真实**产品父行；customer_id 置 NULL 是标准价合法语义，FK 天然允许 NULL）
async fn seed_sales_price(
    db: &Arc<DatabaseConnection>,
    status: &str,
    product_id: i32,
) -> sales_price::Model {
    sales_price::ActiveModel {
        product_id: Set(product_id),
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

async fn reload_purchase(db: &Arc<DatabaseConnection>, id: i32) -> purchase_price::Model {
    purchase_price::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("采购价目 {id} 应已落库"))
}

/// 断言"审批状态门 = BUSINESS 族、HTTP 400"（41 矩阵 REGISTERED_REJECT_CODES 兼容口径）。
/// 业务拒绝文案在本仓永久脱敏，只钉机器码与状态码，不断文案原文。
fn expect_business_gate(err: &AppError) {
    assert_eq!(
        err.error_code(),
        "BUSINESS_ERROR",
        "状态门拒绝必须归 BUSINESS_ERROR（对齐销售侧 sales_price_service.rs:155），实得 {err:?}"
    );
    let resp = err.to_response();
    assert_eq!(
        resp.code, "BUSINESS_ERROR",
        "出参信封 code 必须是 BUSINESS_ERROR，实得: {resp:?}"
    );
    use axum::response::IntoResponse;
    let http = err.clone().into_response();
    assert_eq!(
        http.status(),
        axum::http::StatusCode::BAD_REQUEST,
        "状态门必须是 400，禁止退化为 500/其它状态"
    );
}

/// 列表端点 HTTP 形态夹具：照邻近 transfer 锁的 inject_auth 写法，
/// 并在更外层挂真实链路上同样注册的 `normalize_empty_query_params`
/// （否则裸测试路由收不到"空串=剔除"语义，验证不了 trim 分支）。
async fn price_list_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let state = bingxi_backend::container::AppState {
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
        username: "w8_price_gate".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = axum::Router::new()
        .route(
            "/purchase/purchase-prices",
            axum::routing::get(bingxi_backend::handlers::purchase_price_handler::list_prices),
        )
        .route(
            "/sales/sales-prices",
            axum::routing::get(bingxi_backend::handlers::sales_price_handler::list_prices),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .layer(axum::middleware::from_fn(
            bingxi_backend::utils::query_params::normalize_empty_query_params,
        ));
    (db, app)
}

async fn get_json(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, serde_json::Value) {
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
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).expect("列表响应必须是 JSON 统一信封");
    (status, v)
}

// ---------------------------------------------------------------------------
// 1. C-1①：非 pending（approved / inactive）审批必拒，BUSINESS_ERROR + 400，且零变化
//    改坏什么必红：把 approve_price 的状态门摘掉（改回直接 Set）→ 本例红；
//    把门换成 500/裸 unwrap → 状态码断言红。
//    inactive 直批正是核查报告 D7 点名的旁路：本例把它钉死。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_non_pending_purchase_price_is_business_error_and_changes_nothing() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "审批门面料甲", "PRD-W8GATE-T1").await;
    let supplier_id = seed_supplier(&db, "SUP-W8GATE-T1").await;
    let svc = PurchasePriceService::new(db.clone());

    for illegal in [price_approval::APPROVED, price_approval::INACTIVE] {
        let row = seed_purchase_price(&db, illegal, SEED_PRODUCT_ID, supplier_id).await;
        let err = svc
            .approve_price(row.id, OPERATOR_ID, APPROVAL_REASON.to_string())
            .await
            .expect_err(&format!(
                "{illegal} 状态不得被 approve_price 直批（状态机前置未满足）"
            ));
        expect_business_gate(&err);

        // 拒绝路径零副作用：状态、审批人、approved_at 都必须保持原值
        let after = reload_purchase(&db, row.id).await;
        assert_eq!(after.status, illegal, "被拒的审批不得改动作前状态");
        assert_eq!(after.approved_by, None, "被拒的审批不得落审批人");
        assert!(
            after.approved_at.is_none(),
            "被拒的审批不得落 approved_at（时间戳只归成功审批）"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. C-1②：pending → approved 成功，且 approved_at 落值（列存在性见报告 §3）
//    改坏什么必红：删掉 approved_at 的 Set（回到"不写时间戳"）→ 本例红；
//    门误伤 pending（比如比较写成 APPROVED）→ 本例红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_pending_purchase_price_transitions_and_stamps_approved_at() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    seed_product(&db, SEED_PRODUCT_ID, "审批门面料甲", "PRD-W8GATE-T2").await;
    let supplier_id = seed_supplier(&db, "SUP-W8GATE-T2").await;
    let svc = PurchasePriceService::new(db.clone());

    let row = seed_purchase_price(&db, price_approval::PENDING, SEED_PRODUCT_ID, supplier_id).await;
    assert!(row.approved_at.is_none(), "种子行必须从空白审批痕迹起步");

    svc.approve_price(row.id, OPERATOR_ID, APPROVAL_REASON.to_string())
        .await
        .expect("pending → approved 权威路径必须成功（门不得误伤合法流转）");

    let after = reload_purchase(&db, row.id).await;
    assert_eq!(after.status, price_approval::APPROVED);
    assert_eq!(after.approved_by, Some(OPERATOR_ID));
    assert_eq!(
        after.approval_reason.as_deref(),
        Some(APPROVAL_REASON),
        "审批通过理由必须逐字落 approval_reason 列（不再只进日志）"
    );
    assert!(
        after.approved_at.is_some(),
        "审批成功必须同步落 approved_at（对齐销售侧 sales_price_service.rs:164）"
    );

    // 批完再批：第二次必须被门拒（防"反复直批"回潮）
    let err = svc
        .approve_price(row.id, OPERATOR_ID, APPROVAL_REASON.to_string())
        .await
        .expect_err("已 approved 的记录不得被重复直批");
    expect_business_gate(&err);
}

// ---------------------------------------------------------------------------
// 3. C-2①：采购列表越界 status → 400 + VALIDATION_ERROR；空串 = 不加筛选
//    改坏什么必红：把校验摘掉（回到静默 200 空列表）→ 越界段红；
//    把空串当越界拦（违反 trim 语义先例）→ 空串段红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn purchase_list_status_filter_rejects_out_of_domain_and_treats_empty_as_no_filter() {
    let (db, app) = price_list_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "审批门面料甲", "PRD-W8GATE-T3").await;
    let supplier_id = seed_supplier(&db, "SUP-W8GATE-T3").await;
    let pending =
        seed_purchase_price(&db, price_approval::PENDING, SEED_PRODUCT_ID, supplier_id).await;
    let inactive =
        seed_purchase_price(&db, price_approval::INACTIVE, SEED_PRODUCT_ID, supplier_id).await;

    for illegal in ["bogus", "ACTIVE", "Pending", "active"] {
        let (status, v) =
            get_json(&app, &format!("/purchase/purchase-prices?status={illegal}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "越界筛选值必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "信封机器码必须 VALIDATION_ERROR，实得 {v}"
        );
    }

    // 空串 = 不加筛选（外沿 normalize_empty_query_params 剔除后 handler 收 None；
    // 两种到达形态都必须回全量，不得 400、不得恒空）
    let (status, v) = get_json(&app, "/purchase/purchase-prices?status=").await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "空串筛选必须 200，实得 {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
    let ids: Vec<i32> = v["data"]
        .as_array()
        .expect("列表 data 必须是数组")
        .iter()
        .filter_map(|r| r["id"].as_i64().map(|i| i as i32))
        .collect();
    assert!(
        ids.contains(&pending.id) && ids.contains(&inactive.id),
        "空串=不加筛选必须回全量（pending 与 inactive 都在），实得 ids={ids:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. C-2②：采购白名单 = price_approval::ALL 全等 —— inactive 属采购合法域必须放行，
//    允许值正常回读（按值过滤命中自建行）。
//    改坏什么必红：把 purchase 白名单缩成 sales 两值 → inactive 段 400 红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn purchase_list_status_filter_allows_full_purchase_domain_and_reads_back() {
    let (db, app) = price_list_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "审批门面料甲", "PRD-W8GATE-T4").await;
    let supplier_id = seed_supplier(&db, "SUP-W8GATE-T4").await;
    let pending =
        seed_purchase_price(&db, price_approval::PENDING, SEED_PRODUCT_ID, supplier_id).await;
    let approved =
        seed_purchase_price(&db, price_approval::APPROVED, SEED_PRODUCT_ID, supplier_id).await;
    let inactive =
        seed_purchase_price(&db, price_approval::INACTIVE, SEED_PRODUCT_ID, supplier_id).await;

    for (value, expect_id, expect_status) in [
        (price_approval::PENDING, pending.id, price_approval::PENDING),
        (
            price_approval::APPROVED,
            approved.id,
            price_approval::APPROVED,
        ),
        (
            price_approval::INACTIVE,
            inactive.id,
            price_approval::INACTIVE,
        ),
    ] {
        let (status, v) =
            get_json(&app, &format!("/purchase/purchase-prices?status={value}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "采购域允许值 {value} 必须放行，实得 {v}"
        );
        assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
        let rows = v["data"].as_array().expect("列表 data 必须是数组");
        let hit: Vec<&serde_json::Value> = rows
            .iter()
            .filter(|r| r["id"].as_i64() == Some(expect_id as i64))
            .collect();
        assert_eq!(
            hit.len(),
            1,
            "允许值 {value} 必须精确命中自建行 {expect_id}"
        );
        assert_eq!(
            hit[0]["status"].as_str(),
            Some(expect_status),
            "按值筛选的命中行状态必须与筛选值逐字符一致"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. C-2③：销售白名单**分表钉**，绝不取并集 —— inactive 在销售侧必须 400
//    （它是采购侧合法值；若有人图省事两侧共用 ALL，本例红）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn sales_list_status_filter_pins_sales_domain_not_union() {
    let (db, app) = price_list_app().await;
    seed_product(&db, SEED_PRODUCT_ID, "审批门面料甲", "PRD-W8GATE-T5").await;
    let pending = seed_sales_price(&db, price_approval::PENDING, SEED_PRODUCT_ID).await;
    let approved = seed_sales_price(&db, price_approval::APPROVED, SEED_PRODUCT_ID).await;

    // 允许值放行 + 回读
    for (value, expect_id) in [
        (price_approval::PENDING, pending.id),
        (price_approval::APPROVED, approved.id),
    ] {
        let (status, v) = get_json(&app, &format!("/sales/sales-prices?status={value}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "销售域允许值 {value} 必须放行，实得 {v}"
        );
        assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
        let ids: Vec<i64> = v["data"]
            .as_array()
            .expect("列表 data 必须是数组")
            .iter()
            .filter_map(|r| r["id"].as_i64())
            .collect();
        assert!(
            ids.contains(&(expect_id as i64)),
            "允许值 {value} 必须回读到自建行 {expect_id}，实得 ids={ids:?}"
        );
    }

    // 越界值：含"采购侧合法但销售侧禁"的 inactive（并集实现即在此露馅），以及裸拼写错误
    for illegal in ["inactive", "bogus", "ACTIVE"] {
        let (status, v) = get_json(&app, &format!("/sales/sales-prices?status={illegal}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "销售列表越界筛选值 {illegal} 必须 400（分表钉，不取并集），实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "信封机器码必须 VALIDATION_ERROR，实得 {v}"
        );
    }

    // 空串 = 不加筛选（与采购侧同语义）
    let (status, v) = get_json(&app, "/sales/sales-prices?status=").await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "空串筛选必须 200，实得 {v}"
    );
}

// ---------------------------------------------------------------------------
// 6. 防回潮源码扫描锁（"把门从落点上摘掉就必红"的结构性证明）
//    ①采购 approve 函数体必须就地判 PENDING、落 approved_at 与 approval_reason
//      （不能只留 API 层校验）；
//    ②两个列表 handler 的入口必须各自调用筛选校验；
//    ③销售白名单常量必须是 PENDING/APPROVED/REJECTED 三值且不含
//      INACTIVE（并集化的源码指纹）。
// ---------------------------------------------------------------------------
#[test]
fn source_scan_price_gates_are_wired_at_the_right_points() {
    let svc_src = std::fs::read_to_string("src/services/purchase_price_service.rs")
        .expect("读取 purchase_price_service.rs 失败");
    let body_start = svc_src
        .find("pub async fn approve_price")
        .expect("approve_price 必须仍在（审批能力不得被删）");
    let body = &svc_src[body_start..];
    let body = body.split("\n    /// ").next().unwrap_or(body);
    assert!(
        body.contains("price_model.status != price_approval::PENDING"),
        "采购审批必须就地判 pending 状态门（改回直接 Set 即视为摘门，C-1/D7 回归）"
    );
    assert!(
        body.contains("AppError::business"),
        "状态门必须归 BUSINESS 族（对齐销售侧口径，不得改族）"
    );
    assert!(
        body.contains("price.approved_at = Set(Some(chrono::Utc::now()));"),
        "审批成功必须同步落 approved_at（对齐销售侧 :164；删掉即 C-1 回归）"
    );
    assert!(
        body.contains("price.approval_reason = Set(Some(approval_reason));"),
        "通过理由必须真实落 approval_reason 列（只进日志即回潮，落库口径见已定案裁定）"
    );

    let po_src = std::fs::read_to_string("src/handlers/purchase_price_handler.rs")
        .expect("读取 purchase_price_handler.rs 失败");
    assert!(
        po_src.contains("validate_purchase_price_status_param(params.status.as_deref())?;"),
        "采购列表入口必须就地校验 status 筛选（摘掉即 C-2/R3 假筛选回归）"
    );

    let sa_src = std::fs::read_to_string("src/handlers/sales_price_handler.rs")
        .expect("读取 sales_price_handler.rs 失败");
    assert!(
        sa_src.contains("validate_sales_price_status_param(params.status.as_deref())?;"),
        "销售列表入口必须就地校验 status 筛选（摘掉即 C-2/R3 假筛选回归）"
    );
    assert!(
        sa_src.contains("SALES_PRICE_STATUS_FILTER_ALLOWED"),
        "销售筛选白名单必须分表钉为常量（与 chk_sales_price_status 同源）"
    );
    // 白名单条目集合按**解析**钉死（rustfmt 对三值定义会逐元素折行，连续字面量口径
    // 已失效；解析后集合相等仍与原"逐条为词表常量"判据等价，不放松）
    let allow_start = sa_src
        .find("const SALES_PRICE_STATUS_FILTER_ALLOWED")
        .expect("销售白名单常量必须存在（分表钉，不得内联临时字面量）");
    let mut allow_def = &sa_src[allow_start..];
    let def_end = allow_def.find("];").expect("销售白名单常量定义未闭合") + 2;
    allow_def = &allow_def[..def_end];
    let lit_start = allow_def
        .rfind("&[")
        .expect("白名单常量 = 号后必须是数组字面量");
    let after_lit = &allow_def[lit_start + 2..];
    let inner = &after_lit[..after_lit.find(']').expect("数组字面量未闭合")];
    let elems: Vec<&str> = inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(
        elems,
        vec![
            "price_approval::PENDING",
            "price_approval::APPROVED",
            "price_approval::REJECTED",
        ],
        "销售白名单取值必须逐条为词表常量 PENDING/APPROVED/REJECTED 三值（rejected 随 m0080 扩集入白名单；增删值是产品口径变更，须走裁定）"
    );
    assert!(
        !sa_src.contains("SALES_PRICE_STATUS_FILTER_ALLOWED")
            || !sa_src.contains("price_approval::INACTIVE"),
        "销售白名单绝不引用 INACTIVE（并集化即把 sales 侧 inactive 放回『合法但恒空』老路）"
    );

    // 词表单源：本锁与实现都不允许出现字符串字面量状态数组
    assert_eq!(
        price_approval::ALL,
        &[
            price_approval::PENDING,
            price_approval::APPROVED,
            price_approval::REJECTED,
            price_approval::INACTIVE,
        ],
        "采购白名单取值集必须逐条等于既有 price_approval 四常量（rejected 随 m0080 扩集入词表与两侧 CHECK；增删值是产品口径变更，须走裁定）"
    );
}
