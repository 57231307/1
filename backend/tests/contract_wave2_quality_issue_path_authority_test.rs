//! 质检上报端点归属权威契约锁（售后先例同构缺陷，本波修复）
//!
//! 锁定的 file:line 契约（修复后形态）：
//! - `backend/src/models/quality_issue_dto.rs::ReportQualityIssueDto`
//!   `custom_order_id` 已从请求体契约移除（修复前为非 Option 必填、无 serde
//!   default，前端不发该键必在反序列化层 missing field —— 上报 100% 失败）
//! - `backend/src/handlers/custom_order_handler.rs::report_quality_issue`
//!   `service.report_issue(id, dto)`：归属由路由 path 权威注入，不再"反序列化后
//!   覆盖"（越权防护由 handler 覆盖升级为结构性排除）
//! - `backend/src/handlers/custom_order_handler.rs::quality_err`
//!   InvalidState → `AppError::business_displayable`、Validation → `AppError::validation_displayable`
//! （拒绝原因均外显，族按 判据拆分，对齐 aftersales_err）
//!
//! 覆盖策略（先例：contract_wave2_after_sales_create_test.rs，全部真实行为无 mock）：
//! - serde 解码（无 DB）：缺 custom_order_id 键必须成功（修复前必失败）；伪造键被
//!   忽略；NOT NULL 必填字段缺失仍须判 Err（不得为过测试放宽）
//! - 真 PostgreSQL（TEST_DATABASE_URL + 迁移建表，夹具清空业务表）+ 真实 handler
//!   端到端（tower oneshot）：先播种 quality_issues.custom_order_id→custom_orders(id=42)、
//!   custom_orders→products/customers 的 FK 前置链；不带/伪造归属键都能落库且归属以
//!   path 为准；业务校验拒绝 400 外显真实文案
//! - 源码扫描防回潮锁

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
use bingxi_backend::models::quality_issue;
use bingxi_backend::models::quality_issue_dto::ReportQualityIssueDto;
use chrono::Utc;
// 注意：本文件大量使用 serde_json::Value（响应体断言），故 sea-orm 的绑定值类型
// 一律写作 sea_orm::Value，不并入上面的导入（并入会触发 E0252 重名并把
// as_object_mut/索引等 serde 用法全带崩）。
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use serde_json::{Value, json};
use tower::ServiceExt;

/// 前端 QualityCheck.vue 的真实 payload 形状（本波修复后不再携带 custom_order_id）
fn frontend_real_payload() -> Value {
    json!({
        "process_node_id": null,
        "issue_type": "color_diff",
        "severity": "high",
        "description": "面料色差超出允许范围",
        "color_delta_e": 4.2,
        "color_fastness_grade": 3
    })
}

// =========================================================
// 1) serde 解码层（无 DB）
// =========================================================

/// 缺 custom_order_id 键 → 解码必须成功（修复前此形状 missing field，上报必失败）
#[test]
fn decode_frontend_payload_without_custom_order_id_succeeds() {
    let dto: ReportQualityIssueDto =
        serde_json::from_value(frontend_real_payload()).expect("缺 custom_order_id 必须解码成功");
    assert_eq!(dto.issue_type, "color_diff");
    assert_eq!(dto.severity, "high");
    assert_eq!(dto.description, "面料色差超出允许范围");
}

/// body 伪造 custom_order_id → 成功解码且该键被忽略；DTO 序列化回 JSON 不含该键
/// （字段确已从 DTO 移除，归属只能来自 path）
#[test]
fn decode_body_with_forged_custom_order_id_is_ignored() {
    let mut body = frontend_real_payload();
    body["custom_order_id"] = json!(999_999);
    let dto: ReportQualityIssueDto =
        serde_json::from_value(body).expect("多余 custom_order_id 键不得导致解码失败");
    let back = serde_json::to_value(&dto).unwrap();
    assert!(
        back.get("custom_order_id").is_none(),
        "custom_order_id 必须已不属于 ReportQualityIssueDto，实际: {back}"
    );
}

/// 真实必填字段（issue_type/severity/description）缺失仍须判 Err——
/// 本波只解耦 path 已提供的归属 ID，不得顺手放宽任何真实必填
#[test]
fn decode_missing_required_fields_still_fails() {
    for key in ["issue_type", "severity", "description"] {
        let mut body = frontend_real_payload();
        body.as_object_mut().unwrap().remove(key);
        let err = serde_json::from_value::<ReportQualityIssueDto>(body)
            .expect_err(&format!("缺必填字段 {key} 必须解码失败"));
        assert!(
            err.to_string().contains(key),
            "错误应指出缺失字段 {key}，实际: {err}"
        );
    }
}

// =========================================================
// 2) 真实 handler 端到端（真 PostgreSQL；表结构唯一来源 = backend/migration，
// 路线一 判责。FK 父行按裁定 R1 自种子
//    quality_issues.custom_order_id → custom_orders(42) → customers/products）
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

/// FK 前置链自种子（逐列对真表）：customers(1) → products(1) →
/// custom_orders(42)（quantity>0 CHECK、status 取词表合法值 dyeing）。
async fn seeded_app() -> (Router, sea_orm::DatabaseConnection) {
    let db = test_common::setup_test_db().await;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO customers (id,customer_code,customer_name,credit_limit,payment_terms,
                 status,customer_type,owner_id,created_at,updated_at) VALUES
                 (1,'W2QI-CUS-1','质检上报客户',0,30,'active','retail',100,
                  '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("customers 种子失败: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO products (id,code,name) VALUES (1,'W2QI-P1','全消光涤纶面料')",
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("products 种子失败: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO custom_orders (id,order_no,customer_id,product_id,spec,quantity,
                 status,created_by,created_at,updated_at) VALUES
                 (42,'W2QI-CO-42',1,1,'150D/48F 平纹 160cm',1000,
                  'dyeing',100,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("custom_orders 种子失败: {e}"));
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/custom-orders/{id}/issues",
            post(custom_order_handler::report_quality_issue),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth));
    (app, db)
}

async fn post_issue(app: &Router, path_id: i64, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/custom-orders/{path_id}/issues"))
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

/// 核心回归锁：前端真实 payload（无 custom_order_id）走上报端点 → 200，
/// 且 DB 回读的 custom_order_id == path 参数（修复前此形状 100% 反序列化失败）
#[tokio::test]
async fn report_without_custom_order_id_in_body_succeeds_and_persists_path_ownership() {
    let (app, db) = seeded_app().await;
    let (status, v) = post_issue(&app, 42, frontend_real_payload()).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "缺 custom_order_id 键的上报必须 200（修复前必失败），实际体: {v}"
    );
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["issue_type"], "color_diff");
    assert_eq!(v["data"]["status"], "open");

    let id = v["data"]["id"].as_i64().expect("data.id 应为数字");
    let row = quality_issue::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("上报成功后 DB 应可回读");
    assert_eq!(
        row.custom_order_id, 42,
        "归属必须由 path 注入为 42（DTO 已不携带该字段）"
    );
    assert_eq!(row.severity, "high");
    assert_eq!(row.description, "面料色差超出允许范围");
}

/// 越权防护锁：body 伪造 custom_order_id=999，path=42 → 落库归属仍为 42
#[tokio::test]
async fn forged_custom_order_id_in_body_cannot_override_path_ownership() {
    let (app, db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["custom_order_id"] = json!(999);
    let (status, v) = post_issue(&app, 42, body).await;
    assert_eq!(status, StatusCode::OK, "伪造键应被忽略而非报错: {v}");
    let id = v["data"]["id"].as_i64().unwrap();
    let row = quality_issue::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("应可回读");
    assert_eq!(
        row.custom_order_id, 42,
        "归属必须以 path=42 为权威，body 伪造 999 不得生效"
    );
}

/// 用户可见性锁：非法严重度 / 色牢度越界 → 400 + code=VALIDATION_ERROR + message
/// 外显真实拒绝文案。断言跟随源码变更：severity 枚举越界与色牢度等级越界
/// 都是「用户提交字段」的输入校验，族归 VALIDATION_ERROR；validation_displayable 仍外显真实原因。
#[tokio::test]
async fn validation_rejections_return_displayable_validation_message() {
    let (app, _db) = seeded_app().await;
    let mut body = frontend_real_payload();
    body["severity"] = json!("catastrophic");
    let (status, v) = post_issue(&app, 42, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], "非法严重度: catastrophic");

    let mut body = frontend_real_payload();
    body["color_fastness_grade"] = json!(9);
    let (status, v) = post_issue(&app, 42, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_eq!(v["message"], "ISO 105 色牢度等级必须在 1-5 之间");
}

/// 拒绝路径不得留脏数据
#[tokio::test]
async fn rejected_reports_leave_no_rows() {
    let (app, db) = seeded_app().await;
    let mut b1 = frontend_real_payload();
    b1["severity"] = json!("catastrophic");
    let (s1, _) = post_issue(&app, 42, b1).await;
    let mut b2 = frontend_real_payload();
    b2["color_fastness_grade"] = json!(0);
    let (s2, _) = post_issue(&app, 42, b2).await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);
    assert_eq!(s2, StatusCode::BAD_REQUEST);
    let rows = quality_issue::Entity::find()
        .filter(quality_issue::Column::CustomOrderId.eq(42))
        .all(&db)
        .await
        .unwrap();
    assert!(
        rows.is_empty(),
        "被拒绝的上报不得留下任何质量异常，实际 {} 行",
        rows.len()
    );
}

// =========================================================
// 3) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层块（先例 contract_wave2_after_sales_create_test.rs；
/// 先剔除 \r 以抗 Windows CRLF）
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

/// handler：不得回潮"反序列化后覆盖 dto.custom_order_id / mut dto"，
/// 必须是 service.report_issue(id, dto) 的 path 权威注入形态，且无 internal 强转
#[test]
fn source_scan_report_quality_issue_contract() {
    let src = include_str!("../src/handlers/custom_order_handler.rs");
    let f = extract_block(src, "pub async fn report_quality_issue");
    assert!(
        !f.contains("dto.custom_order_id"),
        "report_quality_issue 不得回潮对 body 归属字段的读写，实际块:\n{f}"
    );
    assert!(
        !f.contains("mut dto"),
        "body DTO 已无待注入字段，不得再声明 mut，实际块:\n{f}"
    );
    assert!(
        f.contains("service.report_issue(id, dto)"),
        "必须以 path id 调 service.report_issue(id, dto)，实际块:\n{f}"
    );
    assert!(
        !f.contains("AppError::internal"),
        "质检上报错误禁止被强转 500 Internal，实际块:\n{f}"
    );
}

/// quality_err：InvalidState 必须 business_displayable（记录状态门归业务族并外显），
/// Validation 必须 validation_displayable（提交字段校验归校验族并外显）。
/// 断言跟随源码变更： 曾把两者并成 business，本轮按判据拆族。
#[test]
fn source_scan_quality_err_displayable_mapping() {
    let src = include_str!("../src/handlers/custom_order_handler.rs");
    let map = extract_block(src, "fn quality_err");
    assert!(
        map.contains("InvalidState(msg) => AppError::business_displayable(msg)"),
        "InvalidState 状态门必须外显且归 business 族，实际块:\n{map}"
    );
    assert!(
        map.contains("Validation(msg) => AppError::validation_displayable(msg)"),
        "Validation 输入校验必须外显且归 VALIDATION_ERROR 族，实际块:\n{map}"
    );
}

/// DTO 与 service：ReportQualityIssueDto 不得恢复 custom_order_id 字段；
/// report_issue 必须以独立参数取归属并 Set(custom_order_id)
#[test]
fn source_scan_dto_and_service_signature() {
    let dto_src = include_str!("../src/models/quality_issue_dto.rs");
    let dto = extract_block(dto_src, "pub struct ReportQualityIssueDto");
    assert!(
        !dto.contains("custom_order_id"),
        "ReportQualityIssueDto 不得恢复 custom_order_id 字段，实际块:\n{dto}"
    );

    let svc_src = include_str!("../src/services/custom_order_quality_service.rs");
    let f = extract_block(svc_src, "pub async fn report_issue");
    assert!(
        f.contains("custom_order_id: i64,"),
        "report_issue 必须以独立 i64 参数接收归属（path 权威注入），实际块:\n{f}"
    );
    assert!(
        f.contains("Set(custom_order_id)"),
        "ActiveModel 归属必须来自函数参数，实际块:\n{f}"
    );
    assert!(
        !f.contains("Set(dto.custom_order_id)"),
        "禁止回潮从 body DTO 取归属，实际块:\n{f}"
    );
}
