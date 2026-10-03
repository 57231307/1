//! 契约波次 8 · CRM 读侧旁路与归属列收口（PR #942 四条已定位缺陷 D-1~D-4）
//!
//! 四条缺陷与对应锁（详见各用例文档）：
//! - **D-1** `get_customer_360` 内嵌"商机简报"不走 `apply_opportunity_field_permission`
//!   → 列表隐藏的 `estimated_amount`/`actual_amount` 换出口整份可读（读侧旁路）。
//!   锁：同一账号同一数据，列表与 360 逐行金额可见性必须相等，且双向都有方向断言
//!   （他人行两处都隐藏 / 本人行两处都原值——360 若因简报缺 `owner_id` 而被
//!   fail-closed 全剔，本人行方向同样打红，逼简报投影与列表 owner 判据同源）。
//! - **D-2** `services/crm/cust.rs` 行级归属用可空审计列 `created_by` 而非权威列
//!   `owner_id`（RLS 归属列口径见 `handlers/crm_write_guard.rs` 文件头与
//!   `migration/src/domain/rls_dept/mod.rs:74`）→ 转派后现任 owner 被拒（功能坏）、
//!   原创建者越权（写通道）。dept 参数过去恒传 `None`（Dept 范围一律误拒）。
//!   锁：owner≠created_by 交叉行（owner=50/created_by=80）上，self 现任 owner 读 360
//!   与跟进 → 200；self 原创建者 → 403；dept 用户（department_id ∈ 可见集合）→ 200
//!   （钉 dept 参传真实列后 Dept 分支恢复语义）；公海行 owner_id=0 对 self 原创建者
//!   fail-closed 403（旧代码按 created_by 会放行）。
//! - **D-3** `convert_opportunity_to_order` 内 `get_opportunity(id, None)` 跳过行级
//!   读门、且无写门 → 跨主把他人商机转成销售订单（派生落库带对方金额）。
//!   锁：All 范围无 `crm/cross_owner_write` 键转他人商机 → 403+FORBIDDEN+固定脱敏
//!   常量 + **零写入**（订单不生成、商机状态不变）；本人行 → 200（功能不伤）；
//!   显式授键后 → 200（跨主放行只走既有键通道）。
//! - **D-4** admin 例外双源：`if rid == 1`（角色主键字面量）vs 权威源
//!   `admin_checker::is_admin_role`（roles.code='admin'）。字面量在播种漂移时
//!   静默失效/静默扩权两头坏。锁：源码棘轮——apply 门体内必须调
//!   `admin_checker::is_admin_role(&state.db, rid)` 且 `rid == 1` / `role_id != 1`
//!   字面量在本文件清零；功能面把"code='admin' 即 admin（与 id 无关）""all 范围但
//!   code≠'admin' 的角色不享受豁免"两条契约钉住。
//!
//! 断言口径（本批已锁）：失败只断 HTTP status + 信封 code（权限族=403/FORBIDDEN，
//! 属业务失败族、非 5xx）与固定脱敏常量 `err_msg::PERMISSION_PUBLIC`，不断言/不外显
//! 拒绝原因；成功金额按 Decimal 序列化字符串（DECIMAL(15,2) → "111111.00"）钉原值。
//!
//! 通道（路线一）：`test_common::setup_test_db()` 真 PostgreSQL 真跑 + 真 HTTP 装配，
//! 表结构唯一来源 = backend/migration；users/customers/crm_opportunity 自种子
//! （FK 父行先插）；roles/role_permissions 属**密封参照表**（不参与逐用例 TRUNCATE，
//! 见 `services/test_common.rs::SEALED_REFERENCE_TABLES`），本文件用到的自造角色/
//! 授权行一律"先删后插"保持幂等，且仅用高段 id（97/98/99）避免与迁移种子行
//! （id=1 admin / id=2 非 admin）互踩。禁 sqlite、无自建 DDL、无 #[ignore]。

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
use bingxi_backend::handlers::crm_handler::{
    convert_opportunity_to_order, get_customer_360, list_follow_ups, list_opportunities,
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
// 夹具常量（与 contract_wave7_crm_opp_amount_scope_test.rs 同谱系）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50; // 客户 1 的现任 owner（owner_id=50）
const USER_B: i32 = 60; // 他人商机归属人
const USER_EX_CREATOR: i32 = 80; // 客户 1 的原创建者（created_by=80，已非 owner）
const USER_DEPT_MGR: i32 = 55; // 同部门经理（dept 范围，自身不持行）
const USER_ADMIN_SEED: i32 = 70; // 走迁移种子 admin（role_id=1）

const ROLE_SEED_NONADMIN: i32 = 2; // 迁移种子非 admin 角色（data_permissions FK 父行）
const ROLE_PROXY_ALL: i32 = 97; // 自造 all 范围非 admin 角色（D-3 代操作键试验田）
const ROLE_CODE_ADMIN: i32 = 99; // 自造 code='admin' 角色（D-4：admin 与主键无关）
const ROLE_CODE_LEADER: i32 = 98; // 自造非 admin 的 all 范围角色（D-4 对照）

const OPP_A1_EST: i64 = 111_111;
const OPP_A1_ACT: i64 = 122_222;
const OPP_A2_EST: i64 = 133_333;
const OPP_B3_EST: i64 = 88_888;
const OPP_B3_ACT: i64 = 66_666;
const OPP_B4_EST: i64 = 55_555;
const OPP_X_EST: i64 = 210_001; // 交叉行：owner=USER_A / created_by=USER_B
const OPP_X_ACT: i64 = 210_002;
const OPP_Y_EST: i64 = 210_003; // 交叉行：owner=USER_B / created_by=USER_A
const OPP_Y_ACT: i64 = 210_004;

fn amount(raw: i64) -> Decimal {
    Decimal::from(raw)
}

/// 真列 DECIMAL(15,2) 的 Decimal 序列化形状（与 wave7 同源口径）
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

fn make_auth(user_id: i32, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave8_user_{user_id}"),
        role_id,
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
        dept_ids: Some(Arc::new("1".to_string())),
        dept_member_user_ids: Some(Arc::new("50,60".to_string())),
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

/// 商机种子：`owner_id` 与 `creator_id` 是两个独立入参（D-1/D-2 的区分力全在
/// 交叉形态上——未种该形态时按 created_by 判归属与按 owner_id 判归属完全等价）。
async fn insert_opp(
    db: &DatabaseConnection,
    id: i32,
    owner_id: i32,
    creator_id: i32,
    estimated: i64,
    actual: Option<i64>,
    day: u32,
) {
    crm_opportunity::ActiveModel {
        id: Set(id),
        opportunity_no: Set(format!("W8OPP{id:03}")),
        opportunity_name: Set(format!("波次八商机-{id}")),
        customer_id: Set(1),
        lead_id: Set(None),
        opportunity_type: Set(Some("NEW".to_string())),
        opportunity_stage: Set(Some("QUALIFICATION".to_string())),
        win_probability: Set(Some(Decimal::from(10))),
        estimated_amount: Set(Some(amount(estimated))),
        actual_amount: Set(actual.map(amount)),
        currency: Set(Some("CNY".to_string())),
        expected_close_date: Set(Some(ts(day).date_naive())),
        actual_close_date: Set(None),
        product_ids: Set(None),
        product_names: Set(None),
        product_desc: Set(None),
        owner_id: Set(owner_id),
        department_id: Set(Some(1)),
        owner_name: Set(format!("wave8_owner_{owner_id}")),
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
        created_by: Set(Some(creator_id)),
        updated_by: Set(Some(creator_id)),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("商机种子插入失败 id={id}: {e}"));
}

/// 标准种子：
/// - users 50/60/80/55/70（department_id=1，trg_*_dept 触发器回填行部门用）
/// - customers id=1：owner_id=50、created_by=80（**归属已转移**形态，D-2 靶子）
/// - customers id=2：owner_id=0（公海）、created_by=50（公海 fail-closed 靶子；
///   触发器对 owner_id=0 置 department_id=NULL，见 rls_dept/mod.rs:45-49）
/// - crm_opportunity 6 行（全部挂客户 1）：1/2 owner=A；3/4 owner=B；
///   5 owner=A creator=B；6 owner=B creator=A（交叉形态，D-1 owner 判据同源靶子）
///
/// 客户 360 的商机子集**不做行级 scope 过滤**（本轮未扩该契约，见文件头），
/// dept/self 用户都能取到全 6 行——正因如此字段级金额门是否生效才有对比面。
async fn seeded_db() -> Arc<DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (80,'ex_creator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (55,'dept_mgr','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (70,'admin_seed_u','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             credit_limit,payment_terms,status,customer_type,owner_id,created_by,
             created_at,updated_at) VALUES
             (1,'CUS-W8-0001','波次八客户','张三',0,30,'active','retail',50,80,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
             (2,'CUS-W8-0002','公海客户','李四',0,30,'active','retail',0,50,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
    )
    .await;
    insert_opp(&db, 1, USER_A, USER_A, OPP_A1_EST, Some(OPP_A1_ACT), 1).await;
    insert_opp(&db, 2, USER_A, USER_A, OPP_A2_EST, None, 2).await;
    insert_opp(&db, 3, USER_B, USER_B, OPP_B3_EST, Some(OPP_B3_ACT), 3).await;
    insert_opp(&db, 4, USER_B, USER_B, OPP_B4_EST, None, 4).await;
    insert_opp(&db, 5, USER_A, USER_B, OPP_X_EST, Some(OPP_X_ACT), 5).await;
    insert_opp(&db, 6, USER_B, USER_A, OPP_Y_EST, Some(OPP_Y_ACT), 6).await;
    Arc::new(db)
}

fn state_from(db: &Arc<DatabaseConnection>) -> AppState {
    AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    }
}

/// 被测四出口一次性挂全（同一 db，各用例换 auth 复用）
fn build_app(db: &Arc<DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/opportunities", get(list_opportunities))
        .route(
            "/erp/crm/opportunities/{id}/convert",
            post(convert_opportunity_to_order),
        )
        .route("/erp/crm/customers/{id}/360", get(get_customer_360))
        .route("/erp/crm/customers/{id}/follow-ups", get(list_follow_ups))
        .with_state(state_from(db))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
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

/// 失败信封唯一形状锁：AppError{code,message,trace_id,timestamp}，无第五键
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

async fn list_rows(app: &Router) -> Vec<Value> {
    let (status, v) = send(
        app,
        Method::GET,
        "/erp/crm/opportunities?page=1&page_size=100",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    assert_eq!(v["code"], serde_json::json!(200), "成功信封 code 契约: {v}");
    v["data"]["data"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"))
}

async fn customer_360(app: &Router, id: i32) -> (StatusCode, Value) {
    send(
        app,
        Method::GET,
        &format!("/erp/crm/customers/{id}/360"),
        json!({}),
    )
    .await
}

fn row_by_id(rows: &[Value], id: i64) -> Value {
    rows.iter()
        .find(|r| r["id"].as_i64() == Some(id))
        .unwrap_or_else(|| panic!("缺行 id={id}（按主键定位，不依赖排序）: {rows:?}"))
        .clone()
}

/// 金额可见签名：键缺失=被剔除、null=库中无值；两种形态都必须与"原文字符串"区分
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

/// DB 直读（不经被测代码路径）：订单行数
async fn count_orders(db: &DatabaseConnection) -> i64 {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT COUNT(*) FROM sales_orders",
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读 sales_orders 计数失败");
    assert_eq!(rows.len(), 1);
    rows[0]
        .try_get::<i64>("", "count")
        .expect("sales_orders 计数读取失败")
}

/// DB 直读：商机 opportunity_status
async fn opp_status(db: &DatabaseConnection, id: i32) -> String {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &format!("SELECT opportunity_status FROM crm_opportunity WHERE id={id}"),
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读商机状态失败");
    assert_eq!(rows.len(), 1);
    rows[0]
        .try_get::<Option<String>>("", "opportunity_status")
        .expect("opportunity_status 回读失败")
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// D-1：360 内嵌商机与列表端点共用同一个金额隐藏门（同账号同数据两处对比）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn customer_360_opp_subset_matches_list_amount_gate() {
    let db = seeded_db().await;
    // dept 非 admin（role 2 无 data_permissions 行 → 走"仅剔非本人行金额"默认处理）
    let app = build_app(&db, make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "dept"));

    let list = list_rows(&app).await;
    assert_eq!(list.len(), 6, "dept 可见集基线（6 行）");
    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "owner 读 360 应 200: {v}");
    let briefs = v["data"]["opportunities"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("360 出参缺 opportunities 数组: {v}"));
    assert_eq!(briefs.len(), 6, "360 商机子集行数基线");

    // 【逐行等式】同一账号：列表与 360 的金额外显必须一致（旁路=等式破坏处）
    for id in 1..=6i64 {
        let l = row_by_id(&list, id);
        let b = row_by_id(&briefs, id);
        assert_eq!(
            amount_signature(&l),
            amount_signature(&b),
            "商机 {id}：列表与 360 内嵌简报金额口径不一致（读侧旁路/第二口径回潮）\nlist={l}\n360={b}"
        );
    }

    // 【方向断言】防"两处一起错"的假绿（纯等式对双剔不设防）：
    // 本人行（owner=USER_A：1/2/5，含交叉行 5）两处都必须原值可见——
    // 360 若丢了简报 owner_id（判据无源 → fail-closed 全剔），此处即红。
    let own_expectations = [
        (1i64, Some(OPP_A1_EST), Some(OPP_A1_ACT)),
        (2, Some(OPP_A2_EST), None),
        (5, Some(OPP_X_EST), Some(OPP_X_ACT)),
    ];
    for (id, est, act) in own_expectations {
        let want = (
            est.map(amount_text).unwrap_or_default(),
            act.map(amount_text).unwrap_or_default(),
        );
        assert_eq!(
            amount_signature(&row_by_id(&list, id)),
            want.clone(),
            "列表本人行 {id} 金额原值契约（裁定 #2）"
        );
        assert_eq!(
            amount_signature(&row_by_id(&briefs, id)),
            want,
            "360 本人行 {id} 金额原值契约（owner 判据必须与列表同源=行 owner_id）"
        );
    }
    // 他人行（owner=USER_B：3/4/6，含交叉行 6：created_by=本人也必须剔）两处都整键移除
    for id in [3i64, 4, 6] {
        for (exit, rows) in [("列表", &list), ("360", &briefs)] {
            let row = row_by_id(rows, id);
            let sig = amount_signature(&row);
            assert_eq!(
                sig,
                (String::new(), String::new()),
                "{exit} 他人行 {id} 金额必须整键移除（旁路回潮即在此暴露）: {row}"
            );
        }
    }
    // 360 原文不得含他人行金额原文（任何位置的泄漏都判红）
    let raw = serde_json::to_string(&briefs).unwrap();
    for banned in [
        amount_text(OPP_B3_EST),
        amount_text(OPP_B3_ACT),
        amount_text(OPP_B4_EST),
        amount_text(OPP_Y_EST),
        amount_text(OPP_Y_ACT),
    ] {
        assert!(!raw.contains(&banned), "360 出参含他人行金额原文 {banned}");
    }
    // 简报行必须携带 owner_id（与列表同一判据列；缺列=门失去判据来源）
    assert_eq!(
        row_by_id(&briefs, 3)["owner_id"].as_i64(),
        Some(USER_B as i64),
        "360 简报行必须投影 owner_id（owner 判据同源）"
    );
}

// ---------------------------------------------------------------------------
// D-2：客户行级归属 = owner_id（+ department_id），不是 created_by
// ---------------------------------------------------------------------------

#[tokio::test]
async fn customer_gate_keys_owner_id_not_created_by() {
    // 现任 owner（50，self）：客户 1（owner=50，created_by=80）必须读得到——
    // 修复前按 created_by=80 判归属，self 范围的现任 owner 被 403（功能坏）。
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "self"));
    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "转派后现任 owner（self 范围）必须能读 360（归属判据=owner_id）: {v}"
    );

    // 原创建者（80，self，已非 owner）：修复前按 created_by=80 放行 → 越权读通道。
    let app = build_app(
        &db,
        make_auth(USER_EX_CREATOR, Some(ROLE_SEED_NONADMIN), "self"),
    );
    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "原创建者（非现任 owner）读他人名下客户 360 必须 403（越权读/写通道回潮判红点）: {v}"
    );
    assert_eq!(v["code"], serde_json::json!("FORBIDDEN"), "机器码契约: {v}");
    assert_eq!(
        v["message"],
        serde_json::json!(err_msg::PERMISSION_PUBLIC),
        "权限拒绝出参永久脱敏：恒为固定常量，不外显归属人/判定原因"
    );
    assert_error_envelope_shape(&v);

    // 同一判据的第二出口：跟进记录列表（cust.rs 两处同型调用点，防改一漏一）
    let (status, v) = send(
        &app,
        Method::GET,
        "/erp/crm/customers/1/follow-ups",
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "原创建者读他人客户跟进记录同样必须 403: {v}"
    );
    assert_eq!(v["code"], serde_json::json!("FORBIDDEN"));

    // dept 用户（55，可见部门 [1]）：客户 1 的 department_id 由 trg_customers_dept
    // 回填=1 → Dept 分支放行。修复前 dept 参数恒传 None ⇒ Dept 恒 false（误拒），
    // 本断言即"dept 参按真实列传 department_id"的落地锁。
    let app = build_app(
        &db,
        make_auth(USER_DEPT_MGR, Some(ROLE_SEED_NONADMIN), "dept"),
    );
    let (status, v) = customer_360(&app, 1).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "dept 用户对 department_id ∈ 可见集合的客户必须可读（dept 参传真实列）: {v}"
    );
}

#[tokio::test]
async fn pool_customer_owner_zero_denies_creator_read() {
    // 公海行（customers id=2：owner_id=0、created_by=50）：fail-closed 语义——
    // 修复前按 created_by 判归属，原创建者（self）可对公海行直读 360（越权通道）；
    // 修复后 owner_id=0 ≠ 任何本人 → Self 拒；Dept 用户因触发器对公海行置
    // department_id=NULL → None → 同样拒（公海行的进入/领取走 pool 自己的门）。
    let db = seeded_db().await;
    let app = build_app(&db, make_auth(USER_A, Some(ROLE_SEED_NONADMIN), "self"));
    let (status, v) = customer_360(&app, 2).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "公海行（owner_id=0）对 self 原创建者必须 fail-closed 403: {v}"
    );
    assert_eq!(v["code"], serde_json::json!("FORBIDDEN"));

    let app = build_app(
        &db,
        make_auth(USER_DEPT_MGR, Some(ROLE_SEED_NONADMIN), "dept"),
    );
    let (status, _v) = customer_360(&app, 2).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "公海行 department_id=NULL，dept 用户同样拒（读门 None 分支语义，不放松）"
    );

    // All 范围（迁移种子 admin role 1）：读门 All 恒真维持不变（方案 A 读侧不收紧）
    let app = build_app(&db, make_auth(USER_ADMIN_SEED, Some(1), "all"));
    let (status, _v) = customer_360(&app, 2).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "All 范围读公海客户 360 维持 200（读门语义逐字不变）"
    );
}

// ---------------------------------------------------------------------------
// D-3：商机转销售订单 = 派生落库的跨主写入口，读门+写门与 update/delete 同形
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cross_owner_conversion_denied_key_owner_and_grant_channels() {
    let db = seeded_db().await;
    // 自造 all 范围非 admin 角色；roles/role_permissions 为密封表：先删后插保幂等
    exec(
        &db,
        "DELETE FROM role_permissions WHERE role_id=97 AND resource_type='crm' AND action='cross_owner_write'",
    )
    .await;
    exec(&db, "DELETE FROM roles WHERE id=97").await;
    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (97,'代操作主管','proxy_all',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    // 1) All 范围、无代表键：读得到他人商机（读门 All 恒真），**转单必须 403**——
    //    修复前 handler 直通 service（内部 get_opportunity(id,None) 跳过行级判定）
    //    ⇒ 旧代码此处 200 + 订单落库，本断言即缺陷抓取点。
    let mgr_app = build_app(&db, make_auth(USER_EX_CREATOR, Some(ROLE_PROXY_ALL), "all"));
    let (status, v) = send(
        &mgr_app,
        Method::POST,
        "/erp/crm/opportunities/3/convert",
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "All 范围无 crm/cross_owner_write 键转他人商机必须 403（跨主派生写通道）: {v}"
    );
    assert_eq!(v["code"], serde_json::json!("FORBIDDEN"), "机器码契约: {v}");
    assert_eq!(
        v["message"],
        serde_json::json!(err_msg::PERMISSION_PUBLIC),
        "权限拒绝出参永久脱敏：不得外显商机归属人等内部信息"
    );
    assert_error_envelope_shape(&v);
    // 被拒必须零写入：订单不落库、商机状态不被翻转（先判后写）
    assert_eq!(
        count_orders(&db).await,
        0,
        "无键拒绝后 sales_orders 必须零落库"
    );
    assert_eq!(
        opp_status(&db, 3).await,
        "OPEN",
        "无键拒绝后商机 3 状态不得被翻转"
    );

    // 2) owner 本人（60，self，无键）：本人行恒可转——门不得打死日常功能。
    let owner_app = build_app(&db, make_auth(USER_B, Some(ROLE_SEED_NONADMIN), "self"));
    let (status, v) = send(
        &owner_app,
        Method::POST,
        "/erp/crm/opportunities/3/convert",
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "owner 本人转自己商机应 200（本人行恒可写，无需代表键）: {v}"
    );
    assert!(
        v["data"]["order_no"].is_string(),
        "转单成功必须回订单编号摘要: {v}"
    );
    assert_eq!(count_orders(&db).await, 1, "本人转单必须真实落库");
    assert_eq!(opp_status(&db, 3).await, "CLOSED_WON");

    // 3) 显式授予 crm/cross_owner_write 后，同一 All 用户转**另一条**他人商机（id=4）
    //    → 200：跨主放行只走这条既有键通道（该键不播种，须运维显式授予）。
    exec(
        &db,
        "INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
         VALUES (97,'crm','cross_owner_write',TRUE,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let mgr_app2 = build_app(&db, make_auth(USER_EX_CREATOR, Some(ROLE_PROXY_ALL), "all"));
    let (status, v) = send(
        &mgr_app2,
        Method::POST,
        "/erp/crm/opportunities/4/convert",
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持代表键的 All 范围代他人转单按方案 A 应 200: {v}"
    );
    assert_eq!(opp_status(&db, 4).await, "CLOSED_WON");
    assert_eq!(
        count_orders(&db).await,
        2,
        "带键代操作必须真实落库（不放空炮）"
    );
}

// ---------------------------------------------------------------------------
// D-4：admin 例外唯一权威源 = admin_checker::is_admin_role（roles.code='admin'）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_exception_follows_role_code_not_literal_one() {
    let db = seeded_db().await;
    // 自造角色同样先删后插（密封表幂等）：99 code='admin'（id≠1）、98 code≠'admin'
    exec(&db, "DELETE FROM roles WHERE id IN (98,99)").await;
    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (98,'运营主管','ops_leader',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (99,'系统管理员二','admin',FALSE,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    // code='admin' 的角色（无论主键）：他人行金额原值契约（裁定 #3 既有原值契约
    // 的**权威源**表述——admin 身份来自 roles.code，不来自 id==1 字面量）
    let app = build_app(&db, make_auth(USER_A, Some(ROLE_CODE_ADMIN), "all"));
    let rows = list_rows(&app).await;
    for (id, est, act) in [(3i64, OPP_B3_EST, OPP_B3_ACT), (6, OPP_Y_EST, OPP_Y_ACT)] {
        assert_eq!(
            amount_signature(&row_by_id(&rows, id)),
            (amount_text(est), amount_text(act)),
            "code='admin' 角色（role_id≠1）他人行 {id} 金额必须原值（admin 例外契约，判据=roles.code）"
        );
    }

    // 对照段：同账号、all 范围、但 code≠'admin'（role 98）**不享受**豁免——
    // 宁缺勿泄不静默扩权；若把 admin 例外错绑到"scope=all"或任何主键字面量，此处即红。
    let app2 = build_app(&db, make_auth(USER_A, Some(ROLE_CODE_LEADER), "all"));
    let rows2 = list_rows(&app2).await;
    assert_eq!(
        amount_signature(&row_by_id(&rows2, 3)),
        (String::new(), String::new()),
        "非 admin 的 all 范围角色他人行金额必须整键移除（admin 判据唯一=roles.code）"
    );
    assert_eq!(
        amount_signature(&row_by_id(&rows2, 1)),
        (amount_text(OPP_A1_EST), amount_text(OPP_A1_ACT)),
        "对照段本人行金额仍原值（默认门只认归属，不认 all）"
    );
}

// ---------------------------------------------------------------------------
// 源码棘轮（shrink-only，四条缺陷的"改坏必红"负例锁）
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

fn handler_body_slice(src: &str, name: &str) -> String {
    src.split(&format!("pub async fn {name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("handler {name} 定义缺失（改名/删除须同步本棘轮）"))
        .split("\npub async fn ")
        .next()
        .expect("函数体边界")
        .to_string()
}

#[test]
fn read_gate_bypass_source_ratchet() {
    let handler = include_str!("../src/handlers/crm_handler.rs").replace('\r', "");

    // D-1：360 的商机子集必须挂商机域字段级唯一实现（禁止内联第二口径）
    let body_360 = handler_body_slice(&handler, "get_customer_360");
    assert!(
        canon(&body_360).contains(
            "apply_opportunity_field_permission(&state,auth.role_id,auth.user_id,opps).await"
        ),
        "D-1 回潮棘轮：get_customer_360 不再与列表共用 apply_opportunity_field_permission（旁路复活）"
    );
    for banned in ["EXPORT_AMOUNT_COLUMNS", "remove(", "filter_fields_batch"] {
        assert!(
            !body_360.contains(banned),
            "D-1 回潮棘轮：get_customer_360 内联了第二套字段处理逻辑 → {banned}"
        );
    }
    // 简报投影必须携带 owner_id（360 与列表 owner 判据同源的物理前提）
    let crm_mod = include_str!("../src/services/crm/mod.rs").replace('\r', "");
    let brief_slice = crm_mod
        .split("pub struct OpportunityBrief")
        .nth(1)
        .expect("OpportunityBrief 定义缺失")
        .split("pub struct CustomerTagBrief")
        .next()
        .expect("OpportunityBrief 边界缺失");
    assert!(
        brief_slice.contains("pub owner_id: i32"),
        "D-1 回潮棘轮：OpportunityBrief 丢 owner_id 投影（360 商机行判据无源，门只能全剔或全放，皆失真）"
    );

    // D-3：convert 入口必须 ctx 行级读 + 跨主写门（复用既有机制，不新造权限语义）
    let body_convert = handler_body_slice(&handler, "convert_opportunity_to_order");
    assert!(
        canon(&body_convert).contains("service.get_opportunity(id,Some(&data_scope_ctx)).await?")
            && body_convert.contains("ensure_cross_owner_write_allowed"),
        "D-3 回潮棘轮：convert 入口绕门形态复活（无 ctx 取行或写门缺失）"
    );
    assert!(
        !canon(&body_convert).contains("service.get_opportunity(id,None)"),
        "D-3 回潮棘轮：convert handler 又出现无 ctx 取行"
    );

    // D-4：admin 例外单一权威源，角色主键字面量在本文件清零（含导出通道）
    let apply_body = handler
        .split("pub(crate) async fn apply_opportunity_field_permission")
        .nth(1)
        .expect("apply_opportunity_field_permission 定义缺失")
        .split("pub async fn create_lead")
        .next()
        .expect("函数体边界缺失");
    assert!(
        canon(&apply_body).contains("admin_checker::is_admin_role(&state.db,rid).await"),
        "D-4 回潮棘轮：商机金额门 admin 例外不再调用权威源 admin_checker::is_admin_role"
    );
    assert_eq!(
        handler.matches("rid == 1").count(),
        0,
        "D-4 回潮棘轮：crm_handler.rs 出现 rid == 1（角色主键字面量判定）"
    );
    assert_eq!(
        handler.matches("role_id != 1").count(),
        0,
        "D-4 回潮棘轮：crm_handler.rs 出现 role_id != 1（导出通道字面量判定回潮）"
    );

    // D-2：客户行级判定必须取 owner_id + department_id（created_by 访问器清零，两处都在）
    let cust = include_str!("../src/services/crm/cust.rs").replace('\r', "");
    assert_eq!(
        cust.matches("customer_info.created_by").count(),
        0,
        "D-2 回潮棘轮：cust.rs 又用 created_by 审计列当归属权威"
    );
    assert_eq!(
        canon(&cust)
            .matches(
                "check_resource_owner(ctx,Some(customer_info.owner_id),customer_info.department_id)"
            )
            .count(),
        2,
        "D-2 回潮棘轮：360 与跟进记录两处行级门必须都取 owner_id+department_id（改一漏一即口径分叉）"
    );
}
