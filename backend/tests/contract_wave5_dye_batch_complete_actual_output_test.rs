//! 任务 #168（PR #942 续）：缸号完工强制登记实际产出（kg + 米 + 坯布投料量）契约锁
//!
//! 锁定的契约（修复后形态）：
//! - `backend/migration/src/domain/production/m0062_add_dye_batch_actual_output.rs`
//!   dye_batch 新增 actual_output_kg / actual_output_m / greige_input_kg 三列
//!   DECIMAL(12,2) NULL（历史迁移零改动，注册于 domain/production/mod.rs 域尾）。
//! - `backend/src/handlers/dye_batch_handler.rs::complete_dye_batch`
//!   POST /production/dye-batches/{id}/complete 由零请求体改为必传
//!   CompleteDyeBatchRequest{actual_output_kg, actual_output_m, greige_input_kg}，
//!   validate（正数/≤10亿/2位小数，utils::validator::validate_amount_range 全仓范式）
//!   **先于**状态机门控——校验失败一律 400 且状态与三列零落库（不静默推进）。
//!   失重率不做拒绝门：仓内唯一权威口径（outsourcing_service.rs 染色标准损耗 5%，§5.7
//!   行业中值）语义是正常/异常损耗核算分类、非完工拒绝边界，超阈值仅 fail-visible
//!   warn 日志；完工拒绝阈值待产品给口径。
//! - `backend/src/services/dye_batch_cost_bridge_service.rs::build_draft_cost_request`
//!   draft 成本归集产量分母 output_quantity_kg/_meters 取自 dye_batch 行完工真值
//!   （原恒写 None）。单位成本不另造算法：仍由 CostCollectionService 既有
//!   total_cost/产量 算法推导（cost_collection_service.rs:88-103、276-288）。
//!
//! 修复前必然红的机理：
//! - ①：旧 complete handler 无 body 参数——同一路由请求带 JSON body 时 serde 无中间件
//!   消费、三列在旧模型/旧表不存在，落库真值断言必失败；旧桥接把分母恒写 None，
//!   build_draft_cost_request 不存在（编译即红）。
//! - ②：旧实现零请求体，空 body/缺键照样 200 且把状态推进到 stored——"400 且状态
//!   未推进（completed_at 仍 NULL、三列 NULL）"的回读断言必红。
//! - ③：同上，0/负值旧实现根本不读 body，无法拒绝。
//! - ④：源码扫描——旧桥接含 `output_quantity_kg: None`、旧 handler complete 块不含
//!   `Json(req): Json<CompleteDyeBatchRequest>`，m0062 文件不存在（include_str! 编译即红）。
//!
//! 覆盖策略（无 mock）：sqlite::memory: 自建 dye_batch 表（列与 models/dye_batch.rs
//! 一一对应，先例 contract_wave4 同域形态）+ tower oneshot 真实路由。成本 draft 整链
//! 无法 sqlite 化（cost_collection 取号走 pg_advisory_xact_lock，
//! utils/crud_macro.rs::impl_generate_no → number_generator::lock_prefix），故对
//! "行真值→draft 请求分母"映射用纯函数 build_draft_cost_request 直接断言 + 源码扫描锁
//! 桥接调用点，不造假数据、不 mock。

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
use bingxi_backend::handlers::dye_batch_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::dye_batch;
use bingxi_backend::services::dye_batch_cost_bridge_service::DyeBatchCostBridgeServiceInternal;
use rust_decimal::Decimal;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
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

// =========================================================
// sqlite 自建表（列与 models/dye_batch.rs 一一对应，含 #168 三列）
// =========================================================

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

const DYE_BATCH_DDL: &str = r#"CREATE TABLE dye_batch (
    id INTEGER PRIMARY KEY,
    batch_no TEXT NOT NULL UNIQUE,
    greige_fabric_id INTEGER,
    color_code TEXT NOT NULL,
    color_name TEXT NOT NULL,
    color_no TEXT,
    dye_lot_no TEXT,
    planned_quantity TEXT,
    actual_output_kg TEXT,
    actual_output_m TEXT,
    greige_input_kg TEXT,
    status TEXT,
    started_at TEXT,
    completed_at TEXT,
    remarks TEXT,
    is_deleted INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)"#;

async fn fresh_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    exec(&db, DYE_BATCH_DDL).await;
    db
}

fn now_ts() -> DateTimeWithTimeZone {
    chrono::Utc::now().into()
}

/// 播种一条"验布中"(inspecting) 缸号：三列实际产出为 NULL（未完工形态）
async fn seed_inspecting_batch(
    db: &sea_orm::DatabaseConnection,
    batch_no: &str,
) -> dye_batch::Model {
    dye_batch::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        batch_no: Set(batch_no.to_string()),
        greige_fabric_id: Set(None),
        color_code: Set("C001".to_string()),
        color_name: Set("活性红3BS".to_string()),
        color_no: Set(Some("C001".to_string())),
        dye_lot_no: Set("DL-100".to_string()),
        planned_quantity: Set(Some(dec("105.00"))),
        actual_output_kg: Set(None),
        actual_output_m: Set(None),
        greige_input_kg: Set(None),
        status: Set(Some("inspecting".to_string())),
        started_at: Set(Some(now_ts())),
        completed_at: Set(None),
        remarks: Set(None),
        is_deleted: Set(Some(false)),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
    }
    .insert(db)
    .await
    .expect("播种 inspecting 缸号失败")
}

fn build_app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/production/dye-batches/{id}/complete",
            post(dye_batch_handler::complete_dye_batch),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

/// 返回 (status, json_body)；axum Json 拒绝体是纯文本，解析失败时按 Null 处理
/// （同 contract_wave2 request_json 先例），状态真相一律以回读 DB 为准
async fn call_complete(app: &Router, id: i32, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(Method::POST)
        .uri(format!("/production/dye-batches/{id}/complete"));
    let req = match body {
        Some(v) => builder
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
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

async fn reload(db: &sea_orm::DatabaseConnection, id: i32) -> dye_batch::Model {
    dye_batch::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .expect("被断言的缸号行应存在")
}

// =========================================================
// ① 带三值完工 → 三列落库真值 + 状态推进 + draft 产量分母回填
// =========================================================

#[tokio::test]
async fn complete_with_three_values_persists_actual_output_and_wires_cost_denominator() {
    let db = fresh_db().await;
    let seeded = seed_inspecting_batch(&db, "DB-C1").await;
    let read_db = db.clone();
    let app = build_app(db);

    let (status, v) = call_complete(
        &app,
        seeded.id,
        Some(json!({
            "actual_output_kg": "100.00",
            "actual_output_m": "250.00",
            "greige_input_kg": "105.00"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "合法完工应 200, 实际: {v}");
    assert_eq!(v["data"]["status"], "stored");
    // Decimal 出参序列化为字符串（rust_decimal serde 默认 human-readable str）
    assert_eq!(v["data"]["actual_output_kg"], "100.00");
    assert_eq!(v["data"]["actual_output_m"], "250.00");
    assert_eq!(v["data"]["greige_input_kg"], "105.00");

    let row = reload(&read_db, seeded.id).await;
    assert_eq!(row.status.as_deref(), Some("stored"));
    assert_eq!(row.actual_output_kg, Some(dec("100.00")), "kg 落库真值");
    assert_eq!(row.actual_output_m, Some(dec("250.00")), "米落库真值");
    assert_eq!(
        row.greige_input_kg,
        Some(dec("105.00")),
        "坯布投料量落库真值"
    );
    assert!(row.completed_at.is_some(), "完工时间戳应已写入");

    // 成本 draft 产量分母回填：整链 create() 无法 sqlite 化（pg advisory lock 取号，
    // 见文件头说明），对"行真值→draft 请求分母"的纯映射直接断言（桥接监听器喂入的
    // 正是同一次 find_by_id 读到的同一行三列）
    let req = DyeBatchCostBridgeServiceInternal::build_draft_cost_request(
        Some(row.dye_lot_no.clone()),
        row.actual_output_kg,
        row.actual_output_m,
        row.id,
        &row.batch_no,
        row.color_no.as_deref(),
    );
    assert_eq!(
        req.output_quantity_kg,
        Some(dec("100.00")),
        "draft 成本 kg 分母必须回填完工登记真值（原恒写 None）"
    );
    assert_eq!(
        req.output_quantity_meters,
        Some(dec("250.00")),
        "draft 成本米分母必须回填完工登记真值（原恒写 None）"
    );
}

// =========================================================
// ② 缺 body / 缺任一值 → 400 且状态未推进、三列零落库（回读断无痕）
// =========================================================

#[tokio::test]
async fn complete_without_body_rejected_and_state_untouched() {
    let db = fresh_db().await;
    let seeded = seed_inspecting_batch(&db, "DB-C2").await;
    let read_db = db.clone();
    let app = build_app(db);

    let (status, _body) = call_complete(&app, seeded.id, None).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺 body 必须 400, 实际: {status}"
    );

    let row = reload(&read_db, seeded.id).await;
    assert_eq!(
        row.status.as_deref(),
        Some("inspecting"),
        "被拒绝的完工不得推进状态"
    );
    assert!(
        row.completed_at.is_none(),
        "被拒绝的完工不得写 completed_at"
    );
    assert!(
        row.actual_output_kg.is_none()
            && row.actual_output_m.is_none()
            && row.greige_input_kg.is_none()
    );
}

#[tokio::test]
async fn complete_missing_one_value_rejected_and_state_untouched() {
    let db = fresh_db().await;
    let seeded = seed_inspecting_batch(&db, "DB-C3").await;
    let read_db = db.clone();
    let app = build_app(db);

    // 缺 greige_input_kg（serde 必填报错 → axum Json 拒绝 400，先于任何写库）
    let (status, _body) = call_complete(
        &app,
        seeded.id,
        Some(json!({ "actual_output_kg": "100.00", "actual_output_m": "250.00" })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "缺任一值必须 400, 实际: {status}"
    );

    let row = reload(&read_db, seeded.id).await;
    assert_eq!(row.status.as_deref(), Some("inspecting"), "状态未推进");
    assert!(row.completed_at.is_none());
    assert!(row.actual_output_kg.is_none() && row.actual_output_m.is_none());
}

// =========================================================
// ③ 0 或负值 → 400（validate_amount_range 外显文案）且状态未推进
// =========================================================

#[tokio::test]
async fn complete_with_zero_value_rejected_400_validation() {
    let db = fresh_db().await;
    let seeded = seed_inspecting_batch(&db, "DB-C4").await;
    let read_db = db.clone();
    let app = build_app(db);

    let (status, v) = call_complete(
        &app,
        seeded.id,
        Some(json!({
            "actual_output_kg": "0",
            "actual_output_m": "250.00",
            "greige_input_kg": "105.00"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "0 值必须 400, 实际: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert_ne!(
        v["message"],
        Value::Null,
        "拒绝原因必须外显（validation_displayable）"
    );

    let row = reload(&read_db, seeded.id).await;
    assert_eq!(row.status.as_deref(), Some("inspecting"), "状态未推进");
    assert!(row.actual_output_kg.is_none());
}

#[tokio::test]
async fn complete_with_negative_value_rejected_400_validation() {
    let db = fresh_db().await;
    let seeded = seed_inspecting_batch(&db, "DB-C5").await;
    let read_db = db.clone();
    let app = build_app(db);

    let (status, v) = call_complete(
        &app,
        seeded.id,
        Some(json!({
            "actual_output_kg": "100.00",
            "actual_output_m": "250.00",
            "greige_input_kg": "-1"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "负值必须 400, 实际: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR");

    let row = reload(&read_db, seeded.id).await;
    assert_eq!(row.status.as_deref(), Some("inspecting"), "状态未推进");
    assert!(row.greige_input_kg.is_none());
}

// =========================================================
// ④ 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层函数块:anchor 起,到首个 "\n}"。
/// 先剔除 `\r`:Windows 工作树 CRLF 会使跨行 contains 断言漏检(先例 wave4 测试)
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

#[test]
fn source_scan_m0062_registered_in_production_domain() {
    let mig =
        include_str!("../migration/src/domain/production/m0062_add_dye_batch_actual_output.rs");
    let mig = mig.replace('\r', "");
    for col in ["actual_output_kg", "actual_output_m", "greige_input_kg"] {
        assert!(
            mig.contains(&format!("ADD COLUMN IF NOT EXISTS \"{col}\" DECIMAL(12,2)")),
            "m0062 必须幂等新增 {col} DECIMAL(12,2): {mig}"
        );
        assert!(
            mig.contains(&format!("COMMENT ON COLUMN \"dye_batch\".\"{col}\"")),
            "{col} 必须有列注释（对齐同表 remarks 列风格）"
        );
    }

    let modrs = include_str!("../migration/src/domain/production/mod.rs").replace('\r', "");
    assert!(
        modrs.contains("mod m0062_add_dye_batch_actual_output;"),
        "m0062 必须挂载到 production 域 mod.rs"
    );
    // rustfmt 会把链式调用换行，比对前先剔除全部空白（只锁"确实调用"，不锁排版）
    let flat: String = modrs.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        flat.contains("m0062_add_dye_batch_actual_output::Migration.up(manager).await?;"),
        "up 链尾必须调用 m0062（dye_batch 由 system 域建表，production 晚于 system，顺序安全）"
    );
    assert!(
        flat.contains("m0062_add_dye_batch_actual_output::Migration.down(manager).await?;"),
        "down 逆序链首必须回滚 m0062"
    );
}

#[test]
fn source_scan_complete_handler_requires_body_validates_before_state_gate() {
    let src = include_str!("../src/handlers/dye_batch_handler.rs").replace('\r', "");
    let block = extract_block(&src, "pub async fn complete_dye_batch");
    assert!(
        block.contains("Json(req): Json<CompleteDyeBatchRequest>"),
        "complete 必须必传 CompleteDyeBatchRequest（旧零请求体形态禁止回潮）:\n{block}"
    );
    let validate_pos = block
        .find("req.validate()")
        .expect("complete 必须调用 req.validate()");
    let gate_pos = block
        .find("is_valid_status_transition(&current_status, \"stored\")")
        .expect("完工仍必须走状态机权威流转函数");
    assert!(
        validate_pos < gate_pos,
        "输入校验必须先于状态门：非法产出绝不得推进状态"
    );
    for col in ["actual_output_kg", "actual_output_m", "greige_input_kg"] {
        assert!(
            block.contains(&format!("batch.{col} = Set(Some(")),
            "{col} 必须落库完工提交值:\n{block}"
        );
    }
    // 状态门族不得被改动（另一路专家在统一状态门族，本锁只锚当前 business 族）
    assert!(
        block.contains("AppError::business(format!("),
        "非可完工状态沿用既有 business 族，不在本任务内改族"
    );
}

#[test]
fn source_scan_cost_bridge_no_longer_hardcodes_none_denominators() {
    let src = include_str!("../src/services/dye_batch_cost_bridge_service.rs").replace('\r', "");
    assert!(
        !src.contains("output_quantity_kg: None"),
        "桥接不得再把 kg 分母恒写 None（必须来自 dye_batch 完工真值）"
    );
    assert!(
        !src.contains("output_quantity_meters: None"),
        "桥接不得再把米分母恒写 None（必须来自 dye_batch 完工真值）"
    );
    assert!(
        src.contains("batch.actual_output_kg") && src.contains("batch.actual_output_m"),
        "分母来源必须是查询到的 dye_batch 行实际产出列"
    );
    assert!(
        src.contains("output_quantity_meters: output_m")
            && src.contains("output_quantity_kg: output_kg"),
        "draft 请求组装点必须使用行值透传"
    );
}

#[test]
fn source_scan_model_and_dto_carry_three_columns_as_optional_decimal() {
    let model = include_str!("../src/models/dye_batch.rs").replace('\r', "");
    for col in ["actual_output_kg", "actual_output_m", "greige_input_kg"] {
        assert!(
            model.contains(&format!("pub {col}: Option<Decimal>")),
            "实体列必须 Option<Decimal>（未完工/历史行真实 NULL）: {col}"
        );
        assert!(
            model.contains("column_type = \"Decimal(Some((12, 2)))\""),
            "实体 Decimal 列标注与 planned_quantity/迁移列精度一致"
        );
    }
    let dto = extract_block(
        &include_str!("../src/handlers/dye_batch_handler.rs").replace('\r', ""),
        "pub struct DyeBatchDto",
    );
    for col in ["actual_output_kg", "actual_output_m", "greige_input_kg"] {
        assert!(
            dto.contains(&format!("pub {col}: Option<Decimal>")),
            "列表 DTO 必须透传三列（前端按真实列名取数）: {dto}"
        );
    }
}
