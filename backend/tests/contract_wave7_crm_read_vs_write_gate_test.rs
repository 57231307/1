//! 契约波次 7 · 裁定方案 A 核心锁：**读门 ≠ 写门**（同一角色、同一他人行）
//!
//! 背景（判责 §③"两拨互相矛盾的测试"的根）：`DataScope::All` 的本职是
//! "能看全库"（读门 `check_resource_owner` All 恒真），把它同时当成"能改任何人的行"
//! 就是水平越权的成因；e2e `flow/34-horizontal-privilege`（按 B 越权必拒口径）与
//! Rust 写权限族（按 All=200 口径）因此永远互相打红。用户 2026-10-02 裁定方案 A：
//! "读可用 All 跨 owner；**写**必须是 owner 本人，或显式持 `crm/cross_owner_write`
//! 代表键 + 留痕；该键不播种给任何角色（admin 靠 is_admin_role 放行路径不变）；
//! 读门不得复用为写门。"
//!
//! 本文件把这条裁定在**同一夹具、同一他人行**上锁成三段式，缺一不可：
//! 1. 非 admin 的 `data_scope=all` 角色读他人客户 → 200（读门 All 跨 owner，审计/主管复核需要）；
//! 2. 同一角色**无键**写他人客户 → 403（写门独立于读门；出参只断 status/code/固定脱敏常量，
//!    不断言拒绝原因——权限文案永久脱敏是硬令）；role_id 缺失同样 403（fail-closed）；
//! 3. 显式授予 `crm/cross_owner_write` 后同一写 → 2xx + 真生效 + **审计留痕可查证**
//!    （`update_with_audit` 落 `audit_logs` actor=操作人的行；`crm_write_guard` 的
//!    `tracing::info!` 放行留痕由本文件源码棘轮锁其存在——进程内测试无法捕获日志订阅器，
//!    不做假断言，双轨各锁各的可观测证据）。
//!
//! 另有三道回潮棘轮（shrink-only）：
//! - 已接入的 CRM 写入口清单必须逐个仍在门后（防未来新增写入口绕门 / 存量入口门被删）；
//! - `check_resource_owner`（读门）All 分支必须保持恒真、`check_resource_write_owner`
//!   （写门）All 分支必须回落 `behalf_granted`——两函数判定源禁止合并；
//! - `backend/migration/**` 全树不得出现 `cross_owner_write` 播种（裁定"该键不播种给
//!   任何角色"，默认形态只有 admin 经 is_admin_role 能用）。
//!
//! 通道（路线一，与 真库化测同款）：`test_common::setup_test_db` 真
//! PostgreSQL 真跑 + 真 HTTP 装配，表结构唯一来源 = backend/migration；users/customers
//! 按裁定 R1 自种子（FK 父行先插），roles 取迁移种子参照行（id=1 code='admin'，
//! id=2 非 admin），禁 sqlite、无自建 DDL、无 #[ignore]。

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
use bingxi_backend::handlers::customer_handler::{get_customer, update_customer};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::services::data_permission_service::DataPermissionService;
use bingxi_backend::utils::messages::err_msg;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER: i32 = 50;
const MGR_ALL: i32 = 80;

fn make_auth(user_id: i32, username: &str, role_id: Option<i32>, data_scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
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

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 真库夹具：一条归属 OWNER（created_by=50）的客户行 + 操作人 MGR_ALL（80）。
/// MGR_ALL 的 data_scope=all 由 AuthContext 注入（行级判定读 auth 上下文，
/// roles.data_scope 列仅生产中间件加载用，参照 同款夹具口径）。
async fn base_state() -> AppState {
    let db = test_common::setup_test_db().await;
    exec(
        &db,
        "INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
         (50,'sales_a','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
         (80,'mgr_all','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO customers (id,customer_code,customer_name,contact_person,
             contact_phone,contact_email,address,credit_limit,payment_terms,status,
             customer_type,owner_id,created_by,created_at,updated_at) VALUES
             (1,'CUS-0001','甲客户','张三','13812348888','alice@example.com',
              '河北省邢台市某某路 1 号',0,30,'active','retail',50,50,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    exec(
        &db,
        "SELECT setval(pg_get_serial_sequence('customers','id'),
                       (SELECT COALESCE(MAX(id),1) FROM customers))",
    )
    .await;

    let mut state = AppState::default();
    state.db = Arc::new(db);
    state.data_permission_service = Arc::new(DataPermissionService::new(state.db.clone()));
    state
}

/// 标准客户入口真实挂载形态（routes/crm.rs `customers()`：GET/PUT /customers/{id}）
fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/erp/customers/{id}",
            get(get_customer).put(update_customer),
        )
        .with_state(state)
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

async fn customer_name(db: &Arc<sea_orm::DatabaseConnection>) -> String {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT customer_name FROM customers WHERE id=1",
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读客户行失败");
    assert_eq!(rows.len(), 1, "种子行必须存在且仅一条");
    rows[0]
        .try_get::<String>("", "customer_name")
        .expect("customer_name 回读失败")
}

// ---------------------------------------------------------------------------
// 三段式 1+2：读 200 / 无键写 403（同一角色同一他人行）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_scope_non_admin_can_read_others_customer_but_write_without_key_is_403() {
    let state = base_state().await;
    let db = state.db.clone();
    // 非 admin（role_id=2，迁移种子非 admin 角色）+ data_scope=all，未授予任何代表键
    let app = build_app(state, make_auth(MGR_ALL, "mgr_all", Some(2), "all"));

    // 读门：All 可读他人行（裁定"读可用 All 跨 owner"——本断言钉住读侧未被顺手收紧）
    let (read_status, read_v) = send(&app, Method::GET, "/erp/customers/1", json!({})).await;
    assert_eq!(
        read_status,
        StatusCode::OK,
        "All 范围读他人客户必须 2xx（读门 All 恒真，不得被写门收紧）: {read_v}"
    );

    // 写门：跨 owner 写无键必 403——**只断 status/code/固定脱敏常量，不断言原因**
    let before = customer_name(&db).await;
    let (write_status, wv) = send(
        &app,
        Method::PUT,
        "/erp/customers/1",
        json!({"customer_name": "无键代改"}),
    )
    .await;
    assert_eq!(
        write_status,
        StatusCode::FORBIDDEN,
        "读得到 ≠ 改得动：All 范围无代表键写他人行必须 403（读门复用为写门＝水平越权成因）: {wv}"
    );
    assert_eq!(wv["code"], "FORBIDDEN", "机器码契约: {wv}");
    assert_eq!(
        wv["message"],
        err_msg::PERMISSION_PUBLIC,
        "权限拒绝出参永久脱敏：外显恒为固定脱敏常量，真实原因只进日志"
    );
    assert_error_envelope_shape(&wv);
    assert_eq!(
        customer_name(&db).await,
        before,
        "被拒后必须零写入（先判后写，不落部分成功）"
    );
}

// ---------------------------------------------------------------------------
// fail-closed 侧：role_id 缺失（无法证明授权）→ 读仍可、写必拒
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_scope_missing_role_id_write_is_403_fail_closed() {
    let state = base_state().await;
    let db = state.db.clone();
    let app = build_app(state, make_auth(MGR_ALL, "mgr_all", None, "all"));

    let (status, v) = send(
        &app,
        Method::PUT,
        "/erp/customers/1",
        json!({"customer_name": "角色未加载代改"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "role_id 缺失＝无法证明代操作授权，按最小权限拒绝（fail-closed，不静默放行）: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(v["message"], err_msg::PERMISSION_PUBLIC);
    assert_eq!(
        customer_name(&db).await,
        "甲客户",
        "fail-closed 拒绝同样必须零写入"
    );
}

// ---------------------------------------------------------------------------
// 三段式 3：显式授予 crm/cross_owner_write → 2xx + 真生效 + 审计留痕可查证
// ---------------------------------------------------------------------------

#[tokio::test]
async fn all_scope_non_admin_with_behalf_key_write_succeeds_with_audit_trace() {
    let state = base_state().await;
    let db = state.db.clone();
    // 键仅在本用例内授予（不播种——棘轮锁迁移全树无 cross_owner_write）
    exec(
        &db,
        "INSERT INTO role_permissions (role_id, resource_type, action, allowed, created_at, updated_at)
         VALUES (2,'crm','cross_owner_write',TRUE,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
    )
    .await;
    let app = build_app(state, make_auth(MGR_ALL, "mgr_all", Some(2), "all"));

    let (status, v) = send(
        &app,
        Method::PUT,
        "/erp/customers/1",
        json!({"customer_name": "带键代改"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持代表键的 All 范围代他人写按裁定应 2xx: {v}"
    );
    assert_eq!(customer_name(&db).await, "带键代改", "写入必须真实生效");

    // 代操作不得夺归属：owner 列仍是原归属人（放行的是"改"，不是"转归"）
    let owners = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT created_by FROM customers WHERE id=1",
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读归属失败");
    let created_by: Option<i32> = owners[0]
        .try_get::<Option<i32>>("", "created_by")
        .expect("created_by 回读失败");
    assert_eq!(created_by, Some(OWNER), "代操作写不得改写归属人");

    // 审计留痕（可查证轨）：update_with_audit 落 actor=操作人的 audit_logs 行。
    // 放行日志轨（crm_write_guard 的 tracing::info!）在进程内测试不可捕获订阅器，
    // 不做假断言，由 ratchet_write_gate_logs_grant_trace 源码棘轮锁定其存在。
    let audits = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM audit_logs WHERE resource_type='customer' AND resource_id='1' AND user_id=80",
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("回读审计行失败");
    assert!(
        !audits.is_empty(),
        "代操作写必须有 actor=操作人(80) 的 audit_logs 行——留痕双轨之审计轨硬证据"
    );
}

// ---------------------------------------------------------------------------
// 棘轮 1：读门/写门两函数禁止合并（判定源分离是裁定的机制本体）
// ---------------------------------------------------------------------------

/// 只保留"代码 + 字符串字面量"：整行注释（`//`/`///`/`//!`）逐行剔除。
/// 本节是**禁词棘轮**（"读门不得出现 behalf_granted"），而两扇门之间的文档注释正是
/// 在解释 behalf_granted 的判定规则——不剥注释就把说明当违例。
/// 按行处理而不做字符级扫描：被锁文件含跨行字符串时单行引号配平不可靠，
/// 宁少剥（行尾尾注释、块注释不动）不可错吃代码文本。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（**仅用于正向 contains**）：剔全部空白并消掉闭合定界符前的尾逗号
/// ——rustfmt 把多参数签名拆成一行一个时，末位必有尾逗号，单行字面 needle 会假失败。
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

/// 符号定位取函数体：起点是被锁符号（在 `code_only` 文本上找，注释里的同名词不算），
/// 终点是下一个函数/条目符号。替代 `split(下一个签名)` 那种把两扇门的**文档注释**
/// 一起吞进上一扇门窗口的写法；也不做任何"符号 + 固定字节长"假设（源码含中文，
/// 固定字节窗会在多字节字符中间劈开）。
fn fn_body(code_src: &str, signature: &str) -> String {
    let anchor = code_src
        .find(signature)
        .unwrap_or_else(|| panic!("待锁符号不存在: {signature}"));
    let tail = &code_src[anchor..];
    let next = [
        "\npub async fn ",
        "\npub fn ",
        "\npub(crate) fn ",
        "\nasync fn ",
        "\nfn ",
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    pub(crate) fn ",
        "\n    async fn ",
        "\n    fn ",
        "\npub enum ",
        "\npub struct ",
        "\npub const ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

#[test]
fn read_gate_and_write_gate_must_stay_separate_functions() {
    let src = include_str!("../src/utils/data_scope.rs").replace('\r', "");
    // 两扇门的边界用**符号定位 + 大括号配平**取，不用 `split(下一扇门签名)`：
    // 后者会把写门的整段文档注释（`data_scope.rs:180-193` 解释"为什么读侧和写侧必须
    // 分家"、并逐条列出 `behalf_granted` 判定规则）吞进读门窗口，于是"读门不得掺入
    // 代操作键"这条棘轮被**文档**判红（判责 B1①）。注释先剥，再按符号切片。
    let code = code_only(&src);
    let read_gate = fn_body(&code, "pub fn check_resource_owner(");
    assert!(
        read_gate.contains("DataScope::All => true"),
        "回潮棘轮：读门 All 分支被改（裁定要求读侧 All 跨 owner 保持不变）"
    );
    assert!(
        !read_gate.contains("behalf_granted"),
        "回潮棘轮：读门被掺入代操作键参数（两判定源必须物理分离，防再被当同一门复用）"
    );
    // 读门签名本身也不得出现第四个参数（把写门的键参数并进读门 = 同扇门复用）
    assert!(
        canon(&read_gate).contains(&canon(
            "pub fn check_resource_owner(ctx: &DataScopeContext, resource_owner_id: Option<i32>, resource_dept_id: Option<i32>) -> bool"
        )),
        "回潮棘轮：读门签名形态被改（判定源分离是裁定的机制本体）"
    );

    let write_gate = fn_body(&code, "pub fn check_resource_write_owner(");
    assert!(
        write_gate.contains("DataScope::All => behalf_granted"),
        "回潮棘轮：写门 All 分支不再回落代表键（＝读门语义渗入写侧，水平越权回潮）"
    );
    assert!(
        write_gate.contains("DataScope::Self_ => false"),
        "回潮棘轮：Self_ 跨 owner 写不再是无条件拒绝"
    );
    // 两扇门必须是两个独立函数体（同一实现里既判读又判写 = 本锁要防的合并形态）
    assert!(
        canon(&write_gate).contains(&canon(
            "pub fn check_resource_write_owner(ctx: &DataScopeContext, resource_owner_id: Option<i32>, resource_dept_id: Option<i32>, behalf_granted: bool) -> bool"
        )),
        "回潮棘轮：写门签名少了 behalf_granted 参数（代操作授权无处判定）"
    );
    assert!(
        !read_gate.contains("cross_owner_write") && !read_gate.contains("behalf"),
        "回潮棘轮：读门出现代操作授权字样（读侧不得复用写门判定）"
    );
}

// ---------------------------------------------------------------------------
// 棘轮 2：放行留痕的源码锁（日志轨不可进程内捕获，锁构造本身存在，不做假断言）
// ---------------------------------------------------------------------------

#[test]
fn ratchet_write_gate_logs_grant_trace() {
    let guard = include_str!("../src/handlers/crm_write_guard.rs");
    assert!(
        guard.contains("CROSS_OWNER_WRITE_KEY: (&str, &str) = (\"crm\", \"cross_owner_write\")"),
        "代操作键定义漂移（必须与 role_permissions.resource_type/action 成对一致）"
    );
    assert!(
        guard.contains("tracing::info!(") && guard.contains("代操作写放行"),
        "回潮棘轮：放行留痕（tracing::info! 结构化日志）被删——代操作必须可审计回溯"
    );
    assert!(
        guard.contains("auth.role_id.ok_or_else"),
        "回潮棘轮：role_id 缺失的 fail-closed 拒绝路径被改"
    );
}

// ---------------------------------------------------------------------------
// 棘轮 3：该键不播种给任何角色——迁移全树扫描
// ---------------------------------------------------------------------------

#[test]
fn cross_owner_write_key_is_never_seeded_in_any_migration() {
    fn walk(dir: &std::path::Path, hits: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("遍历迁移目录失败 {}: {e}", dir.display()))
        {
            let path = entry.expect("迁移目录项读取失败").path();
            if path.is_dir() {
                walk(&path, hits);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let content = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()));
                if content.contains("cross_owner_write") {
                    hits.push(path.display().to_string());
                }
            }
        }
    }
    let mut hits = Vec::new();
    walk(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("migration"),
        &mut hits,
    );
    assert!(
        hits.is_empty(),
        "裁定硬约束被破坏：迁移为角色播种了 crm/cross_owner_write（该键默认不播种，须运维在权限界面显式授予）: {hits:?}"
    );
}

// ---------------------------------------------------------------------------
// 棘轮 4：已接入的 CRM 写入口逐个仍在门后（shrink-only 清单，防绕门回潮）
// ---------------------------------------------------------------------------

#[test]
fn all_wired_crm_write_entries_still_behind_the_gate() {
    // (文件, 函数名, 允许的门形态)——单行入口 ensure_cross_owner_write_allowed；
    // 批量/合并族（行集合由 service 锁内确定）允许 handler 查键一次 + service 写门形态
    let entries: &[(&str, &str, &[&str])] = &[
        (
            "src/handlers/customer_handler.rs",
            "update_customer",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/customer_handler.rs",
            "delete_customer",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_customer_handler.rs",
            "update_customer",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_customer_handler.rs",
            "delete_customer",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_customer_handler.rs",
            "add_tags",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "update_lead",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "delete_lead",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "update_lead_status",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "update_opportunity",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "delete_opportunity",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "close_opportunity_as_lost",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "auto_assign_lead",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "score_lead",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_handler.rs",
            "merge_leads",
            &["cross_owner_write_behalf_granted"],
        ),
        (
            "src/handlers/customer_merge_handler.rs",
            "merge_customers",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_assignment_handler.rs",
            "assign_customer",
            &["ensure_cross_owner_write_allowed"],
        ),
        (
            "src/handlers/crm_assignment_handler.rs",
            "batch_assign",
            &["cross_owner_write_behalf_granted"],
        ),
        (
            "src/handlers/crm_assignment_handler.rs",
            "transfer_lead",
            &["ensure_cross_owner_write_allowed"],
        ),
    ];
    for (file, func, accepted_markers) in entries {
        let src =
            std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file))
                .unwrap_or_else(|e| panic!("读取 {file} 失败: {e}"));
        let body = src
            .split(&format!("pub async fn {func}"))
            .nth(1)
            .unwrap_or_else(|| panic!("{file}::{func} 定义缺失（入口被删/改名须同步本棘轮清单）"));
        let body = body.split("\npub async fn ").next().expect("函数体边界");
        assert!(
            accepted_markers.iter().any(|m| body.contains(m)),
            "回潮棘轮：CRM 写入口 {file}::{func} 不再在跨 owner 写门之后（水平越权入口复活）"
        );
    }

    // 合并 service 的行级写门（handler 查键 + service 判定分离形态）同步锁
    let lead_service = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/crm/lead.rs"),
    )
    .expect("读取 services/crm/lead.rs 失败");
    let merge_body = lead_service
        .split("pub async fn merge_leads")
        .nth(1)
        .expect("merge_leads service 缺失")
        .split("pub async fn lead_funnel_report")
        .next()
        .expect("merge_leads 边界缺失");
    assert!(
        merge_body.contains("check_resource_write_owner"),
        "回潮棘轮：合并 service 的行级判定不再是写门（读门复用为写门）"
    );
}
