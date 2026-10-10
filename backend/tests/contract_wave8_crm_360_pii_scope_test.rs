//! 契约测试（360 专项）：CRM 客户 360 出口 PII 门与行级 scope 同源锁（G-A / G-B）
//!
//! 两条同型残留（前波 D-1 只修了 360 内嵌商机的**字段级**金额门）：
//! - **G-A** `get_customer_360` 顶层 `data.customer` 整行原文直出，未经客户域字段门
//!   → 列表/详情看不到的 PII（`contact_phone`/`contact_email`/`address` 及
//!   `field_permissions` 配置列），打开 360 即可读到（读侧旁路的第二出口）。
//!   正解形态（本文件锁）：360 出口与客户域标准列表/详情**共用同一实现**两层门，
//!   顺序同为"配置层在前、默认脱敏+data_permissions 行在后"：
//!   1. `customer_handler::apply_customer_field_config_mask`（由列表/详情两处内联块
//!      **纯提取**的唯一实现，`field_permissions` 表驱动）；
//!   2. `crm_customer_handler::apply_customer_field_permission`（客户域字段级权限唯一
//!      实现：`mask_customer_pii_defaults` 默认掩码 + `data_permissions` 行过滤）。
//!   等式断言（非"包含掩码"式弱断言）：同一账号同一数据，逐列比较列表行与 360
//!   customer 对象；对配了权限行的角色两处形状**逐列相等**，并另有方向断言防"两处
//!   一起错"（列表与 360 都必须等于已知掩码常量，不接受双双缺键糊弄过去）。
//!   对**未配权限行**的角色，标准列表在查询层按 `DEFAULT_HIDDEN_FIELDS` 整列下推
//!   （键缺失）、360 走客户域默认出参门（掩码保留键）——两种形态都是本域改造前
//!   既有口径（更严/脱敏两形态），本文件把两侧各自钉成确定值而不是笼统"含掩码"。
//! - **G-B** 360 内嵌 opportunities 子集不做行级 data_scope 过滤 → 无权用户仍能经
//!   360 看到他人/他部门商机行的存在性/标题/阶段（列表端点用
//!   `apply_department_scope`（`services/crm/opp.rs::list_opportunities`，判据列
//!   OwnerId/DepartmentId）滤掉的行在 360 原样出现）。
//!   正解形态：服务层对子集查询复用**同一个** `apply_department_scope`（同判据列），
//!   行集与列表逐 id 相等；字段级金额门仍由 handler 侧共用实现叠加，两层互不复制。
//!
//! 公海行（`owner_id=0`）在 360 各门的实际形态**如实钉住**（不臆造期望）：
//! - Self：客户行门 `check_resource_owner` 拒 403（0≠本人）；
//! - Dept：触发器对公海行置 `department_id=NULL` → None → 同样拒 403；
//! - All 非 admin：200 可读（读侧 All 语义不变），customer PII 走默认掩码；
//!   公海商机行在 All 下随列表同形出现（All 不添加行过滤），但金额门按"非本人行"
//!   剔除（owner_id=0≠本人）；
//! - All admin（role 1）：customer 原文 + 金额原文（admin 例外契约逐字维持）。
//!
//! 断言口径（本批已锁）：权限拒绝只断 HTTP status + 信封 `code`（FORBIDDEN）+ 固定
//! 脱敏常量 `err_msg::PERMISSION_PUBLIC` + 信封键集合，不断言/不外显拒绝原因；
//! 金额按 DECIMAL(15,2) 序列化字符串钉原值。
//!
//! 通道（路线一）：`test_common::setup_test_db()` 真 PostgreSQL 真跑 + 真 HTTP 装配，
//! 表结构唯一来源 = backend/migration；users/customers/crm_opportunity/data_permissions/
//! field_permissions 自种子（FK 父行先插）；roles 属**密封参照表**（不参与逐用例
//! TRUNCATE，见 `services/test_common.rs::SEALED_REFERENCE_TABLES`），本文件自造角色
//! 98 一律先删后插保持幂等，且仅用高段 id 避免与迁移种子行（id=1 admin / id=2 非
//! admin）互踩。禁 sqlite、无自建 DDL、无 #[ignore]。

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
use bingxi_backend::handlers::crm_handler::{get_customer_360, list_opportunities};
use bingxi_backend::handlers::customer_handler::{
    get_customer as standard_get_customer, list_customers as standard_list_customers,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::crm_opportunity;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::utils::messages::err_msg;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, Set, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 夹具常量
// ---------------------------------------------------------------------------

const USER_A: i32 = 50; // 客户 1 归属（dept 1，本人）
const USER_B: i32 = 60; // 客户 2 归属（dept 1，同部门他人）
const USER_C: i32 = 70; // 客户 3 归属（dept 2，跨部门他人）

const ROLE_SEED_NONADMIN: i32 = 2; // 迁移种子非 admin；不配任何权限行
const ROLE_PROBE: i32 = 98; // 自造探针角色：配 customer 权限行 + field_permissions 列

const OPP_A_ID: i64 = 101; // 客户 1 · owner=A（dept 1）
const OPP_B_ID: i64 = 102; // 客户 1 · owner=B（dept 1，他人行）
const OPP_C_ID: i64 = 103; // 客户 1 · owner=C（dept 2，跨部门行）
const OPP_POOL_ID: i64 = 104; // 客户 4（公海客户）· owner=0

const OPP_A_EST: i64 = 111_111;
const OPP_A_ACT: i64 = 122_222;
const OPP_B_EST: i64 = 222_222;
const OPP_C_EST: i64 = 333_333;
const OPP_C_ACT: i64 = 344_444;
const OPP_POOL_EST: i64 = 400_000;

const A_PHONE: &str = "13812348888";
const A_EMAIL: &str = "alice@example.com";
const A_ADDRESS: &str = "河北省邢台市甲路1号";
const B_PHONE: &str = "13900001111";
const B_EMAIL: &str = "bob@example.com";
const B_ADDRESS: &str = "江苏省南京市乙路2号";
const C_PHONE: &str = "13600003333";
const POOL_PHONE: &str = "13700002222";
const POOL_EMAIL: &str = "pool@example.com";
const POOL_ADDRESS: &str = "广东省广州市丁路4号";

// 已知掩码常量（= utils/field_mask::mask_phone/mask_email 对上面原值的确定输出，
// 与 wave6 同源口径；两侧出参都必须钉到这些值，禁止"包含掩码"式弱断言）
const A_PHONE_MASKED: &str = "138****8888";
const A_EMAIL_MASKED: &str = "a***@example.com";
const B_PHONE_MASKED: &str = "139****1111";
const B_EMAIL_MASKED: &str = "b***@example.com";
const POOL_PHONE_MASKED: &str = "137****2222";

const ALL_A_PHONE_RAW: [&str; 3] = [A_PHONE, A_EMAIL, A_ADDRESS];
const ALL_B_PHONE_RAW: [&str; 3] = [B_PHONE, B_EMAIL, B_ADDRESS];

fn amount(raw: i64) -> Decimal {
    Decimal::from(raw)
}

/// 真列 DECIMAL(15,2) 的序列化形状（与 wave7/wave8 同源）
fn amount_text(raw: i64) -> String {
    format!("{:.2}", amount(raw))
}

fn ts(day: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(2026, 1, day)
        .expect("种子日期非法")
        .and_hms_opt(0, 0, 0)
        .expect("种子时间非法")
        .and_utc()
}

fn make_auth(
    user_id: i32,
    role_id: Option<i32>,
    data_scope: &str,
    dept_ids: Option<&str>,
    dept_members: Option<&str>,
) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w9_user_{user_id}"),
        role_id,
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
        dept_ids: dept_ids.map(|s| Arc::new(s.to_string())),
        dept_member_user_ids: dept_members.map(|s| Arc::new(s.to_string())),
    }
}

/// Dept 范围的标准 auth：可见部门 [1]、成员 {50,60}
fn dept_auth(user_id: i32, role_id: Option<i32>) -> AuthContext {
    make_auth(user_id, role_id, "dept", Some("1"), Some("50,60"))
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

async fn insert_opp(
    db: &DatabaseConnection,
    id: i32,
    customer_id: i32,
    owner_id: i32,
    est: i64,
    act: Option<i64>,
    day: u32,
) {
    crm_opportunity::ActiveModel {
        id: Set(id),
        opportunity_no: Set(format!("W9OPP{id:03}")),
        opportunity_name: Set(format!("波九商机-{id}")),
        customer_id: Set(customer_id),
        lead_id: Set(None),
        opportunity_type: Set(Some("NEW".to_string())),
        opportunity_stage: Set(Some("QUALIFICATION".to_string())),
        win_probability: Set(Some(Decimal::from(10))),
        estimated_amount: Set(Some(amount(est))),
        actual_amount: Set(act.map(amount)),
        currency: Set(Some("CNY".to_string())),
        expected_close_date: Set(Some(ts(day).date_naive())),
        actual_close_date: Set(None),
        product_ids: Set(None),
        product_names: Set(None),
        product_desc: Set(None),
        owner_id: Set(owner_id),
        // department_id 不手写有效值：BEFORE INSERT 触发器按 owner 的部门重算
        // （owner=0 公海 → NULL；owner=70 → 部门 2），列表行级过滤吃的就是该列。
        department_id: Set(None),
        owner_name: Set(format!("w9_owner_{owner_id}")),
        last_follow_up_date: Set(None),
        next_follow_up_date: Set(None),
        follow_up_plan: Set(None),
        competitor_names: Set(None),
        competitive_advantage: Set(None),
        opportunity_status: Set(Some("OPEN".to_string())),
        won_reason: Set(None),
        lost_reason: Set(None),
        priority: Set(Some("high".to_string())),
        rating: Set(None),
        tags: Set(None),
        created_at: Set(Some(ts(day))),
        updated_at: Set(Some(ts(day))),
        created_by: Set(Some(owner_id)),
        updated_by: Set(Some(owner_id)),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("商机种子插入失败 id={id}: {e}"));
}

/// 标准种子：
/// - users：50/60（dept 1）、70（dept 2）
/// - customers：1 owner=A；2 owner=B（同部门他人）；3 owner=C（跨部门）；
///   4 owner=0（公海，触发器置 department_id=NULL）
/// - crm_opportunity：101/102/103 全挂客户 1（owner 分别 A/B/C → 部门 1/1/2）；
///   104 挂公海客户 4、owner=0
/// - roles 98（先删后插，密封表幂等）+ data_permissions 98（resource=customer，
///   hidden=["tax_id"]，allowed NULL=不建白名单）+ field_permissions 98
///   （customer_industry 无读权限 → 配置层整键移除）
async fn seeded_db() -> Arc<DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'w9_sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'w9_sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'w9_sales_c','x',TRUE,FALSE,2,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(&db, "DELETE FROM roles WHERE id=98").await;
    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (98,'波九探针','w9_probe',FALSE,'dept','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        &format!(
            "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             contact_phone,contact_email,address,tax_id,customer_industry,
             credit_limit,payment_terms,status,customer_type,owner_id,created_at,updated_at) VALUES
             (1,'CUS-W9-0001','波九客户A','张三','{A_PHONE}','{A_EMAIL}','{A_ADDRESS}',
              'TAX-A-0001','面料',0,30,'active','retail',{USER_A},
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (2,'CUS-W9-0002','波九客户B','李四','{B_PHONE}','{B_EMAIL}','{B_ADDRESS}',
              'TAX-B-0002','针织',0,30,'active','retail',{USER_B},
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (3,'CUS-W9-0003','波九客户C','王五','{C_PHONE}','carol@example.com',
              '广东省广州市丙路3号','TAX-C-0003','纱线',0,30,'active','retail',{USER_C},
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (4,'CUS-W9-0004','波九公海客户','无主','{POOL_PHONE}','{POOL_EMAIL}','{POOL_ADDRESS}',
              'TAX-D-0004','杂项',0,30,'active','retail',0,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
        ),
    )
    .await;
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
    )
    .await;
    insert_opp(
        &db,
        OPP_A_ID as i32,
        1,
        USER_A,
        OPP_A_EST,
        Some(OPP_A_ACT),
        1,
    )
    .await;
    insert_opp(&db, OPP_B_ID as i32, 1, USER_B, OPP_B_EST, None, 2).await;
    insert_opp(
        &db,
        OPP_C_ID as i32,
        1,
        USER_C,
        OPP_C_EST,
        Some(OPP_C_ACT),
        3,
    )
    .await;
    insert_opp(&db, OPP_POOL_ID as i32, 4, 0, OPP_POOL_EST, None, 4).await;
    // data_permissions/field_permissions 属业务表（逐用例清空），仍按同 id 先删后插
    // 保持幂等，防止与迁移可能播种的行相撞。
    exec(&db, "DELETE FROM data_permissions WHERE id=98").await;
    exec(
        &db,
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
         allowed_fields,hidden_fields,is_enabled,created_at,updated_at) VALUES
         (98,98,'customer','DEPT',NULL,'[\"tax_id\"]',TRUE,
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(&db, "DELETE FROM field_permissions WHERE id=98").await;
    exec(
        &db,
        "INSERT INTO field_permissions (id,role_id,resource_type,field_name,
         can_read,can_write,mask_strategy,is_enabled,created_at,updated_at) VALUES
         (98,98,'customer','customer_industry',FALSE,FALSE,'MASK',TRUE,
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
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

/// 被测出口一次性挂全（360 + 客户域标准列表/详情 + 商机列表；各用例换 auth）
fn build_app(db: &Arc<DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/customers", get(standard_list_customers))
        .route("/erp/crm/customers/{id}", get(standard_get_customer))
        .route("/erp/crm/customers/{id}/360", get(get_customer_360))
        .route("/erp/crm/opportunities", get(list_opportunities))
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

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    send(app, Method::GET, uri).await
}

/// 失败信封唯一形状锁（AppError：code/message/trace_id/timestamp，无第五键）
fn assert_error_envelope_shape(v: &Value) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("失败信封必须是 JSON object: {v}"));
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["code", "message", "timestamp", "trace_id"],
        "失败信封键集合必须恰为 code/message/trace_id/timestamp，实际: {keys:?}"
    );
}

fn assert_permission_denied(status: StatusCode, v: &Value, where_label: &str) {
    assert_eq!(status, StatusCode::FORBIDDEN, "{where_label}：应 403: {v}");
    assert_eq!(
        v["code"],
        serde_json::json!("FORBIDDEN"),
        "{where_label}：机器码契约: {v}"
    );
    assert_eq!(
        v["message"],
        serde_json::json!(err_msg::PERMISSION_PUBLIC),
        "{where_label}：权限拒绝出参永久脱敏，恒为固定常量，不外显归属人/判定原因"
    );
    assert_error_envelope_shape(v);
}

async fn list_customer_rows(app: &Router) -> Vec<Value> {
    let (status, v) = get_json(app, "/erp/crm/customers?page=1&page_size=50").await;
    assert_eq!(status, StatusCode::OK, "标准客户列表 200 契约: {v}");
    v["data"]["items"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("客户列表形状漂移（期望 data.items 数组）: {v}"))
}

fn row_by_id(rows: &[Value], id: i64) -> Value {
    rows.iter()
        .find(|r| r["id"].as_i64() == Some(id))
        .cloned()
        .unwrap_or_else(|| panic!("缺行 id={id}（按主键定位，不依赖排序）: {rows:?}"))
}

async fn customer_360(app: &Router, id: i32) -> (StatusCode, Value) {
    get_json(app, &format!("/erp/crm/customers/{id}/360")).await
}

fn briefs_of(v: &Value) -> Vec<Value> {
    v["data"]["opportunities"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("360 出参缺 opportunities 数组: {v}"))
}

fn sorted_ids(rows: &[Value]) -> Vec<i64> {
    let mut ids: Vec<i64> = rows.iter().filter_map(|r| r["id"].as_i64()).collect();
    ids.sort_unstable();
    ids
}

async fn opp_list_rows(app: &Router) -> Vec<Value> {
    let (status, v) = get_json(app, "/erp/crm/opportunities?page=1&page_size=100").await;
    assert_eq!(status, StatusCode::OK, "商机列表 200 契约: {v}");
    v["data"]["data"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"))
}

fn opp_rows_for_customer(rows: &[Value], customer_id: i64) -> Vec<Value> {
    rows.iter()
        .filter(|r| r["customer_id"].as_i64() == Some(customer_id))
        .cloned()
        .collect()
}

/// 金额可见签名：键缺失=被剔除、null=库中无值；两种形态都必须与"原文字符串"区分
/// （与 wave8 同源口径，用于两侧等式而非弱断言）
fn amount_signature(row: &Value) -> (String, String) {
    let pick = |col: &str| -> String {
        match row.get(col) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => panic!("金额列序列化形状漂移（期望 JSON 字符串/null）: {other}"),
        }
    };
    (pick("estimated_amount"), pick("actual_amount"))
}

/// 出参不得含任何给定原文 PII（无论键名、无论嵌套）
fn assert_no_raw(value: &Value, banned_values: &[&str], where_label: &str) {
    let raw = value.to_string();
    for banned in banned_values {
        assert!(
            !raw.contains(banned),
            "{where_label}：响应体含未脱敏个人信息 {banned}（读侧旁路回潮）\n出参: {raw}"
        );
    }
}

// ---------------------------------------------------------------------------
// G-A 1 · 配权限行角色：360 customer 与列表端点**逐列等式**（他人行/本人行双向）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w9_ga_360_customer_columns_strictly_equal_list_exit_for_role_with_config() {
    let db = seeded_db().await;
    // 角色 98：data_permissions(customer, hidden=[tax_id]) + field_permissions(
    // customer_industry 无读权限)。选 All + 该角色的原因（如实记录既有口径）：
    // customers 列表在 Dept 分支是历史组合 `(self∪dept) AND pool`（Scoped，
    // utils/data_scope.rs::build_department_scope_with_pool_condition，可见面待拍板），
    // Dept 用户在列表拿不到任何私海行——列对比取 All（All 亦跳过查询层默认下推，
    // 两侧出参完全落在同一条出参门链上，等式最有区分力）。
    let app = build_app(&db, make_auth(USER_A, Some(ROLE_PROBE), "all", None, None));

    let list = list_customer_rows(&app).await;
    let (status, v) = customer_360(&app, 2).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "All 范围读他人客户 360 应 200（行门维持，收紧的是列）: {v}"
    );
    let cust = v["data"]["customer"].clone();
    assert_eq!(cust["id"].as_i64(), Some(2));

    // 【逐列等式】PII/配置列：列表行与 360 customer 完全一致（含 None≡None）
    for col in [
        "contact_phone",
        "contact_email",
        "address",
        "tax_id",
        "customer_industry",
    ] {
        assert_eq!(
            row_by_id(&list, 2).get(col),
            cust.get(col),
            "G-A 等式：列 {col} 在列表与 360 所见不一致（第二口径/旁路回潮）\nlist={}\n360={cust}",
            row_by_id(&list, 2)
        );
    }
    // 【方向断言】防"两处一起错"：不是双双缺键蒙混，两侧都必须钉到已知掩码常量
    for (exit, row) in [("列表", row_by_id(&list, 2)), ("360", cust.clone())] {
        assert_eq!(
            row["contact_phone"],
            json!(B_PHONE_MASKED),
            "{exit}：他人行 contact_phone 必须为客户域默认门已知掩码值（掩码保留键契约）"
        );
        assert_eq!(
            row["contact_email"],
            json!(B_EMAIL_MASKED),
            "{exit}：他人行 contact_email 必须为已知掩码值"
        );
        assert!(
            row.get("address").is_none(),
            "{exit}：非 admin address 必须整键移除: {row}"
        );
        assert!(
            row.get("tax_id").is_none(),
            "{exit}：data_permissions hidden=[tax_id] 必须生效（整键移除）: {row}"
        );
        assert!(
            row.get("customer_industry").is_none(),
            "{exit}：field_permissions 无读权限列必须被配置层整键移除（360 未接该层即在此变红——\
             若只接默认门，此列在 360 仍是原文）: {row}"
        );
    }
    assert_no_raw(&v["data"], &ALL_B_PHONE_RAW, "360 customer（他人行）");

    // 【本人行不受误伤】own 行 200、行在、非 PII 键完整、PII 列与列表同样逐列相等
    let (status, v_own) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner 读自己客户 360 必须 200（收紧误伤即在此变红）: {v_own}"
    );
    let own = v_own["data"]["customer"].clone();
    assert_eq!(
        own["customer_name"],
        json!("波九客户A"),
        "非 PII 列不得被误删"
    );
    let own_list = row_by_id(&list, 1);
    for col in [
        "contact_phone",
        "contact_email",
        "address",
        "tax_id",
        "customer_industry",
    ] {
        assert_eq!(
            own_list.get(col),
            own.get(col),
            "G-A 等式（本人行）：列 {col} 列表与 360 不一致"
        );
    }
    // 客户域默认掩码按角色不按归属（列表/详情改造前即如此）：本人行在两侧同为掩码值
    assert_eq!(
        own["contact_phone"],
        json!(A_PHONE_MASKED),
        "本人行与列表同形（掩码），不得原文也不得更严消失"
    );

    // 跨部门客户的**行级**门（360 出口独立于列表可见面组合）：Dept 用户（可见部门 [1]）
    // 读 department_id=2 的客户 → 403；拒绝出参仅 status+信封 code+固定脱敏常量。
    let dept_app = build_app(&db, dept_auth(USER_A, Some(ROLE_PROBE)));
    let (status, v) = customer_360(&dept_app, 3).await;
    assert_permission_denied(status, &v, "dept 用户读跨部门客户 360");
}

// ---------------------------------------------------------------------------
// G-A 2 · 未配权限行角色：360 走客户域默认门（钉确定形态，两侧均无原文）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w9_ga_default_gate_shape_for_role_without_config_and_fail_closed() {
    let db = seeded_db().await;
    // 角色 2：无任何 customer 权限行 → 默认脱敏层恒定执行。
    let app = build_app(&db, dept_auth(USER_A, Some(ROLE_SEED_NONADMIN)));

    // 他人（同部门）行：360 可读（行门 Dept 放行），列必须钉到默认门确定形态
    let (status, v) = customer_360(&app, 2).await;
    assert_eq!(status, StatusCode::OK, "同部门他人客户 360 可读: {v}");
    let cust = v["data"]["customer"].clone();
    assert_eq!(
        cust["contact_phone"],
        json!(B_PHONE_MASKED),
        "默认门掩码保留键契约（钉常量非弱断言）"
    );
    assert_eq!(
        cust["contact_email"],
        json!(B_EMAIL_MASKED),
        "默认门邮箱掩码契约"
    );
    assert!(
        cust.get("address").is_none(),
        "非 admin address 整键移除契约"
    );
    assert_no_raw(&v["data"], &ALL_B_PHONE_RAW, "360 customer（无权限行角色）");

    // 本人行防误伤：360 200、contact_phone 同为已知掩码（客户域默认门按角色不按归属，
    // 与列表/详情改造前契约一致），非 PII 键完整
    let (status, v_own) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "本人行 360 防误伤: {v_own}");
    let own = v_own["data"]["customer"].clone();
    assert_eq!(
        own["contact_phone"],
        json!(A_PHONE_MASKED),
        "本人行默认门掩码契约"
    );
    assert_eq!(
        own["contact_email"],
        json!(A_EMAIL_MASKED),
        "本人行邮箱默认门掩码契约（防误伤=掩码保留键，非原文非缺键）"
    );
    assert_eq!(
        own["customer_name"],
        json!("波九客户A"),
        "非 PII 列不得被误删"
    );

    // 同角色标准列表（self 范围取本人行；Dept 分支列表是历史组合 `(self∪dept) AND
    // pool`——本域可见面既有待拍板项，不在本波顺带，见 utils/data_scope.rs Scoped 注释）：
    // 无权限行角色在查询层按 DEFAULT_HIDDEN_FIELDS 整列下推 → 键缺失是既有形态；
    // 若形态改为原文（放松）即红。
    let self_app = build_app(
        &db,
        make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "self", None, None),
    );
    let list = list_customer_rows(&self_app).await;
    let row1 = row_by_id(&list, 1);
    match row1.get("contact_phone") {
        None | Some(Value::Null) => {}
        Some(Value::String(s)) => assert_eq!(
            s.as_str(),
            A_PHONE_MASKED,
            "列表侧 contact_phone 只允许 [键缺失(既有下推形态) 或 已知掩码] 两种确定形态"
        ),
        Some(other) => panic!("列表 contact_phone 形状异常: {other}"),
    }
    assert_no_raw(&json!(list), &ALL_A_PHONE_RAW, "标准列表（无权限行角色）");
    assert_no_raw(
        &json!(list),
        &ALL_B_PHONE_RAW,
        "标准列表（无权限行角色·他人）",
    );

    // fail-closed：role_id 缺失 → 360 默认层恒定脱敏（第 2 层配置判定跳过）
    let app_none = build_app(&db, dept_auth(USER_A, None));
    let (status, v) = customer_360(&app_none, 1).await;
    assert_eq!(status, StatusCode::OK, "role 缺失不影响可读性: {v}");
    assert_eq!(
        v["data"]["customer"]["contact_phone"],
        json!(A_PHONE_MASKED),
        "role_id 缺失必须 fail-closed 走默认脱敏（原文直通即旁路）"
    );
    assert_no_raw(
        &v["data"],
        &ALL_A_PHONE_RAW,
        "360 customer（role 缺失 fail-closed）",
    );
}

// ---------------------------------------------------------------------------
// G-B · 360 商机子集行级 scope 与列表逐 id 相等（self/dept/all 三态）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w9_gb_dept_user_opp_rowset_equals_list() {
    let db = seeded_db().await;
    let app = build_app(&db, dept_auth(USER_A, Some(ROLE_SEED_NONADMIN)));

    let list = opp_list_rows(&app).await;
    let list_mine = opp_rows_for_customer(&list, 1);
    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "owner 读 360 应 200: {v}");
    let briefs = briefs_of(&v);

    // 【等式】行集：360 子集与列表（同客户）逐 id 相等；跨部门 103 两处都不得出现
    assert_eq!(
        sorted_ids(&briefs),
        sorted_ids(&list_mine),
        "G-B 等式：dept 用户 360 商机行集与列表不一致（行级旁路回潮）\nlist={list_mine:?}\n360={briefs:?}"
    );
    assert_eq!(
        sorted_ids(&briefs),
        [OPP_A_ID, OPP_B_ID],
        "dept 可见集基线：仅 dept1 行"
    );
    let raw = v.to_string();
    assert!(
        !raw.contains("波九商机-103"),
        "他部门商机行存在性/标题不得经 360 外显（行级旁路）: {raw}"
    );

    // 【字段级仍与列表同源】共享行金额签名两侧相等 + 方向断言（本人行原值、他人行剔）
    for id in [OPP_A_ID, OPP_B_ID] {
        assert_eq!(
            amount_signature(&row_by_id(&list_mine, id)),
            amount_signature(&row_by_id(&briefs, id)),
            "商机 {id}：360 与列表金额口径不一致（D-1/G-B 双门任一失守）"
        );
    }
    assert_eq!(
        amount_signature(&row_by_id(&briefs, OPP_A_ID)),
        (amount_text(OPP_A_EST), amount_text(OPP_A_ACT)),
        "本人行金额原值契约（防误伤）"
    );
    assert_eq!(
        amount_signature(&row_by_id(&briefs, OPP_B_ID)),
        (String::new(), String::new()),
        "同部门他人行金额整键移除（默认门）"
    );
}

#[tokio::test]
async fn w9_gb_self_user_opp_rowset_equals_list() {
    let db = seeded_db().await;
    let app = build_app(
        &db,
        make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "self", None, None),
    );

    let list_mine = opp_rows_for_customer(&opp_list_rows(&app).await, 1);
    let (_status, v) = customer_360(&app, 1).await;
    let briefs = briefs_of(&v);
    assert_eq!(
        sorted_ids(&briefs),
        sorted_ids(&list_mine),
        "G-B 等式：self 用户 360 商机行集与列表不一致"
    );
    assert_eq!(
        sorted_ids(&briefs),
        [OPP_A_ID],
        "self 仅本人行；他人行标题/阶段不得外显: {v}"
    );
    assert!(
        !v.to_string().contains("波九商机-102"),
        "他人行存在性外显回潮: {v}"
    );
}

#[tokio::test]
async fn w9_gb_all_user_rowset_equals_list_and_amount_gate_still_applies() {
    let db = seeded_db().await;
    let app = build_app(
        &db,
        make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "all", None, None),
    );

    let list_mine = opp_rows_for_customer(&opp_list_rows(&app).await, 1);
    let (_status, v) = customer_360(&app, 1).await;
    let briefs = briefs_of(&v);
    // All 范围：列表与 360 同为"无行过滤"（等式两侧同形），金额门仍剔非本人行
    assert_eq!(
        sorted_ids(&briefs),
        sorted_ids(&list_mine),
        "G-B 等式：All 用户 360 行集应与列表一致（All 不添加过滤也不等于放松金额门）"
    );
    assert_eq!(sorted_ids(&briefs), [OPP_A_ID, OPP_B_ID, OPP_C_ID]);
    for id in [OPP_B_ID, OPP_C_ID] {
        assert_eq!(
            amount_signature(&row_by_id(&briefs, id)),
            amount_signature(&row_by_id(&list_mine, id)),
            "商机 {id} 金额签名两处必须相等"
        );
        assert_eq!(
            amount_signature(&row_by_id(&briefs, id)),
            (String::new(), String::new()),
            "All 范围非本人行金额仍必须整键移除（行在≠金额在）"
        );
    }
    assert_eq!(
        amount_signature(&row_by_id(&briefs, OPP_A_ID)),
        (amount_text(OPP_A_EST), amount_text(OPP_A_ACT)),
        "本人行原值契约"
    );
}

// ---------------------------------------------------------------------------
// 公海行（owner_id=0）在 360 各门的实际形态（如实钉住）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn w9_pool_customer_shape_across_all_gates() {
    let db = seeded_db().await;

    // Self：客户行门 0≠本人 → 403（仅 status+code+固定脱敏常量）
    let app = build_app(
        &db,
        make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "self", None, None),
    );
    let (status, v) = customer_360(&app, 4).await;
    assert_permission_denied(status, &v, "self 读公海客户 360");

    // Dept：触发器对公海行置 department_id=NULL → None → 拒（与既有读门语义逐字一致）
    let app = build_app(&db, dept_auth(USER_A, Some(ROLE_SEED_NONADMIN)));
    let (status, v) = customer_360(&app, 4).await;
    assert_permission_denied(status, &v, "dept 读公海客户 360");

    // All 非 admin：200（读侧 All 不变）；customer 走默认掩码；公海商机行随列表
    // 同形出现（All 无行过滤），金额按"非本人行"（owner 0≠本人）剔除——如实钉住。
    let app = build_app(
        &db,
        make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "all", None, None),
    );
    let (status, v) = customer_360(&app, 4).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "All 读公海客户 360 维持 200（读门语义未放松未收紧）: {v}"
    );
    let cust = v["data"]["customer"].clone();
    assert_eq!(
        cust["contact_phone"],
        json!(POOL_PHONE_MASKED),
        "公海行 PII 同样过默认门（无原文）"
    );
    assert!(cust.get("address").is_none(), "公海行 address 整键移除");
    assert_no_raw(
        &v["data"],
        &[POOL_PHONE, POOL_EMAIL, POOL_ADDRESS],
        "360（All 非 admin）",
    );
    let briefs = briefs_of(&v);
    assert_eq!(
        sorted_ids(&briefs),
        [OPP_POOL_ID],
        "All 行集=列表行集（公海商机行在）"
    );
    assert_eq!(
        amount_signature(&row_by_id(&briefs, OPP_POOL_ID)),
        (String::new(), String::new()),
        "公海商机行（owner_id=0）对 All 非 admin 用户按非本人剔金额（实际形态钉桩）"
    );
    assert_eq!(
        briefs[0]["opportunity_name"],
        json!("波九商机-104"),
        "All 用户仍可见公海行标题（行级过滤 All 分支=不添加，与列表逐字同形）"
    );

    // All admin（role 1）：customer 原文 + 金额原文（admin 例外契约维持，收紧误伤即红）
    let app = build_app(&db, make_auth(USER_A, Some(1), "all", None, None));
    let (status, v) = customer_360(&app, 4).await;
    assert_eq!(status, StatusCode::OK, "admin 读公海客户 360: {v}");
    assert_eq!(
        v["data"]["customer"]["contact_phone"],
        json!(POOL_PHONE),
        "admin customer 原值契约"
    );
    assert_eq!(
        v["data"]["customer"]["address"],
        json!(POOL_ADDRESS),
        "admin address 原值契约（不得移除）"
    );
    assert_eq!(
        amount_signature(&row_by_id(&briefs_of(&v), OPP_POOL_ID)),
        (amount_text(OPP_POOL_EST), String::new()),
        "admin 金额原值契约（默认处理不进 admin 分支）"
    );
}

// ---------------------------------------------------------------------------
// 源码棘轮（shrink-only）：360 出口必须共用同一实现；复制第二套判定/字面量掩码/
// unwrap_or 回落 / 服务层内联字段门 即红
// ---------------------------------------------------------------------------

/// 剔全部空白并消掉闭合定界符前的尾逗号（防 rustfmt 换行/尾逗号改变被锁调用形状；
/// 与 wave8 read_gate 同源 canon）
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

fn impl_fn_body(src: &str, name: &str) -> String {
    src.split(&format!("pub async fn {name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("方法 {name} 定义缺失（改名/删除须同步本棘轮）"))
        .split("\n    pub async fn ")
        .next()
        .expect("方法体边界")
        .to_string()
}

#[test]
fn w9_360_exit_shares_same_gates_source_ratchet() {
    // 1) 360 handler：客户行两层门必须调**共享实现**（canon 比对，rustfmt 换行免疫），
    //    商机字段门保持与列表同源，且不得内联任何第二套判定/字面量掩码/回落兜底。
    let handler = include_str!("../src/handlers/crm_handler.rs").replace('\r', "");
    let body = top_fn_body(&handler, "get_customer_360");
    let c = canon(&body);
    assert!(
        c.contains(
            "crate::handlers::customer_handler::apply_customer_field_config_mask(\
             &state,auth.role_id,std::slice::from_mut(customer)).await"
        ),
        "G-A 回潮棘轮：360 不再挂 field_permissions 配置层共享实现（与列表口径分叉/原文直通复活）"
    );
    assert!(
        c.contains(
            "crate::handlers::crm_customer_handler::apply_customer_field_permission(\
             &state,auth.role_id,std::slice::from_mut(customer)).await"
        ),
        "G-A 回潮棘轮：360 customer 行不再挂客户域字段级权限唯一实现（第二出口旁路复活）"
    );
    assert!(
        c.contains(
            "apply_opportunity_field_permission(&state,auth.role_id,auth.user_id,opps).await"
        ),
        "D-1 回潮棘轮镜像：360 商机字段门与列表共用实现的接线被拆"
    );
    for banned in [
        "remove(",
        "filter_fields_batch",
        "mask_contact_fields_for_role",
        "DEFAULT_HIDDEN_FIELDS",
        "EXPORT_AMOUNT_COLUMNS",
        "\"***\"",
        "138****",
        "unwrap_or(",
    ] {
        assert!(
            !body.contains(banned),
            "G-A 回潮棘轮：360 出口出现复制的第二套判定/字面量掩码/回落兜底 → {banned}"
        );
    }
    assert_eq!(
        body.matches("tracing::error!").count(),
        3,
        "customer/opportunities/shipping_addresses 三处门定位失败都必须显式记 error（静默跳过形状漂移即缺陷）"
    );

    // 2) 服务层：360 商机子集必须复用列表同一个 apply_department_scope（同判据列），
    //    且不得把字段级金额门/客户门复制进服务层（字段门归 handler，行门归查询）。
    let cust = include_str!("../src/services/crm/cust.rs").replace('\r', "");
    let cust_body = impl_fn_body(&cust, "get_customer_360");
    assert!(
        canon(&cust_body).contains(
            "apply_department_scope(opps_query,ctx,crm_opportunity::Column::OwnerId,\
             crm_opportunity::Column::DepartmentId)"
        ),
        "G-B 回潮棘轮：360 商机子集不再与列表复用同一 apply_department_scope（行级旁路复活）"
    );
    for banned in [
        "estimated_amount",
        "actual_amount",
        "apply_opportunity_field_permission",
        "apply_customer_field_permission",
        "apply_customer_field_config_mask",
    ] {
        assert!(
            !cust_body.contains(banned),
            "G-B/G-A 分层棘轮：服务层 360 出现字段门复制形态 → {banned}（出参门唯一归口 handler）"
        );
    }

    // 3) 客户域标准入口：配置层判定收敛为单一定义（纯提取后原两处内联清零），
    //    列表/详情/360 三个出口调用同一实现；`list_field_permissions` 在本文件
    //    仅允许出现在共享实现内。
    let standard = include_str!("../src/handlers/customer_handler.rs").replace('\r', "");
    assert_eq!(
        standard
            .matches("list_field_permissions(Some(\"customer\")")
            .count(),
        1,
        "field_permissions 判定必须单实现（出现第二处查询=第二套口径回潮）"
    );
    assert_eq!(standard.matches(".mask_fields(").count(), 1);
    assert_eq!(
        standard
            .matches(".filter_fields_by_read_permission(")
            .count(),
        1
    );
    let list_body = top_fn_body(&standard, "list_customers");
    let detail_body = top_fn_body(&standard, "get_customer");
    assert!(
        canon(&list_body).contains(
            "apply_customer_field_config_mask(&state,auth.role_id,&mutmasked_items).await"
        ),
        "回潮棘轮：标准列表不再调共享配置层实现"
    );
    assert!(
        canon(&list_body).contains(
            "apply_customer_field_permission(&state,auth.role_id,&mutmasked_items).await"
        ),
        "回潮棘轮：标准列表又脱离客户域字段级权限唯一实现"
    );
    assert!(
        canon(&detail_body).contains(
            "apply_customer_field_config_mask(&state,auth.role_id,\
             std::slice::from_mut(&mutcustomer_json)).await"
        ),
        "回潮棘轮：标准详情不再调共享配置层实现"
    );
    for body in [list_body.as_str(), detail_body.as_str()] {
        for banned in [
            "field_perm_svc",
            ".mask_fields(",
            ".filter_fields_by_read_permission(",
        ] {
            assert!(
                !body.contains(banned),
                "回潮棘轮：客户域标准出口又内联配置层判定 → {banned}"
            );
        }
    }
}
