//! PII 按需揭示（customers:reveal）契约锁
//!
//! 锁定五类判据（对应验收 R3/R4/R5/R6）：
//! ① 越权 ⇒ 403 + 机器码 FORBIDDEN，且 `pii_reveal_audit` **不落成功痕**
//!   （行为级：真 PG + 真 handler，行级 `data_scope` 门对非本人行拒绝；
//!   RBAC 缺键通道的 403 由权限中间件在路由层拦截，其授予面由本文件
//!   的三通道集合相等钉测静态锁住，不由本行为用例重复承担）；
//! ② 合法揭示 ⇒ 200 且返回所请求字段的**原文**，审计恰好落一行，
//!   审计行内**不含任何原文**（整行序列化后逐值断言不含电话/邮箱字面量）；
//!   请求体伪造 `operator_id/user_id/revealed_by` 被 serde 忽略，
//!   操作人恒等于会话 `AuthContext.user_id`（与建单人取会话同源判据）；
//! ③ 默认路径掩码回归锁：标准客户详情/列表出口（收口前形态）仍为
//!   `field_mask` 权威掩码形态——本批不得让任何常规出口因新键而放松；
//! ④ 审计表列形状锁：`pii_reveal_audit` 的列名集合与关键列类型逐字钉死
//!   （多列/少列/改型即红；`revealed_fields` 必须是 ARRAY 列）；
//! ⑤ 揭示字段集合严格等于请求白名单：响应 `fields` 键集合与请求 token 集
//!   合**相等**（多给一个键即判红）；白名单外 token（含 `id_card` 这类
//!   本域无载体列的）整笔 400 + VALIDATION_ERROR 且不落痕。
//!
//! 拒绝类断言口径（R5 红线）：只断 HTTP 状态 + 机器码 `code`，
//! 不断中文文案；额外断言拒绝出参 message 不含记录 ID（固定脱敏常量）。
//!
//! 三通道同口径钉测（照抄 pieces:print 批次判据，防"一边改一边忘"）：
//! ⓪ 注册表 `init_service.rs::PERMISSION_RESOURCES` 含 `customers`，且
//!    `middleware/permission.rs::PATH_ACTION_KEYWORDS` 含 `reveal`
//!    （URL 段推导 `customers:reveal` 的前提）；
//! ① 新装库矩阵 `init_service_ops/permission.rs`：("customers","reveal")
//!    恰 2 处（sales_rep/crm_rep），与 ② 在剔除在册别名码后**双向集合相等**；
//! ② 存量库迁移 m0086：REVEAL_ROLE_CODES = ①集合 ∪ {salesperson}；
//! ③ e2e `global-setup.ts` SEED_ROLE_EXTRA_PERMISSIONS：承载
//!    `customers:reveal` 的角色集是 ①∩②（并在册别名）的**子集**（③ 是
//!    CI 补建面而非全等通道，fabric 类角色缺口口径同 wave7 文件头）。
//! 另钉 m0085 建表/m0086 授权在 business 域 up 链尾与 down 链首逆序对称、
//! m0085 down 真实回滚（不留空实现）、授予不得静默（NOTICE）、禁 ON CONFLICT。

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
use bingxi_backend::handlers::{crm_customer_handler, customer_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{customer, pii_reveal_audit};
use bingxi_backend::utils::field_mask;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend,
    DatabaseConnection, EntityTrait, QueryFilter, Statement,
};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话操作人（合法揭示场景的本人行归属者；私有 id 段 955x 不与其它夹具冲突）
const OPERATOR: i32 = 9551;
/// 他人行的归属人（越权场景：OPERATOR 读不到、也揭示不了其行）
const OTHER_OWNER: i32 = 9552;
/// 请求体伪造身份键的值——永远不允许落进审计列
const FORGED_B: i32 = 9553;

/// 夹具原文（测试私有字面量；断言"审计不含原文"时逐字引用）
const RAW_PHONE: &str = "13900139551";
const RAW_EMAIL: &str = "pii-reveal-locked@example.com";
const RAW_ADDRESS: &str = "杭州市滨江区PII揭示契约路551号";

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn make_auth(user_id: i32, scope: &str, role_id: Option<i32>) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_pii_{user_id}"),
        role_id,
        department_id: Some(1),
        data_scope: Some(scope.to_string()),
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

fn layered_app(
    state: AppState,
    auth: AuthContext,
    routes: Vec<(&'static str, axum::routing::MethodRouter<AppState>)>,
) -> Router {
    let mut router: Router<AppState> = Router::new();
    for (path, method_route) in routes {
        router = router.route(path, method_route);
    }
    router
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn call(
    app: &Router,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(path);
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let req = builder
        .body(match body {
            Some(v) => Body::from(v.to_string()),
            None => Body::empty(),
        })
        .unwrap();
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

/// 种一个客户行（归属 owner；原文三列全部落库，供揭示与掩码两侧断言）
async fn seed_customer(db: &DatabaseConnection, owner: i32, tag: &str) -> customer::Model {
    customer::ActiveModel {
        customer_code: Set(format!("CUST-PII-{tag}")),
        customer_name: Set(format!("PII揭示契约客户 {tag}")),
        contact_person: Set(Some("李联系人".to_string())),
        contact_phone: Set(Some(RAW_PHONE.to_string())),
        contact_email: Set(Some(RAW_EMAIL.to_string())),
        address: Set(Some(RAW_ADDRESS.to_string())),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("other".to_string()),
        owner_id: Set(owner),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子客户 {tag} 失败: {e}"))
}

async fn audit_rows(db: &DatabaseConnection, record_id: i32) -> Vec<pii_reveal_audit::Model> {
    pii_reveal_audit::Entity::find()
        .filter(pii_reveal_audit::Column::RecordId.eq(record_id))
        .all(db)
        .await
        .unwrap_or_else(|e| panic!("审计直读失败: {e}"))
}

// =========================================================
// 行为级活体锁（真 PG + 真 handler）
// =========================================================

#[tokio::test]
async fn pii_reveal_foreign_row_returns_403_forbidden_without_success_trace() {
    let db = setup_test_db().await;
    let foreign = seed_customer(&db, OTHER_OWNER, "FOREIGN").await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    // OPERATOR 为 self 范围：他人行必须被行级门拒绝（与 RBAC 缺键同一出参形状）
    let app = layered_app(
        state,
        make_auth(OPERATOR, "self", Some(2)),
        vec![(
            "/customers/{id}/pii/reveal",
            post(crm_customer_handler::reveal_customer_pii),
        )],
    );
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/customers/{}/pii/reveal", foreign.id),
        Some(json!({
            "fields": ["phone"],
            "reason": "联系客户前核对号码",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "越权揭示必须 403, 实际: {v}");
    assert_eq!(
        v["code"], "FORBIDDEN",
        "拒绝出参机器码必须是 FORBIDDEN（只断状态与机器码，不断文案）, 实际: {v}"
    );
    // 拒绝文案永久脱敏：不得回显记录 ID
    let message = v["message"].as_str().unwrap_or_default();
    assert!(
        !message.contains(&foreign.id.to_string()),
        "拒绝文案禁止携带记录 ID, 实际: {message}"
    );
    let rows = audit_rows(&read_db, foreign.id).await;
    assert!(
        rows.is_empty(),
        "被拒绝的揭示绝不允许落成功痕，实际 {} 行: {:?}",
        rows.len(),
        rows
    );
}

#[tokio::test]
async fn pii_reveal_own_row_returns_raw_and_writes_single_trace_without_raw_values() {
    let db = setup_test_db().await;
    let owned = seed_customer(&db, OPERATOR, "OWN").await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        make_auth(OPERATOR, "self", Some(2)),
        vec![(
            "/customers/{id}/pii/reveal",
            post(crm_customer_handler::reveal_customer_pii),
        )],
    );
    // 请求体故意携带伪造身份键：serde 必须直接忽略（操作人只认会话）
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/customers/{}/pii/reveal", owned.id),
        Some(json!({
            "fields": ["phone", "email"],
            "reason": "  联系客户前核对号码  ",
            "operator_id": FORGED_B,
            "user_id": FORGED_B,
            "revealed_by": FORGED_B,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "合法揭示必须 200, 实际: {v}");
    assert_eq!(v["code"], 200, "成功信封 code 必须为 200, 实际: {v}");
    let fields = v["data"]["fields"]
        .as_object()
        .expect("成功揭示出参必须有 data.fields 对象");
    // 返回的是原文本身（非掩码形态）
    assert_eq!(
        fields.get("phone"),
        Some(&json!(RAW_PHONE)),
        "phone 必须揭示原文"
    );
    assert_eq!(
        fields.get("email"),
        Some(&json!(RAW_EMAIL)),
        "email 必须揭示原文"
    );

    let rows = audit_rows(&read_db, owned.id).await;
    assert_eq!(rows.len(), 1, "合法揭示后审计恰好落一行");
    let row = &rows[0];
    assert_eq!(row.record_type, "customer");
    assert_eq!(row.record_id, owned.id);
    assert_eq!(
        row.operator_user_id, OPERATOR,
        "操作人必须等于会话用户，绝不等于 body 伪造值 {FORGED_B}"
    );
    assert_ne!(
        row.operator_user_id, FORGED_B,
        "body 伪造身份绝不允许落审计列"
    );
    assert_eq!(
        row.reason, "联系客户前核对号码",
        "用途落库为 trim 后的原文请求值"
    );
    let mut got_fields = row.revealed_fields.clone();
    got_fields.sort();
    assert_eq!(got_fields, vec!["email".to_string(), "phone".to_string()]);

    // 审计表不存原文：整行序列化后逐字面量断言缺席
    let row_json = serde_json::to_string(row).expect("审计行序列化");
    for secret in [RAW_PHONE, RAW_EMAIL, RAW_ADDRESS] {
        assert!(
            !row_json.contains(secret),
            "审计行禁止包含 PII 原文: {secret}"
        );
    }
}

#[tokio::test]
async fn pii_reveal_rejects_unlisted_token_and_returns_400_without_trace() {
    let db = setup_test_db().await;
    let owned = seed_customer(&db, OPERATOR, "BADTOK").await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        make_auth(OPERATOR, "self", Some(2)),
        vec![(
            "/customers/{id}/pii/reveal",
            post(crm_customer_handler::reveal_customer_pii),
        )],
    );
    // id_card 在本域无载体列，未登记白名单：混入即整笔拒绝（不部分放行）
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/customers/{}/pii/reveal", owned.id),
        Some(json!({ "fields": ["phone", "id_card"], "reason": "字段值域越界探测" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "白名单外字段必须 400");
    assert_eq!(v["code"], "VALIDATION_ERROR", "实际: {v}");
    assert!(
        audit_rows(&read_db, owned.id).await.is_empty(),
        "入参被拒同样不得落成功痕"
    );
}

#[tokio::test]
async fn pii_reveal_rejects_blank_reason_with_400_and_no_trace() {
    let db = setup_test_db().await;
    let owned = seed_customer(&db, OPERATOR, "NOREASON").await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        make_auth(OPERATOR, "self", Some(2)),
        vec![(
            "/customers/{id}/pii/reveal",
            post(crm_customer_handler::reveal_customer_pii),
        )],
    );
    for reason in ["", "   "] {
        let (status, v) = call(
            &app,
            Method::POST,
            &format!("/customers/{}/pii/reveal", owned.id),
            Some(json!({ "fields": ["phone"], "reason": reason })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "空用途必须 400, 实际: {v}");
        assert_eq!(v["code"], "VALIDATION_ERROR", "实际: {v}");
    }
    assert!(
        audit_rows(&read_db, owned.id).await.is_empty(),
        "缺用途的揭示不得落痕"
    );
}

#[tokio::test]
async fn pii_reveal_response_field_set_equals_request_whitelist_exactly() {
    let db = setup_test_db().await;
    let owned = seed_customer(&db, OPERATOR, "EXACT").await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        make_auth(OPERATOR, "self", Some(2)),
        vec![(
            "/customers/{id}/pii/reveal",
            post(crm_customer_handler::reveal_customer_pii),
        )],
    );
    // 只请求 phone：响应 fields 键集合必须恰为 {phone}——多给 email/address/
    // contact_* 任一键即判红（揭示面不得比申请面宽）
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/customers/{}/pii/reveal", owned.id),
        Some(json!({ "fields": ["phone"], "reason": "只核对号码" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "实际: {status} {v}");
    let fields = v["data"]["fields"].as_object().expect("fields 对象存在");
    let mut keys: Vec<&String> = fields.keys().collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["phone"],
        "响应字段集合必须严格等于请求白名单（多给键即扩大揭示面）"
    );
    for leaked in ["email", "address", "contact_email", "contact_phone"] {
        assert!(
            !fields.contains_key(leaked),
            "未申请的字段 {leaked} 绝不允许出现在揭示响应里"
        );
    }
    // 留痕字段集合同样只含 phone
    let rows = audit_rows(&read_db, owned.id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].revealed_fields, vec!["phone".to_string()]);
}

#[tokio::test]
async fn pii_reveal_default_detail_and_list_stay_masked_regression_lock() {
    let db = setup_test_db().await;
    let owned = seed_customer(&db, OPERATOR, "MASKLOCK").await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    // 无角色账号（role_id=None ⇒ fail-closed 非 admin）走标准详情/列表出口：
    // 收口实现 apply_customer_field_permission 的默认脱敏层必须仍产出权威掩码
    let app = layered_app(
        state,
        make_auth(OPERATOR, "all", None),
        vec![
            ("/customers/{id}", get(customer_handler::get_customer)),
            ("/customers", get(customer_handler::list_customers)),
        ],
    );
    let detail_expect = field_mask::mask_phone(RAW_PHONE);
    let email_expect = field_mask::mask_email(RAW_EMAIL);

    let (status, v) = call(&app, Method::GET, &format!("/customers/{}", owned.id), None).await;
    assert_eq!(status, StatusCode::OK, "默认详情必须照常 200, 实际: {v}");
    assert_eq!(
        v["data"]["contact_phone"].as_str(),
        Some(detail_expect.as_str()),
        "详情出口 phone 必须是权威掩码形态（本批不得放松默认路径）"
    );
    assert_eq!(
        v["data"]["contact_email"].as_str(),
        Some(email_expect.as_str()),
        "详情出口 email 必须是权威掩码形态"
    );
    assert!(
        v["data"].get("address").is_none(),
        "非 admin 详情出口 address 必须维持整键移除形态"
    );

    let (status, v) = call(&app, Method::GET, "/customers?page=1&page_size=10", None).await;
    assert_eq!(status, StatusCode::OK, "默认列表必须照常 200, 实际: {v}");
    let items = v["data"]["items"]
        .as_array()
        .expect("列表出参 data.items 数组");
    let row = items
        .iter()
        .find(|it| it["id"].as_i64() == Some(owned.id as i64))
        .expect("列表应包含种子客户行");
    assert_eq!(
        row["contact_phone"].as_str(),
        Some(detail_expect.as_str()),
        "列表出口 phone 必须是权威掩码形态"
    );
    assert_eq!(
        row["contact_email"].as_str(),
        Some(email_expect.as_str()),
        "列表出口 email 必须是权威掩码形态"
    );
    assert!(
        row.get("address").is_none(),
        "非 admin 列表出口 address 必须维持整键移除形态"
    );
}

#[tokio::test]
async fn pii_reveal_audit_table_column_shape_is_locked() {
    let db = setup_test_db().await;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"SELECT "column_name", "data_type" FROM information_schema.columns
                WHERE "table_name" = 'pii_reveal_audit' ORDER BY "column_name""#,
            Vec::<sea_orm::Value>::new(),
        ))
        .await
        .expect("列形状查询失败");
    let mut cols: Vec<(String, String)> = Vec::new();
    for r in rows {
        let name: String = r.try_get_by_index(0).expect("column_name");
        let dtype: String = r.try_get_by_index(1).expect("data_type");
        cols.push((name, dtype));
    }
    let names: Vec<&str> = cols.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "id",
            "operator_user_id",
            "reason",
            "record_id",
            "record_type",
            "revealed_at",
            "revealed_fields",
        ],
        "pii_reveal_audit 列集合必须恰为该七列（增删改列名需同步建表迁移与本锁）"
    );
    let dtype = |want: &str| {
        cols.iter()
            .find(|(n, _)| n == want)
            .map(|(_, t)| t.as_str())
            .unwrap_or("<missing>")
    };
    assert_eq!(dtype("revealed_fields"), "ARRAY", "字段集合列必须是数组列");
    assert_eq!(
        dtype("operator_user_id"),
        "integer",
        "操作人列宽与 users.id 一致"
    );
    assert_eq!(
        dtype("record_id"),
        "integer",
        "记录 ID 列宽与 customers.id 一致"
    );
    assert_eq!(dtype("reason"), "text", "用途列为 TEXT");
    // 表里绝不允许出现存原文的列（判据②的 schema 面防线：白名单外新增
    // phone/email/contact_* 列即红）
    for forbidden in [
        "phone",
        "email",
        "contact_phone",
        "contact_email",
        "address",
    ] {
        assert!(
            !names.contains(&forbidden),
            "审计表禁止出现原文列: {forbidden}"
        );
    }
}

// =========================================================
// 三通道同口径钉测（静态源解析，形态照抄 pieces:print 批次）
// =========================================================

/// 通道①矩阵 + 通道②迁移 + 通道③e2e 的**裁定口径集**（本文件头载明）：
/// 显式授 reveal 的规范角色码恰为销售/CRM 两接触岗
const REVEAL_ROLES: &[&str] = &["crm_rep", "sales_rep"];
/// m0086 在册部署/e2e 别名码（CI 库建 salesperson 不建 sales_rep）
const REGISTERED_REVEAL_ALIAS_CODES: &[&str] = &["salesperson"];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn reveal_migration_src() -> String {
    include_str!("../migration/src/domain/business/m0086_grant_customers_pii_reveal.rs")
        .replace('\r', "")
}

fn audit_migration_src() -> String {
    include_str!("../migration/src/domain/business/m0085_add_pii_reveal_audit.rs").replace('\r', "")
}

fn registry_src() -> String {
    include_str!("../src/services/init_service.rs").replace('\r', "")
}

fn middleware_src() -> String {
    include_str!("../src/middleware/permission.rs").replace('\r', "")
}

fn business_mod_src() -> String {
    include_str!("../migration/src/domain/business/mod.rs").replace('\r', "")
}

fn routes_src() -> String {
    include_str!("../src/routes/crm.rs").replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：整行注释逐行剔除（判据同 wave7：
/// "禁止写 customers:*"这类说明文字不是授予，计入即假判）
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（仅用于正向存在性判定）：剔全部空白并消掉闭合定界符前尾逗号
fn canon(src: &str) -> String {
    let mut out: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            return out;
        }
        out = next;
    }
}

/// 区间内全部成对双引号 token
fn quoted_tokens(seg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = seg.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut tok = String::new();
            for n in chars.by_ref() {
                if n == '"' {
                    break;
                }
                tok.push(n);
            }
            out.push(tok);
        }
    }
    out
}

/// 迁移常量清单解析（取源文件真实清单而非本测手抄，防"改一边改测试"掩盖漂移）
fn migration_role_codes(mig_code: &str, const_name: &str) -> Vec<String> {
    let anchor = mig_code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("m0086 必须存在 const {const_name}（通道② 集合判据定位符）"));
    let rest = &mig_code[anchor..];
    let open = rest.find("&[").expect("m0086 常量应为 &[&str] 形态");
    let close = rest[open..].find("];").expect("m0086 常量清单未闭合");
    let mut codes = quoted_tokens(&rest[open..open + close]);
    codes.sort();
    codes.dedup();
    codes
}

/// 通道① 矩阵分组（分组签名 = 角色码行 + 紧随的 &[ 行；判据同 wave7）
fn matrix_role_groups(matrix_code: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = matrix_code.lines().collect();
    let is_role_code_line = |l: &str| -> bool {
        let t = l.trim();
        t.len() > 3
            && t.starts_with('"')
            && t.ends_with("\",")
            && t[1..t.len() - 2]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_')
    };
    let mut groups = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "&[" && i > 0 && is_role_code_line(lines[i - 1]) {
            let code = lines[i - 1].trim().trim_end_matches(',').trim_matches('"');
            let mut end = lines.len();
            let mut j = i + 1;
            while j < lines.len() {
                if lines[j].trim() == "&[" && is_role_code_line(lines[j - 1]) {
                    end = j - 1;
                    break;
                }
                j += 1;
            }
            groups.push((code.to_string(), lines[i..end].join("\n")));
            i = j;
        } else {
            i += 1;
        }
    }
    groups
}

fn matrix_roles_with(matrix_code: &str, resource: &str, action: &str) -> Vec<String> {
    let needle = format!(r#"("{resource}", "{action}")"#);
    let mut codes: Vec<String> = matrix_role_groups(matrix_code)
        .into_iter()
        .filter(|(_, body)| body.contains(&needle))
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// 通道③：解析 e2e SEED_ROLE_EXTRA_PERMISSIONS 中承载指定权限码的角色码集合
fn e2e_seed_roles_with(perm: &str) -> Vec<String> {
    let src = code_only(&e2e_seed_src());
    let anchor = src
        .find("const SEED_ROLE_EXTRA_PERMISSIONS")
        .expect("global-setup 必须存在 SEED_ROLE_EXTRA_PERMISSIONS");
    let rest = &src[anchor..];
    let open = rest
        .find('{')
        .expect("SEED_ROLE_EXTRA_PERMISSIONS 对象字面量未开启");
    let close = rest
        .find("\n};")
        .expect("SEED_ROLE_EXTRA_PERMISSIONS 对象字面量未闭合");
    let body = &rest[open..close];
    let mut roles = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let colon = t.find(':');
        let starts_entry = match colon {
            Some(c) => {
                c > 0
                    && t[..c].starts_with(|ch: char| ch.is_ascii_lowercase())
                    && t[..c]
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch == '_')
                    && t[c + 1..].trim_start().starts_with('[')
            }
            None => false,
        };
        if starts_entry {
            if let Some((code, acc)) = current.take() {
                if acc.contains(perm) {
                    roles.push(code);
                }
            }
            let c = colon.expect("starts_entry 已含冒号位");
            current = Some((t[..c].to_string(), t[c + 1..].to_string()));
        } else if let Some((_, acc)) = current.as_mut() {
            acc.push('\n');
            acc.push_str(t);
        }
    }
    if let Some((code, acc)) = current {
        if acc.contains(perm) {
            roles.push(code);
        }
    }
    roles.sort();
    roles
}

fn sorted_refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

fn adjudicated_sorted<'a>(list: &'a [&'a str]) -> Vec<&'a str> {
    let mut v = list.to_vec();
    v.sort_unstable();
    v
}

/// 通道⓪：注册表资源名在册 + 动作关键字表含 reveal（URL 推导前提）
#[test]
fn reveal_runtime_key_is_derivable_from_url() {
    let registry = code_only(&registry_src());
    assert!(
        registry.contains("pub const PERMISSION_RESOURCES"),
        "PERMISSION_RESOURCES 注册表缺失"
    );
    assert!(
        registry.contains("\"customers\","),
        "customers 必须登记进 PERMISSION_RESOURCES（揭示端点资源段即 customers）"
    );
    let middleware = code_only(&middleware_src());
    assert!(
        middleware.contains("const PATH_ACTION_KEYWORDS"),
        "PATH_ACTION_KEYWORDS 缺失"
    );
    assert!(
        middleware.contains("\"reveal\","),
        "reveal 必须登记进 PATH_ACTION_KEYWORDS，否则 POST /crm/customers/{{id}}/pii/reveal \
         的动作会回落成 create，运行时键漂出 customers:reveal（三通道全部失配）"
    );
    // 端点真实挂载在 crm 域路由（资源段推导的前提）
    let routes = code_only(&routes_src());
    assert!(
        routes.contains("\"/customers/{id}/pii/reveal\""),
        "揭示端点必须挂载在 crm 域路由（路径写法即运行时键推导输入）"
    );
    assert!(
        routes.contains("reveal_customer_pii"),
        "路由必须绑定 reveal_customer_pii handler"
    );
}

/// 通道①：矩阵 reveal 授予恰 2 处且与裁定口径一致；最小授权禁项
#[test]
fn role_matrix_grants_exactly_reveal_for_customer_contact_roles() {
    let src = code_only(&matrix_src());
    assert_eq!(
        src.matches(r#"("customers", "reveal")"#).count(),
        2,
        "customers:reveal 在角色矩阵中应恰为 2 处（sales_rep/crm_rep）"
    );
    let holders = matrix_roles_with(&src, "customers", "reveal");
    assert_eq!(
        sorted_refs(&holders),
        adjudicated_sorted(REVEAL_ROLES),
        "矩阵 reveal 集合须等于裁定口径集 {:?}",
        REVEAL_ROLES
    );
    // 通道内自洽：reveal ⊆ read（先能在本域读到行，才谈得上按需揭示原文）
    let readers = matrix_roles_with(&src, "customers", "read");
    for code in &holders {
        assert!(
            readers.contains(code),
            "矩阵 customers:reveal 角色 {code} 必须同时持有 customers:read"
        );
    }
    // 最小授权：禁止借 reveal 之名扩授第二套 PII 键（揭示只有这一个动作）
    for banned in [
        r#"("customers", "pii")"#,
        r#"("customers", "pii-reveal")"#,
        r#"("crm", "pii-reveal")"#,
    ] {
        assert!(
            !src.contains(banned),
            "禁止注册表外自造 PII 揭示键（三通道同源判据）: {banned}"
        );
    }
}

/// 通道①↔②：剔除在册别名码后**双向集合相等**（检出漂移的那条锁）
#[test]
fn matrix_and_reveal_migration_role_sets_are_equal_modulo_registered_aliases() {
    let matrix = matrix_roles_with(&code_only(&matrix_src()), "customers", "reveal");
    let mig = migration_role_codes(&code_only(&reveal_migration_src()), "REVEAL_ROLE_CODES");
    let mig_without_alias: Vec<&str> = mig
        .iter()
        .map(String::as_str)
        .filter(|c| !REGISTERED_REVEAL_ALIAS_CODES.contains(c))
        .collect();
    assert_eq!(
        sorted_refs(&matrix),
        mig_without_alias,
        "通道① 矩阵与通道② m0086 对 customers:reveal 的集合须在剔除在册别名码 \
         {:?} 后双向相等（改一处必须同步改另一处并在此改判）",
        REGISTERED_REVEAL_ALIAS_CODES
    );
    assert_eq!(
        sorted_refs(&matrix),
        adjudicated_sorted(REVEAL_ROLES),
        "reveal 矩阵集合须等于裁定口径集"
    );
    for alias in REGISTERED_REVEAL_ALIAS_CODES {
        assert!(
            mig.iter().any(|c| c == alias),
            "在册别名码 {alias} 必须在 m0086 在场（CI/部署库真实受授面）"
        );
        assert!(
            !matrix.iter().any(|c| c == alias),
            "矩阵不得双写别名码 {alias}（销售面矩阵只认规范码 sales_rep）"
        );
    }
}

/// 通道③：e2e SEED 的 reveal 授予面必须是 ①∩②（并在册别名）的子集；
/// 解析签名为空亦红（空集会让子集恒真——假绿通道）
#[test]
fn e2e_seed_reveal_roles_are_subset_of_backend_channels() {
    let matrix = matrix_roles_with(&code_only(&matrix_src()), "customers", "reveal");
    let mig = migration_role_codes(&code_only(&reveal_migration_src()), "REVEAL_ROLE_CODES");
    let mut allowed: Vec<String> = matrix.iter().filter(|c| mig.contains(c)).cloned().collect();
    for alias in REGISTERED_REVEAL_ALIAS_CODES {
        if mig.iter().any(|c| c == alias) {
            allowed.push(alias.to_string());
        }
    }
    let seeded = e2e_seed_roles_with("'customers:reveal'");
    assert!(
        !seeded.is_empty(),
        "e2e SEED 未解析出任何 'customers:reveal' 授予角色——授予被删或分组签名变化，\
         空集会让子集断言恒真，拒绝放行"
    );
    for role in &seeded {
        assert!(
            allowed.contains(role),
            "通道③ e2e SEED 给 {role} 授 'customers:reveal'，但该码不在通道①② 规范\
             受授面（∪在册别名）内：①={matrix:?} ②={mig:?}"
        );
    }
    let seed_src = code_only(&e2e_seed_src());
    assert!(
        !seed_src.contains("'customers:*'"),
        "e2e 侧同样禁止超范围授 customers 键"
    );
}

/// 通道②：m0086 幂等授予判据（NOT EXISTS、禁 ON CONFLICT、授予结果可见化、
/// down 按角色码真实回收）
#[test]
fn reveal_migration_grants_are_idempotent_and_rolled_back_symmetrically() {
    let mig = code_only(&reveal_migration_src());
    assert_eq!(
        mig.matches(r#"SELECT r."id", 'customers'"#).count(),
        1,
        "m0086 应恰有一条 reveal 授予 INSERT"
    );
    assert_eq!(
        mig.matches("AND NOT EXISTS (").count(),
        1,
        "授予必须带 NOT EXISTS 幂等守卫"
    );
    let up_start = mig
        .find("async fn up(")
        .expect("m0086 必须有 async fn up（执行体判据起点）");
    let down_start = mig
        .find("async fn down(")
        .expect("m0086 必须有 async fn down（回滚执行体）");
    let up_body = &mig[up_start..down_start];
    for banned in ["ON CONFLICT", "CREATE UNIQUE INDEX"] {
        assert!(
            !up_body.contains(banned),
            "m0086 执行体不得使用 {banned}（依赖 v15 才建的唯一索引，见文件头）"
        );
    }
    assert!(
        up_body.contains("RAISE NOTICE") && up_body.contains("hit = 0"),
        "m0086 必须把「一条都没授上」的情形可见化（禁止静默通过）"
    );
    let down_body = &mig[down_start..];
    assert!(
        down_body.contains("DELETE FROM \"role_permissions\"")
            && down_body.contains("rp.\"action\" = 'reveal'"),
        "m0086 down 必须按角色码精确回收 reveal 授予（不留空实现）"
    );
}

/// 通道②：m0085 建表迁移自包含判据（IF NOT EXISTS、关键列自检、down 真实回滚、
/// 不回填不加业务列）
#[test]
fn audit_migration_is_self_contained_idempotent_and_reversible() {
    let mig = code_only(&audit_migration_src());
    assert!(
        mig.contains("CREATE TABLE IF NOT EXISTS \"pii_reveal_audit\""),
        "m0085 建表必须幂等（IF NOT EXISTS）"
    );
    let up_start = mig.find("async fn up(").expect("m0085 必须有 async fn up");
    let down_start = mig
        .find("async fn down(")
        .expect("m0085 必须有 async fn down（回滚执行体）");
    let up_body = &mig[up_start..down_start];
    for col in [
        "\"record_type\"",
        "\"record_id\"",
        "\"revealed_fields\"",
        "\"operator_user_id\"",
        "\"reason\"",
        "\"revealed_at\"",
    ] {
        assert!(up_body.contains(col), "m0085 必须建列 {col}");
    }
    assert!(
        !up_body.contains("ALTER TABLE"),
        "m0085 禁止给任何业务表加列（判据：只建自包含新表）"
    );
    assert!(
        up_body.contains("RAISE NOTICE") && up_body.contains("RAISE EXCEPTION"),
        "m0085 必须把重跑形态可见化且形状自检缺失即中止（禁止静默）"
    );
    let down_body = &mig[down_start..];
    assert!(
        down_body.contains("DROP TABLE IF EXISTS \"pii_reveal_audit\""),
        "m0085 down 必须真实回收本表（不留空实现）"
    );
}

/// 通道②：两迁移注册在 business 域 up 链尾 / down 链首且逆序对称
#[test]
fn pii_migrations_are_registered_in_business_domain_chain() {
    let src = code_only(&business_mod_src());
    assert!(
        src.contains("mod m0085_add_pii_reveal_audit;"),
        "m0085 未在 business 域声明"
    );
    assert!(
        src.contains("mod m0086_grant_customers_pii_reveal;"),
        "m0086 未在 business 域声明"
    );
    let up = &src[..src.find("async fn down").expect("business mod 缺 down")];
    let down = &src[src.find("async fn down").expect("business mod 缺 down")..];
    assert!(
        canon(up).contains(&canon("m0085_add_pii_reveal_audit::Migration.up(manager)")),
        "m0085 未接入 business 域 up 链"
    );
    assert!(
        canon(up).contains(&canon(
            "m0086_grant_customers_pii_reveal::Migration.up(manager)"
        )),
        "m0086 未接入 business 域 up 链"
    );
    // 应用顺序 m0085 → m0086，回滚顺序必须镜像（后应用者先回滚）
    let i85_up = up.find("m0085_add_pii_reveal_audit").unwrap();
    let i86_up = up.find("m0086_grant_customers_pii_reveal").unwrap();
    assert!(i85_up < i86_up, "up 顺序必须 m0085 先于 m0086");
    let i86_down = down.find("m0086_grant_customers_pii_reveal").unwrap();
    let i85_down = down.find("m0085_add_pii_reveal_audit").unwrap();
    let i84_down = down.find("m0084_concession_receiving_channel").unwrap();
    assert!(
        i86_down < i85_down && i85_down < i84_down,
        "down 顺序必须与 up 逆序对称：m0086 → m0085 → m0084"
    );
}

/// 白名单单源钉测：揭示服务是唯一白名单出处，域内无第二套手写清单；
/// 客户域无载体列的 token（id_card）绝不允许被登记（幽灵键缺陷族防线）
#[test]
fn reveal_whitelist_is_single_source_and_free_of_ghost_tokens() {
    let svc = code_only(&include_str!("../src/services/crm/pii_reveal.rs").replace('\r', ""));
    assert!(
        svc.contains("pub const PII_REVEAL_WHITELIST"),
        "PII_REVEAL_WHITELIST 必须是揭示服务导出的唯一定义"
    );
    assert!(svc.contains(r#"("phone", "contact_phone")"#));
    assert!(svc.contains(r#"("email", "contact_email")"#));
    assert!(svc.contains(r#"("address", "address")"#));
    assert!(
        !svc.contains("\"id_card\""),
        "id_card 在客户域无载体列，禁止登记成恒空揭示面"
    );
    // handler 侧不得再写私有 mask/私有清单（掩码单源纪律：掩码实现与私有清单只允许一处出处）
    let handler =
        code_only(&include_str!("../src/handlers/crm_customer_handler.rs").replace('\r', ""));
    let start = handler
        .find("pub async fn reveal_customer_pii")
        .expect("handler 必须存在 reveal_customer_pii");
    let body = &handler[start..];
    assert!(
        !body.contains("fn mask") && !body.contains("WHITELIST"),
        "揭示 handler 禁止内联私有掩码函数或第二套白名单（白名单单源=揭示服务）"
    );
}
