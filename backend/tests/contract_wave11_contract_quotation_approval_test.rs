//! 合同与报价单审批「两动作两理由」真库契约锁（后端线）
//!
//! 已定案口径（本文件逐条钉死，锁口径见各用例头注释）：
//! - 审批 = 「通过」与「拒绝」两条动作、两条理由列：approve 端点只处理通过且
//!   **通过理由本域必填**（缺体/缺键/空串/纯空白 → 400 VALIDATION_ERROR），理由真实
//!   落 `approval_reason` 列；拒绝走独立 `/reject` 端点、**拒绝理由全域必填**
//!   （trim 非空）并落库——两合同落 `rejected_reason`，报价单落既有专列
//!   `rejection_reason`（不重命名、不挪用、不另建第二份）。
//! - 状态门：合同拒绝只允许 draft 起拒，active（权利义务已生效）/cancelled（已作废）
//!   /rejected（终态）一律拦；报价单两动作均要求 pending_approval。门拦截只断
//!   HTTP 码 + 机器码 BUSINESS_ERROR，不断文案原文（本仓业务拒绝文案永久脱敏）。
//! - 入参形态锁：approve 端点入参为 `Option<Json<T>>`。改回强类型 `Json<T>` 时，
//!   不带 body 的现存调用方（前端 api/quotation.ts、api/purchase-contract.ts、
//!   api/sales-contract.ts 批准均不发体）会在 axum 解码层收到无信封裸 400，
//!   本文件「形态①」断言即红——那正是被拍板否掉的破坏性变更。
//! - 两动作两列互不越界：approve 只写 `approval_reason`，reject 只写拒绝理由列，
//!   任一路径都不得覆写对方列（被拦形态零副作用一并钉住）。
//! - 报价单 BPM 接入路径的裁决意见仍只落 `bpm_task.approval_opinion`，业务列
//!   `approval_reason` 只承载业务侧裁量，禁双写；本文件不建 BPM 实例，钉的是
//!   业务侧端点→业务列这条真实写入链（`approval_instance_id IS NULL` 走该分支）。
//!
//! 夹具形态照 `contract_wave11_price_reject_test.rs`：真库 `setup_test_db`
//! （已迁移 PostgreSQL、nextest 串行 db-integration 组）。FK 前提：sales_quotations
//! 的 customer_id/sales_user_id/approved_by 都是真外键，须先种真实父行；
//! suppliers 属封存参照表不被清空，逐例用唯一 supplier_code 自增插入后回读主键。
//! 路由仅在测试 Router 内注册（src routes 为枢纽文件，注册动作由主智能体落地），
//! 路径形态与将注册的真实路由逐字符一致（`/{id}/approve`、`/{id}/reject`）。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    purchase_contract_handler, quotation_handler, sales_contract_handler,
};
use bingxi_backend::models::status::{contract, master_data, quotation, quotation_ext};
use bingxi_backend::models::{
    customer, purchase_contract, sales_contract, sales_quotation, supplier, user,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{json, Value};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

/// 注入到 AuthContext 并落 created_by/审计的操作人主键（users 逐例被夹具清空，可显式固定）
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
        username: Set("w11_contract_quotation_approval".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("合同报价审批契约锁操作人".to_string())),
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

/// 种子真实客户父行（sales_quotations.customer_id 与 sales_contracts.customer_id 的前提）
async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = customer::ActiveModel {
        customer_code: Set(format!("CUS-W11CA-{tag}")),
        customer_name: Set("合同报价审批契约锁客户".to_string()),
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

/// 种子真实供应商父行：suppliers 属封存参照表不被清空，显式 id 会跨例撞主键，
/// 故自增插入后回读真实主键（形状照 `contract_wave11_price_reject_test::seed_supplier`）
async fn seed_supplier(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W11CA-{tag}")),
        supplier_name: Set("合同报价审批契约锁供应商".to_string()),
        supplier_short_name: Set("审供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商 {tag} 插入失败: {e}"));
    m.id
}

/// 直落一行采购合同（状态即夹具入参，门控前提由用例显式声明）
async fn seed_purchase_contract(
    db: &Arc<DatabaseConnection>,
    tag: &str,
    supplier_id: i32,
    status: &str,
) -> i32 {
    let m = purchase_contract::ActiveModel {
        contract_no: Set(format!("PC-W11CA-{tag}")),
        contract_name: Set("合同报价审批契约锁采购合同".to_string()),
        supplier_id: Set(supplier_id),
        total_amount: Set(Some(dec("120000"))),
        status: Set(status.to_string()),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子采购合同 {tag}（{status}）插入失败: {e}"));
    m.id
}

/// 直落一行销售合同
async fn seed_sales_contract(
    db: &Arc<DatabaseConnection>,
    tag: &str,
    customer_id: i32,
    status: &str,
) -> i32 {
    let m = sales_contract::ActiveModel {
        contract_no: Set(format!("SC-W11CA-{tag}")),
        contract_name: Set("合同报价审批契约锁销售合同".to_string()),
        customer_id: Set(customer_id),
        total_amount: Set(Some(dec("120000"))),
        status: Set(status.to_string()),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售合同 {tag}（{status}）插入失败: {e}"));
    m.id
}

/// 直落一行报价单并返回主键。
///
/// `pending_approval` 是本锁要钉的两条动作的唯一前置状态：走 handler→service 的
/// 业务侧批准/拒绝路径（非 submit 内的小额自批、非 BPM 待办回写路径）。
async fn seed_quotation(
    db: &Arc<DatabaseConnection>,
    tag: &str,
    customer_id: i32,
    status: &str,
) -> i64 {
    let today = Utc::now().date_naive();
    let m = sales_quotation::ActiveModel {
        quotation_no: Set(format!("QT-W11CA-{tag}")),
        customer_id: Set(customer_id),
        sales_user_id: Set(OPERATOR_ID as i64),
        quotation_date: Set(today),
        valid_until: Set(today),
        price_terms: Set("FOB".to_string()),
        subtotal: Set(dec("120000")),
        tax_amount: Set(dec("0")),
        total_amount: Set(dec("120000")),
        status: Set(status.to_string()),
        created_by: Set(OPERATOR_ID as i64),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子报价单 {tag}（{status}）插入失败: {e}"));
    m.id
}

/// 报价单行直读（真库回读，用于与出参逐字比对）
async fn read_quotation(db: &Arc<DatabaseConnection>, id: i64) -> sales_quotation::Model {
    sales_quotation::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("报价单 {id} 回读失败: {e}"))
        .unwrap_or_else(|| panic!("报价单 {id} 必须存在"))
}

/// HTTP 形态夹具：三域 approve/reject/详情九端点；auth 注入照 price 锁先例
async fn approval_app() -> (Arc<DatabaseConnection>, axum::Router) {
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
        username: "w11_contract_quotation_approval".to_string(),
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
            "/purchase/purchase-contracts/{id}/approve",
            axum::routing::post(purchase_contract_handler::approve_contract),
        )
        .route(
            "/purchase/purchase-contracts/{id}/reject",
            axum::routing::post(purchase_contract_handler::reject_contract),
        )
        .route(
            "/purchase/purchase-contracts/{id}",
            axum::routing::get(purchase_contract_handler::get_contract),
        )
        .route(
            "/sales/sales-contracts/{id}/approve",
            axum::routing::post(sales_contract_handler::approve_contract),
        )
        .route(
            "/sales/sales-contracts/{id}/reject",
            axum::routing::post(sales_contract_handler::reject_contract),
        )
        .route(
            "/sales/sales-contracts/{id}",
            axum::routing::get(sales_contract_handler::get_contract),
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

/// 无 body 的 POST（`Option<Json<T>>` 形态专用：不带 content-type、零长度 body）
async fn post_empty_body(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .body(axum::body::Body::empty())
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
// 1. 三域通过理由缺失/空白 ⇒ 400 + VALIDATION_ERROR，且批准路径零副作用。
//    改坏什么必红：把 approve 入参改回强类型 `Json<T>` ⇒ 形态①（不带 body）落到
//    axum 解码层裸 400 文本、无 VALIDATION_ERROR 信封，红；摘掉必填门或降级成
//    静默放行 ⇒ 空理由直达服务层落库，400 断言与"列保持 NULL"断言双双红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_missing_or_blank_reason_is_400_validation_three_domains() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T1").await;
    let supplier_id = seed_supplier(&db, "T1").await;

    let quotation_id =
        seed_quotation(&db, "T1", customer_id, quotation_ext::PENDING_APPROVAL).await;
    let purchase_id = seed_purchase_contract(&db, "T1", supplier_id, contract::DRAFT).await;
    let sales_id = seed_sales_contract(&db, "T1", customer_id, contract::DRAFT).await;

    let cases = [
        (
            "quotation",
            format!("/quotations/{quotation_id}/approve"),
            quotation_ext::PENDING_APPROVAL,
        ),
        (
            "purchase_contract",
            format!("/purchase/purchase-contracts/{purchase_id}/approve"),
            contract::DRAFT,
        ),
        (
            "sales_contract",
            format!("/sales/sales-contracts/{sales_id}/approve"),
            contract::DRAFT,
        ),
    ];
    for (domain, approve_uri, expect_status) in &cases {
        // 形态①：完全不带 body——Option<Json<T>> 必须给出统一 AppError 信封
        let (status, v) = post_empty_body(&app, &approve_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} 无 body 批准必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "{domain} 无 body 批准信封必须 VALIDATION_ERROR（解码层裸文本即形态退化），实得 {v}"
        );

        // 形态②：带 JSON 体但通过理由缺键 / 空串 / 纯空白
        for body in [
            json!({}),
            json!({ "approval_reason": "" }),
            json!({ "approval_reason": "   " }),
            json!({ "approval_reason": null }),
        ] {
            let (status, v) = post_json(&app, &approve_uri, body).await;
            assert_eq!(
                status,
                axum::http::StatusCode::BAD_REQUEST,
                "{domain} 通过理由缺失形态必须 400，实得 {v}"
            );
            assert_eq!(
                v["code"], "VALIDATION_ERROR",
                "{domain} 通过理由必填信封必须 VALIDATION_ERROR，实得 {v}"
            );
        }

        // 全部拒绝形态零副作用：状态不动、approval_reason 保持空白起点
        if *domain == "quotation" {
            let row = read_quotation(&db, quotation_id).await;
            assert_eq!(row.status, *expect_status, "{domain} 被拒批准不得改状态");
            assert_eq!(
                row.approval_reason, None,
                "{domain} 被拒批准不得落 approval_reason"
            );
        } else {
            let detail_uri = approve_uri.replace("/approve", "");
            let (status, detail) = get_json(&app, &detail_uri).await;
            assert_eq!(
                status,
                axum::http::StatusCode::OK,
                "详情回读必须 200：{detail}"
            );
            assert_eq!(
                detail["data"]["status"], *expect_status,
                "{domain} 被拒批准不得改状态"
            );
            assert_eq!(
                detail["data"]["approval_reason"],
                Value::Null,
                "{domain} 被拒批准不得落 approval_reason"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 拒绝理由空/纯空白 ⇒ 400 + VALIDATION_ERROR，拒绝路径零副作用。
//    两合同为本轮新增端点；报价单 reject 为既有端点，同形态一并核对（服务端必填
//    仍在、可外显文案不带记录 ID——此处钉行为）。
//    缺 `reason` 键的形态不在本例覆盖：拒绝端点入参为强类型 `Json<T>`（与价目域
//    reject_price 同形），缺键在 axum 解码层被拒，生产链路由
//    `normalize_extractor_rejection` 映射为 400 VALIDATION_ERROR，测试 Router 不挂
//    该层，断言它会锁到与生产不同的形态，故只钉 trim 必填门本身。
//    改坏什么必红：摘掉 trim 门 ⇒ 空白理由直达服务层落库，红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn reject_blank_reason_is_400_validation_three_domains() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T2").await;
    let supplier_id = seed_supplier(&db, "T2").await;

    let quotation_id =
        seed_quotation(&db, "T2", customer_id, quotation_ext::PENDING_APPROVAL).await;
    let purchase_id = seed_purchase_contract(&db, "T2", supplier_id, contract::DRAFT).await;
    let sales_id = seed_sales_contract(&db, "T2", customer_id, contract::DRAFT).await;

    let cases = [
        (
            "quotation",
            format!("/quotations/{quotation_id}/reject"),
            quotation_ext::PENDING_APPROVAL,
        ),
        (
            "purchase_contract",
            format!("/purchase/purchase-contracts/{purchase_id}/reject"),
            contract::DRAFT,
        ),
        (
            "sales_contract",
            format!("/sales/sales-contracts/{sales_id}/reject"),
            contract::DRAFT,
        ),
    ];
    for (domain, reject_uri, expect_status) in &cases {
        for body in [
            json!({ "reason": "" }),
            json!({ "reason": "   " }),
            json!({ "reason": "\t  " }),
        ] {
            // 先固化用例文本：body 随后被 post_json 取走，消息里不能再内联引用它
            let label = body.to_string();
            let (status, v) = post_json(&app, &reject_uri, body).await;
            assert_eq!(
                status,
                axum::http::StatusCode::BAD_REQUEST,
                "{domain} 拒绝理由 {label} 必须 400，实得 {v}"
            );
            assert_eq!(
                v["code"], "VALIDATION_ERROR",
                "{domain} 拒绝理由必填必须归 VALIDATION_ERROR 信封，实得 {v}"
            );
        }

        // 零副作用：状态不动、拒绝理由列保持空白起点
        if *domain == "quotation" {
            let row = read_quotation(&db, quotation_id).await;
            assert_eq!(row.status, *expect_status, "{domain} 被拒拒绝不得改状态");
            assert_eq!(
                row.rejection_reason, None,
                "{domain} 被拒拒绝不得落 rejection_reason"
            );
        } else {
            let detail_uri = reject_uri.replace("/reject", "");
            let (status, detail) = get_json(&app, &detail_uri).await;
            assert_eq!(
                status,
                axum::http::StatusCode::OK,
                "详情回读必须 200：{detail}"
            );
            assert_eq!(
                detail["data"]["status"], *expect_status,
                "{domain} 被拒拒绝不得改状态"
            );
            assert_eq!(
                detail["data"]["rejected_reason"],
                Value::Null,
                "{domain} 被拒拒绝不得落 rejected_reason"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. draft → rejected 逐字回读 `rejected_reason`（两合同）：状态落词表常量值、
//    理由为 trim 后原文、`approval_reason` 保持 NULL（两动作两列不互写）。
//    改坏什么必红：拒绝理由只进日志不落库 ⇒ 逐字回读红；写错列（挪用
//    approval_reason）⇒ 两列断言同时红；门放行非 draft ⇒ 状态断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn draft_to_rejected_persists_rejected_reason_verbatim_both_contracts() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T3").await;
    let supplier_id = seed_supplier(&db, "T3").await;

    // 带前后空白的理由：必填门与落库值必须同源（trim 后的值）
    let raw_reason = "  供应商资质证明已过期且交付能力不可核，不予通过  ";

    let cases = [
        (
            "purchase_contract",
            seed_purchase_contract(&db, "T3a", supplier_id, contract::DRAFT).await,
        ),
        (
            "sales_contract",
            seed_sales_contract(&db, "T3b", customer_id, contract::DRAFT).await,
        ),
    ];
    for (domain, id) in &cases {
        let (reject_uri, detail_uri) = match *domain {
            "purchase_contract" => (
                format!("/purchase/purchase-contracts/{id}/reject"),
                format!("/purchase/purchase-contracts/{id}"),
            ),
            _ => (
                format!("/sales/sales-contracts/{id}/reject"),
                format!("/sales/sales-contracts/{id}"),
            ),
        };

        let (status, v) = post_json(&app, &reject_uri, json!({ "reason": raw_reason })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} draft→rejected 权威路径必须 200，实得 {v}"
        );
        assert_eq!(v["code"], 200, "{domain} 成功信封 code 必须 200，实得 {v}");

        let (status, detail) = get_json(&app, &detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            contract::REJECTED,
            "{domain} 拒绝成功必须落 rejected 态（值与写入方词表常量逐字符同源）"
        );
        assert_eq!(
            detail["data"]["status"], "rejected",
            "{domain} rejected 落值逐字钉死（词表漂移即此断言红）"
        );
        assert_eq!(
            detail["data"]["rejected_reason"].as_str(),
            Some(raw_reason.trim()),
            "{domain} rejected_reason 必须逐字回读（trim 后原样，禁止只进日志）"
        );
        assert_eq!(
            detail["data"]["approval_reason"],
            Value::Null,
            "{domain} 两动作两列：拒绝不得写 approval_reason 列"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. 批准权威路径逐字回读 `approval_reason`（三域）：
//    - 两合同 draft→active；报价单 pending_approval→approved；
//    - 理由真实落列（不再只进日志）、`rejected_reason`/`rejection_reason` 保持 NULL；
//    - 报价单结论人/结论时间列（approved_by/approved_at）同步落值。
//    改坏什么必红：approve 未落理由列 ⇒ 逐字回读红；批准误写拒绝列 ⇒ NULL 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn approve_persists_approval_reason_verbatim_three_domains() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T4").await;
    let supplier_id = seed_supplier(&db, "T4").await;

    let raw_reason = "  价格与账期均在协议区间内，同意生效  ";

    // —— 两合同：draft → active ——
    let purchase_id = seed_purchase_contract(&db, "T4a", supplier_id, contract::DRAFT).await;
    let sales_id = seed_sales_contract(&db, "T4b", customer_id, contract::DRAFT).await;
    for (domain, id) in [
        ("purchase_contract", purchase_id),
        ("sales_contract", sales_id),
    ] {
        let (approve_uri, detail_uri) = match domain {
            "purchase_contract" => (
                format!("/purchase/purchase-contracts/{id}/approve"),
                format!("/purchase/purchase-contracts/{id}"),
            ),
            _ => (
                format!("/sales/sales-contracts/{id}/approve"),
                format!("/sales/sales-contracts/{id}"),
            ),
        };
        let (status, v) =
            post_json(&app, &approve_uri, json!({ "approval_reason": raw_reason })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "{domain} draft→active 权威路径必须 200，实得 {v}"
        );
        let (status, detail) = get_json(&app, &detail_uri).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"],
            contract::ACTIVE,
            "{domain} 批准成功必须落 active 态"
        );
        assert_eq!(
            detail["data"]["approval_reason"].as_str(),
            Some(raw_reason.trim()),
            "{domain} approval_reason 必须逐字落库回读（不再只进日志）"
        );
        assert_eq!(
            detail["data"]["rejected_reason"],
            Value::Null,
            "{domain} 两动作两列：批准不得写 rejected_reason 列"
        );
    }

    // —— 报价单：pending_approval → approved（业务列由业务侧端点承载）——
    let quotation_id =
        seed_quotation(&db, "T4c", customer_id, quotation_ext::PENDING_APPROVAL).await;
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{quotation_id}/approve"),
        json!({ "approval_reason": raw_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "quotation pending_approval→approved 权威路径必须 200，实得 {v}"
    );
    // 出参端点同批钉：详情走 QuotationResponseDto，批准理由必须同时经端点读回。
    let (status, detail) = get_json(&app, &format!("/quotations/{quotation_id}")).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "报价单详情回读必须 200：{detail}"
    );
    assert_eq!(
        detail["data"]["status"],
        quotation::APPROVED,
        "quotation 批准结果必须经详情端点逐字回读"
    );
    let row = read_quotation(&db, quotation_id).await;
    assert_eq!(
        row.status,
        quotation::APPROVED,
        "报价批准必须落 approved 态"
    );
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw_reason.trim()),
        "quotation approval_reason 必须逐字落库回读（不再只进日志）"
    );
    assert_eq!(
        detail["data"]["approval_reason"]
            .as_str()
            .map(str::to_string),
        row.approval_reason.clone(),
        "报价通过理由必须经详情端点读回，且与落库值逐字一致"
    );
    assert_eq!(
        row.rejection_reason, None,
        "quotation 两动作两列：批准不得写 rejection_reason 列"
    );
    assert_eq!(
        row.approved_by,
        Some(OPERATOR_ID as i64),
        "quotation 批准结论必须记录结论人"
    );
    assert!(
        row.approved_at.is_some(),
        "quotation 批准结论必须记录结论时间"
    );
}

// ---------------------------------------------------------------------------
// 5. rejected 终态双向拦截（三域）：终态行再 approve、再 reject 均被状态门拦
//    （400 + BUSINESS_ERROR，只断 HTTP 码与机器码、不断文案），行零变化——
//    本批不出 rejected 出边。
//    改坏什么必红：门只拦一个动作 ⇒ 另一段红；门回潮放行 ⇒ 400 断言红；
//    覆写理由 ⇒ 逐字保持断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn rejected_row_is_terminal_for_both_approve_and_reject_three_domains() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T5").await;
    let supplier_id = seed_supplier(&db, "T5").await;
    let first_reason = "首轮拒绝成立";

    // 三域各一行，先走权威拒绝路径进入 rejected 终态
    let purchase_id = seed_purchase_contract(&db, "T5a", supplier_id, contract::DRAFT).await;
    let (status, v) = post_json(
        &app,
        &format!("/purchase/purchase-contracts/{purchase_id}/reject"),
        json!({ "reason": first_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "purchase_contract 前置：draft→rejected 必须成功，实得 {v}"
    );

    let sales_id = seed_sales_contract(&db, "T5b", customer_id, contract::DRAFT).await;
    let (status, v) = post_json(
        &app,
        &format!("/sales/sales-contracts/{sales_id}/reject"),
        json!({ "reason": first_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "sales_contract 前置：draft→rejected 必须成功，实得 {v}"
    );

    let quotation_id =
        seed_quotation(&db, "T5c", customer_id, quotation_ext::PENDING_APPROVAL).await;
    let (status, v) = post_json(
        &app,
        &format!("/quotations/{quotation_id}/reject"),
        json!({ "reason": first_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "quotation 前置：pending_approval→rejected 必须成功，实得 {v}"
    );
    // 拒绝结论同样记录结论人/结论时间（与 approve 同一列套口径）
    let row = read_quotation(&db, quotation_id).await;
    assert_eq!(
        row.approved_by,
        Some(OPERATOR_ID as i64),
        "quotation 拒绝结论必须记录结论人"
    );
    assert!(
        row.approved_at.is_some(),
        "quotation 拒绝结论必须记录结论时间（与 approve 路径同口径）"
    );

    for (domain, approve_uri, reject_uri) in [
        (
            "purchase_contract",
            format!("/purchase/purchase-contracts/{purchase_id}/approve"),
            format!("/purchase/purchase-contracts/{purchase_id}/reject"),
        ),
        (
            "sales_contract",
            format!("/sales/sales-contracts/{sales_id}/approve"),
            format!("/sales/sales-contracts/{sales_id}/reject"),
        ),
        (
            "quotation",
            format!("/quotations/{quotation_id}/approve"),
            format!("/quotations/{quotation_id}/reject"),
        ),
    ] {
        // 再批准：理由给满，排除必填档干扰，只测门
        let (status, v) = post_json(
            &app,
            &approve_uri,
            json!({ "approval_reason": "终态后重试批准" }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} rejected 行再批准必须 400（终态无出边），实得 {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{domain} 批准状态门必须归 BUSINESS_ERROR（只断机器码不断文案），实得 {v}"
        );

        // 再拒绝：同样被门拦
        let (status, v) = post_json(&app, &reject_uri, json!({ "reason": "终态后重试拒绝" })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} rejected 行再拒绝必须 400（终态无出边），实得 {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{domain} 拒绝状态门必须归 BUSINESS_ERROR，实得 {v}"
        );
    }

    // 零副作用：状态与首轮拒绝理由逐字保持，批准列始终为空
    let purchase = purchase_contract::Entity::find_by_id(purchase_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(purchase.status, contract::REJECTED, "被拒终态不得改状态");
    assert_eq!(
        purchase.rejected_reason.as_deref(),
        Some(first_reason),
        "purchase_contract 被拒的再批准不得覆写 rejected_reason"
    );
    assert_eq!(
        purchase.approval_reason, None,
        "purchase_contract 被拒的再批准不得落 approval_reason"
    );

    let sales = sales_contract::Entity::find_by_id(sales_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(sales.status, contract::REJECTED, "被拒终态不得改状态");
    assert_eq!(
        sales.rejected_reason.as_deref(),
        Some(first_reason),
        "sales_contract 被拒的再批准不得覆写 rejected_reason"
    );
    assert_eq!(
        sales.approval_reason, None,
        "sales_contract 被拒的再批准不得落 approval_reason"
    );

    let quotation = read_quotation(&db, quotation_id).await;
    assert_eq!(quotation.status, quotation::REJECTED, "报价终态不得改状态");
    assert_eq!(
        quotation.rejection_reason.as_deref(),
        Some(first_reason),
        "quotation 被拒的再拒绝不得覆写 rejection_reason"
    );
    assert_eq!(
        quotation.approval_reason, None,
        "quotation 被拒的再批准不得落 approval_reason"
    );
}

// ---------------------------------------------------------------------------
// 6. 合同生效后不可拒（两合同 × active / cancelled）：状态门一律拦
//    （400 + BUSINESS_ERROR），行零变化。
//    改坏什么必红：门写成 `!= DRAFT || != CANCELLED` 之类放行形态、或门只认
//    active 不认 cancelled ⇒ 对应段 400 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn non_draft_contract_reject_is_400_business_error_both_contracts() {
    let (db, app) = approval_app().await;
    let customer_id = seed_customer(&db, "T6").await;
    let supplier_id = seed_supplier(&db, "T6").await;

    let cases = [
        (
            "purchase_contract",
            seed_purchase_contract(&db, "T6a", supplier_id, contract::ACTIVE).await,
            contract::ACTIVE,
            "/purchase/purchase-contracts",
        ),
        (
            "purchase_contract",
            seed_purchase_contract(&db, "T6b", supplier_id, contract::CANCELLED).await,
            contract::CANCELLED,
            "/purchase/purchase-contracts",
        ),
        (
            "sales_contract",
            seed_sales_contract(&db, "T6c", customer_id, contract::ACTIVE).await,
            contract::ACTIVE,
            "/sales/sales-contracts",
        ),
        (
            "sales_contract",
            seed_sales_contract(&db, "T6d", customer_id, contract::CANCELLED).await,
            contract::CANCELLED,
            "/sales/sales-contracts",
        ),
    ];
    for (domain, id, start_status, base) in &cases {
        let reject_uri = format!("{base}/{id}/reject");
        let (status, v) =
            post_json(&app, &reject_uri, json!({ "reason": "生效后将不允许拒绝" })).await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{domain} {start_status} 合同拒绝必须 400（仅草稿可拒），实得 {v}"
        );
        assert_eq!(
            v["code"], "BUSINESS_ERROR",
            "{domain} 拒绝状态门必须归 BUSINESS_ERROR，实得 {v}"
        );

        // 零副作用：状态与被拦的拒绝理由列都不动
        let (status, detail) = get_json(&app, &format!("{base}/{id}")).await;
        assert_eq!(
            status,
            axum::http::StatusCode::OK,
            "详情回读必须 200：{detail}"
        );
        assert_eq!(
            detail["data"]["status"], *start_status,
            "{domain} 被拒拒绝不得改状态"
        );
        assert_eq!(
            detail["data"]["rejected_reason"],
            Value::Null,
            "{domain} 被拒拒绝不得落 rejected_reason"
        );
    }
}
