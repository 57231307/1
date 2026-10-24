//! 验布打卷与预算写端口"静默缺省拆除"契约锁
//!
//! 锁定的契约（以现行工作树为准）：
//! - `src/services/fabric_inspection_service.rs::roll_fabric`：产品缺失点名拒绝
//!   （VALIDATION_ERROR 且出参外显），与缺缸号门同落点；`build_piece_active_model`
//!   的 product_id 直取调用方保证的入参，写值处无兜底。inventory_piece.product_id
//!   是 NOT NULL 且外键指向 products 的列（migration/src/domain/business/
//!   m0010_add_inventory_extensions.rs 建表），代填 0 的真实形态是撞外键整单 500。
//! - `src/services/fabric_inspection_service.rs::grade_inspection`：产品取不到时
//!   拒绝发布 QualityInspectionCompleted（该事件经 event_kafka 外发出仓，假 0 会
//!   污染下游台账），但评级本身照常完成；result 与 FabricInspectionGraded 的 grade
//!   取本次评级的本地权威值，不再对回读行做缺省链。
//! - `src/handlers/budget_management_handler.rs::create_plan`：plan_no、plan_name、
//!   total_amount 缺键点名拒绝（budget_plans 三列 NOT NULL 无列默认，business 域
//!   m0012 建表）；显式 0 是合法声明值（零基预算草稿建账总额即 0，见
//!   budget_management_service 的 ZeroBased 分支）。
//! - `src/handlers/budget_management_handler.rs::create_budget_with_mode`：部门缺键
//!   点名拒绝；此前 unwrap_or(1) 硬编码代填真实部门，静默把预算记到别人部门名下。
//!
//! 违例形态回潮时必然红的机理（回归锁判据）：
//! - 缺产品打卷：若回潮写值处兜底 0，insert 撞外键整单 500，"400、文案点名、零落库"
//!   三断言全红；
//! - 预算缺键：若回潮空串或 0 代填，返回 200 且落库，"400、点名、行数不变"必红；
//! - 显式 0 与显式合法值：若被过度收紧拒绝，正向用例的 200 与逐列回读相等必红；
//! - 源码扫描：被修的五个函数块内出现 `unwrap_or(0)`、`unwrap_or(Decimal::ZERO)`、
//!   `unwrap_or_default()` 任一回潮即红；两文件整文件级 `unwrap_or(Decimal::ZERO)`
//!   与 `model.product_id.unwrap_or(0)`、`department_id.unwrap_or(1)` 绝迹。
//!   说明：`unwrap_or(0)` 与 `unwrap_or_default()` 在两个文件内另有两处派工范围外
//!   读侧存量（十分制码数截断换算、预算科目导出行格式化），整文件级封锁会把读侧
//!   车道卷进来，故对这两个 token 采用"被改函数块级"封锁，块外存量点名在案。
//!
//! 覆盖策略（无 mock）：表结构唯一来源是 backend/migration ——
//! `test_common::setup_test_db()`（已迁移 PostgreSQL，TRUNCATE 业务表并 RESTART
//! IDENTITY），加真实 handler 端到端（tower oneshot，同 contract_wave4 先例）。
//! 注意 budget_plans 属迁移种子参照表（setup_test_db 不清空），拒绝类断言用
//! "调用前后 COUNT 不变"，正向断言用唯一 plan_no、piece_no 定位回读。

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
use bingxi_backend::handlers::{budget_management_handler, fabric_inspection_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::budget as budget_status;
use bingxi_backend::models::status::fabric_inspection as inspection_status;
use bingxi_backend::models::{
    batch_dye_lot, budget_plan, fabric_inspection_record, inventory_piece, product, warehouse,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

mod test_common;

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 验布记录与打卷件的 created_at 与 updated_at 列在实体里是带偏移的时间戳
/// （DateTime<FixedOffset>），与预算表的 DateTime<Utc> 不同型，播种时要按各自列型取值，
/// 统一走本仓权威取法 utils date_utils::utc_now_fixed。
fn now_fixed() -> chrono::DateTime<chrono::FixedOffset> {
    bingxi_backend::utils::date_utils::utc_now_fixed()
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
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

async fn call_json(app: &Router, method: Method, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// =========================================================
// 真库夹具（列形态由迁移保证；种子按 models/*.rs 逐列核对）
// =========================================================

async fn fresh_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection) {
    warehouse::ActiveModel {
        id: Set(1),
        warehouse_code: Set("WH-NFAKE-1".to_string()),
        name: Set("契约成品仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子仓库插入失败");
}

async fn seed_product(db: &sea_orm::DatabaseConnection) {
    product::ActiveModel {
        id: Set(5),
        code: Set("FAB-NFAKE-5".to_string()),
        name: Set("契约染色布".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子产品 5 插入失败");
}

async fn seed_dye_lot(db: &sea_orm::DatabaseConnection, dye_lot_no: &str) {
    batch_dye_lot::ActiveModel {
        batch_no: Set(format!("BATCH-{dye_lot_no}")),
        product_id: Set(5),
        dye_lot_no: Set(dye_lot_no.to_string()),
        dye_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
        quantity: Set(Decimal::from(500)),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子染色批次插入失败");
}

/// 播种一条已评级(graded)验布记录；product_id、color_no 传 None 模拟"用户未录"
async fn seed_graded_inspection(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    dye_lot_no: &str,
    product_id: Option<i32>,
    color_no: Option<&str>,
) {
    fabric_inspection_record::ActiveModel {
        id: Set(id),
        inspection_no: Set(format!("FIR-NFAKE-{id:03}")),
        dye_lot_no: Set(Some(dye_lot_no.to_string())),
        product_id: Set(product_id),
        product_name: Set(product_id.map(|_| "契约染色布".to_string())),
        color_no: Set(color_no.map(str::to_string)),
        inspection_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()),
        scoring_system: Set(bingxi_backend::models::status::fabric_scoring::FOUR_POINT.to_string()),
        inspected_yards: Set(Decimal::from(50)),
        fabric_width_inches: Set(Some(Decimal::from(44))),
        total_defect_points: Set(0),
        points_per_100_sq_yards: Set(None),
        grade: Set(Some(
            bingxi_backend::models::status::fabric_grade::FIRST.to_string(),
        )),
        qualification_rate: Set(None),
        abc_grade: Set(Some("C".to_string())),
        total_rolls: Set(0),
        total_roll_length: Set(Decimal::ZERO),
        total_roll_weight: Set(Decimal::ZERO),
        status: Set(inspection_status::GRADED.to_string()),
        is_deleted: Set(false),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子验布记录 {id} 插入失败: {e}"));
}

fn build_fabric_app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/fabric-inspections/{id}/roll",
            post(fabric_inspection_handler::roll_fabric),
        )
        .route(
            "/fabric-inspections/{id}/grade",
            post(fabric_inspection_handler::grade_inspection),
        )
        .with_state(state)
}

fn build_budget_app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/budgets/plans",
            post(budget_management_handler::create_plan),
        )
        .route(
            "/budgets/create-with-mode",
            post(budget_management_handler::create_budget_with_mode),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

fn roll_body(warehouse_id: i32) -> Value {
    json!({
        "warehouse_id": warehouse_id,
        "roll_length": 100.0,
        "roll_weight": 12.5,
        "roll_width": 160.0,
        "roll_gram_weight": 200.0
    })
}

/// 从 inventory_piece 落库行直读关键列（raw SELECT，Postgres 方言 $1 绑定）
async fn fetch_piece_row(
    db: &sea_orm::DatabaseConnection,
    piece_no: &str,
) -> (i32, String, String, Decimal, Decimal) {
    let row = db
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "SELECT product_id, color_no, dye_lot_no, length, weight FROM inventory_piece \
             WHERE piece_no = $1",
            vec![piece_no.to_string().into()],
        ))
        .await
        .unwrap()
        .expect("被断言的落库布卷行应存在");
    (
        row.try_get::<i32>("", "product_id").unwrap(),
        row.try_get::<String>("", "color_no").unwrap(),
        row.try_get::<String>("", "dye_lot_no").unwrap(),
        row.try_get::<Decimal>("", "length").unwrap(),
        row.try_get::<Decimal>("", "weight").unwrap(),
    )
}

// =========================================================
// 验布打卷，缺产品点名拒绝与零落库；显式合法值逐列回读
// =========================================================

#[tokio::test]
async fn roll_without_product_rejected_400_and_no_piece_persisted() {
    let db = fresh_db().await;
    seed_warehouse(&db).await;
    seed_product(&db).await;
    seed_dye_lot(&db, "DL-NFAKE-1").await;
    seed_graded_inspection(&db, 1, "DL-NFAKE-1", None, None).await;
    let app = build_fabric_app(db.clone());
    let (status, v) = call_json(
        &app,
        Method::POST,
        "/fabric-inspections/1/roll",
        roll_body(1),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺产品打卷必须 400（修复前代填 0 撞外键整单 500）, 实际: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    let message = v["message"].as_str().unwrap_or_default().to_string();
    assert!(
        message.contains("产品") && message.contains("验布记录"),
        "拒绝文案必须点名缺什么、去哪补，实际: {message}"
    );
    let rows = inventory_piece::Entity::find().count(&db).await.unwrap();
    assert_eq!(rows, 0, "被拒绝的打卷必须零落库");
}

#[tokio::test]
async fn roll_with_product_persists_declared_product_and_measured_values() {
    let db = fresh_db().await;
    seed_warehouse(&db).await;
    seed_product(&db).await;
    seed_dye_lot(&db, "DL-NFAKE-2").await;
    // 一号有实测色号、二号无色号（color_no 列默认空串分支）
    seed_graded_inspection(&db, 1, "DL-NFAKE-2", Some(5), Some("C-RED-1")).await;
    seed_graded_inspection(&db, 2, "DL-NFAKE-2", Some(5), None).await;
    let app = build_fabric_app(db.clone());
    for id in [1, 2] {
        let (status, v) = call_json(
            &app,
            Method::POST,
            &format!("/fabric-inspections/{id}/roll"),
            roll_body(1),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "显式合法值打卷应 200, 实际: {v}");
    }
    let expect_color = ["C-RED-1", ""];
    // 匹号由 generate_next_piece_no 按缸号内既有最大 piece_seq 加一生出，两次打卷
    // 落 001 与 002；期望序号与打卷次序同号（从 1 起），逐列对照声明值
    for id in 1..=2usize {
        let piece_no = format!("DL-NFAKE-2-{id:03}");
        let (product_id, color_no, dye_lot_no, length, weight) =
            fetch_piece_row(&db, &piece_no).await;
        assert_eq!(product_id, 5, "布卷产品必须等于验布记录声明的产品");
        assert_eq!(color_no, expect_color[id - 1], "色号回读必须与声明一致");
        assert_eq!(dye_lot_no, "DL-NFAKE-2");
        assert_eq!(length, Decimal::from(100));
        assert_eq!(weight, Decimal::from_str("12.5").unwrap());
    }
}

#[tokio::test]
async fn grade_without_product_still_completes_state_transition() {
    // 事件族收口只拒绝发布 QualityInspectionCompleted 外发事件，不得把评级整单判死
    let db = fresh_db().await;
    seed_graded_inspection_preparing(&db).await;
    let app = build_fabric_app(db.clone());
    let (status, v) = call_json(
        &app,
        Method::POST,
        "/fabric-inspections/1/grade",
        json!({ "inspected_yards": 50.0 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "缺产品评级本身不受事件收口影响，应 200, 实际: {v}"
    );
    let record = fabric_inspection_record::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .expect("评级后的验布记录应存在");
    assert_eq!(record.status, inspection_status::GRADED);
    assert_eq!(record.product_id, None, "记录的产品列保持 NULL，不代填");
}

async fn seed_graded_inspection_preparing(db: &sea_orm::DatabaseConnection) {
    fabric_inspection_record::ActiveModel {
        id: Set(1),
        inspection_no: Set("FIR-NFAKE-G01".to_string()),
        dye_lot_no: Set(Some("DL-NFAKE-G".to_string())),
        product_id: Set(None),
        inspection_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()),
        scoring_system: Set(bingxi_backend::models::status::fabric_scoring::FOUR_POINT.to_string()),
        inspected_yards: Set(Decimal::ZERO),
        fabric_width_inches: Set(Some(Decimal::from(44))),
        total_defect_points: Set(0),
        points_per_100_sq_yards: Set(None),
        grade: Set(None),
        qualification_rate: Set(None),
        abc_grade: Set(None),
        total_rolls: Set(0),
        total_roll_length: Set(Decimal::ZERO),
        total_roll_weight: Set(Decimal::ZERO),
        status: Set(inspection_status::INSPECTING.to_string()),
        is_deleted: Set(false),
        created_at: Set(now_fixed()),
        updated_at: Set(now_fixed()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子验布记录(inspecting)插入失败");
}

// =========================================================
// 预算方案，plan_no、plan_name、total_amount 缺键点名拒绝
// =========================================================

fn valid_plan_body() -> Value {
    json!({
        "plan_no": "BP-NFAKE-FULL",
        "plan_name": "契约完整方案",
        "budget_year": 2026,
        "budget_type": "年度预算",
        "department_id": 3,
        "total_amount": 8800.50
    })
}

async fn count_plans(db: &sea_orm::DatabaseConnection) -> u64 {
    budget_plan::Entity::find().count(db).await.unwrap()
}

#[tokio::test]
async fn create_plan_missing_required_keys_rejected_400_by_name() {
    let db = fresh_db().await;
    let app = build_budget_app(db.clone());
    let cases = [
        ("plan_no", "方案编号"),
        ("plan_name", "方案名称"),
        ("total_amount", "总金额"),
    ];
    let before = count_plans(&db).await;
    for (drop_key, expect_token) in cases {
        let mut body = valid_plan_body();
        body.as_object_mut().unwrap().remove(drop_key);
        let (status, v) = call_json(&app, Method::POST, "/budgets/plans", body).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "缺 {drop_key} 必须 400（修复前空串/0 代填静默建单）, 实际: {v}"
        );
        assert_eq!(v["code"], "VALIDATION_ERROR");
        let message = v["message"].as_str().unwrap_or_default().to_string();
        assert!(
            message.contains(expect_token),
            "拒绝文案必须点名缺哪个字段（{drop_key}），实际: {message}"
        );
    }
    assert_eq!(
        count_plans(&db).await,
        before,
        "被拒绝的建单必须零落库（budget_plans 含迁移种子行，按总数不变判定）"
    );
}

#[tokio::test]
async fn create_plan_with_explicit_values_persists_every_declared_column() {
    let db = fresh_db().await;
    let app = build_budget_app(db.clone());
    let (status, v) = call_json(&app, Method::POST, "/budgets/plans", valid_plan_body()).await;
    assert_eq!(status, StatusCode::OK, "完整提交应 200, 实际: {v}");
    let plan = budget_plan::Entity::find()
        .filter(budget_plan::Column::PlanNo.eq("BP-NFAKE-FULL"))
        .one(&db)
        .await
        .unwrap()
        .expect("显式提交的方案应可按 plan_no 回读");
    assert_eq!(plan.plan_name, "契约完整方案");
    assert_eq!(plan.budget_year, 2026);
    assert_eq!(plan.budget_type, "年度预算");
    assert_eq!(plan.department_id, Some(3));
    assert_eq!(plan.total_amount, Decimal::from_str("8800.50").unwrap());
    assert_eq!(plan.status.as_deref(), Some(budget_status::DRAFT));
}

#[tokio::test]
async fn create_plan_with_declared_zero_total_persists_zero_not_rejected() {
    // 显式 0 是声明值：零基草稿口径承认总额 0 合法，只有缺键才拒绝
    let db = fresh_db().await;
    let app = build_budget_app(db.clone());
    let mut body = valid_plan_body();
    body["plan_no"] = json!("BP-NFAKE-ZERO");
    body["total_amount"] = json!(0);
    let (status, v) = call_json(&app, Method::POST, "/budgets/plans", body).await;
    assert_eq!(status, StatusCode::OK, "显式 0 总额应 200, 实际: {v}");
    let plan = budget_plan::Entity::find()
        .filter(budget_plan::Column::PlanNo.eq("BP-NFAKE-ZERO"))
        .one(&db)
        .await
        .unwrap()
        .expect("显式 0 总额方案应落库");
    assert_eq!(plan.total_amount, Decimal::ZERO);
}

// =========================================================
// 多模式预算，部门缺键点名拒绝，不代填真实部门 1
// =========================================================

#[tokio::test]
async fn create_with_mode_missing_department_rejected_400_and_no_plan_created() {
    let db = fresh_db().await;
    let app = build_budget_app(db.clone());
    let before = count_plans(&db).await;
    let (status, v) = call_json(
        &app,
        Method::POST,
        "/budgets/create-with-mode",
        json!({ "mode": "zero_based", "target_year": 2027 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺部门必须 400（修复前 unwrap_or(1) 静默挂到部门 1）, 实际: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    let message = v["message"].as_str().unwrap_or_default().to_string();
    assert!(
        message.contains("部门"),
        "拒绝文案必须点名缺部门，实际: {message}"
    );
    assert_eq!(count_plans(&db).await, before, "被拒绝的创建必须零落库");
}

#[tokio::test]
async fn create_with_mode_explicit_department_persists_declared_department() {
    let db = fresh_db().await;
    let app = build_budget_app(db.clone());
    let (status, v) = call_json(
        &app,
        Method::POST,
        "/budgets/create-with-mode",
        json!({ "mode": "zero_based", "target_year": 2027, "department_id": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "显式部门零基建账应 200, 实际: {v}");
    // 服务层零基分支的 plan_name 由 target_year 与 department_id 拼成，可精确定位回读
    let plan = budget_plan::Entity::find()
        .filter(budget_plan::Column::PlanName.eq("2027年度零基预算-部门7"))
        .one(&db)
        .await
        .unwrap()
        .expect("零基方案应按声明部门落库");
    assert_eq!(
        plan.department_id,
        Some(7),
        "归属部门必须等于声明值，不是代填的 1"
    );
    assert_eq!(plan.budget_year, 2027);
    assert_eq!(plan.total_amount, Decimal::ZERO, "零基草稿建账总额即为 0");
}

// =========================================================
// 源码扫描防回潮锁（无 DB）
// =========================================================

fn read_src(path: &str) -> String {
    let raw = match path {
        "fabric" => include_str!("../src/services/fabric_inspection_service.rs"),
        "budget" => include_str!("../src/handlers/budget_management_handler.rs"),
        _ => unreachable!("内部用例分支固定"),
    };
    raw.replace('\r', "")
}

/// 截取 start 锚点到 end 锚点之间的源码块（含 start 自身，不含 end）
fn extract_between(src: &str, start: &str, end: &str) -> String {
    let i = src
        .find(start)
        .unwrap_or_else(|| panic!("源码起点锚丢失: {start}"));
    let j = src[i..]
        .find(end)
        .unwrap_or_else(|| panic!("源码终点锚丢失: {end}（起点 {start} 之后未出现）"));
    src[i..i + j].to_string()
}

const FAKE_DEFAULT_TOKENS: [&str; 3] = [
    "unwrap_or(Decimal::ZERO)",
    "unwrap_or(0)",
    "unwrap_or_default()",
];

#[test]
fn source_scan_fabric_write_blocks_have_no_fake_default_tokens() {
    let src = read_src("fabric");
    let blocks = [
        (
            "grade_inspection",
            extract_between(
                &src,
                "pub async fn grade_inspection",
                "pub async fn roll_fabric",
            ),
        ),
        (
            "roll_fabric",
            extract_between(
                &src,
                "pub async fn roll_fabric",
                "fn validate_roll_preconditions",
            ),
        ),
        (
            "build_piece_active_model",
            extract_between(
                &src,
                "fn build_piece_active_model",
                "async fn apply_roll_summary",
            ),
        ),
    ];
    for (name, block) in blocks {
        for token in FAKE_DEFAULT_TOKENS {
            assert!(
                !block.contains(token),
                "fabric_inspection_service.rs 写侧函数 {name} 块内禁止出现 {token}，块:\n{block}"
            );
        }
    }
    // 写值处与事件族的正向判据
    let roll = extract_between(
        &src,
        "pub async fn roll_fabric",
        "fn validate_roll_preconditions",
    );
    assert!(
        roll.contains("model.product_id.ok_or_else"),
        "roll_fabric 必须保留产品缺失去点名拒绝的门，实际:\n{roll}"
    );
    let grade = extract_between(
        &src,
        "pub async fn grade_inspection",
        "pub async fn roll_fabric",
    );
    assert!(
        grade.contains("if let Some(product_id) = updated.product_id"),
        "QualityInspectionCompleted 必须按产品存在与否条件发布，不得回潮压 0，实际:\n{grade}"
    );
    let builder = extract_between(
        &src,
        "fn build_piece_active_model",
        "async fn apply_roll_summary",
    );
    assert!(
        builder.contains("product_id: Set(product_id)"),
        "布卷 product_id 必须直取调用方保证的入参，实际:\n{builder}"
    );
    // 整文件级：两处被点名修复的假值写行绝迹
    assert!(
        !src.contains("model.product_id.unwrap_or(0)"),
        "fabric_inspection_service.rs 禁止回潮 model.product_id 代填 0"
    );
}

#[test]
fn source_scan_budget_write_blocks_have_no_fake_default_tokens() {
    let src = read_src("budget");
    let create_plan = extract_between(&src, "pub async fn create_plan", "pub async fn get_plan");
    let create_mode = extract_between(
        &src,
        "pub async fn create_budget_with_mode",
        "pub async fn budget_execution_warnings",
    );
    for (name, block) in [("create_plan", create_plan), ("create_mode", create_mode)] {
        for token in FAKE_DEFAULT_TOKENS {
            assert!(
                !block.contains(token),
                "budget_management_handler.rs 写侧函数 {name} 块内禁止出现 {token}，块:\n{block}"
            );
        }
    }
    // 整文件级：Decimal::ZERO 兜底与部门代填 1 绝迹（读侧导出块外的存量不含这两个 token）
    assert!(
        !src.contains("unwrap_or(Decimal::ZERO)"),
        "budget_management_handler.rs 整文件禁止出现 unwrap_or(Decimal::ZERO)"
    );
    assert!(
        !src.contains("department_id.unwrap_or(1)"),
        "budget_management_handler.rs 禁止回潮部门代填 unwrap_or(1)"
    );
    assert!(
        !src.contains("req.plan_no.unwrap_or_default()")
            && !src.contains("req.plan_name.unwrap_or_default()"),
        "budget_management_handler.rs 禁止回潮方案编号/名称空串代填"
    );
    let mode = extract_between(
        &src,
        "pub async fn create_budget_with_mode",
        "pub async fn budget_execution_warnings",
    );
    assert!(
        mode.contains("缺少部门ID") && mode.contains("ok_or_else"),
        "create_budget_with_mode 必须保留部门缺键点名拒绝门，实际:\n{mode}"
    );
}

#[test]
fn source_scan_fabric_file_has_no_decimal_zero_fake_default() {
    let src = read_src("fabric");
    assert!(
        !src.contains("unwrap_or(Decimal::ZERO)"),
        "fabric_inspection_service.rs 整文件禁止出现 unwrap_or(Decimal::ZERO)"
    );
}
