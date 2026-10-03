//! 契约波次 7 · 商机金额"仅非本人行"统一口径（用户 2026-10-02 拍板落地锁）
//!
//! 根因（调查报告 #1/#2/#4）：
//! 1. `apply_opportunity_field_permission` 默认分支写的是 `obj.remove("amount")`，
//!    而 `crm_opportunity` 出参真实列是 `estimated_amount`/`actual_amount`（无 `amount`
//!    键，见 `models/crm_opportunity.rs:43/:46/:67`）——该分支恒不生效；
//! 2. 隐藏范围裁定 = **仅非本人行**（行 `owner_id` ≠ 当前登录用户且非 admin 才剔金额；
//!    本人行金额必须可见）；
//! 3. `role_id` 缺失由旧"不处理"收紧为 fail-closed 走默认处理（与线索/客户域同口径）；
//! 4. 入口一致：导出与列表/详情/写响应共用同一个
//!    `crm_handler::apply_opportunity_field_permission`，不得再留导出独立分支
//!    （"列表打码、导出原文"同资源不同入口旁路，本族已收口四轮）。
//! 5. 判据字段：隐藏与否只看**行 `owner_id`** 是否等于当前登录用户
//!    （`handlers/crm_handler.rs:176-179`；导出侧同源 `services/crm/opp.rs:318`），
//!    与建单人 `created_by` 无关（本仓缺陷族：归属错用 created_by）。交叉归属种子
//!    （owner ≠ created_by 两行）由 `amount_hide_gate_keys_on_owner_id_not_created_by`
//!    覆盖——未种该形态时两种实现出参完全等价，锁没有区分力。
//! 6. admin（`roles.code='admin'` / role_id==1）例外口径：跨 owner **读**含他人行金额
//!    原文（`handlers/crm_handler.rs:168-170` + `data_permission_service.rs:64-71`
//!    的空操作分支，即裁定 #3"既有原值契约，不叠加"）；跨 owner 限制只落在**写**侧
//!    （`crm_write_guard::ensure_cross_owner_write_allowed`）。因此本文件的 admin 用例
//!    必须同时带"非 admin 对照段"，否则"隐藏门整体丢失"会以"admin 全显"的形态假绿。
//!
//! 本文件断言口径：只断 HTTP `status` + 信封 `code`，不断案文案原文（权限拒绝出参
//! 永久脱敏，见 utils/error.rs 固定信封）；金额可见性按"键是否存在/单元格是否空"断。
//!
//! 通道（路线一，#4669 判责）：用例经 `test_common::setup_test_db()` 连已迁移
//! PostgreSQL 真跑；表结构唯一来源 = backend/migration，不再自建 DDL。
//! roles/data_permissions 语义不变（roles 为迁移种子参照表不再插）；
//! users（归属人父行，trg_crm_opportunity_dept 触发器按其 department_id 回填行部门）
//! 与 customers（crm_opportunity.customer_id 真 FK 父行，裁定 R1）自种子。
//! 金额外显形状按真列定标：crm_opportunity.estimated_amount/actual_amount 为
//! DECIMAL(15,2)，真库回读经 Decimal 序列化为带列标度的字符串（如 "111111.00"），
//! amount_text 即该口径（sqlite 时代 TEXT 列的裸整数文本正是 #4669 的失真源）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler::{
    export_opportunities, get_opportunity, list_opportunities,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::crm_opportunity;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use calamine::{Data, Reader, open_workbook_auto_from_rs};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, Set, Statement};
use serde_json::Value;
use std::io::Cursor;
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_opp_export_scope_test.rs 同款）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50; // dept 测试用户，拥有 OPP A 两行
const USER_B: i32 = 60; // 他人，拥有 OPP B 两行
const USER_OUTSIDER: i32 = 55; // 同部门但无任何行
const USER_ADMIN: i32 = 70;

const A_EST_1: i64 = 111_111;
const A_ACT_1: i64 = 122_222;
const A_EST_2: i64 = 133_333;
const B_EST_1: i64 = 88_888;
const B_ACT_1: i64 = 66_666;
const B_EST_2: i64 = 55_555;
// 交叉归属行（"归属转移后"形态，判定字段锁专用）：owner_id 与 created_by 不同人。
// 未种这种形态时，误用 created_by 判归属与正确用 owner_id 判归属在所有出参上完全等价
// （本仓缺陷族：归属判据错用 created_by 而非 owner_id），任何断言都区分不出来。
const T_OWN_EST: i64 = 210_001; // owner=USER_A / created_by=USER_B
const T_OWN_ACT: i64 = 210_002;
const T_OTHER_EST: i64 = 210_003; // owner=USER_B / created_by=USER_A
const T_OTHER_ACT: i64 = 210_004;

fn amount(raw: i64) -> Decimal {
    Decimal::from(raw)
}

/// 金额的外显形状：真列 DECIMAL(15,2) 回读经 Decimal 序列化为带列标度的字符串
/// （"111111.00"），列表 JSON 与导出单元格共用该口径（与 wave1 真库测试
/// 断 "100.00"/"5.00" 同源先例）。
fn amount_text(raw: i64) -> String {
    format!("{:.2}", amount(raw))
}

fn make_auth(user_id: i32, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave7_opp_user_{user_id}"),
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

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

fn ts(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(y, m, d)
        .expect("种子日期非法")
        .and_hms_opt(0, 0, 0)
        .expect("种子时间非法")
        .and_utc()
}

/// 商机种子。`creator_id` 与 `owner_id` 是**两个独立入参**：隐藏判定的行归属来源是
/// `owner_id`（`handlers/crm_handler.rs:176-179` 取 `owner_id` 比对当前登录用户；
/// 导出侧同源 = `services/crm/opp.rs` 回传的 `opp.owner_id`），`created_by` 只是建单人
/// 留痕、不参与判定。未发生归属转移的真实行传 `creator_id == owner_id`。
async fn insert_opp(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    owner_id: i32,
    owner_name: &str,
    creator_id: i32,
    estimated: i64,
    actual: Option<i64>,
    created_at: DateTime<Utc>,
) {
    crm_opportunity::ActiveModel {
        id: Set(id),
        opportunity_no: Set(format!(
            "OPP{a}{id:03}",
            a = if owner_id == USER_A { "A" } else { "B" }
        )),
        opportunity_name: Set(format!("商机标题-{id}")),
        // 真 FK fk_crm_opportunity_customer：4 行商机统一挂同一 customers 父行
        //（seeded_db 自种子，裁定 R1；本文件不校验 customer 维度）
        customer_id: Set(1),
        lead_id: Set(None),
        opportunity_type: Set(Some("NEW".to_string())),
        opportunity_stage: Set(Some("QUALIFICATION".to_string())),
        win_probability: Set(Some(Decimal::from(10))),
        estimated_amount: Set(Some(amount(estimated))),
        actual_amount: Set(actual.map(amount)),
        currency: Set(Some("CNY".to_string())),
        expected_close_date: Set(Some(created_at.date_naive())),
        actual_close_date: Set(None),
        product_ids: Set(None),
        product_names: Set(None),
        product_desc: Set(None),
        owner_id: Set(owner_id),
        department_id: Set(Some(1)),
        owner_name: Set(owner_name.to_string()),
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
        created_at: Set(Some(created_at)),
        updated_at: Set(Some(created_at)),
        created_by: Set(Some(creator_id)),
        updated_by: Set(Some(creator_id)),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("商机种子插入失败 id={id}: {e}"));
}

async fn seeded_db(permissions: Option<&str>) -> Arc<sea_orm::DatabaseConnection> {
    let db = test_common::setup_test_db().await;
    // roles 不插：迁移种子参照表（id=1 code='admin' = is_admin_role 判定源，
    // id=2 为 data_permissions.role_id 外键父行）。
    // users：归属人父行（裁定 R1）——trg_crm_opportunity_dept 按 owner 的
    // department_id 回填行部门列，dept 可见集依赖其为 1。
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (60,'sales_b','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    // customers 父行（裁定 R1）：crm_opportunity.customer_id NOT NULL + 真 FK
    exec(
        &db,
        "INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,
         status,customer_type,owner_id,created_at,updated_at) VALUES
         (1,'CUS-0001','契约客户',0,30,'active','retail',50,
         '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    insert_opp(
        &db,
        1,
        USER_A,
        "销售甲",
        USER_A,
        A_EST_1,
        Some(A_ACT_1),
        ts(2026, 1, 1),
    )
    .await;
    insert_opp(
        &db,
        2,
        USER_A,
        "销售甲",
        USER_A,
        A_EST_2,
        None,
        ts(2026, 1, 2),
    )
    .await;
    insert_opp(
        &db,
        3,
        USER_B,
        "销售乙",
        USER_B,
        B_EST_1,
        Some(B_ACT_1),
        ts(2026, 1, 3),
    )
    .await;
    insert_opp(
        &db,
        4,
        USER_B,
        "销售乙",
        USER_B,
        B_EST_2,
        None,
        ts(2026, 1, 4),
    )
    .await;
    if let Some(insert_sql) = permissions {
        exec(&db, insert_sql).await;
    }
    Arc::new(db)
}

fn state_from(db: &Arc<sea_orm::DatabaseConnection>) -> AppState {
    AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    }
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/opportunities", get(list_opportunities))
        .route("/erp/crm/opportunities/export", get(export_opportunities))
        .route("/erp/crm/opportunities/{id}", get(get_opportunity))
        .with_state(state_from(db))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
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

/// 列表行索引：商机编号 → 行对象
async fn list_items(app: &Router) -> Vec<Value> {
    let (status, v) = get_json(app, "/erp/crm/opportunities?page=1&page_size=100").await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    assert_eq!(v["code"], serde_json::json!(200), "成功信封 code 契约: {v}");
    v["data"]["data"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"))
}

fn item_no(item: &Value) -> &str {
    item["opportunity_no"].as_str().unwrap_or_default()
}

/// 按商机编号取列表行：**禁止按位置索引取行**（列表排序契约 = `created_at DESC`，
/// 见 `services/crm/opp.rs:179`；位置索引把排序当契约，种子时间一调整就假判红/假判绿）。
/// 找不到行属可见集/契约漂移，直接 panic 打出全部单号，不静默降级。
fn item_by_no<'a>(items: &'a [Value], no: &str) -> &'a Value {
    items.iter().find(|i| item_no(i) == no).unwrap_or_else(|| {
        panic!(
            "列表缺行 {no}（按单号定位，不依赖排序）: {:?}",
            items.iter().map(item_no).collect::<Vec<_>>()
        )
    })
}

/// 金额键是否仍在外显出参中（键被整键移除 = 隐藏；键存在但为 null = 库中本无值，
/// 属"未剔除"，两种形态必须区分——隐藏实现是 `obj.remove(column)`，不是置 null）
fn has_amount_key(item: &Value, column: &str) -> bool {
    item.get(column).is_some()
}

async fn export_table(app: &Router) -> (Vec<String>, Vec<Vec<String>>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/erp/crm/opportunities/export?page=1&page_size=100")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    assert_eq!(status, StatusCode::OK, "导出 200 契约");
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let mut wb =
        open_workbook_auto_from_rs(Cursor::new(bytes)).expect("导出响应应为可解析的 xlsx（zip）");
    let sheet = wb.sheet_names().first().cloned().expect("xlsx 应含工作表");
    let range = wb.worksheet_range(&sheet).expect("读取工作表失败");
    let mut rows: Vec<Vec<String>> = range
        .rows()
        .map(|r| {
            r.iter()
                .map(|c| match c {
                    Data::String(s) => s.clone(),
                    Data::Empty => String::new(),
                    other => other.to_string(),
                })
                .collect()
        })
        .collect();
    assert!(!rows.is_empty(), "xlsx 至少应有表头行");
    let headers = rows.remove(0);
    (headers, rows)
}

fn column(headers: &[String], label: &str) -> usize {
    headers
        .iter()
        .position(|h| h == label)
        .unwrap_or_else(|| panic!("导出缺列「{label}」，表头定义漂移: {headers:?}"))
}

fn row_of<'a>(headers: &[String], rows: &'a [Vec<String>], no: &str) -> &'a Vec<String> {
    let no_idx = column(headers, "商机编号");
    rows.iter()
        .find(|r| r.get(no_idx).map(String::as_str) == Some(no))
        .unwrap_or_else(|| panic!("导出缺行「{no}」: {rows:?}"))
}

/// 列表行金额签名：键缺失=被剔除、null=库中无值，两者在导出侧同为空单元格，
/// 归一为空串参与"导出=列表"等式（金额原文则必须是 JSON 字符串本体）
fn list_amount_signature(items: &[Value], no: &str) -> (String, String) {
    let item = items
        .iter()
        .find(|i| item_no(i) == no)
        .unwrap_or_else(|| panic!("列表缺行 {no}: {items:?}"));
    let pick = |col: &str| -> String {
        match item.get(col) {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(other) => panic!("金额列序列化形状漂移（期望 JSON 字符串/null）: {other}"),
        }
    };
    (pick("estimated_amount"), pick("actual_amount"))
}

/// 导出行金额签名：空单元格归一为空串
fn export_amount_signature(headers: &[String], rows: &[Vec<String>], no: &str) -> (String, String) {
    let row = row_of(headers, rows, no);
    let pick =
        |label: &str| -> String { row.get(column(headers, label)).cloned().unwrap_or_default() };
    (pick("预估金额"), pick("实际金额"))
}

/// 逐行等式：同一用户、同一可见行集，导出金额签名 == 列表金额签名（同资源同口径）
async fn assert_export_equals_list_for_amounts(app: &Router) {
    let items = list_items(app).await;
    let (headers, rows) = export_table(app).await;
    assert_eq!(
        rows.len(),
        items.len(),
        "导出行数与列表可见行数不一致（行集旁路）"
    );
    for item in &items {
        let no = item_no(item).to_string();
        let list_sig = list_amount_signature(&items, &no);
        let export_sig = export_amount_signature(&headers, &rows, &no);
        assert_eq!(
            list_sig, export_sig,
            "商机 {no}：列表与导出的金额外显口径不一致（同资源不同入口旁路）"
        );
    }
}

// ---------------------------------------------------------------------------
// 1) dept 用户（本人持有一部分行）：他人行剔真实金额列，本人行金额真实外显
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dept_user_list_hides_amounts_only_on_non_own_rows() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_A, Some(2), "dept"));

    let items = list_items(&app).await;
    assert_eq!(items.len(), 4, "dept 可见集基线（本部门 4 行）");
    for item in &items {
        let own = item_no(item).starts_with("OPPA");
        let has_est = item.get("estimated_amount").is_some();
        let has_act = item.get("actual_amount").is_some();
        if own {
            // 裁定 #2：本人行金额必须可见（否则销售日常功能被做没）
            assert!(has_est, "本人行 {no} 预估金额应可见", no = item_no(item));
            assert_eq!(
                item["estimated_amount"],
                Value::String(amount_text(if item_no(item) == "OPPA001" {
                    A_EST_1
                } else {
                    A_EST_2
                })),
                "本人行金额为 JSON 字符串（Decimal 序列化的真实形状）"
            );
        } else {
            assert!(
                !has_est && !has_act,
                "他人行 {} 金额未剔除（remove(\"amount\") 恒不生效缺陷回潮）: {item}",
                item_no(item)
            );
        }
    }
    // 他人行金额原文（含键存在的 null 伪形）不得出现在出参任何位置
    let raw = serde_json::to_string(&items).unwrap();
    for banned in [
        amount_text(B_EST_1),
        amount_text(B_ACT_1),
        amount_text(B_EST_2),
    ] {
        assert!(
            !raw.contains(&banned),
            "dept 列表出参含他人行金额原文 {banned}"
        );
    }
}

// ---------------------------------------------------------------------------
// 2) 详情出口与列表同函数：他人行（dept 合法可见）金额键被移除；
//    self 用户直接请求他人行 → 403 + FORBIDDEN（只断 status/code，文案永久脱敏）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn detail_exit_shares_scope_own_row_keeps_amount_other_row_dropped() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_A, Some(2), "dept"));

    // 他人同部门行（OPPB003，id=3）：行级可见，字段级"非本人行"剔金额
    let (status, v) = get_json(&app, "/erp/crm/opportunities/3").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "dept 用户读取同部门他人商机应 200: {v}"
    );
    assert_eq!(v["code"], serde_json::json!(200), "成功信封 code: {v}");
    assert!(
        v["data"].get("estimated_amount").is_none() && v["data"].get("actual_amount").is_none(),
        "详情出口他人行金额应整键移除（与列表同一实现）: {}",
        v["data"]
    );
    assert_eq!(
        v["data"]["owner_id"],
        serde_json::json!(USER_B),
        "非金额列不误伤"
    );

    // 本人行（id=1）：金额真实外显
    let (status, v) = get_json(&app, "/erp/crm/opportunities/1").await;
    assert_eq!(status, StatusCode::OK, "本人商机详情应 200: {v}");
    assert_eq!(
        v["data"]["estimated_amount"],
        Value::String(amount_text(A_EST_1)),
        "本人行金额真实外显（裁定 #2）"
    );
}

#[tokio::test]
async fn self_user_detail_of_other_opp_is_403_forbidden_masked_envelope() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_A, Some(2), "self"));
    let (status, v) = get_json(&app, "/erp/crm/opportunities/3").await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "self 越权读他人商机应 403: {v}"
    );
    assert_eq!(
        v["code"],
        serde_json::json!("FORBIDDEN"),
        "失败信封 code: {v}"
    );
    // 出参形状红线：AppError 固定键；不断言文案原文（永久脱敏）
    assert!(
        v.get("trace_id").is_some() && v.get("timestamp").is_some(),
        "信封缺 trace_id/timestamp: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        !msg.contains(&amount_text(B_EST_1)) && !msg.contains("owner_id"),
        "越权拒绝出参泄漏金额或判定原因: {msg}"
    );
}

// ---------------------------------------------------------------------------
// 3) admin（role_id==1）：金额全显（既有原值契约，不叠加）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_sees_all_amounts_on_all_rows() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_ADMIN, Some(1), "all"));
    let items = list_items(&app).await;
    assert_eq!(items.len(), 4, "admin 可见全库");
    // **按单号定位，禁按位置索引**（判责 ci4671-triage.md :134）：列表契约排序是
    // created_at DESC（services/crm/opp.rs:179），种子 ts 递增时 items[2] 恰落在
    // OPPB003 —— 那是把排序当契约的巧合，排序一漂移就假判红/假判绿（本轮该用例
    // Expected 88888.00 / Received 133333.00 即位置漂移产物）。改按单号逐行钉
    // 金额原值，且由"只断一行"收紧为"4 行预估 + 2 行实际金额全覆盖"。
    let est = |no: &str| item_by_no(&items, no)["estimated_amount"].clone();
    let act = |no: &str| item_by_no(&items, no)["actual_amount"].clone();
    assert_eq!(
        est("OPPA001"),
        Value::String(amount_text(A_EST_1)),
        "admin 对 OPPA001 预估金额原值契约"
    );
    assert_eq!(
        est("OPPA002"),
        Value::String(amount_text(A_EST_2)),
        "admin 对 OPPA002 预估金额原值契约"
    );
    assert_eq!(
        est("OPPB003"),
        Value::String(amount_text(B_EST_1)),
        "admin 对他行（OPPB003）金额原值契约"
    );
    assert_eq!(
        est("OPPB004"),
        Value::String(amount_text(B_EST_2)),
        "admin 对他行（OPPB004）金额原值契约"
    );
    assert_eq!(
        act("OPPA001"),
        Value::String(amount_text(A_ACT_1)),
        "admin 对 OPPA001 实际金额原值契约"
    );
    assert_eq!(
        act("OPPB003"),
        Value::String(amount_text(B_ACT_1)),
        "admin 对 OPPB003 实际金额原值契约"
    );
    assert_export_equals_list_for_amounts(&app).await;
    // admin 导出同样原值（等式已锁"导出=列表"，此处补一句直接断言便于定位）
    let (headers, rows) = export_table(&app).await;
    assert_eq!(
        row_of(&headers, &rows, "OPPB003")[column(&headers, "预估金额")],
        amount_text(B_EST_1)
    );

    // 详情通道与列表同源同口径：admin 读**他人行**（OPPB003，owner_id=USER_B）详情原值。
    // 依据：get_opportunity（handlers/crm_handler.rs:919-940）对单元素切片调同一个
    // apply_opportunity_field_permission；admin 命中 get_role_data_permission 的
    // 空操作分支（services/data_permission_service.rs:64-71 → allowed/hidden 均 None）
    // 或 rid==1 提前返回（handlers/crm_handler.rs:168-170），两条都不剔金额列。
    // 即"读跨 owner"在 admin 口径下**含**他人行金额原文（裁定 #3 既有原值契约；
    // 跨 owner 的限制只落在写侧，见 update/delete/close 的 ensure_cross_owner_write_allowed）。
    let (status, detail) = get_json(&app, "/erp/crm/opportunities/3").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "admin 读他人商机详情应 200: {detail}"
    );
    assert_eq!(
        detail["data"]["estimated_amount"],
        Value::String(amount_text(B_EST_1)),
        "admin 详情出口他人行预估金额原值契约（与列表同口径，不得只在列表放行）"
    );
    assert_eq!(
        detail["data"]["actual_amount"],
        Value::String(amount_text(B_ACT_1)),
        "admin 详情出口他人行实际金额原值契约"
    );

    // 【本用例的区分力所在】admin 全显有两种成因：①admin 豁免生效（正确）；
    // ②"仅非本人行"隐藏门整体丢失（缺陷）。只断 admin 一侧的数无法区分二者——门被删
    // 掉时本用例照样全绿。故同一份种子再以非 admin 的 dept 用户复跑，必须仍然
    // "他人行金额整键移除 **且** 本人行金额原值可见"，两个方向同时断。
    let dept_app = build_app(&db, make_auth(USER_A, Some(2), "dept"));
    let dept_items = list_items(&dept_app).await;
    assert_eq!(dept_items.len(), 4, "对照段可见集基线（dept 本部门 4 行）");
    for no in ["OPPB003", "OPPB004"] {
        let row = item_by_no(&dept_items, no);
        assert!(
            !has_amount_key(row, "estimated_amount") && !has_amount_key(row, "actual_amount"),
            "对照段：非 admin 的他人行 {no} 金额应整键移除（门丢失即在此暴露）: {row}"
        );
    }
    for no in ["OPPA001", "OPPA002"] {
        let row = item_by_no(&dept_items, no);
        assert!(
            has_amount_key(row, "estimated_amount"),
            "对照段：非 admin 的本人行 {no} 金额键不得被移除: {row}"
        );
    }
    assert_eq!(
        item_by_no(&dept_items, "OPPA001")["estimated_amount"],
        Value::String(amount_text(A_EST_1)),
        "对照段：本人行金额必须是原值而非掩码/占位（裁定 #2）"
    );
    assert_eq!(
        item_by_no(&dept_items, "OPPA002")["estimated_amount"],
        Value::String(amount_text(A_EST_2)),
        "对照段：本人行金额必须是原值而非掩码/占位（裁定 #2）"
    );
}

// ---------------------------------------------------------------------------
// 3b) 隐藏判据字段锁：行归属一律按 `owner_id`，不得按 `created_by`
//     （本仓缺陷族：归属判据错用 created_by）。种子刻意造出 owner_id ≠ created_by
//     的"归属转移后"形态两行——若两种实现都能让本用例通过，说明种子没区分力。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn amount_hide_gate_keys_on_owner_id_not_created_by() {
    let db = seeded_db(None).await;
    // OPPA005：owner=USER_A（本人行）、created_by=USER_B（他人建单）⇒ 金额必须可见
    insert_opp(
        &db,
        5,
        USER_A,
        "销售甲",
        USER_B,
        T_OWN_EST,
        Some(T_OWN_ACT),
        ts(2026, 1, 5),
    )
    .await;
    // OPPB006：owner=USER_B（他人行）、created_by=USER_A（本人建单）⇒ 金额必须被隐藏
    insert_opp(
        &db,
        6,
        USER_B,
        "销售乙",
        USER_A,
        T_OTHER_EST,
        Some(T_OTHER_ACT),
        ts(2026, 1, 6),
    )
    .await;

    let app = build_app(&db, make_auth(USER_A, Some(2), "dept"));
    let items = list_items(&app).await;
    assert_eq!(items.len(), 6, "dept 可见集（本部门 6 行，含两行交叉归属）");

    let own = item_by_no(&items, "OPPA005");
    assert_eq!(
        own["owner_id"],
        serde_json::json!(USER_A),
        "种子前提：OPPA005 的 owner_id 是本人"
    );
    assert_eq!(
        own["created_by"],
        serde_json::json!(USER_B),
        "种子前提：OPPA005 的 created_by 是他人（与 owner_id 不同人）"
    );
    assert_eq!(
        own["estimated_amount"],
        Value::String(amount_text(T_OWN_EST)),
        "OPPA005 按 owner_id 判为本人行 ⇒ 预估金额原值可见（若判据误用 created_by 即被剔除，此处红）"
    );
    assert_eq!(
        own["actual_amount"],
        Value::String(amount_text(T_OWN_ACT)),
        "OPPA005 实际金额同样按 owner_id 放行"
    );

    let other = item_by_no(&items, "OPPB006");
    assert_eq!(
        other["owner_id"],
        serde_json::json!(USER_B),
        "种子前提：OPPB006 的 owner_id 是他人"
    );
    assert_eq!(
        other["created_by"],
        serde_json::json!(USER_A),
        "种子前提：OPPB006 的 created_by 是本人（若判据误用 created_by 就会被放行）"
    );
    assert!(
        !has_amount_key(other, "estimated_amount") && !has_amount_key(other, "actual_amount"),
        "OPPB006 按 owner_id 判为他人行 ⇒ 金额整键移除（若判据误用 created_by 则原文外显，此处红）: {other}"
    );
    let raw = serde_json::to_string(&items).unwrap();
    for banned in [amount_text(T_OTHER_EST), amount_text(T_OTHER_ACT)] {
        assert!(
            !raw.contains(&banned),
            "列表出参含按 owner_id 判定应隐藏的他人行金额原文 {banned}"
        );
    }

    // 详情通道同一判据（同一个函数的单元素路径）
    let (status, detail) = get_json(&app, "/erp/crm/opportunities/6").await;
    assert_eq!(status, StatusCode::OK, "同部门他人行详情应 200: {detail}");
    assert!(
        detail["data"].get("estimated_amount").is_none()
            && detail["data"].get("actual_amount").is_none(),
        "详情出口按 owner_id 判他人行即剔金额（判据漂移会在此暴露）: {}",
        detail["data"]
    );
    let (status, detail) = get_json(&app, "/erp/crm/opportunities/5").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "本人行（转移后 owner=本人）详情应 200: {detail}"
    );
    assert_eq!(
        detail["data"]["estimated_amount"],
        Value::String(amount_text(T_OWN_EST)),
        "详情出口本人行（created_by 为他人）金额原值可见"
    );

    // 导出通道同一判据：行归属来源 = service 回传的 opp.owner_id
    //（services/crm/opp.rs:318），不是 created_by；列表/导出仍逐行等值
    assert_export_equals_list_for_amounts(&app).await;
    let (headers, rows) = export_table(&app).await;
    assert_eq!(
        row_of(&headers, &rows, "OPPA005")[column(&headers, "预估金额")],
        amount_text(T_OWN_EST),
        "导出：owner=本人的转移行金额真实外显"
    );
    assert_eq!(
        row_of(&headers, &rows, "OPPB006")[column(&headers, "预估金额")],
        "",
        "导出：owner=他人的行（即便 created_by=本人）金额不外显"
    );

    // admin 侧同一交叉形态：全显（豁免与判据无关，跨 owner 读含金额原文）
    let admin_app = build_app(&db, make_auth(USER_ADMIN, Some(1), "all"));
    let admin_items = list_items(&admin_app).await;
    assert_eq!(
        item_by_no(&admin_items, "OPPB006")["estimated_amount"],
        Value::String(amount_text(T_OTHER_EST)),
        "admin 对交叉他人行仍原值（裁定 #3 既有契约）"
    );
    assert_export_equals_list_for_amounts(&admin_app).await;
}

// ---------------------------------------------------------------------------
// 4) 配了 hidden_fields 的角色：仍走配置分支，不叠加"仅非本人行"默认处理（裁定 #3）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn configured_role_uses_config_only_without_default_overlay() {
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'crm_opportunity','SELF',NULL,'["estimated_amount"]',TRUE,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let db = seeded_db(Some(insert)).await;
    let app = build_app(&db, make_auth(USER_OUTSIDER, Some(2), "dept"));

    let items = list_items(&app).await;
    for item in &items {
        assert!(
            item.get("estimated_amount").is_none(),
            "配置 hidden_fields 应移除该列: {}",
            item_no(item)
        );
        // 关键区分点：OUTSIDER 对全部行都是"非本人行"，但配置分支不叠加默认处理 ⇒
        // 未列入 hidden 的 actual_amount 必须保留（旧导出/新列表若叠加会连它一起剔掉）
        assert!(
            item.get("actual_amount").is_some(),
            "配置角色不得叠加默认剔列（actual_amount 非 hidden 应保留）: {}",
            item_no(item)
        );
    }
    assert_export_equals_list_for_amounts(&app).await;
}

// ---------------------------------------------------------------------------
// 5) role_id 缺失：fail-closed 收紧（裁定后与线索/客户域同口径）——
//    不再"不处理"，而是按无权限行走默认处理（仅非本人行剔金额）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_role_id_fails_closed_to_default_scope() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_A, None, "dept"));
    let items = list_items(&app).await;
    for item in &items {
        let own = item_no(item).starts_with("OPPA");
        assert_eq!(
            item.get("estimated_amount").is_some(),
            own,
            "role_id 缺失应 fail-closed 走默认处理（仅非本人行剔金额）: {}",
            item_no(item)
        );
    }
    assert_export_equals_list_for_amounts(&app).await;
}

// ---------------------------------------------------------------------------
// 6) 导出与列表对同一角色结果一致（dept 混合归属：本人两行 + 他人两行）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn export_equals_list_amounts_for_dept_mixed_rows() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_A, Some(2), "dept"));
    assert_export_equals_list_for_amounts(&app).await;
    // 具体期望（不只靠等式）：本人行导出金额为原文、他人行为空单元格
    let (headers, rows) = export_table(&app).await;
    assert_eq!(
        row_of(&headers, &rows, "OPPA001")[column(&headers, "预估金额")],
        amount_text(A_EST_1),
        "导出本人行金额真实外显"
    );
    assert_eq!(
        row_of(&headers, &rows, "OPPB003")[column(&headers, "预估金额")],
        "",
        "导出他人行金额不外显"
    );
    assert_eq!(
        row_of(&headers, &rows, "OPPB003")[column(&headers, "实际金额")],
        "",
        "导出他人行实际金额不外显"
    );
}

// ---------------------------------------------------------------------------
// 7) 源码扫描棘轮（shrink-only）：
//    a. 读不存在键 `remove("amount")` 永久禁止回潮；
//    b. 默认处理必须按真实列集合循环剔除；
//    c. 导出与列表共用同一判定（导出体不得再出现独立剔除函数/独立判定分支）。
// ---------------------------------------------------------------------------

#[test]
fn amount_scope_ratchet_source_scan() {
    let handler = include_str!("../src/handlers/crm_handler.rs");
    assert_eq!(
        handler.matches("remove(\"amount\")").count(),
        0,
        "回潮棘轮：crm_handler.rs 出现 remove(\"amount\")（crm_opportunity 出参不存在 amount 键，恒不生效）"
    );
    assert!(
        handler.contains("for column in CrmService::EXPORT_AMOUNT_COLUMNS"),
        "默认处理必须按 EXPORT_AMOUNT_COLUMNS 真实列集合剔除（单源列名，不写死字面量）"
    );
    assert_eq!(
        handler.matches("drop_export_amount_columns").count(),
        0,
        "回潮棘轮：导出独立金额分支重新出现（导出必须与列表共用 apply_opportunity_field_permission）"
    );
    let export_body = handler
        .split("pub async fn export_opportunities")
        .nth(1)
        .expect("export_opportunities 定义缺失")
        .split("pub async fn get_opportunity")
        .next()
        .expect("export_opportunities 函数体边界缺失");
    assert!(
        export_body.contains(
            "apply_opportunity_field_permission(&state, auth.role_id, auth.user_id, &mut rows_json)"
        ),
        "导出字段级处理必须调用与列表同一个 apply_opportunity_field_permission（同资源同口径）"
    );
    assert!(
        export_body.contains("obj.insert(\"owner_id\""),
        "导出行对象必须注入 owner_id 供\"仅非本人行\"判定（行归属来源不得缺位）"
    );
    // fail-closed 收紧：role_id 缺失不得再"不处理"直出
    let apply_body = handler
        .split("pub(crate) async fn apply_opportunity_field_permission")
        .nth(1)
        .expect("apply_opportunity_field_permission 定义缺失")
        .split("pub async fn create_lead")
        .next()
        .expect("函数体边界缺失");
    assert!(
        apply_body.contains("current_user_id as i64"),
        "默认处理必须比对行 owner_id 与当前登录用户（仅非本人行判据）"
    );
    assert!(
        apply_body.contains("get(\"owner_id\")"),
        "默认处理的行归属判据必须取 owner_id: {apply_body}"
    );
    assert_eq!(
        apply_body.matches("created_by").count(),
        0,
        "回潮棘轮：商机金额隐藏判据出现 created_by（本仓缺陷族：归属错用 created_by 而非 owner_id）"
    );
    assert!(
        apply_body.contains("if rid == 1 {\n            return;"),
        "admin 保持既有原值契约（不进默认处理）"
    );
    // 导出通道的行归属来源与列表同源（handler 侧只能从 service 回传的 owner_id 注入，
    // 不得改为 created_by，否则导出与列表的"本人行"集合会分叉）
    let opp_svc = include_str!("../src/services/crm/opp.rs");
    assert!(
        opp_svc.contains("opportunities.iter().map(|opp| opp.owner_id).collect()"),
        "导出回传的行归属来源必须是 opp.owner_id（与列表出参 owner_id 同一列）"
    );
}
