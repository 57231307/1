//! 选填 JSON 请求体提取器 `OptionalJson<T>` 契约锁
//!
//! 锁定的契约：
//! - `backend/src/utils/optional_json.rs` 语义表（四条形态全部在本文件钉死）：
//!   ① 无 `Content-Type` + 空体 ⇒ 未采集，**必须进业务门**（存在性/状态机门真实执行，
//!      不得停在解码层 400）；
//!   ② `Content-Type: application/json` + **空体（含纯空白）** ⇒ 同上进业务门——
//!      这正是 `Option<Json<T>>` 假可选形态失效的那条（带 JSON 头却无体），本锁防回潮；
//!   ③ 合法 JSON 体 ⇒ `Some(T)` 正常进业务门（证明"可选"没有吞掉有体请求）；
//!   ④ 畸形 JSON / 字段类型错 / 非 JSON content-type 且体非空 ⇒ **仍必须 400
//!      `code=VALIDATION_ERROR`**（防"顺手把门拆掉"的负向锁：缺这组，①②就是假绿）。
//! - 断言口径：只断 HTTP 状态码 + 机器 `code`，**严禁断错误文案原文**
//!   （本仓校验/业务拒绝文案永久脱敏，口径同 contract_wave11_po_so_reason_readback）。
//! - 扫描锁（不连库、纯源码普查）：`src/handlers/**` 中函数名含
//!   `approve|issue|submit|close|cancel|confirm|complete` 的 handler，若入参仍是
//!   **裸 `Json<T>`** 且对应 DTO **无任何非 Option 必填字段**，判红并逐条列出
//!   file:line——「假可选」通道禁止回潮；真必填端点（DTO 有非 Option 字段）不受本锁约束。
//!
//! 夹具形态：真 PostgreSQL `setup_test_db`（已迁移库 + TRUNCATE 业务表），
//! tower `oneshot` 真实 handler；auth 注入照 contract_wave5_dye_batch 先例。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{chemical_handler, purchase_order_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::chemical_requisition;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use test_common::setup_test_db;
use tower::ServiceExt;

const OPERATOR_ID: i32 = 9731;
/// 不存在于清空后的 purchase_orders 表的 id：命中服务层存在性门（404 NOT_FOUND）
const ABSENT_PO_ID: i32 = 99_999_999;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: OPERATOR_ID,
        username: format!("w11_optional_json_{OPERATOR_ID}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

async fn fresh_app() -> Router {
    let db = setup_test_db().await;
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/purchase/orders/{id}/approve",
            post(purchase_order_handler::approve_order),
        )
        .route(
            "/chemical-requisitions/{id}/approve",
            post(chemical_handler::approve_requisition),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
}

/// 种子一条「已审批」领用单：对 approved 行再 approve 必命中状态机门
/// （requisition.rs:201-219「仅草稿(draft)状态可审批」→ BUSINESS_ERROR），
/// 用于证明缺体请求真正走进了业务门而不是解码层。
async fn seed_approved_requisition(db: &DatabaseConnection, tag: &str) -> i32 {
    let row = chemical_requisition::ActiveModel {
        requisition_no: Set(format!("CR-W11OJ-{tag}")),
        requisition_type: Set("production".to_string()),
        requisition_date: Set(Utc::now().date_naive()),
        status: Set("approved".to_string()),
        total_amount: Set(Decimal::ZERO),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子领用单 {tag} 失败: {e}"));
    row.id
}

async fn read_requisition(db: &DatabaseConnection, id: i32) -> chemical_requisition::Model {
    chemical_requisition::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap_or_else(|e| panic!("领用单 {id} 直读失败: {e}"))
        .unwrap_or_else(|| panic!("领用单 {id} 必须存在"))
}

/// 请求形态枚举：双向锁的四类输入在同一端点上逐一对打
enum ReqShape<'a> {
    /// 带 Content-Type: application/json 但**完全无体**（e2e apiCallExpectFail 形态）
    JsonCtEmptyBody,
    /// 无 Content-Type 且无体（axios 无体调用形态）
    NoCtEmptyBody,
    /// Content-Type: application/json + 体为 JSON 文本
    JsonBody(&'a str),
    /// 非 JSON content-type 且体非空（如 text/plain）
    NonJsonCtWithBody(&'a str, &'a str),
}

async fn call(app: &Router, path: &str, shape: ReqShape<'_>) -> (StatusCode, Value) {
    let builder = Request::builder().method(Method::POST).uri(path);
    let req = match shape {
        ReqShape::JsonCtEmptyBody => builder
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap(),
        ReqShape::NoCtEmptyBody => builder.body(Body::empty()).unwrap(),
        ReqShape::JsonBody(text) => builder
            .header("content-type", "application/json")
            .body(Body::from(text.to_string()))
            .unwrap(),
        ReqShape::NonJsonCtWithBody(ct, text) => builder
            .header("content-type", HeaderValue::from_str(ct).unwrap())
            .body(Body::from(text.to_string()))
            .unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

// =========================================================
// 形态①②③：缺体/无头/空对象/合法体 一律进业务门（存在性门 404 NOT_FOUND）
// =========================================================

#[tokio::test]
async fn absent_body_with_json_content_type_reaches_business_gate_not_decoding_400() {
    // 「带 JSON 头 + 空体」曾被 Option<Json<T>> 假可选形态在解码层判 400
    // VALIDATION_ERROR，现必须进服务层存在性门 → 404
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::JsonCtEmptyBody,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "缺体+JSON 头必须进业务门（不存在单据 → 404），实际: {status} {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND", "机器码必须来自业务门信封");
}

#[tokio::test]
async fn absent_body_without_content_type_reaches_business_gate() {
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::NoCtEmptyBody,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "无头+缺体必须进业务门, 实际: {status} {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
}

#[tokio::test]
async fn empty_object_body_reaches_business_gate() {
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::JsonBody("{}"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "体 {{}} 必须按合法选填输入进业务门, 实际: {status} {v}"
    );
    assert_eq!(v["code"], "NOT_FOUND");
}

#[tokio::test]
async fn valid_reason_body_still_parsed_and_reaches_business_gate() {
    // 正向锁：OptionalJson 不得把「有合法体」的请求吞成 None——
    // 理由正常解析后同样走到存在性门（同一 404，证明 Some 路径通畅）
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::JsonBody(r#"{"approval_reason":"OK，同意采购"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "实际: {status} {v}");
    assert_eq!(v["code"], "NOT_FOUND");
}

// =========================================================
// 形态② 的业务门族：缺体命中状态机真实门（e2e 62-04/70-04 期望码）
// =========================================================

#[tokio::test]
async fn absent_body_on_requisition_approve_hits_real_state_machine_business_error() {
    let db = setup_test_db().await;
    let id = seed_approved_requisition(&db, "APPR").await;
    let read_db = db.clone();
    let state = AppState {
        db: std::sync::Arc::new(db),
        ..Default::default()
    };
    // 路由按占位符注册（同真实 routes 形态），调用 URI 用具体 id
    let app = Router::new()
        .route(
            "/chemical-requisitions/{id}/approve",
            post(chemical_handler::approve_requisition),
        )
        .with_state(state);
    // 带 JSON 头 + 空体：必须命中 requisition.rs 的「仅 draft 可审批」状态门
    let (status, v) = call(
        &app,
        &format!("/chemical-requisitions/{id}/approve"),
        ReqShape::JsonCtEmptyBody,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "已审批单重复 approve 必须被状态机拒绝（400），实际: {status} {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "缺体必须走到真实状态机门（业务族），不得是解码层 VALIDATION_ERROR"
    );

    // 无痕断言：被状态门拒绝的行状态未被半途推进/改写
    let row = read_requisition(&read_db, id).await;
    assert_eq!(row.status, "approved", "状态门拒绝后行状态必须保持原值");

    // 无 content-type + 空体：同一业务门形态
    let (status2, v2) = call(
        &app,
        &format!("/chemical-requisitions/{id}/approve"),
        ReqShape::NoCtEmptyBody,
    )
    .await;
    assert_eq!(status2, StatusCode::BAD_REQUEST, "实际: {status2} {v2}");
    assert_eq!(v2["code"], "BUSINESS_ERROR");
}

// =========================================================
// 形态④：负向锁——「有体但非法」绝不许被归一成 None 吞掉
// =========================================================

#[tokio::test]
async fn malformed_json_body_still_rejected_400_validation_error() {
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::JsonBody("{"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "畸形 JSON 必须仍 400（不得退化成未采集放行）, 实际: {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn wrong_field_type_body_still_rejected_400_validation_error() {
    let app = fresh_app().await;
    // approval_reason 是 Option<String>，传数字即字段类型错：必须 400 VALIDATION_ERROR
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::JsonBody(r#"{"approval_reason":123}"#),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "字段类型错必须仍 400, 实际: {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn non_json_content_type_with_body_rejected_400_validation_error() {
    let app = fresh_app().await;
    let (status, v) = call(
        &app,
        &format!("/purchase/orders/{ABSENT_PO_ID}/approve"),
        ReqShape::NonJsonCtWithBody("text/plain", "hello"),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "非 JSON content-type 且体非空必须显式拒绝, 实际: {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
}

// =========================================================
// 扫描锁（不连库）：动作名端点禁止「裸 Json<T> + DTO 无必填字段」假可选回潮
// =========================================================

fn read_utf8_lossy(p: &Path) -> String {
    std::fs::read_to_string(p)
        .unwrap_or_else(|e| panic!("扫描锁夹具：读取 {} 失败: {e}", p.display()))
        .replace('\r', "")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("扫描锁夹具：目录 {} 读取失败: {e}", dir.display()))
    {
        let path = entry.expect("目录项读取").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// 括号配平截取：从 open_idx 处的 `(` 或 `{` 起，返回其配对闭合后的内部内容
fn balanced_inner(src: &str, open_idx: usize) -> Option<String> {
    let bytes: Vec<char> = src.chars().collect();
    let opener = bytes.get(open_idx).copied()?;
    let closer = match opener {
        '(' => ')',
        '{' => '}',
        _ => return None,
    };
    let mut depth = 0usize;
    let mut i = open_idx;
    while i < bytes.len() {
        if bytes[i] == opener {
            depth += 1;
        } else if bytes[i] == closer {
            depth -= 1;
            if depth == 0 {
                return Some(bytes[open_idx + 1..i].iter().collect());
            }
        }
        i += 1;
    }
    None
}

/// `pub struct NAME { ... }` 体里是否含非 Option 且无 serde(default/skip) 的必填字段
fn struct_body_has_required_field(body: &str) -> bool {
    let mut pending_serde_default = false;
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//!") || line.starts_with("//") {
            continue;
        }
        if line.starts_with("#[") {
            pending_serde_default = line.contains("serde(default") || line.contains("serde(skip");
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("pub ") {
            // `field: Type,`；rustfmt 会把超长类型换行（如 Vec<crate::…>），
            // 类型不随行时按「未判定」保守跳过（宁可漏判不误判，防误红 CI）
            if let Some(colon) = rest.find(':') {
                let ty = rest[colon + 1..].trim().trim_end_matches(',').trim();
                if ty.is_empty() {
                    continue;
                }
                let optional = ty.starts_with("Option<") || pending_serde_default;
                if !optional {
                    return true;
                }
            }
        }
        pending_serde_default = false;
    }
    false
}

/// 全 src 建 struct 表：名字 → 「任一同名定义含必填字段」
fn build_struct_required_map(files: &[PathBuf]) -> BTreeMap<String, bool> {
    let mut map: BTreeMap<String, bool> = BTreeMap::new();
    for f in files {
        let src = read_utf8_lossy(f);
        let mut from = 0usize;
        while let Some(rel) = src[from..].find("pub struct ") {
            let start = from + rel + "pub struct ".len();
            // 结构名：到第一个非 [A-Za-z0-9_] 为止
            let name: String = src[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                from = start;
                continue;
            }
            // 找到该 struct 的 `{`（跳过 `: 父类` 与泛型参数）
            let Some(brace_rel) = src[start..].find('{') else {
                from = start;
                continue;
            };
            let brace_idx = start + brace_rel;
            let Some(body) = balanced_inner(&src, brace_idx) else {
                from = brace_idx + 1;
                continue;
            };
            let has_req = struct_body_has_required_field(&body);
            let e = map.entry(name.clone()).or_insert(false);
            *e |= has_req;
            // from 用字节偏移推进：body.len() 是字节长（chars().count() 会与
            // 中文字段注释混用字节/字符索引而错位）
            from = brace_idx + body.len() + 2;
        }
    }
    map
}

/// 函数名按 snake_case 逐词精确匹配动作词表（不做子串匹配——
/// `detect_duplicate_leads` 含子串 "complete" 会被误伤；词级匹配既不遗漏
/// （首/尾/中间词，如 approve_l1、cancel_writeoff、finance_approve）也不误咬）。
fn action_name(fn_name: &str) -> bool {
    const ACTIONS: [&str; 7] = [
        "approve", "issue", "submit", "close", "cancel", "confirm", "complete",
    ];
    fn_name.split('_').any(|tok| ACTIONS.contains(&tok))
}

#[test]
fn source_scan_action_endpoints_must_not_use_false_optional_bare_json() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_root = root.join("src");
    let handlers_dir = root.join("src").join("handlers");

    let mut all_files = Vec::new();
    collect_rs_files(&src_root, &mut all_files);
    let struct_map = build_struct_required_map(&all_files);

    let mut handler_files = Vec::new();
    collect_rs_files(&handlers_dir, &mut handler_files);
    assert!(
        handler_files.len() > 50,
        "扫描锁前提：handler 文件数异常（夹具目录指错？）读到 {}",
        handler_files.len()
    );

    let mut violations: Vec<String> = Vec::new();
    let mut unresolved: Vec<String> = Vec::new();
    for f in &handler_files {
        let src = read_utf8_lossy(f);
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f.as_path())
            .display()
            .to_string()
            .replace('\\', "/");
        let mut from = 0usize;
        while let Some(relpos) = src[from..].find("pub async fn ") {
            let start = from + relpos + "pub async fn ".len();
            let fn_name: String = src[start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if fn_name.is_empty() {
                from = start;
                continue;
            }
            let paren = start + fn_name.chars().count();
            let Some(paren_idx) = src[paren..].find('(').map(|p| paren + p) else {
                from = start;
                continue;
            };
            let Some(sig) = balanced_inner(&src, paren_idx) else {
                from = paren_idx + 1;
                continue;
            };
            let line_no = src[..start].chars().filter(|c| *c == '\n').count() + 1;
            if action_name(&fn_name) {
                // 裸 Json 形态判据：`: Json<` 且前面不是 `Optional`
                // （`OptionalJson<` 里 "Json<" 前是字母，`: ` 后直接 `Json<` 才是裸形态）
                let mut scan = 0usize;
                while let Some(r) = sig[scan..].find(": Json<") {
                    let pos = scan + r;
                    // 排除 `): Optional<Json<...>>` 假可选旧形态：
                    // 其前缀是 "Option<"；也排除 `Option<Json<` 出现在值位的情形
                    let prefix_ok = pos == 0 || !sig[..pos].ends_with(": Option<");
                    if prefix_ok {
                        let dto_start = pos + ": Json<".len();
                        let dto: String = sig[dto_start..]
                            .chars()
                            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                            .collect();
                        let dto_name = dto.rsplit("::").next().unwrap_or(dto.as_str()).to_string();
                        match struct_map.get(&dto_name) {
                            Some(true) => {} // 真必填端点：不在本锁范围（任务禁为清零改选填）
                            Some(false) => violations.push(format!(
                                "{rel}:{line_no} {} 裸 Json<{dto_name}> 且 DTO 无非 Option 必填字段（假可选必须改 OptionalJson）",
                                fn_name
                            )),
                            None => unresolved.push(format!(
                                "{rel}:{line_no} {} 裸 Json<{dto_name}> 的 DTO 定义未在 src 树中解析到",
                                fn_name
                            )),
                        }
                    }
                    scan = pos + 1;
                }
            }
            from = paren_idx + 1;
        }
    }
    assert!(
        unresolved.is_empty(),
        "扫描锁：出现无法解析 DTO 定义的动作端点裸 Json 入参（判据盲区必须显式处置，不许静默跳过）:\n{}",
        unresolved.join("\n")
    );
    assert!(
        violations.is_empty(),
        "扫描锁：以下动作端点仍是「裸 Json<T> + DTO 无必填字段」假可选形态，必须换成 OptionalJson（真必填端点不在本锁范围，禁止为清零改选填）:\n{}",
        violations.join("\n")
    );
}

#[test]
fn source_scan_optional_json_absent_from_handlers_means_ratchet_holds() {
    // 棘轮旁证：OptionalJson 提取器已在 handler 层被真实使用（防「加了工具没人用」的假修）
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut handler_files = Vec::new();
    collect_rs_files(&root.join("src").join("handlers"), &mut handler_files);
    let users = handler_files
        .iter()
        .filter(|f| read_utf8_lossy(f).contains("OptionalJson("))
        .count();
    assert!(
        users >= 15,
        "OptionalJson 解构使用点不应少于既有换装站点数（当前统计 {users}）"
    );
}
