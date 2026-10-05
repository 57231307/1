//! 报价单两理由出参可读回 + 合同状态词表唯一来源静态锁（后端线）
//!
//! 锁定的口径：
//! - `QuotationResponseDto` 与列 `sales_quotations.approval_reason` /
//!   `rejection_reason` 逐字同名（snake_case 出参键），两动作两列对称可读回：
//!   approve 后详情与列表都能原样取回通过理由，reject 后同理取回拒绝理由，
//!   理由"落库但没人能读回"即追溯断链，本文件把它钉死。
//! - 填充链证据：详情 `QuotationService::get_by_id` 与列表
//!   `QuotationService::list` 均取回完整 `sales_quotation::Model`（SeaORM 全列
//!   SELECT），再经唯一的 `From<sales_quotation::Model> for QuotationResponseDto`
//!   构造 DTO——不存在第二套列集/手工 builder，所以本文件用端点出参断言即覆盖
//!   真实链路，无需（也不允许）逐行再查。
//! - 合同状态唯一来源：`models/status/bpm_crm_contract.rs` 内 `contract` 模块是
//!   sales/purchase 两合同 status 取值域的唯一词表，文件内不得再出现第二套合同
//!   状态字面量（历史上的 `contract_status` 子集模块零使用，已删除）。
//!
//! 夹具形态照 `contract_wave11_contract_quotation_approval_test.rs`：真库
//! `setup_test_db`（已迁移 PostgreSQL、nextest 串行 db-integration 组），FK 前提
//! 先种真实 customer/user 父行；路由仅在测试 Router 内注册，路径形态与生产路由
//! 逐字符一致。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::quotation_handler;
use bingxi_backend::models::status::{master_data, quotation, quotation_ext};
use bingxi_backend::models::{customer, sales_quotation, user};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

/// 注入到 AuthContext 并落 created_by 的操作人主键（users 逐例被夹具清空，可显式固定）
const OPERATOR_ID: i32 = 9621;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("夹具金额常量必须可解析为 Decimal")
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w11_quotation_reason_readback".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("报价理由回读契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = customer::ActiveModel {
        customer_code: Set(format!("CUS-W11QR-{tag}")),
        customer_name: Set("报价理由回读契约锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {tag} 插入失败: {e}"));
    m.id
}

/// 直落一行报价单（status 即夹具入参，两动作前置态由用例显式声明）
async fn seed_quotation(db: &Arc<DatabaseConnection>, tag: &str, customer_id: i32) -> i64 {
    let today = Utc::now().date_naive();
    let m = sales_quotation::ActiveModel {
        quotation_no: Set(format!("QT-W11QR-{tag}")),
        customer_id: Set(customer_id),
        sales_user_id: Set(OPERATOR_ID as i64),
        quotation_date: Set(today),
        valid_until: Set(today),
        price_terms: Set("FOB".to_string()),
        subtotal: Set(dec("120000")),
        tax_amount: Set(dec("0")),
        total_amount: Set(dec("120000")),
        status: Set(quotation_ext::PENDING_APPROVAL.to_string()),
        created_by: Set(OPERATOR_ID as i64),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子报价单 {tag} 插入失败: {e}"));
    m.id
}

/// 真库直读（出参断言之外，落库列值以数据库行做交叉取证）
async fn read_quotation(db: &Arc<DatabaseConnection>, id: i64) -> sales_quotation::Model {
    sales_quotation::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("报价单 {id} 回读失败: {e}"))
        .unwrap_or_else(|| panic!("报价单 {id} 必须存在"))
}

/// HTTP 形态夹具：详情/列表/approve/reject 四端点；auth 注入照审批锁先例
async fn quotation_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<bingxi_backend::middleware::auth_context::AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = bingxi_backend::middleware::auth_context::AuthContext {
        user_id: OPERATOR_ID,
        username: "w11_quotation_reason_readback".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = axum::Router::new()
        .route(
            "/quotations/{id}/approve",
            axum::routing::post(quotation_handler::approve_quotation),
        )
        .route(
            "/quotations/{id}/reject",
            axum::routing::post(quotation_handler::reject_quotation),
        )
        .route(
            "/quotations/{id}",
            axum::routing::get(quotation_handler::get_quotation),
        )
        .route(
            "/quotations",
            axum::routing::get(quotation_handler::list_quotations),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth));
    (db, app)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

async fn get_json(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let v: Value = serde_json::from_slice(&bytes).expect("响应必须是 JSON 统一信封");
    (status, v)
}

// ---------------------------------------------------------------------------
// 1. approve 后详情端点逐字回读 `approval_reason`（对称补上 reject 侧已可读回的
//    `rejection_reason` 键存在性）：出参键与列名逐字同名、值为 trim 后落库原文、
//    对方列保持 NULL（两动作两列不互写）。
//    改坏什么必红：DTO 缺 `approval_reason` 字段 ⇒ 键不存在断言红；From 构造点
//    不带列 ⇒ 值 null 红；approve 挪用 rejection_reason ⇒ NULL 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_then_detail_endpoint_returns_approval_reason_verbatim() {
    let (db, app) = quotation_app().await;
    let customer_id = seed_customer(&db, "A1").await;
    let quotation_id = seed_quotation(&db, "A1", customer_id).await;

    let raw_reason = "  单价与账期均在授权区间内，同意通过  ";
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{quotation_id}/approve"),
        json!({ "approval_reason": raw_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending_approval→approved 权威路径必须 200，实得 {v}"
    );

    let (status, detail) = get_json(&app, &format!("/quotations/{quotation_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        quotation::APPROVED,
        "详情状态必须读回 approved"
    );
    assert!(
        data.get("approval_reason").is_some(),
        "详情出参必须含与列名逐字同名的 approval_reason 键，实得 {data}"
    );
    assert_eq!(
        data["approval_reason"].as_str(),
        Some(raw_reason.trim()),
        "approval_reason 必须经详情端点逐字可读回（落库即可见，禁只进日志）"
    );
    assert!(
        data.get("rejection_reason").is_some(),
        "详情出参必须含 rejection_reason 键（两理由对称可读回），实得 {data}"
    );
    assert_eq!(
        data["rejection_reason"],
        Value::Null,
        "两动作两列：批准不得写 rejection_reason"
    );

    // 交叉取证：端点回读值与真库列值同源
    let row = read_quotation(&db, quotation_id).await;
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw_reason.trim()),
        "落库列值必须与端点回读一致"
    );
}

// ---------------------------------------------------------------------------
// 2. reject 后详情端点逐字回读 `rejection_reason`：值落既有专列（不挪用
//    approval_reason），状态落 approved/rejected 之外的拒绝终态。
//    改坏什么必红：拒绝理由只进日志 ⇒ 逐字回读红；挪用 approval_reason ⇒ 两断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn reject_then_detail_endpoint_returns_rejection_reason_verbatim() {
    let (db, app) = quotation_app().await;
    let customer_id = seed_customer(&db, "A2").await;
    let quotation_id = seed_quotation(&db, "A2", customer_id).await;

    let reason = "低于成本价且无客户授信支撑，不予通过";
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{quotation_id}/reject"),
        json!({ "reason": reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending_approval→rejected 权威路径必须 200，实得 {v}"
    );

    let (status, detail) = get_json(&app, &format!("/quotations/{quotation_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    assert_eq!(
        detail["data"]["status"],
        quotation::REJECTED,
        "拒绝结果必须经详情端点回读为 rejected"
    );
    assert_eq!(
        detail["data"]["rejection_reason"].as_str(),
        Some(reason),
        "rejection_reason 必须经详情端点逐字可读回"
    );
    assert_eq!(
        detail["data"]["approval_reason"],
        Value::Null,
        "两动作两列：拒绝不得写 approval_reason"
    );

    let row = read_quotation(&db, quotation_id).await;
    assert_eq!(
        row.rejection_reason.as_deref(),
        Some(reason),
        "落库列值必须与端点回读一致"
    );
}

// ---------------------------------------------------------------------------
// 3. 列表端点同样逐字回读两理由列：列表与详情共用同一 `From<Model>` 全列构造链
//    （无第二套 SELECT 列集），故同一行的两列值必须在列表项里原样出现。
//    改坏什么必红：列表若另立列集漏掉理由列 ⇒ 值断言红；DTO 字段缺失 ⇒ 键存在性红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn list_endpoint_returns_both_reason_columns_verbatim() {
    let (db, app) = quotation_app().await;
    let customer_id = seed_customer(&db, "A3").await;

    let approved_id = seed_quotation(&db, "A3a", customer_id).await;
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{approved_id}/approve"),
        json!({ "approval_reason": "列表回读锁通过理由" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "批准前置必须成功，实得 {v}"
    );

    let rejected_id = seed_quotation(&db, "A3b", customer_id).await;
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{rejected_id}/reject"),
        json!({ "reason": "列表回读锁拒绝理由" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "拒绝前置必须成功，实得 {v}"
    );

    let (status, body) = get_json(
        &app,
        &format!("/quotations?page=1&page_size=50&customer_id={customer_id}"),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "列表必须 200：{body}");
    let items = body["data"]["list"]
        .as_array()
        .expect("列表信封 data.list 必须是数组");

    let approved_row = items
        .iter()
        .find(|d| d["id"] == json!(approved_id))
        .expect("列表必须含刚批准的行");
    assert!(
        approved_row.get("approval_reason").is_some(),
        "列表项必须含 approval_reason 键（与列名逐字同名），实得 {approved_row}"
    );
    assert_eq!(
        approved_row["approval_reason"].as_str(),
        Some("列表回读锁通过理由"),
        "列表 approval_reason 必须逐字回读"
    );
    assert_eq!(
        approved_row["rejection_reason"],
        Value::Null,
        "批准行列表回读不得混入 rejection_reason"
    );

    let rejected_row = items
        .iter()
        .find(|d| d["id"] == json!(rejected_id))
        .expect("列表必须含刚拒绝的行");
    assert!(
        rejected_row.get("rejection_reason").is_some(),
        "列表项必须含 rejection_reason 键，实得 {rejected_row}"
    );
    assert_eq!(
        rejected_row["rejection_reason"].as_str(),
        Some("列表回读锁拒绝理由"),
        "列表 rejection_reason 必须逐字回读"
    );
    assert_eq!(
        rejected_row["approval_reason"],
        Value::Null,
        "拒绝行列表回读不得混入 approval_reason"
    );
}

// ---------------------------------------------------------------------------
// 4. 静态锁：`contract` 是合同状态词表唯一来源，源文件内不得再出现第二套合同
//    状态字面量数组/模块。
//    双向推演：
//    - 正向（有人重立第二套，如旧 contract_status 子集或 contract 内 inline 字面量）
//      ⇒ "draft"+"cancelled" 双指纹模块断言、块内单 literal 恰一次计数、ALL 引用式
//      常量行断言，三处之一必红；
//    - 反向（锁被绕过/contract 被改名删除）⇒ "pub mod contract {" 计数与切片
//      find 断言直接红，本用例不会静默空转。
// ---------------------------------------------------------------------------
#[test]
fn contract_status_vocabulary_has_single_source_file_lock() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/models/status/bpm_crm_contract.rs"
    ))
    .expect("合同状态词表源文件必须可读");

    // 反向锁：contract 模块必须存在且唯一（改名/删除即红，不空转）
    assert_eq!(
        src.match_indices("pub mod contract {").count(),
        1,
        "文件内 `pub mod contract` 必须恰好一处（合同状态唯一来源）"
    );
    // 第二套词表模块（历史 contract_status）不得复活，任何同前缀模块同样红
    assert!(
        !src.contains("pub mod contract_"),
        "禁止再出现 contract 之外的第二套合同状态模块"
    );

    // contract 模块块内：四常量各定义一次、字面量各出现恰一次（inline 复制即红）
    let start = src.find("pub mod contract {").expect("反向锁已保证存在");
    let contract_block = &src[start..{
        src[start + "pub mod contract {".len()..]
            .find("\npub mod ")
            .map(|i| start + "pub mod contract {".len() + i)
            .unwrap_or(src.len())
    }];
    for literal in ["\"draft\"", "\"active\"", "\"cancelled\"", "\"rejected\""] {
        assert_eq!(
            contract_block.matches(literal).count(),
            1,
            "contract 模块内字面量 {literal} 必须恰出现一次（第二份字面量清单即红）"
        );
    }
    assert!(
        contract_block.contains("pub const ALL: &[&str] = &[DRAFT, ACTIVE, CANCELLED, REJECTED];"),
        "ALL 必须由本模块常量引用派生（不得改为第二套字面量数组），实得：{contract_block}"
    );

    // 正向锁：其余任何顶层模块都不得同时含 "draft" 与 "cancelled"
    // （budget 有 draft 无 cancelled、bpm_task 有 cancelled 无 draft，均为合法他域词表；
    //   两指纹同现 = 合同状态子集另立门户的特征形态）
    let mut cursor = 0;
    while let Some(i) = src[cursor..].find("pub mod ") {
        let abs = cursor + i;
        let name_end = src[abs + "pub mod ".len()..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .expect("模块名必须可解析");
        let name = &src[abs + "pub mod ".len()..abs + "pub mod ".len() + name_end];
        let next = src[abs + "pub mod ".len()..]
            .find("\npub mod ")
            .map(|j| abs + "pub mod ".len() + j)
            .unwrap_or(src.len());
        let block = &src[abs..next];
        if name != "contract" {
            assert!(
                !(block.contains("\"draft\"") && block.contains("\"cancelled\"")),
                "模块 `{name}` 同时含合同状态字面量 draft 与 cancelled，\
                 疑似第二套合同词表（合同状态唯一来源是 contract 模块）"
            );
        }
        cursor = next;
    }
}
