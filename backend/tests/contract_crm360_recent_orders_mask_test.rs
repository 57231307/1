//! 契约测试（360 出口 · recent_orders 子集专项）：客户 360 出口对 recent_orders
//! 每一行套用与销售订单列表/详情同一联系方式脱敏真源的活体锁。
//!
//! 六要素（功能·调用方·入参·传给谁·存什么·存哪里）：
//! - 功能：钉死 GET /api/v1/erp/crm/customers/:id/360 出参 recent_orders 子集的
//!   读侧掩码契约（非 admin contact_phone 命中权威掩码、admin 原文放行）；
//! - 调用方：本测试文件自身，经真 HTTP 装配直接调用 handler；
//! - 入参：注入的 AuthContext（user 50 = 客户 1 owner；role 1 admin / role 2 非 admin）
//!   与路径 id=1；
//! - 传给谁：bingxi_backend::handlers::crm_handler::get_customer_360；
//! - 存什么：sales_orders 行的 contact_phone/contact_person/shipping_address/
//!   billing_address 明文快照（掩码只应作用于电话/邮箱键，其余列必须原样在场）；
//! - 存哪里：test_common::setup_test_db() 的本地真 PostgreSQL（迁移后 schema），
//!   需真库执行，属 CI 侧跑的集成测试。
//!
//! 被锁判据（对应 get_customer_360 里 recent_orders 门）：
//! - 非 admin 会话读 360：recent_orders 每行 contact_phone 必须逐字符等于
//!   utils::field_mask::mask_phone 对种入原文的确定输出（不手拼 *** 字面量），
//!   键仍在（打码保留键，非移除非置空）；order_no/total_amount/contact_person/
//!   shipping_address/billing_address 等非掩码键全部在场（正向对照，防"整行清空"型假绿）；
//! - 检测力自证：种入的 11 位号码字面量与其掩码结果必不相等且掩码结果非空，
//!   防掩码实现退化为恒等或空串时本锁双双空转；
//! - admin 会话读同一 360：contact_phone 为落库原文（收紧误伤 admin 即在此变红）；
//! - 同次响应内非订单子集口径不变：customer 主数据行仍走客户域既有门（非 admin
//!   contact_phone 已知掩码、address 整键移除；admin 原文放行）——recent_orders 门
//!   挂错位置、或顺带改了别的子集口径，即在此变红；
//! - 纵深防御：非 admin 全响应文本不含任何种下的订单/客户明文手机号；
//! - 源码棘轮：get_customer_360 体内必须逐字符出现"行循环外 match 出 is_admin 一次 +
//!   循环内对每行调用 field_mask::mask_contact_fields_for_role(std::mem::take(order),
//!   is_admin)"的完整结构，且定位失败分支必须显式 tracing::error! 点名
//!   recent_orders——改为循环内反复判定、内联第二套掩码或静默跳过即红。
//!
//! 通道（与同 handler 的 wave14 地址活体锁同源）：test_common::setup_test_db() 真
//! PostgreSQL 真跑 + 真 HTTP 装配；表结构唯一来源 = backend/migration；users/customers/
//! sales_orders 自种子（FK 父行先插；customers/sales_orders 属逐用例 TRUNCATE 的业务表，
//! 不在 SEALED_REFERENCE_TABLES 密封清单内，自插 id 不与迁移种子相撞）。roles 属密封
//! 参照表，本文件只用迁移种子角色（id=1 admin / id=2 非 admin），不自造角色。
//! sales_orders id 用私有高段（950x）避免与迁移种子/其他夹具互踩。
//! 活体用例全仓统一按 --test-threads=1 执行（共享真库夹具，并行会互相 TRUNCATE）。
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
use bingxi_backend::models::sales_order;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::utils::field_mask::mask_phone;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
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

const SO1_ID: i32 = 9501; // 私有高段 id，最近订单行 1
const SO2_ID: i32 = 9502; // 私有高段 id，最近订单行 2

// 客户主数据行自身 PII（与订单行取值刻意不同，令"订单行门漏挂"可被单点判红）
const CUST_PHONE: &str = "13812348888";

// 销售订单行的收货联系电话快照（种入 sales_orders，非 admin 出口必须打码）
const SO1_PHONE: &str = "13500007777";
const SO2_PHONE: &str = "13611112222";
const SO1_CONTACT_PERSON: &str = "收货联系人甲";
const SO2_CONTACT_PERSON: &str = "收货联系人乙";
const SO1_SHIPPING_ADDRESS: &str = "河北省邢台市订单甲路77号";
const SO2_SHIPPING_ADDRESS: &str = "江苏省南京市订单乙路22号";
const SO_BILLING_ADDRESS: &str = "发票地址邢台市丙街9号";

// 非 admin 出口不得出现的明文手机号（两条订单行 + 客户行）
const BANNED_RAW: [&str; 3] = [SO1_PHONE, SO2_PHONE, CUST_PHONE];

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
        username: format!("w360so_user_{user_id}"),
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

/// 订单行：只种联系方式快照与 NOT NULL 业务列所需字段，其余可空列走 ActiveModel
/// 默认（NotSet 不进 INSERT 列清单），amount 全部置 0（不影响本锁）。
async fn insert_order(
    db: &DatabaseConnection,
    id: i32,
    order_no: &str,
    phone: &str,
    contact_person: &str,
    shipping_address: &str,
    day: u32,
) {
    sales_order::ActiveModel {
        id: Set(id),
        order_no: Set(order_no.to_string()),
        customer_id: Set(1),
        order_date: Set(ts(day)),
        status: Set("approved".to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        contact_person: Set(Some(contact_person.to_string())),
        contact_phone: Set(Some(phone.to_string())),
        shipping_address: Set(Some(shipping_address.to_string())),
        billing_address: Set(Some(SO_BILLING_ADDRESS.to_string())),
        created_by: Set(Some(USER_OWNER)),
        created_at: Set(ts(day)),
        updated_at: Set(ts(day)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("销售订单种子插入失败 id={id}: {e}"));
}

/// 种子：用户 50（dept 1）；客户 1 owner=50（含 CUST_PHONE）；两条挂客户 1 的
/// sales_orders（id 9501/9502，contact_phone/contact_person/shipping_address/
/// billing_address 均有值）。roles 用迁移种子 1（admin）/2（非 admin），不新插。
async fn seeded_db() -> Arc<DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'w360so_sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let cust_sql = format!(
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,
         contact_phone,contact_email,address,tax_id,customer_industry,
         credit_limit,payment_terms,status,customer_type,owner_id,created_at,updated_at) VALUES
         (1,'CUS-W360SO-0001','360订单门客户','张三','{CUST_PHONE}','so@example.com',
          '河北省邢台市主街0号','TAX-A-0001','面料',0,30,'active','retail',{USER_OWNER},
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    exec(&db, &cust_sql).await;
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
    )
    .await;
    insert_order(
        &db,
        SO1_ID,
        "SO-W360SO-0001",
        SO1_PHONE,
        SO1_CONTACT_PERSON,
        SO1_SHIPPING_ADDRESS,
        1,
    )
    .await;
    insert_order(
        &db,
        SO2_ID,
        "SO-W360SO-0002",
        SO2_PHONE,
        SO2_CONTACT_PERSON,
        SO2_SHIPPING_ADDRESS,
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

/// 取 360 出参里的 recent_orders 数组，形状漂移即 panic（不接受缺键糊弄）
fn recent_orders_of(v: &Value) -> Vec<Value> {
    v["data"]["recent_orders"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("360 出参缺 recent_orders 数组: {v}"))
}

fn row_by_order_no(rows: &[Value], order_no: &str) -> Value {
    rows.iter()
        .find(|r| r["order_no"].as_str() == Some(order_no))
        .cloned()
        .unwrap_or_else(|| panic!("缺订单行 order_no={order_no}: {rows:?}"))
}

/// 出参不得含任何给定原文 PII（无论键名、无论嵌套）
fn assert_no_raw(value: &Value, banned_values: &[&str], where_label: &str) {
    let raw = value.to_string();
    for banned in banned_values {
        assert!(
            !raw.contains(banned),
            "{where_label}：响应体含未脱敏个人信息 {banned}（订单联系方式读侧旁路回潮）\n出参: {raw}"
        );
    }
}

// ---------------------------------------------------------------------------
// 检测力自证：掩码对 11 位字面量必须改变取值且非空（防锁空转）
// ---------------------------------------------------------------------------

#[test]
fn w360so_mask_true_source_self_proof() {
    for raw in [SO1_PHONE, SO2_PHONE, CUST_PHONE] {
        assert_eq!(raw.len(), 11, "种入号码必须是 11 位手机号字面量: {raw}");
        let masked = mask_phone(raw);
        assert_ne!(masked, raw, "掩码必须改变取值（恒等输出=门空转）: {raw}");
        assert!(!masked.is_empty(), "掩码输出不得为空串: {raw}");
    }
}

// ---------------------------------------------------------------------------
// 判据 1 · 非 admin：recent_orders 每行 contact_phone 命中权威掩码、掩码键保留
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w360so_non_admin_recent_orders_phone_masked_keys_kept() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_NONADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "非 admin 读本人客户 360 应 200（行门维持，收紧的是订单行联系方式列）: {v}"
    );

    let rows = recent_orders_of(&v);
    assert_eq!(
        rows.len(),
        2,
        "两条订单行都应出现在出参里（逐行脱敏，非只处理首行）: {rows:?}"
    );

    // 同次响应内 customer 子集口径不变：recent_orders 门只可能挂在订单数组上，
    // 客户行仍走客户域默认门（掩码保留键 + address 整键移除），挂错位置/顺带改动即红。
    let cust = &v["data"]["customer"];
    assert_eq!(
        cust["contact_phone"].as_str(),
        Some(mask_phone(CUST_PHONE).as_str()),
        "非 admin：customer 行的客户域默认掩码口径不得因订单门改动: {cust}"
    );
    assert!(
        cust.get("address").is_none(),
        "非 admin：customer 行 address 必须仍整键移除（客户域门被放宽即在此变红）: {cust}"
    );

    for (order_no, raw_phone) in [("SO-W360SO-0001", SO1_PHONE), ("SO-W360SO-0002", SO2_PHONE)] {
        let row = row_by_order_no(&rows, order_no);
        // 掩码值取仓内权威源 mask_phone 对种入原文的确定输出（与销单列表/详情同一实现）
        assert_eq!(
            row.get("contact_phone").and_then(Value::as_str),
            Some(mask_phone(raw_phone).as_str()),
            "非 admin：订单行 {order_no} 的 contact_phone 必须命中权威掩码值\
             （原文直通即旁路，整行清空即误伤）: {row}"
        );
        // 检测力显式负断言：把订单门摘掉后本断言直接红（配合自证用例"掩码≠原文"，
        // 本锁不存在"摘门仍绿"的退路）
        assert_ne!(
            row.get("contact_phone").and_then(Value::as_str),
            Some(raw_phone),
            "非 admin：订单行 {order_no} 的 contact_phone 不得为原文（读侧旁路复活）: {row}"
        );
        assert!(
            row.get("contact_phone").is_some(),
            "非 admin：订单行 {order_no} 的 contact_phone 键必须保留（打码保留键契约）: {row}"
        );
        // 正向对照：销单域实现只掩电话/邮箱键，其余业务列不得被"整行清空"型假绿顺手抹掉
        for kept in [
            "order_no",
            "total_amount",
            "status",
            "contact_person",
            "shipping_address",
            "billing_address",
        ] {
            assert!(
                row.get(kept).is_some(),
                "非 admin：订单行 {order_no} 的 {kept} 不属联系方式掩码键，不得被移除: {row}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 判据 2 · admin：recent_orders 每行 contact_phone 原文放行
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w360so_admin_recent_orders_phone_raw() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_ADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "admin 读客户 360: {v}");

    let rows = recent_orders_of(&v);
    assert_eq!(rows.len(), 2, "admin 出参两条订单行俱在: {rows:?}");

    // 同次响应内 customer 子集口径不变：admin 客户行原文放行，不因订单门收紧。
    let cust = &v["data"]["customer"];
    assert_eq!(
        cust["contact_phone"].as_str(),
        Some(CUST_PHONE),
        "admin：customer 行原文口径不得因订单门收紧: {cust}"
    );
    assert!(
        cust.get("address").is_some(),
        "admin：customer 行 address 键必须仍存在: {cust}"
    );

    for (order_no, raw_phone) in [("SO-W360SO-0001", SO1_PHONE), ("SO-W360SO-0002", SO2_PHONE)] {
        let row = row_by_order_no(&rows, order_no);
        assert_eq!(
            row["contact_phone"].as_str(),
            Some(raw_phone),
            "admin：订单行 {order_no} 的 contact_phone 必须原文放行\
             （收紧误伤 admin 即在此变红）: {row}"
        );
    }
}

// ---------------------------------------------------------------------------
// 判据 3 · 纵深防御：非 admin 全响应文本不含任何种下的明文手机号
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w360so_non_admin_whole_response_has_no_raw_phone() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_OWNER, Some(ROLE_SEED_NONADMIN), "all"));

    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "非 admin 读客户 360: {v}");

    // 订单行明文手机号若在别处（含 recent_orders 之外的子集）又漏一份即在此变红，
    // 而不只盯着 recent_orders 数组本身。
    assert_no_raw(
        &v["data"],
        &BANNED_RAW,
        "非 admin 全响应（订单行 + 客户行明文）",
    );
}

// ---------------------------------------------------------------------------
// 判据 4 · 源码棘轮：真源调用逐字符钉形、admin 判定在行循环外算一次、
//         定位失败必须显式 error 点名 recent_orders
// ---------------------------------------------------------------------------

/// 剔全部空白并消掉闭合定界符前的尾逗号（防 rustfmt 换行/尾逗号改变被锁调用形状；
/// 与 wave8/wave14 同源 canon）
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
fn w360so_360_orders_gate_shares_true_source_ratchet() {
    let handler = include_str!("../src/handlers/crm_handler.rs").replace('\r', "");
    let body = top_fn_body(&handler, "get_customer_360");
    let c = canon(&body);

    // 真源调用整段结构钉桩：数组定位 + admin 判定在行循环外算一次（match 出 is_admin），
    // 循环体内对每行调用 field_mask::mask_contact_fields_for_role(std::mem::take(order),
    // is_admin)。改 clone、改循环内判定或换实现即在此变红。
    assert!(
        c.contains(
            "ifletSome(orders)=value.get_mut(\"recent_orders\").and_then(Value::as_array_mut)\
             {letis_admin=matchauth.role_id{Some(role_id)=>admin_checker::is_admin_role(\
             &state.db,role_id).await,None=>false};fororderinorders.iter_mut(){\
             *order=field_mask::mask_contact_fields_for_role(std::mem::take(order),is_admin);}}"
        ),
        "回潮棘轮：360 recent_orders 子集不再逐行调用销单域同一真源 \
         mask_contact_fields_for_role，或 admin 判定被搬进行循环/改为 clone（第二口径复活）"
    );

    // 定位失败分支：必须显式 tracing::error! 点名 recent_orders，静默跳过即红。
    assert!(
        body.contains("客户 360 出参未定位到 recent_orders 数组"),
        "回潮棘轮：recent_orders 定位失败不再显式记 error（静默形状漂移=原文直通无人知晓）"
    );

    // 兜底防线：本门区段内 admin 判定必须出现在行循环之前（循环外算一次）。
    let gate_start = body
        .find("if let Some(orders)")
        .expect("recent_orders 门区段起点缺失（写法漂移须同步本棘轮）");
    let loop_head = body[gate_start..]
        .find("for order in orders.iter_mut()")
        .expect("recent_orders 行循环缺失");
    let admin_call = body[gate_start..]
        .find("admin_checker::is_admin_role")
        .expect("admin_checker::is_admin_role 调用缺失（admin 判定源漂移）");
    assert!(
        admin_call < loop_head,
        "回潮棘轮：recent_orders 门的 admin 判定必须在行循环外算一次（现判定出现在循环内或之后）"
    );

    // 掩码键集合不得由本出口自造：门区段内除真源调用外不得出现字面量掩码/整键删除/兜底。
    let gate_end = gate_start
        + body[gate_start..]
            .find("销售订单联系方式默认脱敏未应用")
            .expect("recent_orders 门 error 文案缺失（error 口径漂移须同步本棘轮）")
        + "销售订单联系方式默认脱敏未应用".len();
    let gate = &body[gate_start..gate_end];
    for banned in ["\"***\"", "****", ".remove(", "unwrap_or("] {
        assert!(
            !gate.contains(banned),
            "回潮棘轮：360 recent_orders 门区段出现内联第二套掩码/字面量/兜底形态 → {banned}"
        );
    }
}
