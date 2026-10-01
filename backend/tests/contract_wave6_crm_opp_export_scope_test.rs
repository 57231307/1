//! 契约波次 6 · 任务 #208 遗留项：CRM 商机导出（xlsx）行级 scope + 金额列字段级权限
//!
//! 根因（修复前实证，与已提交的线索导出 #207 同一模式的第二条旁路）：
//! `handlers/crm_handler.rs::export_opportunities` 调
//! `service.export_opportunities(query)`（`services/crm/opp.rs`），该服务签名
//! **不接收** `DataScopeContext`，只过滤 `opportunity_stage`；handler 既不构造
//! `auth.to_data_scope_context()`，也不查 `get_role_data_permission(role_id,
//! "crm_opportunity")`、不做任何字段处理。而同文件的 `list_opportunities` 注入了 ctx
//! 并在"无数据权限行且非 admin"时移除金额。
//! ⇒ self/dept 用户点一次"导出"即拿到全库商机（limit 10000）含金额，
//!   形成"列表隐藏金额、导出原文外显"的双层旁路（越权读 + 商业秘密外泄）。
//!
//! 修复口径（与 #207 线索导出严格同构，不新造权限模型、不新增权限键）：
//! - 行级：handler 与 `list_opportunities` 同法构造 ctx 注入 service，service 内套用
//!   与列表**同一个** `apply_department_scope`（opp.rs:162 口径）。
//!   **商机没有公海语义**（`models/crm_opportunity.rs:69-73`：RLS 策略无 pool 分支，
//!   公海机制仅存在于 `crm_lead.lead_status`），故不得误用
//!   `apply_department_scope_with_pool` —— 用错即凭空放行一整类行；
//! - 字段级：同一判定源 + 同一 `filter_fields_batch`（有权限行分支），
//!   无权限行且非 admin 走 `EXPORT_AMOUNT_COLUMNS`（`estimated_amount`/`actual_amount`
//!   真实列名）整列剔除；列名未命中导出列定义表 ⇒ fail-closed 报错不出文件；
//! - 导出列序/表头/取值由 `CrmService::EXPORT_OPP_COLUMNS` 单源定义（列名 =
//!   crm_opportunity 出参键），改造前后表头逐列一致，前端与既有模板不受影响。
//!
//! 一致性证明方式：断言"导出解析出的数据行数与 `商机编号` 集合 == 同一用户同一查询
//! 条件下 `GET /erp/crm/opportunities` 的行集"（等式而非写死数字），xlsx 用 calamine
//! 解析回读单元格原文判定，不看 200 就收工。
//!
//! 覆盖边界（诚实声明）：
//! - `list_opportunities`/`get_opportunity` 的默认分支写的是 `obj.remove("amount")`，
//!   而 `crm_opportunity` 出参不存在 `amount` 键（真实列是上面两列）——即列表侧那处
//!   "移除"当前恒不生效，属同类读错键缺陷；连带收紧列表出参会影响前端金额列展示，
//!   需用户拍板，本批不动（见交付报告"待决点"）。本文件只锁导出侧（更严的一侧）。
//! - 导出审计事件（V15 P0-S11 `record_async`）为 best-effort 异步落库，时序不可判定，
//!   本文件不断言。

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
use bingxi_backend::handlers::crm_handler::{export_opportunities, list_opportunities};
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
// 脚手架（与 contract_wave6_crm_lead_export_scope_test.rs 同款：每用例同构种子，
// 规避 ADMIN_ROLE_CACHE 进程级缓存在任意执行顺序下的串味）
// ---------------------------------------------------------------------------

const USER_A: i32 = 50;
const USER_B: i32 = 60;
const USER_ADMIN: i32 = 70;

/// A/B 两方商机的金额原文（整数分值，构造 Decimal 与期望单元格文本用同一来源，
/// 避免"期望值写死一个字面量、实际值另一处拼出来"的假绿）
const A_EST_1: i64 = 111_111;
const A_ACT_1: i64 = 122_222;
const A_EST_2: i64 = 133_333;
const B_EST_1: i64 = 88_888;
const B_ACT_1: i64 = 66_666;
const B_EST_2: i64 = 55_555;

/// rust_decimal 的 `Decimal::from` / `Decimal::new` 均非 const fn（见 opp.rs 顶部
/// 同一结论），故金额在运行期构造；期望单元格文本 = 同一个 Decimal 的 to_string，
/// 与导出侧 `opp.rs::export_opp_cell` 的取值口径逐字符一致。
fn amount(raw: i64) -> Decimal {
    Decimal::from(raw)
}

fn amount_text(raw: i64) -> String {
    amount(raw).to_string()
}

fn make_auth(user_id: i32, role_id: i32, data_scope: &str) -> AuthContext {
    make_auth_full(user_id, role_id, data_scope, None, None)
}

fn make_auth_full(
    user_id: i32,
    role_id: i32,
    data_scope: &str,
    dept_ids: Option<&str>,
    dept_member_user_ids: Option<&str>,
) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave6_oppexport_user_{user_id}"),
        role_id: Some(role_id),
        department_id: Some(1),
        data_scope: Some(data_scope.to_string()),
        dept_ids: dept_ids.map(|s| Arc::new(s.to_string())),
        dept_member_user_ids: dept_member_user_ids.map(|s| Arc::new(s.to_string())),
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
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL/种子 执行失败: {e}\nSQL: {sql}"));
}

/// 与 models/role.rs::Model 对应（is_admin_role 判定源，admin_checker.rs:86-87）
const CREATE_ROLES: &str = r#"CREATE TABLE roles (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, code TEXT NOT NULL,
    description TEXT, permissions TEXT, is_system INTEGER NOT NULL,
    data_scope TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const CREATE_DATA_PERMISSIONS: &str = r#"CREATE TABLE data_permissions (
    id INTEGER PRIMARY KEY, role_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL, scope_type TEXT NOT NULL,
    custom_condition TEXT, allowed_fields TEXT, hidden_fields TEXT,
    is_enabled INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

/// 与 models/crm_opportunity.rs::Model 逐列对应（表 crm_opportunity，含 department_id）。
/// Decimal / NaiveDate / DateTime 一律按本仓 sqlite 既有用例（如
/// contract_wave1_ar_list_paginated_shape_test.rs）的 TEXT 口径建表，种子走 SeaORM
/// ActiveModel 写入，避免手写字符串与 sqlx 解码口径不一致造成的假绿/假红。
const CREATE_CRM_OPPORTUNITY: &str = r#"CREATE TABLE crm_opportunity (
    id INTEGER PRIMARY KEY,
    opportunity_no TEXT NOT NULL UNIQUE, opportunity_name TEXT NOT NULL,
    customer_id INTEGER NOT NULL, lead_id INTEGER, opportunity_type TEXT,
    opportunity_stage TEXT, win_probability TEXT,
    estimated_amount TEXT, actual_amount TEXT, currency TEXT,
    expected_close_date TEXT, actual_close_date TEXT,
    product_ids TEXT, product_names TEXT, product_desc TEXT,
    owner_id INTEGER NOT NULL, department_id INTEGER, owner_name TEXT NOT NULL,
    last_follow_up_date TEXT, next_follow_up_date TEXT, follow_up_plan TEXT,
    competitor_names TEXT, competitive_advantage TEXT, opportunity_status TEXT,
    won_reason TEXT, lost_reason TEXT, priority TEXT, rating INTEGER, tags TEXT,
    created_at TEXT, updated_at TEXT, created_by INTEGER, updated_by INTEGER
)"#;

fn ts(y: i32, m: u32, d: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(y, m, d)
        .expect("种子日期非法")
        .and_hms_opt(0, 0, 0)
        .expect("种子时间非法")
        .and_utc()
}

/// 插入一条商机（列取自 models/crm_opportunity.rs；金额走 Decimal，
/// 与导出的 `d.to_string()` 单元格原文一一对应）
async fn insert_opp(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    owner_id: i32,
    owner_name: &str,
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
        customer_id: Set(id),
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
        created_by: Set(Some(owner_id)),
        updated_by: Set(Some(owner_id)),
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("商机种子插入失败 id={id}: {e}"));
}

/// 建表 + 种子数据（4 条商机，两个归属人各两行，金额均非空可判定）：
/// - id=1/2：owner A（USER_A）——OPP A 前缀；
/// - id=3/4：owner B（USER_B）——OPP B 前缀。
async fn seeded_db(permissions: Option<&str>) -> Arc<sea_orm::DatabaseConnection> {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        CREATE_CRM_OPPORTUNITY,
        CREATE_ROLES,
        CREATE_DATA_PERMISSIONS,
    ] {
        exec(&db, ddl).await;
    }

    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (1,'系统管理员','admin',1,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'销售专员','sales',0,'self','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    insert_opp(
        &db,
        1,
        USER_A,
        "销售甲",
        A_EST_1,
        Some(A_ACT_1),
        ts(2026, 1, 1),
    )
    .await;
    insert_opp(&db, 2, USER_A, "销售甲", A_EST_2, None, ts(2026, 1, 2)).await;
    insert_opp(
        &db,
        3,
        USER_B,
        "销售乙",
        B_EST_1,
        Some(B_ACT_1),
        ts(2026, 1, 3),
    )
    .await;
    insert_opp(&db, 4, USER_B, "销售乙", B_EST_2, None, ts(2026, 1, 4)).await;

    if let Some(insert_sql) = permissions {
        exec(&db, insert_sql).await;
    }

    Arc::new(db)
}

fn state_from(db: &Arc<sea_orm::DatabaseConnection>) -> AppState {
    let mut state = AppState::default();
    state.db = db.clone();
    // data_permission_service 内部持有独立 db 引用，必须与 state.db 同源重建，
    // 否则 get_role_data_permission 落在 Disconnected 连接上恒 Err（假绿）。
    state.data_permission_service = Arc::new(DataPermissionService::new(db.clone()));
    state
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/opportunities", get(list_opportunities))
        .route("/erp/crm/opportunities/export", get(export_opportunities))
        .with_state(state_from(db))
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, uri: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = send(app, uri).await;
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

/// 列表可见集锚点：`data.total` + 商机编号集合（services/crm/opp.rs 出参
/// `{data:[...],total,page,page_size}`）
async fn list_visible(app: &Router) -> (u64, Vec<String>) {
    let (status, v) = get_json(app, "/erp/crm/opportunities?page=1&page_size=100").await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    let items = v["data"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"));
    let nos = items
        .iter()
        .map(|i| i["opportunity_no"].as_str().unwrap_or_default().to_string())
        .collect();
    let total = v["data"]["total"]
        .as_u64()
        .unwrap_or_else(|| panic!("列表缺 total 键: {v}"));
    assert_eq!(
        total as usize,
        items.len(),
        "page_size=100 下 total 应与本页行数一致（否则等式失去意义）"
    );
    (total, nos)
}

/// 把 xlsx 响应体解析成 (表头, 数据行)——真读文件，不做硬编码 JSON 假断言
async fn export_table(app: &Router) -> (Vec<String>, Vec<Vec<String>>) {
    let resp = send(app, "/erp/crm/opportunities/export?page=1&page_size=100").await;
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

/// 按表头标签取某列（表头与列序由 CrmService::EXPORT_OPP_COLUMNS 单源定义）
fn column(headers: &[String], label: &str) -> usize {
    headers
        .iter()
        .position(|h| h == label)
        .unwrap_or_else(|| panic!("导出缺列「{label}」，表头定义漂移: {headers:?}"))
}

fn cell(row: &[String], idx: usize) -> String {
    row.get(idx).cloned().unwrap_or_default()
}

/// 可见集等式：导出行数与商机编号集合 == 列表 total 与商机编号集合
fn assert_same_visible_set(list: &(u64, Vec<String>), headers: &[String], rows: &[Vec<String>]) {
    let no_idx = column(headers, "商机编号");
    let mut exported: Vec<String> = rows
        .iter()
        .map(|r| cell(r, no_idx))
        .filter(|s| !s.is_empty())
        .collect();
    exported.sort();
    let mut list_nos = list.1.clone();
    list_nos.sort();
    assert_eq!(
        rows.len() as u64,
        list.0,
        "导出行数与列表可见行数不一致（导出仍是未受行级 scope 约束的旁路）"
    );
    assert_eq!(
        exported, list_nos,
        "导出行集合与列表可见行集合不一致（逐行比对商机编号）"
    );
}

/// 按商机编号定位导出行（断言不依赖行序）
fn row_of<'a>(headers: &[String], rows: &'a [Vec<String>], no: &str) -> &'a Vec<String> {
    let no_idx = column(headers, "商机编号");
    rows.iter()
        .find(|r| cell(r, no_idx) == no)
        .unwrap_or_else(|| panic!("导出缺行「{no}」: {rows:?}"))
}

/// 金额列不外显：两张金额列在每一行都必须是空单元格
fn assert_amount_columns_blank(headers: &[String], rows: &[Vec<String>], who: &str) {
    for label in ["预估金额", "实际金额"] {
        let idx = column(headers, label);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(
                cell(row, idx),
                "",
                "{who}：第 {i} 行的「{label}」外显了金额原文（字段级权限旁路）: {row:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 1) self 用户：导出行集 == 列表可见集，且金额列整列不外显（两层旁路同时锁）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_opp_export_rows_equal_list_visible_and_amounts_dropped() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let list = list_visible(&app).await;
    assert_eq!(
        list.0, 2,
        "self 用户列表可见集基线（仅本人两行）: {:?}",
        list.1
    );

    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);

    // 越权读锁死：他人（A）的商机编号不得出现在导出体的任何单元格
    let all_cells: Vec<&String> = rows.iter().flatten().collect();
    for banned_no in ["OPPA001", "OPPA002"] {
        assert!(
            !all_cells.iter().any(|c| c.as_str() == banned_no),
            "self 用户导出含他人商机行 {banned_no}（行级 scope 未接入）"
        );
    }

    assert_amount_columns_blank(&headers, &rows, "self 用户（无数据权限行分支）");
    // 反证：金额原文数字也不得以任何形式外显（含 Decimal.to_string 的变体）
    for banned_amount in [
        amount_text(A_EST_1),
        amount_text(A_ACT_1),
        amount_text(B_EST_1),
        amount_text(B_ACT_1),
    ] {
        assert!(
            !all_cells.iter().any(|c| c.as_str() == banned_amount),
            "self 用户导出含金额原文 {banned_amount}"
        );
    }

    // 其余列不受影响（证明"剔除"是精准的，不是把整行清空冒充合规）
    let own = row_of(&headers, &rows, "OPPB003");
    assert_eq!(cell(own, column(&headers, "商机阶段")), "QUALIFICATION");
    assert_eq!(cell(own, column(&headers, "负责人")), "销售乙");
    assert_eq!(cell(own, column(&headers, "优先级")), "high");
}

// ---------------------------------------------------------------------------
// 2) dept 用户：行级口径与列表同源（apply_department_scope，不含任何 pool 放行）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dept_user_opp_export_rows_equal_list_visible() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth_full(55, 2, "dept", Some("1"), Some("50,60")));

    let list = list_visible(&app).await;
    // Dept 分支「本人行 OR department_id ∈ 可见部门」由同一个函数生成；
    // 商机无公海语义，故可见集 = 4 行（若误用带 pool 放行的函数，此处行数不变，
    // 但服务层函数名由用例 6 的源码扫描锁单独钉死）
    assert_eq!(list.0, 4, "dept 用户列表可见集基线: {:?}", list.1);
    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);
    assert_amount_columns_blank(&headers, &rows, "dept 用户（无数据权限行分支）");
}

// ---------------------------------------------------------------------------
// 3) admin（role_id=1、roles.code='admin'）：行数 == 全库，金额保持原文
//    依据：get_role_data_permission 对 admin 返回 Ok(Some{allowed:None,hidden:None})
//    → filter_fields_batch 空操作；DataScope::All 行级不过滤。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_opp_export_keeps_amounts_and_all_rows() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(USER_ADMIN, 1, "all"));

    let list = list_visible(&app).await;
    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);

    let est_idx = column(&headers, "预估金额");
    let act_idx = column(&headers, "实际金额");
    let row_a = row_of(&headers, &rows, "OPPA001");
    assert_eq!(
        cell(row_a, est_idx),
        amount_text(A_EST_1),
        "admin 原值契约（预估金额）"
    );
    assert_eq!(
        cell(row_a, act_idx),
        amount_text(A_ACT_1),
        "admin 原值契约（实际金额）"
    );
}

// ---------------------------------------------------------------------------
// 4) 配了 hidden_fields 的角色：仅该列被剔除，其余列不叠加默认处理
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_with_hidden_amount_field_blanks_that_column_only() {
    let insert = r#"INSERT INTO data_permissions (id,role_id,resource_type,scope_type,
        allowed_fields,hidden_fields,is_enabled,created_at,updated_at)
        VALUES (1,2,'crm_opportunity','SELF',NULL,'["actual_amount"]',1,
        '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#;
    let db = seeded_db(Some(insert)).await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (headers, rows) = export_table(&app).await;
    let own = row_of(&headers, &rows, "OPPB003");
    assert_eq!(
        cell(own, column(&headers, "实际金额")),
        "",
        "hidden_fields 应剔除该列"
    );
    assert_eq!(
        cell(own, column(&headers, "预估金额")),
        amount_text(B_EST_1),
        "有数据权限行分支不叠加默认剔除（既有语义，与线索导出一致）"
    );
}

// ---------------------------------------------------------------------------
// 5) 配了 allowed_fields 的角色：白名单内的金额列放行原文，白名单外列被剔除
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_with_allowed_fields_keeps_amounts_and_drops_unlisted_column() {
    let allowed = r#"["opportunity_no","opportunity_name","customer_id","opportunity_stage",
        "estimated_amount","actual_amount","expected_close_date","actual_close_date",
        "owner_name","created_at"]"#;
    let insert = format!(
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,\
         allowed_fields,hidden_fields,is_enabled,created_at,updated_at) \
         VALUES (1,2,'crm_opportunity','SELF','{allowed}',NULL,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    let db = seeded_db(Some(&insert)).await;
    let app = build_app(&db, make_auth(USER_B, 2, "self"));

    let (headers, rows) = export_table(&app).await;
    let own = row_of(&headers, &rows, "OPPB003");
    assert_eq!(
        cell(own, column(&headers, "预估金额")),
        amount_text(B_EST_1),
        "allowed_fields 含 estimated_amount 时应放行原文（受控通道）"
    );
    assert_eq!(
        cell(own, column(&headers, "优先级")),
        "",
        "未列入 allowed_fields 的列应与列表一致被剔除（导出侧空单元格，表头保留）"
    );
}

// ---------------------------------------------------------------------------
// 6) 源码扫描锁（shrink-only 棘轮）：商机导出不得再省略行级 scope / 不得再
//    无字段处理直出金额；服务层不得误用带 pool 放行的行级函数
// ---------------------------------------------------------------------------

#[test]
fn opp_export_handler_and_service_must_inject_data_scope_and_drop_amounts() {
    let handler = include_str!("../src/handlers/crm_handler.rs");
    assert!(
        handler.contains("export_opportunities(query, Some(&data_scope_ctx))"),
        "商机导出 handler 必须以 Some(ctx) 接入行级 scope（与 list_opportunities 同口径）"
    );
    assert_eq!(
        handler.matches("export_opportunities(query)").count(),
        0,
        "回潮棘轮：crm_handler.rs 出现省略 ctx 的 export_opportunities(query) 调用"
    );
    assert!(
        handler.contains("drop_export_amount_columns(&mut table)"),
        "商机导出缺默认金额列剔除分支（无数据权限行且非 admin 时不得外显金额）"
    );

    let service = include_str!("../src/services/crm/opp.rs");
    let body = service
        .split("pub async fn export_opportunities")
        .nth(1)
        .expect("export_opportunities 定义缺失")
        .split("pub async fn get_opportunity")
        .next()
        .expect("export_opportunities 函数体边界缺失");
    assert!(
        body.contains("data_scope: Option<&DataScopeContext>"),
        "export_opportunities 签名必须接收行级数据权限上下文"
    );
    assert!(
        body.contains("apply_department_scope("),
        "export_opportunities 必须复用列表同一个行级过滤函数 apply_department_scope"
    );
    assert!(
        !body.contains("apply_department_scope_with_pool"),
        "商机无公海语义：export_opportunities 不得用带 pool 放行的行级函数（凭空放行一类行）"
    );
    assert!(
        service.contains("pub const EXPORT_OPP_COLUMNS"),
        "商机导出列定义必须是单一事实来源（列名 = crm_opportunity 出参键）"
    );
    assert!(
        service.contains("pub const EXPORT_AMOUNT_COLUMNS"),
        "金额列集合必须在服务层单源定义，供 handler 按列名定位而非魔法下标"
    );
}
