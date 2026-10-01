//! 契约波次 6 · 任务 #207：CRM 线索导出（xlsx）行级 scope + 字段级掩码
//!
//! 根因（修复前实证）：
//! 1. 行级：`services/crm/lead.rs` 的 `export_leads` 签名不吃 `DataScopeContext`，
//!    handler `crm_handler.rs` 也不构造 ctx —— 而 `list_leads`（:143-151）会套
//!    `apply_department_scope_with_pool`。于是 self/dept 用户点一次"导出"就拿到
//!    全库线索（limit 10000），列表/详情/公海三条读路径本波次前已分别做了行级 scope
//!    与掩码（#200/#202），导出是第四条旁路（"三打一漏"）。
//! 2. 字段级：`export_leads` 把 `mobile_phone`/`tel_phone`/`email` 原文写进 xlsx，
//!    无任何 allowed_fields/hidden_fields/默认掩码分支。
//!
//! 修复口径（全部沿用既有机制，不新造权限模型/权限键）：
//! - handler 与 `list_leads`（crm_handler.rs:53-54）同法构造
//!   `auth.to_data_scope_context()` 并注入 service，service 内套用与列表**同一个**
//!   `apply_department_scope_with_pool`（含 Dept 分支的公海放行）；
//! - 字段级与列表**同一判定源**（`get_role_data_permission`，admin 依据 roles.code='admin'）
//!   与**同一实现**：有数据权限行走 `filter_fields_batch`（allowed_fields 白名单/
//!   hidden_fields 移除，不叠加默认打码）；无权限行且 role_id != 1 走
//!   `utils/field_mask::mask_phone` / `mask_email`。因此"配了 allowed_fields 的角色"
//!   天然就是放行原文的受控通道；
//! - 既有 P0-S11 导出审计事件（`crm_handler.rs`）保持原样，本文件不断言
//!   （`record_async` 为 best-effort 异步落库，时序不可判定，是否升级 fail-closed 待用户拍板）。
//!
//! 一致性证明方式：断言"导出解析出的数据行数 == 同一用户同一查询条件下
//! `GET /erp/crm/leads` 返回的 `data.total`"，并逐列比对导出行的公司名称集合与列表行的
//! 公司名称集合（而非写死一个数字），从而把"导出可见集 == 列表可见集"锁成等式；
//! 行级过滤函数与公海放行条件与列表同源，故 Dept 用户（含公海放行）也纳入等式。
//!
//! 覆盖边界（诚实声明）：
//! - 本文件所有断言均可在 sqlite 完成，不依赖 PG 专有特性（导出路径无取号咨询锁、
//!   无 lock_exclusive），因此没有需要 #[ignore] 的活库用例；
//!   `create_lead` 的 LD 取号（advisory_xact_lock）不在导出链上，种子一律走 raw SQL。
//! - `export_opportunities`（商机导出）是同一模式的另一处旁路，属 #207 描述之外的
//!   独立端点，本批未改、未断言（见交付报告"未覆盖项"）。
//! - 掩码列由 `CrmService::EXPORT_LEAD_COLUMNS` 的列名定位（与列表出参键同源）；
//!   若有人改表头顺序而不改定义，本文件的列名取值断言会失败，不会静默放行。

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
use bingxi_backend::handlers::crm_handler::{export_leads, list_leads};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use calamine::{Data, Reader, open_workbook_auto_from_rs};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::Value;
use std::io::Cursor;
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 脚手架（与 contract_wave6_crm_lead_mask_test.rs / _pool_mask_test.rs 同款：
// 每个用例同构种子，规避 ADMIN_ROLE_CACHE 进程级缓存在任意执行顺序下的串味）
// ---------------------------------------------------------------------------

const A_PHONE: &str = "13812348888";
const A_EMAIL: &str = "alice@example.com";
const B_PHONE: &str = "13711112222";
const B_TEL: &str = "03191234567";
const B_EMAIL: &str = "bob@example.com";

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
        username: format!("wave6_export_user_{user_id}"),
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

/// 与 models/crm_lead.rs::Model 逐列对应（表 crm_lead，含 department_id）
const CREATE_CRM_LEAD: &str = r#"CREATE TABLE crm_lead (
    id INTEGER PRIMARY KEY,
    lead_no TEXT NOT NULL UNIQUE, lead_source TEXT NOT NULL,
    lead_status TEXT, company_name TEXT,
    contact_name TEXT NOT NULL, contact_title TEXT,
    mobile_phone TEXT, tel_phone TEXT, email TEXT, wechat TEXT, qq TEXT,
    address TEXT, product_interest TEXT,
    estimated_quantity TEXT, estimated_amount TEXT,
    expected_delivery_date TEXT, requirement_desc TEXT,
    owner_id INTEGER NOT NULL, department_id INTEGER, owner_name TEXT NOT NULL,
    last_follow_up_date TEXT, next_follow_up_date TEXT, follow_up_plan TEXT,
    converted_at TEXT, converted_customer_id INTEGER, converted_opportunity_id INTEGER,
    lost_reason TEXT, priority TEXT, rating INTEGER, tags TEXT, industry TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    created_by INTEGER, updated_by INTEGER, custom_fields TEXT
)"#;

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

/// 建表 + 种子数据（4 条线索，两个归属人 + 一条公海行）：
/// - id=1 A（owner 50）公海行，携带 A 的原始手机号/邮箱；
/// - id=2 A（owner 50）私海行；
/// - id=3 B（owner 60）私海行，携带 B 的手机号/座机/邮箱（用于默认掩码断言）；
/// - id=4 B（owner 60）公海行（Dept 用户的公海放行与列表同口径，参与行数等式）。
async fn seeded_db(permissions: Option<&str>) -> Arc<sea_orm::DatabaseConnection> {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, CREATE_CRM_LEAD).await;
    exec(&db, CREATE_ROLES).await;
    exec(&db, CREATE_DATA_PERMISSIONS).await;

    exec(
        &db,
        "INSERT INTO roles (id,name,code,is_system,data_scope,created_at,updated_at) VALUES
         (1,'系统管理员','admin',1,'all','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (2,'销售专员','sales',0,'self','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;

    exec(
        &db,
        &format!(
            "INSERT INTO crm_lead (id,lead_no,lead_source,lead_status,company_name,
             contact_name,mobile_phone,tel_phone,email,address,owner_id,owner_name,
             department_id,priority,industry,created_at,updated_at) VALUES
             (1,'LD001','website','pool','甲公司','张三','{A_PHONE}',NULL,'{A_EMAIL}',
              '地址甲',50,'销售甲',1,'low','针织','2026-01-01T00:00:00Z','2020-01-01T00:00:00Z'),
             (2,'LD002','ad','new','乙公司','李四','13700001111','01088886666','carol@example.com',
              '地址乙',50,'销售甲',1,'high','梭织','2026-01-02T00:00:00Z','2020-01-01T00:00:00Z'),
             (3,'LD003','referral','new','丙公司','王五','{B_PHONE}','{B_TEL}','{B_EMAIL}',
              '地址丙',60,'销售乙',1,'medium','针织','2026-01-03T00:00:00Z','2020-01-01T00:00:00Z'),
             (4,'LD004','website','pool','丁公司','赵六','13600002222',NULL,'dave@example.com',
              '地址丁',60,'销售乙',1,'urgent','针织','2026-01-04T00:00:00Z','2020-01-01T00:00:00Z')"
        ),
    )
    .await;

    if let Some(insert_sql) = permissions {
        exec(&db, insert_sql).await;
    }

    Arc::new(db)
}

fn state_from(db: &Arc<sea_orm::DatabaseConnection>) -> AppState {
    // data_permission_service 内部持有独立 db 引用，必须与 state.db 同源重建，
    // 否则 get_role_data_permission 落在 Disconnected 连接上恒 Err（假绿）。
    AppState {
        db: db.clone(),
        data_permission_service: Arc::new(DataPermissionService::new(db.clone())),
        ..Default::default()
    }
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    Router::new()
        .route("/erp/crm/leads", get(list_leads))
        .route("/erp/crm/leads/export", get(export_leads))
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

/// 列表可见集锚点：`data.total` + 公司名集合（services/crm/lead.rs 出参
/// `{data:[...],total,page,page_size}`，handler 用 list/data 双键兼容取列表）
async fn list_visible(app: &Router) -> (u64, Vec<String>) {
    let (status, v) = get_json(app, "/erp/crm/leads?page=1&page_size=100").await;
    assert_eq!(status, StatusCode::OK, "列表 200 契约: {v}");
    let items = v["data"]["data"]
        .as_array()
        .unwrap_or_else(|| panic!("分页列表形状漂移（期望 data.data 数组）: {v}"));
    let names = items
        .iter()
        .map(|i| i["company_name"].as_str().unwrap_or_default().to_string())
        .collect();
    let total = v["data"]["total"]
        .as_u64()
        .unwrap_or_else(|| panic!("列表缺 total 键: {v}"));
    assert_eq!(
        total as usize,
        items.len(),
        "page_size=100 下 total 应与本页行数一致（否则等式失去意义）"
    );
    (total, names)
}

/// 把 xlsx 响应体解析成 (表头, 数据行)——真读文件，不做硬编码 JSON 假断言
async fn export_table(app: &Router) -> (Vec<String>, Vec<Vec<String>>) {
    let resp = send(app, "/erp/crm/leads/export?page=1&page_size=100").await;
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

/// 按表头标签取某列（表头与列序由 CrmService::EXPORT_LEAD_COLUMNS 单源定义）
fn column(headers: &[String], label: &str) -> usize {
    headers
        .iter()
        .position(|h| h == label)
        .unwrap_or_else(|| panic!("导出缺列「{label}」，表头定义漂移: {headers:?}"))
}

fn cell(row: &[String], idx: usize) -> String {
    row.get(idx).cloned().unwrap_or_default()
}

/// 可见集等式：导出行数与公司名称集合 == 列表 total 与公司名集合
fn assert_same_visible_set(list: &(u64, Vec<String>), headers: &[String], rows: &[Vec<String>]) {
    let name_idx = column(headers, "公司名称");
    let mut exported_names: Vec<String> = rows
        .iter()
        .map(|r| cell(r, name_idx))
        .filter(|s| !s.is_empty())
        .collect();
    exported_names.sort();
    let mut list_names = list.1.clone();
    list_names.sort();
    assert_eq!(
        rows.len() as u64,
        list.0,
        "导出行数与列表可见行数不一致（导出仍是不受行级 scope 约束的旁路）"
    );
    assert_eq!(
        exported_names, list_names,
        "导出行集合与列表可见行集合不一致（逐行比对公司名称）"
    );
}

/// 按公司名称定位导出行（导出与列表同序 created_at DESC，但断言不依赖顺序）
fn row_of<'a>(headers: &[String], rows: &'a [Vec<String>], company: &str) -> &'a Vec<String> {
    let name_idx = column(headers, "公司名称");
    rows.iter()
        .find(|r| cell(r, name_idx) == company)
        .unwrap_or_else(|| panic!("导出缺行「{company}」: {rows:?}"))
}

// ---------------------------------------------------------------------------
// 1) self 用户：导出可见集 == 列表可见集，且 PII 列默认掩码
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_user_export_rows_equal_list_visible_and_pii_masked() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(60, 2, "self"));

    let list = list_visible(&app).await;
    // 基线自检：self 用户列表可见集 = 本人 owner 的两行（id=3 私海 + id=4 自己回收前
    // 遗留的公海行仍按 owner_id=60 命中 Self 分支；他人公海行 id=1 不可见，
    // utils/data_scope.rs 的 Self 分支公海放行属 #203 待拍板项，本文件不依赖）
    assert_eq!(list.0, 2, "self 用户列表可见集基线: {:?}", list.1);

    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);

    let phone_idx = column(&headers, "手机号");
    let tel_idx = column(&headers, "座机");
    let email_idx = column(&headers, "邮箱");
    let own = row_of(&headers, &rows, "丙公司");
    assert_eq!(cell(own, phone_idx), "137****2222", "mask_phone 未生效");
    assert_eq!(cell(own, tel_idx), "031****4567", "座机列未掩码");
    assert_eq!(
        cell(own, email_idx),
        "b***@example.com",
        "mask_email 未生效"
    );

    // 泄露面锁死：导出体任何单元格都不得出现他人（A）或本人（B）的原文手机号/邮箱
    let all_cells: Vec<&String> = rows.iter().flatten().collect();
    for banned in [A_PHONE, A_EMAIL, B_PHONE, B_TEL, B_EMAIL] {
        assert!(
            !all_cells.iter().any(|c| c.as_str() == banned),
            "导出含未掩码个人信息 {banned}"
        );
    }
}

// ---------------------------------------------------------------------------
// 2) dept 用户：含公海放行的行级口径与列表同源（同一 apply_department_scope_with_pool）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dept_user_export_rows_equal_list_visible_with_pool_branch() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth_full(55, 2, "dept", Some("1"), Some("50,60")));

    let list = list_visible(&app).await;
    let (headers, rows) = export_table(&app).await;
    // Dept 分支的「本人行 OR department_id ∈ 可见部门 OR 公海行」由同一个函数生成，
    // 导出与列表必须落在同一个集合上（此处 department_id=1 覆盖 4 行）
    assert_eq!(list.0, 4, "dept 用户列表可见集基线: {:?}", list.1);
    assert_same_visible_set(&list, &headers, &rows);

    let phone_idx = column(&headers, "手机号");
    for row in &rows {
        let phone = cell(row, phone_idx);
        if phone.is_empty() {
            continue;
        }
        assert!(
            phone.contains("****"),
            "dept 用户导出手机号未掩码（无数据权限行分支）: {phone}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3) admin（role_id=1、roles.code='admin'）：行数 == 全库，且保持原值
//    依据：get_role_data_permission 对 admin 返回 Ok(Some{allowed:None,hidden:None})
//    → filter_fields_batch 空操作；DataScope::All 行级不过滤。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_export_keeps_raw_values_and_sees_all_rows() {
    let db = seeded_db(None).await;
    let app = build_app(&db, make_auth(70, 1, "all"));

    let list = list_visible(&app).await;
    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);

    let phone_idx = column(&headers, "手机号");
    let email_idx = column(&headers, "邮箱");
    let row_a = row_of(&headers, &rows, "甲公司");
    assert_eq!(cell(row_a, phone_idx), A_PHONE, "admin 原值契约");
    assert_eq!(cell(row_a, email_idx), A_EMAIL, "admin 原值契约");
}

// ---------------------------------------------------------------------------
// 4) 配了 allowed_fields 的角色：受控通道放行原文，白名单外列被剔除（列表同语义）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_with_allowed_fields_exports_raw_pii_and_drops_unlisted_column() {
    let allowed = r#"["lead_no","company_name","contact_name","contact_title","mobile_phone","tel_phone","email","lead_source","lead_status","owner_name","created_at"]"#;
    let insert = format!(
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,allowed_fields,hidden_fields,is_enabled,created_at,updated_at) \
         VALUES (1,2,'crm_lead','SELF','{allowed}',NULL,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    let db = seeded_db(Some(&insert)).await;
    let app = build_app(&db, make_auth(60, 2, "self"));

    let list = list_visible(&app).await;
    let (headers, rows) = export_table(&app).await;
    assert_same_visible_set(&list, &headers, &rows);

    let phone_idx = column(&headers, "手机号");
    let priority_idx = column(&headers, "优先级");
    let own = row_of(&headers, &rows, "丙公司");
    assert_eq!(
        cell(own, phone_idx),
        B_PHONE,
        "allowed_fields 含 mobile_phone 时应放行原文（既有白名单语义，导出不得另加默认打码）"
    );
    assert_eq!(
        cell(own, priority_idx),
        "",
        "未列入 allowed_fields 的列应与列表一致被剔除（导出侧为空单元格，表头保留）"
    );
}

// ---------------------------------------------------------------------------
// 5) 配了 hidden_fields 的角色：该列被移除（导出侧空单元格），其余列不叠加默认打码
// ---------------------------------------------------------------------------

#[tokio::test]
async fn role_with_hidden_fields_blanks_that_column_only() {
    let hidden = r#"["mobile_phone"]"#;
    let insert = format!(
        "INSERT INTO data_permissions (id,role_id,resource_type,scope_type,allowed_fields,hidden_fields,is_enabled,created_at,updated_at) \
         VALUES (1,2,'crm_lead','SELF',NULL,'{hidden}',1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"
    );
    let db = seeded_db(Some(&insert)).await;
    let app = build_app(&db, make_auth(60, 2, "self"));

    let (headers, rows) = export_table(&app).await;
    let own = row_of(&headers, &rows, "丙公司");
    assert_eq!(cell(own, column(&headers, "手机号")), "");
    assert_eq!(cell(own, column(&headers, "座机")), B_TEL);
    assert_eq!(cell(own, column(&headers, "邮箱")), B_EMAIL);
}

// ---------------------------------------------------------------------------
// 6) 源码扫描锁（shrink-only 棘轮）：导出不得再省略行级 scope / 不得再无掩码直出
// ---------------------------------------------------------------------------

#[test]
fn export_handler_and_service_must_inject_data_scope_and_mask() {
    let handler = include_str!("../src/handlers/crm_handler.rs");
    assert!(
        handler.contains("export_leads(query, Some(&data_scope_ctx))"),
        "导出 handler 必须以 Some(ctx) 接入行级 scope（与 list_leads 同口径）"
    );
    assert_eq!(
        handler.matches("export_leads(query)").count(),
        0,
        "回潮棘轮：crm_handler.rs 出现省略 ctx 的 export_leads(query) 调用"
    );
    assert!(
        handler.contains("fn mask_export_pii_columns"),
        "导出默认掩码分支缺失（无数据权限行且非 admin 时不得直出原文）"
    );

    let service = include_str!("../src/services/crm/lead.rs");
    assert!(
        service.contains("pub async fn export_leads"),
        "export_leads 服务函数应存在"
    );
    let body = service
        .split("pub async fn export_leads")
        .nth(1)
        .expect("export_leads 定义缺失")
        .split("pub async fn import_leads")
        .next()
        .expect("export_leads 函数体边界缺失");
    assert!(
        body.contains("data_scope: Option<&DataScopeContext>"),
        "export_leads 签名必须接收行级数据权限上下文"
    );
    assert!(
        body.contains("apply_department_scope_with_pool"),
        "export_leads 必须复用列表同一个行级过滤函数（含公海放行）"
    );
}
