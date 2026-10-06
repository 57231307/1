//! 后端安全「建单人/登记人只认服务端会话」行为级活体证明
//!
//! 锁定的契约：以下创建型端点的审计归属列 `created_by` 唯一来源是
//! `AuthContext.user_id`（服务端会话），请求 DTO 不再承载该身份字段；
//! 请求体里伪造的 `created_by` 会被 serde 忽略，绝不落库。
//! - `POST /environmental-tax/discharge-records`（服务层直证：适用税额为进程级
//!   部署配置单例，测试二进制无法注入，HTTP 成功路径不可控，故按税额显式注入
//!   走 `EnvironmentalTaxService::create_discharge_record(req, user_id)` 真库真落库）；
//! - `POST /export-refund/customs-declarations`；
//! - `POST /period-adjustments`；
//! - `POST /labor-contracts`；
//! - `POST /occupational-health/hazard-monitorings`、`/health-exams`、`/ppe-distributions`；
//! - `POST /social-insurance`；
//! - `POST /pollution-monitoring/solid-waste`。
//!
//! 证明形态（夹具范式同 contract_wave11_audit_identity_from_session）：
//! ① 会话注入用户 A（`from_fn_with_state(make_auth(A), inject_auth)`）；
//! ② 请求体故意携带伪造用户 B 的 `created_by`；③ 动作必须成功（该键非必填）；
//! ④ 直读实体行断言 `row.created_by == Some(SESSION_A)` 且 `!= Some(FORGED_B)`。
//!
//! 另含一条形态锁：解析源码断言这批请求 DTO 已不存在 `created_by` 入参字段，
//! 并带检测力地板（九个 DTO 结构体逐个定位，任一取不到即判红，禁止空集合=绿）。
//!
//! 检测力（改坏源码即红的点位）：
//! - 若 handler 退回从 body 取 `req.created_by` → 直读断言 `== Some(SESSION_A)` 红；
//! - 若 service 退回 `created_by: Set(req.created_by)` 或 `unwrap_or` 兜底 → 断言红；
//! - 若 DTO 重新加回 `created_by` 字段 → 形态锁当场红；
//! - 若形态锁解析不到任何目标结构体 → 地板断言当场红（防空集合假绿）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    environmental_tax_handler, export_refund_handler, labor_contract_handler,
    occupational_health_handler, period_adjustment_handler, pollution_monitoring_handler,
    social_insurance_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    export_customs_declaration, labor_contract, occupational_hazard_monitoring,
    occupational_health_exam, period_adjustment_record, pollutant_discharge_record,
    ppe_distribution_record, social_insurance_record, solid_waste_disposal_record,
};
use bingxi_backend::services::environmental_tax_service::{
    CreateDischargeRecordRequest, EnvironmentalTaxService,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 会话用户 A：AuthContext.user_id 注入的合法建单人（私有 id 段 951x，
/// 与同族锁 contract_wave11_audit_identity_from_session 保持同一对常量）
const SESSION_A: i32 = 9511;
/// 伪造用户 B：只出现在请求体里，永远不允许落进任何 created_by 列
const FORGED_B: i32 = 9472;

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("w11_createdby_{user_id}"),
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

/// 请求体统一掺入伪造身份键（handler 不认的键 serde 直接忽略——正是要证明的「发了也无效」）
fn with_forged_identity(mut obj: serde_json::Map<String, Value>) -> String {
    obj.insert("created_by".to_string(), Value::from(FORGED_B));
    obj.insert("operator_id".to_string(), Value::from(FORGED_B));
    obj.insert("user_id".to_string(), Value::from(FORGED_B));
    serde_json::to_string(&Value::Object(obj)).expect("伪造身份体必须是合法 JSON（夹具自证）")
}

async fn call_post(app: &Router, path: &str, body: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
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

/// 以 SESSION_A 为会话用户组装带 auth 注入层的 Router（路由按占位符注册，同真实 routes 形态）
fn layered_app(
    state: AppState,
    routes: Vec<(&'static str, axum::routing::MethodRouter<AppState>)>,
) -> Router {
    let mut router: Router<AppState> = Router::new();
    for (path, method_route) in routes {
        router = router.route(path, method_route);
    }
    router
        .with_state(state)
        .layer(from_fn_with_state(make_auth(SESSION_A), inject_auth))
}

/// 成功信封取新建行 id，并断动作真实成功（缺身份键合法、伪造键被忽略）
fn created_id(status: StatusCode, v: &Value, what: &str) -> i32 {
    assert_eq!(
        status,
        StatusCode::OK,
        "{what} 必须成功（created_by 不再是必填入参），实际: {status} {v}"
    );
    assert_eq!(v["code"], 200, "{what} 成功信封 code 必须为 200, 实际: {v}");
    v["data"]["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("{what} 出参必须携带新行 id, 实际: {v}")) as i32
}

/// 断言直读行的 created_by 归会话用户 A、绝归伪造用户 B
fn assert_created_by_is_session(row_created_by: Option<i32>, what: &str) {
    assert_eq!(
        row_created_by,
        Some(SESSION_A),
        "直读实体：{what} 建单人必须等于会话用户 A，实际 {row_created_by:?}"
    );
    assert_ne!(
        row_created_by,
        Some(FORGED_B),
        "直读实体：body 伪造的 B 绝不许落进 {what} 的建单人列"
    );
}

macro_rules! read_row {
    ($mod:ident, $id:expr, $db:expr, $what:literal) => {
        $mod::Entity::find_by_id($id)
            .one($db)
            .await
            .unwrap_or_else(|e| panic!(concat!($what, " 直读失败: {e}")))
            .unwrap_or_else(|| panic!(concat!($what, " 必须存在")))
    };
}

// =========================================================
// 域 1：环保税污染物排放记录（服务层直证）
// =========================================================

#[tokio::test]
async fn discharge_record_created_by_comes_from_caller_user_not_body() {
    let db = setup_test_db().await;
    // 适用税额显式注入服务实例（进程级配置单例在测试二进制不可控，与
    // contract_wave5_env_tax_rates_from_config 同口径）；请求 DTO 已无
    // created_by 字段，身份只能由 user_id 形参进入。
    let svc = EnvironmentalTaxService::new(Arc::new(db.clone()), Some(Decimal::new(24, 1)));
    let req = CreateDischargeRecordRequest {
        discharge_type: "wastewater".to_string(),
        pollutant_name: "COD".to_string(),
        discharge_amount: Decimal::new(100, 0),
        discharge_unit: Some("kg".to_string()),
        concentration: None,
        concentration_unit: None,
        period_year: 2026,
        period_month: 9,
        monitoring_point: Some("_createdby_b1_lock".to_string()),
        remarks: None,
    };
    let created = svc
        .create_discharge_record(req, SESSION_A)
        .await
        .expect("会话用户建单必须成功");
    assert_created_by_is_session(created.created_by, "污染物排放记录");

    let row = read_row!(
        pollutant_discharge_record,
        created.id,
        &db,
        "污染物排放记录"
    );
    assert_created_by_is_session(row.created_by, "污染物排放记录（回读）");
    assert_eq!(row.pollutant_name, "COD", "业务列不受身份改造牵连");
}

// =========================================================
// 域 2：出口报关单（export_refund_handler）
// =========================================================

#[tokio::test]
async fn customs_declaration_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/export-refund/customs-declarations",
            post(export_refund_handler::create_customs_declaration),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert(
        "declaration_no".to_string(),
        json!(format!(
            "CD-W11CB1-{}",
            now().timestamp_nanos_opt().unwrap()
        )),
    );
    body.insert("sales_order_id".to_string(), json!(null));
    body.insert("export_date".to_string(), json!("2026-09-01"));
    body.insert("total_amount".to_string(), json!(1000));
    body.insert("exchange_rate".to_string(), json!("7.1"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/export-refund/customs-declarations", &body).await;
    let id = created_id(status, &v, "出口报关单建单");

    let row = read_row!(export_customs_declaration, id, &read_db, "出口报关单");
    assert_created_by_is_session(row.created_by, "出口报关单");
    assert_eq!(row.total_amount, Decimal::new(1000, 0), "业务列不受牵连");
}

// =========================================================
// 域 3：期末调整（period_adjustment_handler）
// =========================================================

#[tokio::test]
async fn period_adjustment_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/period-adjustments",
            post(period_adjustment_handler::create_adjustment),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("adjustment_type".to_string(), json!("estimate"));
    body.insert("period".to_string(), json!("2026-09"));
    body.insert("description".to_string(), json!("暂估入账（建单人会话锁）"));
    body.insert("debit_subject_code".to_string(), json!("6601"));
    body.insert("debit_subject_name".to_string(), json!("管理费用"));
    body.insert("credit_subject_code".to_string(), json!("2202"));
    body.insert("credit_subject_name".to_string(), json!("应付账款"));
    body.insert("amount".to_string(), json!(100));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/period-adjustments", &body).await;
    let id = created_id(status, &v, "期末调整建单");

    let row = read_row!(period_adjustment_record, id, &read_db, "期末调整记录");
    assert_created_by_is_session(row.created_by, "期末调整记录");
    assert_eq!(
        row.status, "draft",
        "业务状态列不受身份改造牵连（建单仍走真实取号+draft 门）"
    );
}

// =========================================================
// 域 4：劳动合同（labor_contract_handler）
// =========================================================

#[tokio::test]
async fn labor_contract_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![("/labor-contracts", post(labor_contract_handler::create))],
    );
    let mut body = serde_json::Map::new();
    body.insert("worker_id".to_string(), json!(990001));
    body.insert(
        "contract_no".to_string(),
        json!(format!(
            "LC-W11CB1-{}",
            now().timestamp_nanos_opt().unwrap()
        )),
    );
    body.insert("contract_type".to_string(), json!("permanent"));
    body.insert("start_date".to_string(), json!("2026-09-01"));
    body.insert("probation_salary".to_string(), json!(0));
    body.insert("regular_salary".to_string(), json!(6000));
    body.insert("working_hours_system".to_string(), json!("standard"));
    body.insert("sign_date".to_string(), json!("2026-09-01"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/labor-contracts", &body).await;
    let id = created_id(status, &v, "劳动合同建单");

    let row = read_row!(labor_contract, id, &read_db, "劳动合同");
    assert_created_by_is_session(row.created_by, "劳动合同");
    assert_eq!(row.worker_id, 990001, "业务列（工人引用）不受牵连");
}

// =========================================================
// 域 5：职业危害因素检测（occupational_health_handler）
// =========================================================

#[tokio::test]
async fn hazard_monitoring_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/occupational-health/hazard-monitorings",
            post(occupational_health_handler::create_hazard_monitoring),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("hazard_type".to_string(), json!("chemical"));
    body.insert("hazard_name".to_string(), json!("苯"));
    body.insert("monitoring_point".to_string(), json!("染色车间"));
    body.insert("measured_value".to_string(), json!(5));
    body.insert("unit".to_string(), json!("mg/m3"));
    body.insert("limit_value".to_string(), json!(10));
    body.insert("monitoring_date".to_string(), json!("2026-09-01"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/occupational-health/hazard-monitorings", &body).await;
    let id = created_id(status, &v, "职业危害检测建单");

    let row = read_row!(
        occupational_hazard_monitoring,
        id,
        &read_db,
        "职业危害检测记录"
    );
    assert_created_by_is_session(row.created_by, "职业危害检测记录");
    assert_eq!(row.hazard_name, "苯", "业务列不受牵连");
}

// =========================================================
// 域 6：职业健康体检档案（occupational_health_handler）
// =========================================================

#[tokio::test]
async fn health_exam_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/occupational-health/health-exams",
            post(occupational_health_handler::create_health_exam),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("worker_id".to_string(), json!(990002));
    body.insert("exam_type".to_string(), json!("pre_employment"));
    body.insert("exam_date".to_string(), json!("2026-09-01"));
    body.insert("exam_result".to_string(), json!("normal"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/occupational-health/health-exams", &body).await;
    let id = created_id(status, &v, "职业健康体检建单");

    let row = read_row!(occupational_health_exam, id, &read_db, "职业健康体检档案");
    assert_created_by_is_session(row.created_by, "职业健康体检档案");
    assert_eq!(row.worker_id, 990002, "业务列（受检工人）不受牵连");
}

// =========================================================
// 域 7：PPE 发放记录（occupational_health_handler）
// =========================================================

#[tokio::test]
async fn ppe_distribution_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/occupational-health/ppe-distributions",
            post(occupational_health_handler::create_ppe_distribution),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert("worker_id".to_string(), json!(990003));
    body.insert("ppe_name".to_string(), json!("防毒口罩"));
    body.insert("ppe_type".to_string(), json!("mask"));
    body.insert("quantity".to_string(), json!(2));
    body.insert("distribution_date".to_string(), json!("2026-09-01"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/occupational-health/ppe-distributions", &body).await;
    let id = created_id(status, &v, "PPE 发放建单");

    let row = read_row!(ppe_distribution_record, id, &read_db, "PPE 发放记录");
    assert_created_by_is_session(row.created_by, "PPE 发放记录");
    assert_eq!(row.status, "distributed", "业务状态列不受牵连");
}

// =========================================================
// 域 8：社保公积金缴纳记录（social_insurance_handler）
// =========================================================

#[tokio::test]
async fn social_insurance_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![("/social-insurance", post(social_insurance_handler::create))],
    );
    let mut body = serde_json::Map::new();
    body.insert("worker_id".to_string(), json!(990004));
    body.insert("period_year".to_string(), json!(2026));
    body.insert("period_month".to_string(), json!(9));
    body.insert("base_amount".to_string(), json!(10000));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/social-insurance", &body).await;
    let id = created_id(status, &v, "社保记录建单");

    let row = read_row!(social_insurance_record, id, &read_db, "社保缴纳记录");
    assert_created_by_is_session(row.created_by, "社保缴纳记录");
    assert_eq!(
        row.status, "pending",
        "业务状态列不受牵连（五险一金计算照常）"
    );
    assert!(row.total_employer > Decimal::ZERO, "计算列必须真实落库");
}

// =========================================================
// 域 9：固废处置联单（pollution_monitoring_handler）
// =========================================================

#[tokio::test]
async fn solid_waste_disposal_created_by_comes_from_session_not_body() {
    let db = setup_test_db().await;
    let read_db = db.clone();
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    let app = layered_app(
        state,
        vec![(
            "/pollution-monitoring/solid-waste",
            post(pollution_monitoring_handler::create_solid_waste_disposal),
        )],
    );
    let mut body = serde_json::Map::new();
    body.insert(
        "manifest_no".to_string(),
        json!(format!(
            "SW-W11CB1-{}",
            now().timestamp_nanos_opt().unwrap()
        )),
    );
    body.insert("waste_type".to_string(), json!("sludge"));
    body.insert("waste_category".to_string(), json!("general"));
    body.insert("waste_amount".to_string(), json!(5));
    body.insert("generation_date".to_string(), json!("2026-09-01"));
    body.insert("disposal_method".to_string(), json!("landfill"));
    let body = with_forged_identity(body);
    let (status, v) = call_post(&app, "/pollution-monitoring/solid-waste", &body).await;
    let id = created_id(status, &v, "固废处置联单建单");

    let row = read_row!(solid_waste_disposal_record, id, &read_db, "固废处置联单");
    assert_created_by_is_session(row.created_by, "固废处置联单");
    assert_eq!(row.status, "pending", "业务状态列不受牵连");
}

// =========================================================
// 形态锁：这批请求 DTO 不再承载 created_by 入参字段
// =========================================================

/// 在源码文本中定位 `pub struct NAME { ... }` 的结构体体文本。
/// 找不到即返回 None——由调用方按地板判红，禁止静默跳过。
fn struct_body(src: &str, name: &str) -> Option<String> {
    let marker = format!("pub struct {name} {{");
    let start = src.find(&marker)? + marker.len();
    let rest = &src[start..];
    let end = rest.find("\n}")?;
    Some(rest[..end].to_string())
}

#[test]
fn batch1_request_dtos_have_no_created_by_field() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // (服务源码相对路径, DTO 名) 九个建单入参结构体，与上方九条行为锁一一对应
    let targets: [(&str, &str); 9] = [
        (
            "src/services/environmental_tax_service.rs",
            "CreateDischargeRecordRequest",
        ),
        (
            "src/services/export_refund_service.rs",
            "CreateCustomsDeclarationRequest",
        ),
        (
            "src/services/period_adjustment_service.rs",
            "CreatePeriodAdjustmentRequest",
        ),
        (
            "src/services/labor_contract_service.rs",
            "CreateLaborContractRequest",
        ),
        (
            "src/services/occupational_health_service.rs",
            "CreateHazardMonitoringRequest",
        ),
        (
            "src/services/occupational_health_service.rs",
            "CreateHealthExamRequest",
        ),
        (
            "src/services/occupational_health_service.rs",
            "CreatePpeDistributionRequest",
        ),
        (
            "src/services/social_insurance_service.rs",
            "CreateSocialInsuranceRequest",
        ),
        (
            "src/services/pollution_monitoring_service.rs",
            "CreateSolidWasteDisposalRequest",
        ),
    ];
    let mut parsed = 0usize;
    for (file, dto) in targets.iter() {
        let path = std::path::Path::new(manifest_dir).join(file);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("形态锁取不到源码 {}（检测力地板）: {e}", path.display()));
        let body = struct_body(&src, dto)
            .unwrap_or_else(|| panic!("形态锁取不到结构体 {dto}（检测力地板，禁止空集合=绿）"));
        parsed += 1;
        assert!(
            !body.contains("created_by"),
            "请求 DTO {dto}（{file}）不应再承载 created_by 入参字段，实际体: {body}"
        );
    }
    assert_eq!(
        parsed,
        targets.len(),
        "九个建单 DTO 必须全部被解析到（检测力地板），实际 {parsed}"
    );
}
