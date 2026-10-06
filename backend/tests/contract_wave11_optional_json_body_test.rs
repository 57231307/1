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
//! - 扫描锁（不连库、纯源码普查）：`src/handlers/**` 中函数名按词含
//!   `approve|issue|submit|close|cancel|confirm|complete` 的 handler，签名入参出现
//!   ① **裸 `Json<T>`**（含 `axum::Json<T>` 全路径写法）且对应 DTO **无任何非 Option
//!   必填字段**，或 ② **`Option<Json<T>>` 假可选形态**（已被证伪并全量弃用：只在完全
//!   不带 Content-Type 时才 None，带 JSON 头+空体仍在解码层被判 400，业务门永不可达）
//!   ——判红并逐条列出 file:line。判据先把签名逐行去 `//` 注释、去字符串字面量内容、
//!   再去全部空白，rustfmt 折行与全路径写法不得绕过；真必填端点（DTO 有非 Option
//!   字段）不受 ①约束，②对所有动作端点一律禁止。签名截取走括号配平，返回类型里的
//!   `Json<...>` 不进入判据视野。配平与扫描全程按**字节索引**（ASCII 定界符不可能出现
//!   在 UTF-8 多字节序列内部），char/byte 混用会让含中文注释的文件整体错位、动作端点
//!   被静默跳过（假绿），由检测力自测锁钉死。
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
    JsonCtEmpty,
    /// 无 Content-Type 且无体（axios 无体调用形态）
    NoCtEmpty,
    /// Content-Type: application/json + 体为 JSON 文本
    Json(&'a str),
    /// 非 JSON content-type 且体非空（如 text/plain）
    NonJsonCt(&'a str, &'a str),
}

async fn call(app: &Router, path: &str, shape: ReqShape<'_>) -> (StatusCode, Value) {
    let builder = Request::builder().method(Method::POST).uri(path);
    let req = match shape {
        ReqShape::JsonCtEmpty => builder
            .header("content-type", "application/json")
            .body(Body::empty())
            .unwrap(),
        ReqShape::NoCtEmpty => builder.body(Body::empty()).unwrap(),
        ReqShape::Json(text) => builder
            .header("content-type", "application/json")
            .body(Body::from(text.to_string()))
            .unwrap(),
        ReqShape::NonJsonCt(ct, text) => builder
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
        ReqShape::JsonCtEmpty,
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
        ReqShape::NoCtEmpty,
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
        ReqShape::Json("{}"),
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
        ReqShape::Json(r#"{"approval_reason":"OK，同意采购"}"#),
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
        ReqShape::JsonCtEmpty,
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
        ReqShape::NoCtEmpty,
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
        ReqShape::Json("{"),
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
        ReqShape::Json(r#"{"approval_reason":123}"#),
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
        ReqShape::NonJsonCt("text/plain", "hello"),
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

/// 括号配平截取：从 open_idx 处的 `(` 或 `{` 起，返回其配对闭合后的内部内容。
/// **全程字节索引**：定界符 `(` `)` `{` `}` 均为 ASCII，UTF-8 多字节序列的后续字节
/// 恒 ≥0x80，不可能与它们撞码，按字节扫描既正确又不会在中文注释处错位
/// （曾用 `Vec<char>` 配字节索引，导致含中文的 handler 整体错位、动作端点被静默跳过）。
fn balanced_inner(src: &str, open_idx: usize) -> Option<String> {
    let bytes = src.as_bytes();
    let opener = *bytes.get(open_idx)?;
    let closer = match opener {
        b'(' => b')',
        b'{' => b'}',
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
                return Some(String::from_utf8_lossy(&bytes[open_idx + 1..i]).into_owned());
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
            // balanced_inner 返回的是字节跨度切片（body.len() 即真实字节跨度），
            // 因此 from 按字节推进仍然对齐
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

/// 签名预处理：逐行去 `//` 行注释、去字符串字面量内容，再去全部空白。
/// 目的：① 评审注释/属性字符串里出现的被禁形态样例不得误命中；
/// ② rustfmt 对超长类型会在 `Option<` 与 `Json<` 之间（或 `:` 后）折行，
///    去空白后子串判据不得被折行绕过。
/// 已知局限：不处理 `/* */` 块注释与字符串内的转义引号嵌套（签名内出现即畸形，
/// 宁可多判由 unresolved/violations 显式暴露，不静默）。
fn normalize_signature(sig: &str) -> String {
    let mut out = String::with_capacity(sig.len());
    for line in sig.lines() {
        let cs: Vec<char> = line.chars().collect();
        let mut i = 0usize;
        let mut in_str = false;
        while i < cs.len() {
            let c = cs[i];
            if in_str {
                if c == '"' && !(i > 0 && cs[i - 1] == '\\') {
                    in_str = false;
                }
                i += 1;
                continue;
            }
            if c == '/' && i + 1 < cs.len() && cs[i + 1] == '/' {
                break;
            }
            if c == '"' {
                in_str = true;
                i += 1;
                continue;
            }
            if !c.is_whitespace() {
                out.push(c);
            }
            i += 1;
        }
    }
    out
}

/// 检出的「假可选」形态类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FalseOptionalKind {
    /// `Json<T>` 裸入参（含 `axum::Json<T>` 全路径）——是否违规还取决于 DTO 有无必填字段
    BareJson,
    /// `Option<Json<T>>` 假可选入参——本仓已证伪并全量弃用，动作端点见一个红一个，
    /// 与 DTO 是否含必填字段无关
    OptionJson,
}

/// 归一化签名里枚举假可选形态：返回 (类别, DTO 基名)。
/// 判据互不误伤的根据（fixture 自测逐条钉死）：
/// - `":Json<"`：`OptionalJson<T>` 参数位是 `: OptionalJson<`，`Json<` 前紧邻的是字母
///   `al` 而非 `:`，不构成命中；`Vec<Json<T>>` 值位 `Json<` 前是 `<`，同样不命中；
///   而 `axum::Json<T>` 参数位含 `::Json<`，被本判据捕获（旧判据 `": Json<"` 放过它）。
/// - `"Option<Json<"`：`OptionalJson<` 里 `Option` 后紧邻字母 `a` 而非 `<`，不命中；
///   `Option<OptionalJson<T>>` 这类杂交形态里 `Option<` 后是 `Optional`，两判据都不
///   命中——该形态无合法语义，由换装棘轮与返回类型括号配平外的显式盲区清单管理。
fn detect_false_optional(sig: &str) -> Vec<(FalseOptionalKind, String)> {
    let norm = normalize_signature(sig);
    let mut hits = Vec::new();
    for (needle, kind) in [
        (":Json<", FalseOptionalKind::BareJson),
        ("Option<Json<", FalseOptionalKind::OptionJson),
    ] {
        let mut from = 0usize;
        while let Some(rel) = norm[from..].find(needle) {
            let pos = from + rel;
            let dto_raw: String = norm[pos + needle.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            let dto = dto_raw.rsplit("::").next().unwrap_or("").to_string();
            hits.push((kind, dto));
            from = pos + 1;
        }
    }
    hits
}

/// 逐 `pub async fn` 解析源码，返回动作名端点的 (函数名, 签名起始行号, 括号内签名原文)。
/// 括号配平截取保证返回类型（`) -> Result<Json<...>>`）不进入判据视野。
fn iter_action_endpoints(src: &str) -> Vec<(String, usize, String)> {
    let mut out = Vec::new();
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
        let paren = start + fn_name.len();
        let Some(paren_idx) = src[paren..].find('(').map(|p| paren + p) else {
            from = start;
            continue;
        };
        let Some(sig) = balanced_inner(src, paren_idx) else {
            from = paren_idx + 1;
            continue;
        };
        let line_no = src[..start].chars().filter(|c| *c == '\n').count() + 1;
        if action_name(&fn_name) {
            out.push((fn_name, line_no, sig));
        }
        from = paren_idx + 1;
    }
    out
}

/// 对单个动作端点签名执行判据分类，结果写进 violations/unresolved（fixture 与真实扫描共用）
fn classify_action_sig(
    rel: &str,
    line_no: usize,
    fn_name: &str,
    sig: &str,
    struct_map: &BTreeMap<String, bool>,
    violations: &mut Vec<String>,
    unresolved: &mut Vec<String>,
) {
    for (kind, dto) in detect_false_optional(sig) {
        match kind {
            FalseOptionalKind::OptionJson => violations.push(format!(
                "{rel}:{line_no} {fn_name} 假可选 Option<Json<{dto}>> 形态禁止回潮（该形态只在完全不带 Content-Type 时才 None，带 JSON 头+空体仍被解码层判 400、业务门不可达），必须改为 OptionalJson<{dto}>"
            )),
            FalseOptionalKind::BareJson => match struct_map.get(&dto) {
                Some(true) => {} // 真必填端点：不在本锁范围（不得为凑扫描数把必填字段改成选填）
                Some(false) => violations.push(format!(
                    "{rel}:{line_no} {fn_name} 裸 Json<{dto}> 且 DTO 无非 Option 必填字段（假可选必须改 OptionalJson）"
                )),
                None => unresolved.push(format!(
                    "{rel}:{line_no} {fn_name} 裸 Json<{dto}> 的 DTO 定义未在 src 树中解析到"
                )),
            },
        }
    }
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
    let mut scanned_endpoints = 0usize;
    for f in &handler_files {
        let src = read_utf8_lossy(f);
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f.as_path())
            .display()
            .to_string()
            .replace('\\', "/");
        for (fn_name, line_no, sig) in iter_action_endpoints(&src) {
            scanned_endpoints += 1;
            classify_action_sig(
                &rel,
                line_no,
                &fn_name,
                &sig,
                &struct_map,
                &mut violations,
                &mut unresolved,
            );
        }
    }
    // 检测力地板：动作端点实测 142（2026-10-06），断言地板防「解析错位→静默跳过→
    // 违规数假装为 0」的真空回归——扫描数骤降必须在这里变红，而不是让 violations 变干净
    assert!(
        scanned_endpoints >= 100,
        "扫描锁检测力地板：本轮实际解析到动作端点 {scanned_endpoints} 个，低于保守地板 100 \
         （历史实测 142）。签名解析错位会让扫描锁空转成假绿，必须显式处置"
    );
    assert!(
        unresolved.is_empty(),
        "扫描锁：出现无法解析 DTO 定义的动作端点裸 Json 入参（判据盲区必须显式处置，不许静默跳过）:\n{}",
        unresolved.join("\n")
    );
    assert!(
        violations.is_empty(),
        "扫描锁：以下动作端点存在被禁的假可选入参形态（裸 Json<T>+DTO 无必填字段，或已弃用的 Option<Json<T>>），必须换成 OptionalJson（真必填端点不在裸 Json 判据范围，禁止为清零改选填）:\n{}",
        violations.join("\n")
    );
}

#[test]
fn source_scan_optional_json_absent_from_handlers_means_ratchet_holds() {
    // 棘轮旁证：OptionalJson 提取器已在 handler 层被真实使用（防「加了工具没人用」的假修）。
    // 口径 = handler 签名里 `: OptionalJson<` 使用点总数（归一化后匹配，覆盖「解构形参」
    // 与「命名形参 + `.0` 消费」两种形态）。曾按「含 `OptionalJson(` 的文件数」统计，
    // 命名形参形态被整体漏掉，11 < 阈值 15 造成与真实覆盖无关的假红；换装站点为 29 处。
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut handler_files = Vec::new();
    collect_rs_files(&root.join("src").join("handlers"), &mut handler_files);
    let sites: usize = handler_files
        .iter()
        .map(|f| {
            normalize_signature(&read_utf8_lossy(f))
                .matches(":OptionalJson<")
                .count()
        })
        .sum();
    assert!(
        sites >= 15,
        "OptionalJson 签名使用点不应少于既有换装站点数下限（当前统计 {sites}，阈值 15，换装批实测 29 处）"
    );
}

// =========================================================
// 扫描锁检测力双向自证（fixture 级，不连库）：
// 缺检测力的门禁等于假绿——①被禁形态注入必被抓；②合规形态不被抓；
// ③真实树合规侧（29 站点）由上面两个 source_scan 锁负责，这里锁判据本身。
// =========================================================

/// fixture 样例源码走与真实扫描完全相同的代码路径（iter → classify）
fn scan_fixture(src: &str, struct_map: &BTreeMap<String, bool>) -> (Vec<String>, Vec<String>) {
    let mut violations = Vec::new();
    let mut unresolved = Vec::new();
    for (fn_name, line_no, sig) in iter_action_endpoints(src) {
        classify_action_sig(
            "fixture.rs",
            line_no,
            &fn_name,
            &sig,
            struct_map,
            &mut violations,
            &mut unresolved,
        );
    }
    (violations, unresolved)
}

/// ① 注入被禁形态，题面必被抓
#[test]
fn gate_selftest_banned_forms_must_be_caught() {
    // Option<Json<T>>：本批刚证伪弃用的假可选形态，旧判据 `": Json<"` 对它零命中
    // （`Json` 前紧邻 `<` 而非 `": "`），静默回潮通道必须由新判据堵死
    let mut required_map = BTreeMap::new();
    required_map.insert("FakeReq".to_string(), true);
    let option_json = "/// 审批入口（中文文档注释前置：锁死 char/byte 索引错位导致\n\
         /// 动作端点被静默跳过的历史盲区——中文文件里判据必须仍然解析到本函数）
pub async fn approve_x(
    State(state): State<AppState>,
    payload: Option<Json<FakeReq>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _ = state;
    Ok(Json(ApiResponse::success(())))
}
";
    let (v, u) = scan_fixture(option_json, &required_map);
    assert!(u.is_empty(), "fixture 不应产生未解析项: {u:?}");
    assert_eq!(
        v.len(),
        1,
        "Option<Json<FakeReq>> 形态必须被抓（即便 DTO 真必填，形态本身已禁）, 实际: {v:?}"
    );
    assert!(
        v[0].contains("approve_x") && v[0].contains("Option<Json<FakeReq>>"),
        "违规条目必须点名函数与形态, 实际: {v:?}"
    );

    // rustfmt 对超长类型折行不得绕过判据
    let wrapped = "pub async fn approve_y(\n    payload: Option<\n        Json<FakeReq>,\n    >,\n) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> { let _ = payload; Ok(Json(ApiResponse::success(()))) }\n";
    let (v2, u2) = scan_fixture(wrapped, &required_map);
    assert!(u2.is_empty());
    assert_eq!(
        v2.len(),
        1,
        "折行的 Option<Json<… 形态必须被抓, 实际: {v2:?}"
    );

    // 裸 Json 全路径写法不得漏网（旧判据 `": Json<"` 同样放过它）
    let mut optional_only_map = BTreeMap::new();
    optional_only_map.insert("FakeReq".to_string(), false);
    let qualified = "pub async fn approve_z(payload: axum::Json<FakeReq>) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> { let _ = payload; Ok(Json(ApiResponse::success(()))) }\n";
    let (v3, u3) = scan_fixture(qualified, &optional_only_map);
    assert!(u3.is_empty(), "全路径裸 Json 的 DTO 必须可解析: {u3:?}");
    assert_eq!(
        v3.len(),
        1,
        "axum::Json<FakeReq> + DTO 无必填字段必须被抓, 实际: {v3:?}"
    );
}

/// ② 合规形态不被抓
#[test]
fn gate_selftest_compliant_forms_must_not_be_caught() {
    let mut map = BTreeMap::new();
    map.insert("ApproveARequest".to_string(), false);
    map.insert("ApproveCRequest".to_string(), false);
    map.insert("RejectBRequest".to_string(), true); // 真必填 reject 族 DTO
    let cases = [
        (
            "OptionalJson 命名形参（payload.0 消费形态）",
            "pub async fn approve_a(\n    payload: OptionalJson<ApproveARequest>,\n) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> { let _ = payload.0; Ok(Json(ApiResponse::success(()))) }\n",
        ),
        (
            "OptionalJson 解构形参",
            "pub async fn approve_c(\n    OptionalJson(req): OptionalJson<ApproveCRequest>,\n) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> { let _ = req; Ok(Json(ApiResponse::success(()))) }\n",
        ),
        (
            "真必填裸 Json（reject 族 DTO 含非 Option 字段）",
            "pub async fn approve_b(\n    payload: Json<RejectBRequest>,\n) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> { let _ = payload; Ok(Json(ApiResponse::success(()))) }\n",
        ),
    ];
    for (desc, src) in cases {
        let (v, u) = scan_fixture(src, &map);
        assert!(
            v.is_empty() && u.is_empty(),
            "{desc} 是合规形态，不得被判违规或未解析, 实际 v={v:?} u={u:?}"
        );
    }
}

/// ③ 注释、字符串字面量、返回类型、非动作名不得误命中
#[test]
fn gate_selftest_noise_contexts_do_not_false_positive() {
    let mut map = BTreeMap::new();
    map.insert("ApproveZRequest".to_string(), false);
    let sample = r#"pub async fn approve_z(
    State(s): State<AppState>,
    // 评审注释里的被禁形态样例：payload: Option<Json<ApproveZRequest>> 不得触发判据
    #[deprecated = "payload: Option<Json<ApproveZRequest>> 曾在此处使用"]
    payload: OptionalJson<ApproveZRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let _ = (s, payload.0);
    Ok(Json(ApiResponse::success(())))
}

pub async fn fetch_report(
    payload: Option<Json<ApproveZRequest>>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let _ = payload;
    Ok(Json(ApiResponse::success(())))
}
"#;
    // approve_z：签名内含注释+字符串字面量两处形态样例，但真实形参合规 → 0 命中；
    // 返回类型 `Result<Json<...>` 在括号配平截取之外 → 不构成裸 Json 命中，也不产生未解析；
    // fetch_report 非动作名，词级匹配不得咬合 → 整体 0 命中。
    let (v, u) = scan_fixture(sample, &map);
    assert!(
        v.is_empty() && u.is_empty(),
        "注释/字符串字面量/返回类型/非动作名均不得误命中, 实际 v={v:?} u={u:?}"
    );
}
