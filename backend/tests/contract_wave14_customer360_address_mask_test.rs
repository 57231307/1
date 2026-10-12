//! 契约测试（wave14 · 360 收货地址子集专项）：客户 360 出口对 `shipping_addresses`
//! 每一行套用与地址列表出口同一默认脱敏真源的活体锁。
//!
//! 被锁判据（对应 `handlers/crm_handler.rs::get_customer_360` 新增的第三处门）：
//! - 非 admin 会话读 360：`shipping_addresses` 每行 `contact_phone` 命中权威掩码、
//!   `address` **整键移除**（键不存在，非空串非 null），`contact_name`/`province`/
//!   `is_default` 等非 PII 列仍在（正向对照，防"整行清空"型假绿）；
//! - admin 会话读同一 360：`contact_phone` 与 `address` 均为落库原文且 `address` 键存在
//!   （收紧误伤 admin 即在此变红）；
//! - 纵深防御：非 admin 全响应文本不含任何种下的明文手机号/详细地址字串（防在别处又漏一份）；
//! - 源码棘轮：`get_customer_360` 体内必须出现对该真源 `CrmService::mask_customer_pii_defaults`
//!   的调用，且 admin 判定在行循环外算一次；不得内联第二套掩码字面量或 `unwrap_or(` 兜底。
//!
//! 通道（与既有同 handler 活体锁同源）：`test_common::setup_test_db()` 真 PostgreSQL 真跑 +
//! 真 HTTP 装配；表结构唯一来源 = backend/migration；users/customers/customer_addresses 自种子
//! （FK 父行先插）。roles 属密封参照表，本文件只用迁移种子角色（id=1 admin / id=2 非 admin），
//! 不自造角色。customer_addresses id 用私有高段（947x）避免与迁移种子/其他夹具互踩。
//! 禁 sqlite、无 mock、无自建 DDL、无 #[ignore]。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::get_customer_360;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::customer_address;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::utils::field_mask::mask_phone;
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, Set, Statement};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 夹具常量
// ---------------------------------------------------------------------------

const USER_OWNER: i32 = 50; // 客户 1 归属（dept 1，本人）
const ROLE_SEED_ADMIN: i32 = 1; // 迁移种子 admin（roles.code='admin'）
const ROLE_SEED_NONADMIN: i32 = 2; // 迁移种子非 admin

const ADDR1_ID: i64 = 9471; // 私有高段 id，默认地址行
const ADDR2_ID: i64 = 9472; // 私有高段 id，非默认地址行

// 客户主数据行自身 PII（与地址行取值刻意不同，令"地址行门漏挂"可被单点判红、
// 也让"customer 门漏挂"在纵深防御里可区分定位）
const CUST_PHONE: &str = "13812348888";
const CUST_EMAIL: &str = "alice@example.com";
const CUST_ADDRESS: &str = "河北省邢台市主街0号";

// 收货地址行的明文手机号/详细地址（种入 customer_addresses，非 admin 出口必须脱敏/移除）
const ADDR1_PHONE: &str = "13500007777";
const ADDR1_ADDRESS: &str = "河北省邢台市甲路77号";
const ADDR2_PHONE: &str = "13611112222";
const ADDR2_ADDRESS: &str = "江苏省南京市乙路22号";

// 非 admin 出口不得出现的全部明文 PII（customer 行 + 两条地址行）
const BANNED_RAW: [&str; 6] = [
    CUST_PHONE,
    CUST_ADDRESS,
    ADDR1_PHONE,
    ADDR1_ADDRESS,
    ADDR2_PHONE,
    ADDR2_ADDRESS,
];

fn ts(day: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(2026, 1, day)
        .expect("种子日期非法")
        .and_hms_opt(0, 0, 0)
        .expect("种子时间非法")
        .and_utc()
}

fn make_auth(user_id: i32, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w14_user_{user_id}"),
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

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

async fn insert_address(
    db: &DatabaseConnection,
    id: i64,
    contact_name: &str,
    phone: &str,
    address: &str,
    province: Option<&str>,
    is_default: bool,
    day: u32,
) {
    customer_address::ActiveModel {
        id: Set(id),
        customer_id: Set(1),
        contact_name: Set(contact_name.to_string()),
        contact_phone: Set(phone.to_string()),
        province: Set(province.map(|s| s.to_string())),
        city: Set(Some("邢台市".to_string())),
        district: Set(Some("桥西区".to_string())),
        address: Set(address.to_string()),
        postal_code: Set(Some("054001".to_string())),
        is_default: Set(is_default),
        remark: Set(None),
        created_at: Set(ts(day)),
        updated_at: Set(ts(day)),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("收货地址种子插入失败 id={id}: {e}"));
}

/// 种子：用户 50（dept 1）；客户 1 owner=50；两条 customer_addresses（id 9471/9472，
/// contact_name/contact_phone/address 均有值，province/is_default 齐全）。
/// roles 用迁移种子 1（admin）/2（非 admin），不新插。
async fn seeded_db() -> Arc<DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'w14_sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let cust_sql = format!(
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,
         contact_phone,contact_email,address,tax_id,customer_industry,
         credit_limit,payment_terms,status,customer_type,owner_id,created_at,updated_at) VALUES
         (1,'CUS-W14-0001','波十四客户A','张三','{CUST_PHONE}','{CUST_EMAIL}','{CUST_ADDRESS}',
          'TAX-A-0001','面料',0,30,'active','retail',{USER_OWNER},
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    exec(&db, &cust_sql).await;
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
    )
    .await;
    insert_address(
        &db,
        ADDR1_ID,
        "收货人甲",
        ADDR1_PHONE,
        ADDR1_ADDRESS,
        Some("冀东区"),
        true,
        1,
    )
    .await;
    insert_address(
        &db,
        ADDR2_ID,
        "收货人乙",
        ADDR2_PHONE,
        ADDR2_ADDRESS,
        Some("苏北区"),
        false,
        2,
    )
    .await;
    Arc::new(db)
}

fn state_from(db: &Arc<DatabaseConnection>) -> AppState {
    AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    }
}

fn build_app(db: &Arc<DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/customers/{id}/360", get(get_customer_360))
        .with_state(state_from(db))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, method: Method, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::empty())
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

async fn customer_360(app: &Router, id: i32) -> (StatusCode, Value) {
    send(app, Method::GET, &format!("/erp/crm/customers/{id}/360")).await
}

/// 取 360 出参里的 shipping_addresses 数组，形状漂移即 panic（不接受缺键糊弄）
fn shipping_rows_of(v: &Value) -> Vec<Value> {
    v["data"]["shipping_addresses"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("360 出参缺 shipping_addresses 数组: {v}"))
}

fn row_by_name(rows: &[Value], contact_name: &str) -> Value {
    rows.iter()
        .find(|r| r["contact_name"].as_str() == Some(contact_name))
        .cloned()
        .unwrap_or_else(|| panic!("缺地址行 contact_name={contact_name}: {rows:?}"))
}

/// 出参不得含任何给定原文 PII（无论键名、无论嵌套）
fn assert_no_raw(value: &Value, banned_values: &[&str], where_label: &str) {
    let raw = value.to_string();
    for banned in banned_values {
        assert!(
            !raw.contains(banned),
            "{where_label}：响应体含未脱敏个人信息 {banned}（地址脱敏读侧旁路回潮）\n出参: {raw}"
        );
    }
}

// ---------------------------------------------------------------------------
// 判据 1 · 非 admin：shipping_addresses 每行电话打码、address 整键移除、非 PII 列仍在
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w14_non_admin_shipping_address_rows_masked_and_address_removed() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_NONADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "非 admin 读本人客户 360 应 200（行门维持，收紧的是地址行列）: {v}"
    );

    let rows = shipping_rows_of(&v);
    assert_eq!(
        rows.len(),
        2,
        "两条地址行都应出现在出参里（逐行脱敏，非只处理首行）: {rows:?}"
    );

    for (name, raw_phone) in [("收货人甲", ADDR1_PHONE), ("收货人乙", ADDR2_PHONE)] {
        let row = row_by_name(&rows, name);
        // 掩码值取仓内权威源 mask_phone 对种入原文的确定输出（不手拼 *** 字面量）
        assert_eq!(
            row.get("contact_phone").and_then(Value::as_str),
            Some(mask_phone(raw_phone).as_str()),
            "非 admin：地址行 {name} 的 contact_phone 必须命中权威掩码值（原文直通即旁路，\
             整行清空即误伤）: {row}"
        );
        assert!(
            row.get("address").is_none(),
            "非 admin：地址行 {name} 的 address 键必须不存在（整键移除契约，\
             空串/null 都不算移除）: {row}"
        );
        // 正向对照：非 PII 列不得被"整行清空"型假绿顺手抹掉
        assert!(
            row.get("contact_name").is_some(),
            "非 admin：{name} 的 contact_name 属非 PII，不得被移除（整行清空即误伤）: {row}"
        );
        assert!(
            row.get("province").is_some(),
            "非 admin：{name} 的 province 属非 PII，不得被移除: {row}"
        );
        assert!(
            row.get("is_default").is_some(),
            "非 admin：{name} 的 is_default 属非 PII，不得被移除: {row}"
        );
    }
}

// ---------------------------------------------------------------------------
// 判据 2 · admin：shipping_addresses 每行 contact_phone / address 原文放行
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w14_admin_shipping_address_rows_raw_and_address_present() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_ADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "admin 读客户 360: {v}");

    let rows = shipping_rows_of(&v);
    assert_eq!(rows.len(), 2, "admin 出参两条地址行俱在: {rows:?}");

    for (name, raw_phone, raw_address) in [
        ("收货人甲", ADDR1_PHONE, ADDR1_ADDRESS),
        ("收货人乙", ADDR2_PHONE, ADDR2_ADDRESS),
    ] {
        let row = row_by_name(&rows, name);
        assert_eq!(
            row["contact_phone"].as_str(),
            Some(raw_phone),
            "admin：{name} 的 contact_phone 必须原文放行（收紧误伤 admin 即在此变红）: {row}"
        );
        assert_eq!(
            row["address"].as_str(),
            Some(raw_address),
            "admin：{name} 的 address 必须原文放行: {row}"
        );
        assert!(
            row.get("address").is_some(),
            "admin：{name} 的 address 键必须存在（不得整键移除）: {row}"
        );
    }
}

// ---------------------------------------------------------------------------
// 判据 3 · 纵深防御：非 admin 全响应文本不含任何种下的明文手机号/详细地址
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w14_non_admin_whole_response_has_no_raw_pii() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_NONADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "非 admin 读客户 360: {v}");

    // 对整份 data 做纵深防御：地址行的明文手机号/详细地址若在别处（含 customer/子集）
    // 又漏一份即在此变红，而不只盯着 shipping_addresses 数组本身。
    assert_no_raw(
        &v["data"],
        &BANNED_RAW,
        "非 admin 全响应（customer 行 + 两条地址行明文）",
    );
}

// ---------------------------------------------------------------------------
// 判据 4 · 源码棘轮：真源被调用、admin 判定在行循环外算一次、无内联第二套掩码/兜底
// ---------------------------------------------------------------------------

/// 剔全部空白并消掉闭合定界符前的尾逗号（防 rustfmt 换行/尾逗号改变被锁调用形状）
fn canon(src: &str) -> String {
    let mut out: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

fn top_fn_body(src: &str, name: &str) -> String {
    src.split(&format!("pub async fn {name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("函数 {name} 定义缺失（改名/删除须同步本棘轮）"))
        .split("\npub async fn ")
        .next()
        .expect("函数体边界")
        .to_string()
}

#[test]
fn w14_360_address_gate_shares_true_source_ratchet() {
    let handler = include_str!("../src/handlers/crm_handler.rs").replace('\r', "");
    let body = top_fn_body(&handler, "get_customer_360");
    let c = canon(&body);

    // 真源调用整段结构钉桩：admin 判定在行循环外算一次（match 出 is_admin），
    // 循环体内对每行调用 CrmService::mask_customer_pii_defaults(std::mem::take(addr), is_admin)。
    assert!(
        c.contains(
            "{letis_admin=matchauth.role_id{Some(role_id)=>admin_checker::is_admin_role(\
             &state.db,role_id).await,None=>false};foraddrinaddrs.iter_mut(){\
             *addr=CrmService::mask_customer_pii_defaults(std::mem::take(addr),is_admin);}}"
        ),
        "回潮棘轮：360 收货地址子集不再逐行调用真源 mask_customer_pii_defaults，\
         或 admin 判定被搬进行循环/改为 clone+unwrap_or（第二口径与回落兜底复活）"
    );

    // 兜底防线：admin 判定必须出现在行循环之前（循环外算一次），不得在循环内反复调用。
    let admin_call = "admin_checker::is_admin_role(&state.db,role_id).await";
    let loop_head = "foraddrinaddrs.iter_mut()";
    let admin_pos = c
        .find(admin_call)
        .expect("admin_checker::is_admin_role 调用缺失（admin 判定源漂移）");
    let loop_pos = c.find(loop_head).expect("地址行循环缺失");
    assert!(
        admin_pos < loop_pos,
        "回潮棘轮：admin 判定必须在行循环外算一次（现判定出现在循环内或之后，竞争放大/口径漂移）"
    );

    // 内联第二套掩码 / 兜底：出现即红。真源是 mask_customer_pii_defaults，
    // 出口不得直接手拼掩码字面量、不得直接调 utils 层的 mask_contact_fields_for_role、
    // 不得内联 obj.remove("address")、不得 unwrap_or 兜底。
    for banned in [
        "\"***\"",
        "****",
        "mask_contact_fields_for_role",
        ".remove(",
        "unwrap_or(",
    ] {
        assert!(
            !body.contains(banned),
            "回潮棘轮：360 收货地址门出现内联第二套掩码/字面量/兜底形态 → {banned}"
        );
    }
}
