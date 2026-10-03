//! 售后工单创建端点契约锁（任务 #148：本波修复"创建必失败 + 拒绝原因被脱敏"）
//!
//! 锁定的 file:line 契约（修复后形态）：
//! - `backend/src/services/custom_order_aftersales_service.rs::CreateAfterSalesDto`
//!   （`custom_order_id` 已从 DTO 移除：归属由路由 path 权威提供，body 不再必填、
//!   发送也被 serde 忽略未知字段；issue_type/customer_id/description 仍为 NOT NULL 必填）
//! - `backend/src/handlers/custom_order_handler.rs::create_after_sales`
//!   （`service.create(id, dto)`，body 伪造归属结构性不可能覆盖 path）
//! - `backend/src/handlers/custom_order_handler.rs::aftersales_err`
//!   （InvalidState → `AppError::business_displayable`、Validation → `AppError::validation_displayable`：
//!   出参 message 均外显真实拒绝文案，族按 #165 判据拆分；AlreadyLinked 含内部 ID 保持脱敏 business）
//!
//! 修复前缺陷（客诉实证"编辑填写正常、提交保存报参数错误"）：
//! DTO 的 `custom_order_id: i64` 非 Option 无 serde default，而前端
//! `AfterSalesPanel.vue` payload 从不携带该键 → 反序列化层 missing field，创建 100% 失败；
//! 且 handler 的 path 覆盖发生在反序列化之后，body 该字段本就无语义。
//!
//! 覆盖策略（全部真实行为，无 mock）：
//! - serde 解码（无 DB）：缺 custom_order_id 键必须成功（修复前必失败）；伪造键被忽略；
//!   NOT NULL 字段缺失仍须判 Err（不得为过测试放宽）；refund_amount 字符串/数字双形态
//! - 真 PostgreSQL（TEST_DATABASE_URL + 迁移建表，夹具清空业务表）走真实 handler 端到端
//!   （tower oneshot）：先播种 customers/product/custom_orders 满足真表 FK
//!   （after_sales.custom_order_id→custom_orders、customer_id→customers；
//!   custom_orders.product_id→products、customer_id→customers），
//!   前端真实 payload 建单成功并回读归属；伪造 body id 被 path 归属钉死；
//!   refund 缺金额 / 非法类型的拒绝出参 code=VALIDATION_ERROR（输入校验族）且 message 外显原文
//! - 源码扫描防回潮锁（先例：contract_wave1_ar_payment_error_mapping_test.rs）

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{after_sales, custom_order, customer, product};
use bingxi_backend::services::custom_order_aftersales_service::CreateAfterSalesDto;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::str::FromStr;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// 前端 AfterSalesPanel.vue 提交的真实 payload 形状（任务取证：从不携带 custom_order_id）
fn frontend_real_payload() -> Value {
    json!({
        "issue_type": "complaint",
        "customer_id": 7,
        "description": "面料色差超出允许范围",
        "refund_amount": null
    })
}

// =========================================================
// 1) serde 解码层（无 DB）
// =========================================================

/// 缺 custom_order_id 键 → 解码必须成功（本波契约核心：修复前此形状即
/// "missing field custom_order_id"，客诉工单创建 100% 失败）
#[test]
fn decode_frontend_payload_without_custom_order_id_succeeds() {
    let dto: CreateAfterSalesDto =
        serde_json::from_value(frontend_real_payload()).expect("缺 custom_order_id 必须解码成功");
    assert_eq!(dto.customer_id, 7);
    assert_eq!(dto.issue_type, "complaint");
    assert_eq!(dto.description, "面料色差超出允许范围");
    assert!(dto.refund_amount.is_none());
}

/// body 伪造 custom_order_id → 成功解码且该键被忽略（DTO 无此字段，
/// 归属只能来自 path；端到端防覆盖语义见下方 e2e 用例）
#[test]
fn decode_body_with_forged_custom_order_id_is_ignored() {
    let mut body = frontend_real_payload();
    body["custom_order_id"] = json!(999_999);
    let dto: CreateAfterSalesDto =
        serde_json::from_value(body).expect("多余 custom_order_id 键不得导致解码失败");
    // 反向锁：DTO 序列化回 JSON 后不再含该键（字段确已从 DTO 移除）
    let back = serde_json::to_value(&dto).unwrap();
    assert!(
        back.get("custom_order_id").is_none(),
        "custom_order_id 必须已不属于 CreateAfterSalesDto，实际: {back}"
    );
}

/// NOT NULL 必填字段（issue_type/customer_id/description）缺失仍须判 Err——
/// 本波只解耦 path 已提供的归属 ID，不得顺手放宽任何真实必填
#[test]
fn decode_missing_not_null_required_fields_still_fails() {
    for key in ["issue_type", "customer_id", "description"] {
        let mut body = frontend_real_payload();
        body.as_object_mut().unwrap().remove(key);
        let err = serde_json::from_value::<CreateAfterSalesDto>(body)
            .expect_err(&format!("缺 NOT NULL 字段 {key} 必须解码失败"));
        assert!(
            err.to_string().contains(key),
            "错误应指出缺失字段 {key}，实际: {err}"
        );
    }
}

/// refund_amount 双形态：字符串（Decimal JSON 标准形态）与 number（前端
/// el-input-number 实际输出）都必须无损解码为 Decimal
#[test]
fn decode_refund_amount_accepts_string_and_number() {
    let mut body = frontend_real_payload();
    body["issue_type"] = json!("refund");
    body["refund_amount"] = json!("1200.50");
    let dto: CreateAfterSalesDto = serde_json::from_value(body.clone()).unwrap();
    assert_eq!(dto.refund_amount, Some(dec("1200.50")));

    body["refund_amount"] = json!(1200.50);
    let dto: CreateAfterSalesDto = serde_json::from_value(body).unwrap();
    assert_eq!(dto.refund_amount, Some(dec("1200.50")));
}

// =========================================================
// 2) 真实 handler 端到端（真 PostgreSQL，迁移建表 + FK 前置播种）
// =========================================================

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
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

/// FK 前置播种（真表外键链）：
/// after_sales.customer_id→customers(id=7)、after_sales.custom_order_id→custom_orders(id=42)、
/// custom_orders.product_id→products、custom_orders.customer_id→customers。
/// 夹具 TRUNCATE…RESTART IDENTITY 后显式 id 稳定；custom_orders.quantity 有 CHECK >0。
async fn seed_fk_prereq(db: &sea_orm::DatabaseConnection) {
    product::ActiveModel {
        id: Set(1),
        name: Set("售后套件产品".to_string()),
        code: Set("PRD-AS-0001".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    customer::ActiveModel {
        id: Set(7),
        customer_code: Set("CUS-AS-0007".to_string()),
        customer_name: Set("售后套件客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    custom_order::ActiveModel {
        id: Set(42),
        order_no: Set("CO-AS-0042".to_string()),
        customer_id: Set(7),
        product_id: Set(1),
        spec: Set("180gsm 全棉".to_string()),
        quantity: Set(dec("10.00")),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seeded_app() -> (Router, sea_orm::DatabaseConnection) {
    let db = test_common::setup_test_db().await;
    seed_fk_prereq(&db).await;
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/custom-orders/{id}/after-sales",
            post(custom_order_handler::create_after_sales)
                .get(custom_order_handler::list_after_sales),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth));
    (app, db)
}

async fn post_create(app: &Router, path_id: i64, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/custom-orders/{path_id}/after-sales"))
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn get_list(app: &Router, path_id: i64) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/custom-orders/{path_id}/after-sales?page=1&page_size=20"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 核心回归锁：前端真实 payload（无 custom_order_id）走创建端点 → 200，
/// 且 DB 回读的 custom_order_id == path 参数（本波缺陷修复的直接验证）
#[tokio::test]
async fn create_without_custom_order_id_in_body_succeeds_and_persists_path_ownership() {
    let (app, db) = seeded_app().await;
    let (status, v) = post_create(&app, 42, frontend_real_payload()).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "缺 custom_order_id 键的创建必须 200（修复前必失败），实际体: {v}"
    );
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["issue_type"], "complaint");
    assert_eq!(v["data"]["status"], "opened");

    let id = v["data"]["id"].as_i64().expect("data.id 应为数字");
    let row = after_sales::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("创建成功后 DB 应可回读");
    assert_eq!(
        row.custom_order_id, 42,
        "归属必须由 path 注入为 42（DTO 已不携带该字段）"
    );
    assert_eq!(row.customer_id, 7);
    assert_eq!(row.description, "面料色差超出允许范围");
    assert_eq!(row.status, "opened");
}

/// 出参字符串契约锁：refund_amount 有值时 JSON 出参为字符串
/// （rust_decimal 默认序列化行为——前端 AfterSales.refund_amount 声明 string 的依据）
#[tokio::test]
async fn refund_amount_serialized_as_string_in_response_and_roundtrips_exactly() {
    let (app, db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["issue_type"] = json!("refund");
    body["refund_amount"] = json!(960.75); // 前端 el-input-number 发 number
    let (status, v) = post_create(&app, 42, body).await;
    assert_eq!(status, StatusCode::OK, "退款工单应创建成功，实际体: {v}");
    let amount = &v["data"]["refund_amount"];
    assert!(
        amount.is_string(),
        "Decimal 出参必须为 JSON 字符串（前端类型 string 的契约来源），实际: {amount}"
    );
    assert_eq!(amount.as_str().unwrap(), "960.75");

    let id = v["data"]["id"].as_i64().unwrap();
    let row = after_sales::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("退款工单应可回读");
    assert_eq!(row.refund_amount, Some(dec("960.75")), "金额不得失真");
}

/// 越权防护锁：body 伪造 custom_order_id=999，path=42 → 落库归属仍为 42，
/// 伪造值不生效（修复前 body 值也会被 handler 覆盖，语义不变；现为结构性排除）
#[tokio::test]
async fn forged_custom_order_id_in_body_cannot_override_path_ownership() {
    let (app, db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["custom_order_id"] = json!(999);
    let (status, v) = post_create(&app, 42, body).await;
    assert_eq!(status, StatusCode::OK, "伪造键应被忽略而非报错: {v}");
    let id = v["data"]["id"].as_i64().unwrap();
    let row = after_sales::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("应可回读");
    assert_eq!(
        row.custom_order_id, 42,
        "归属必须以 path=42 为权威，body 伪造 999 不得生效"
    );
}

/// 用户可见性锁：退款类型缺金额 → 400 + code=VALIDATION_ERROR + message 外显
/// 真实拒绝文案。断言跟随源码变更（任务 #165）：缺必填金额与非法售后类型是「用户提交
/// 字段」的输入校验，族必须归 VALIDATION_ERROR（此前 #148 误并入 business 使前端把
/// "我填错了"当业务提示）；validation_displayable 仍外显真实文案，不回退脱敏常量。
#[tokio::test]
async fn refund_without_amount_rejected_with_displayable_validation_message() {
    let (app, _db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["issue_type"] = json!("refund");
    let (status, v) = post_create(&app, 42, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(
        v["message"], "退款类型工单必须填写退款金额",
        "validation_displayable 出参必须外显真实拒绝文案，不得脱敏"
    );

    let mut body = frontend_real_payload();
    body["issue_type"] = json!("teleportation");
    let (status, v) = post_create(&app, 42, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], "非法售后类型: teleportation");
    assert_ne!(v["message"], "请求参数验证失败", "不得回潮到脱敏的校验信封");
}

/// 拒绝路径不得留脏数据：缺金额退款与非法类型均被业务校验拒绝，after_sales 零行
#[tokio::test]
async fn rejected_creations_leave_no_rows() {
    let (app, db) = seeded_app().await;
    let mut b1 = frontend_real_payload();
    b1["issue_type"] = json!("refund");
    let (s1, _) = post_create(&app, 42, b1).await;
    let mut b2 = frontend_real_payload();
    b2["issue_type"] = json!("teleportation");
    let (s2, _) = post_create(&app, 42, b2).await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);
    assert_eq!(s2, StatusCode::BAD_REQUEST);
    let rows = after_sales::Entity::find()
        .filter(after_sales::Column::CustomOrderId.eq(42))
        .all(&db)
        .await
        .unwrap();
    assert!(
        rows.is_empty(),
        "被拒绝的创建不得留下任何售后工单，实际 {} 行",
        rows.len()
    );
}

// =========================================================
// 3) 读端回传锁：创建时采集的 customer_id / reason_category / reason_detail
//    必须在创建端点响应与列表端点回读中可见（防"落库了但读不回"复发——
//    写入侧 Set(...) 早已落库，读端 AfterSalesInfo/map_after_sales 曾缺列，
//    前端永远显示不完整内容）
// =========================================================

/// 对单个 AfterSalesInfo 出参对象断言三键齐全且值忠实
fn assert_readback_fields(obj: &Value, expected_detail: &str) {
    for key in ["customer_id", "reason_category", "reason_detail"] {
        assert!(
            obj.get(key).is_some(),
            "响应必须包含读端补齐键 {key}（键名 = 实体 snake_case），实际: {obj}"
        );
    }
    assert_eq!(obj["customer_id"], json!(7), "customer_id 必须如实透传");
    assert_eq!(
        obj["reason_category"],
        json!("quality"),
        "reason_category 必须如实透传"
    );
    assert_eq!(
        obj["reason_detail"],
        json!(expected_detail),
        "reason_detail 必须如实透传"
    );
}

/// 创建带 reason_category/reason_detail 的工单：
/// POST 创建响应 + GET 列表端点都必须回读得到这三个键（真 PG 真实 handler，无 mock）
#[tokio::test]
async fn create_and_list_endpoints_read_back_reason_and_customer_fields() {
    let (app, db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["reason_category"] = json!("quality");
    let detail = "色差超差";
    body["reason_detail"] = json!(detail);

    let (status, v) = post_create(&app, 42, body).await;
    assert_eq!(status, StatusCode::OK, "带原因分类创建应成功，实际体: {v}");
    assert_readback_fields(&v["data"], detail);

    // DB 落库真值反向锁（出参与实体同源，不得只在出参层自证）
    let id = v["data"]["id"].as_i64().expect("data.id 应为数字");
    let row = after_sales::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("应可回读");
    assert_eq!(row.customer_id, 7);
    assert_eq!(row.reason_category.as_deref(), Some("quality"));
    assert_eq!(row.reason_detail.as_deref(), Some(detail));

    // 列表端点回读（定制订单详情的 after_sales 与本列表共用 service::list_by_order
    // 同一条 LEFT JOIN 富化查询链路）
    let (status, lv) = get_list(&app, 42).await;
    assert_eq!(status, StatusCode::OK, "列表端点应 200，实际体: {lv}");
    assert_eq!(lv["code"], 200);
    let items = lv["data"]["items"]
        .as_array()
        .expect("列表响应应为 PagedResponse{items,...}");
    assert_eq!(items.len(), 1, "应恰有 1 条工单，实际: {lv}");
    assert_readback_fields(&items[0], detail);
}

/// 未采集原因时读端不得吞键：Option 列缺值应显式序列化为 null 键
/// （前端契约 reason_category?: string 可空；键整体缺失 = 又一形态的"读不回"）
#[tokio::test]
async fn create_response_keeps_reason_keys_as_null_when_not_provided() {
    let (app, _db) = seeded_app().await;
    let (status, v) = post_create(&app, 42, frontend_real_payload()).await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");
    let data = &v["data"];
    assert!(
        data.get("customer_id").is_some(),
        "NOT NULL 列出参键必须存在: {data}"
    );
    assert!(
        data.get("reason_category").is_some() && data["reason_category"].is_null(),
        "未提供的 Option 列应出 null 键而非整键缺失: {data}"
    );
    assert!(
        data.get("reason_detail").is_some() && data["reason_detail"].is_null(),
        "未提供的 Option 列应出 null 键而非整键缺失: {data}"
    );
}

// =========================================================
// 4) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层块：anchor 起，到首个 "\n}"（结构体亦以行首 } 结束）。
/// 先剔除 `\r`：Windows 工作树文件为 CRLF，跨行 contains 断言必须先归一化，
/// 否则 "\r\n" 会使本应命中的锚点漏检（脆弱断言防御，非放宽断言）
fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n}")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j + 2].to_string()
}

/// create_after_sales：不得回潮"反序列化后覆盖 dto.custom_order_id / mut dto"，
/// 必须是 service.create(id, dto) 的 path 权威注入形态，且错误经 aftersales_err
/// 映射、无 internal 强转
#[test]
fn source_scan_create_after_sales_contract() {
    let src = include_str!("../src/handlers/custom_order_handler.rs");
    let create = extract_block(src, "pub async fn create_after_sales");
    assert!(
        !create.contains("dto.custom_order_id"),
        "create_after_sales 不得回潮对 body 归属字段的读写，实际块:\n{create}"
    );
    assert!(
        !create.contains("mut dto"),
        "body DTO 已无待注入字段，不得再声明 mut，实际块:\n{create}"
    );
    assert!(
        create.contains("service.create(id, dto)"),
        "必须以 path id 调 service.create(id, dto)，实际块:\n{create}"
    );
    assert!(
        !create.contains("AppError::internal"),
        "售后创建错误禁止被强转 500 Internal，实际块:\n{create}"
    );
}

/// aftersales_err：InvalidState 必须 business_displayable（记录状态门归业务族并外显），
/// Validation 必须 validation_displayable（提交字段校验归校验族并外显）；
/// AlreadyLinked 文案含内部 ID，必须保持脱敏 business。
/// 断言跟随源码变更（任务 #165）：#148 曾把两者并成 business_displayable，导致输入校验
/// 也出 BUSINESS_ERROR、前端按 code 分支错乱；本轮按判据拆族——状态门 business、校验 validation。
#[test]
fn source_scan_aftersales_err_displayable_mapping() {
    let src = include_str!("../src/handlers/custom_order_handler.rs");
    let map = extract_block(src, "fn aftersales_err");
    assert!(
        map.contains("InvalidState(msg) => AppError::business_displayable(msg)"),
        "InvalidState 状态门必须外显且归 business 族，实际块:\n{map}"
    );
    assert!(
        map.contains("Validation(msg) => AppError::validation_displayable(msg)"),
        "Validation 输入校验必须外显且归 VALIDATION_ERROR 族，实际块:\n{map}"
    );
    assert!(
        map.contains("AlreadyLinked(after_sales_id, qi_id) => AppError::business("),
        "含内部记录 ID 的文案不得外显，必须保持脱敏 business，实际块:\n{map}"
    );
}

/// service：CreateAfterSalesDto 不得再含 custom_order_id 字段；create 必须以
/// 独立参数取归属并 Set(custom_order_id)（防 DTO 字段回潮 / 参数被忽略）
#[test]
fn source_scan_service_dto_and_create_signature() {
    let src = include_str!("../src/services/custom_order_aftersales_service.rs");
    let dto = extract_block(src, "pub struct CreateAfterSalesDto");
    assert!(
        !dto.contains("custom_order_id"),
        "CreateAfterSalesDto 不得恢复 custom_order_id 字段，实际块:\n{dto}"
    );
    let create = extract_block(src, "pub async fn create");
    assert!(
        create.contains("custom_order_id: i64,\n        dto: CreateAfterSalesDto"),
        "create 必须以独立 i64 参数接收归属（path 权威注入），实际块:\n{create}"
    );
    assert!(
        create.contains("Set(custom_order_id)"),
        "ActiveModel 归属必须来自函数参数，实际块:\n{create}"
    );
    assert!(
        !create.contains("Set(dto.custom_order_id)"),
        "禁止回潮从 body DTO 取归属，实际块:\n{create}"
    );
}

/// 读端防回潮锁：AfterSalesInfo 必须声明读端字段，且售后读侧必须走
/// LEFT JOIN + column_as(customer_name) + into_model::<AfterSalesInfo> 富化链路
/// （本波缺陷本体：写入侧 Set(...) 落库了，读端 DTO/映射缺列导致前端永远读不回；
/// 旧 map_after_sales 逐字段透传形态已被单次 JOIN 富化取代，customer_name
/// 只允许来自该查询，真实回显断言见 contract_wave3 测试文件）
#[test]
fn source_scan_after_sales_readback_dto_and_mapping() {
    let dto_src = include_str!("../src/models/custom_order_response_dto.rs");
    let dto_block = extract_block(dto_src, "pub struct AfterSalesInfo");
    for field in [
        "pub customer_id: i32,",
        "pub customer_name: Option<String>,",
        "pub reason_category: Option<String>,",
        "pub reason_detail: Option<String>,",
    ] {
        assert!(
            dto_block.contains(field),
            "AfterSalesInfo 缺读端字段 {field}，实际块:\n{dto_block}"
        );
    }

    let svc_src = include_str!("../src/services/custom_order_aftersales_service.rs");
    let list_block = extract_block(svc_src, "pub async fn list_by_order");
    for marker in [
        "column_as(customer::Column::CustomerName, \"customer_name\")",
        "JoinType::LeftJoin",
        "after_sales::Relation::Customer.def()",
        "into_model::<AfterSalesInfo>",
    ] {
        assert!(
            list_block.contains(marker),
            "list_by_order 读侧缺富化链路锚点 {marker}，实际块:\n{list_block}"
        );
    }
}
