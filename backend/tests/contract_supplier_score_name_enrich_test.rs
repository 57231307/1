//! 供应商评分/排名出参 supplier_name 契约锁（GET /supplier-evaluations/rankings、
//! GET /supplier-evaluations/suppliers/:id/score）
//!
//! 一、锁定契约（唯一真相 = 源码）：
//! - backend/src/services/supplier_evaluation_service.rs::SupplierScoreResponse 含
//!   pub supplier_name: Option<String>，无 serde rename ⇒ 出参键名逐字 supplier_name；
//!   两个出口（rankings / score）共用该结构，handlers/supplier_evaluation_handler.rs
//!   经 Json(ApiResponse::success(...)) 直接序列化。
//! - average_score 为 Decimal（rust_decimal 仅开 serde 特性 → JSON 字符串），
//!   与 EvaluationRecord.score 同形态；前端 supplier-evaluation.ts::SupplierScore
//!   声明 average_score: string、supplier_name: string | null（键恒存在）。
//!
//! 二、本锁拦截的缺陷形态：排名出参**没有名称键**（前端排名表列绑
//! prop="supplier_name" 恒空）、名称声明成可选幽灵键、average_score 声明成 number
//! （掩盖真实线形，前端拿不到值也不报错）。
//!
//! 三、正确实现口径（本测试逐条钉住）：
//! 1. rankings 出口先排序/截断、再对留榜 supplier_id 做一次 is_in 批量查
//!    suppliers 回填（fetch_supplier_names），严禁逐行再查（N+1）；
//! 2. 名称值必须逐字来自 suppliers.supplier_name（真库真值回显），外键行缺失时
//!    如实 null——禁止编码/id 拼装（format!("供应商 #{}", id) 一类）；
//! 3. supplier_name 为 None 时序列化仍出键（值为 JSON null），前端按 null 渲染。
//!
//! 四、夹具口径（路线一）：test_common::setup_test_db() 打已迁移 PostgreSQL，
//! 缺 TEST_DATABASE_URL 直接 panic ⇒ 本文件属 CI 侧执行（--test-threads=1 串行）；
//! supplier_evaluation_records 对 suppliers / supplier_evaluation_indicators 均有
//! 真实 FK（建表与约束见 migration 的 domain/v15/mod.rs），用例自插合法父行。
//!
//! 五、覆盖策略：service 层真库两出口 + HTTP 信封层 + 无 DB 的键必出序列化锁 +
//! 源码扫描防回潮锁（回填时机、is_in 单次批量、禁拼装），全部真实行为、无 mock。
//!
//! 六、防回潮判据：若名称键再次从出参消失、被改成 skip_serializing_if、或回填被
//! 挪到截断之前/退化为逐行查询，对应断言即红。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::supplier_evaluation_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{supplier, supplier_evaluation_indicator, supplier_evaluation_record};
use bingxi_backend::services::supplier_evaluation_service::{
    SupplierEvaluationService, SupplierScoreResponse,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 出参键集 = SupplierScoreResponse 字段全集（顺序无关，集合必须恰好相等——
/// 多一个"预留键"、少一个 supplier_name 都算破坏契约）
const EXPECTED_KEYS: [&str; 6] = [
    "average_score",
    "latest_evaluation_date",
    "rating",
    "supplier_id",
    "supplier_name",
    "total_records",
];

const SUP_A_NAME: &str = "绍兴华源纺织有限公司";
const SUP_B_NAME: &str = "滨海盛达印染有限公司";

async fn live_db() -> sea_orm::DatabaseConnection {
    setup_test_db().await
}

/// suppliers 种子（NOT NULL 列清单对齐 models/supplier.rs，同 wave11 供应商夹具先例）
async fn seed_supplier(db: &sea_orm::DatabaseConnection, name: &str, tag: &str) -> i32 {
    let now = Utc::now().fixed_offset();
    let model = supplier::ActiveModel {
        supplier_code: Set(format!("SUPNAME-{tag}")),
        supplier_name: Set(name.to_string()),
        supplier_short_name: Set(format!("简{tag}")),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(format!("91330000SN{tag}0001X")),
        registered_address: Set("契约测试注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("契约测试银行".to_string()),
        bank_account: Set(format!("622200000000{tag}")),
        contact_phone: Set("13800000000".to_string()),
        is_processor: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    model
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("seed 供应商失败: {e}"))
        .id
}

/// 指标种子：weight=1.00、max_score=100 ⇒ weighted=score/100、average=score 原值
async fn seed_indicator(db: &sea_orm::DatabaseConnection, tag: &str) -> i32 {
    let now = Utc::now();
    let model = supplier_evaluation_indicator::ActiveModel {
        indicator_name: Set(format!("交付及时率{tag}")),
        indicator_code: Set(format!("EI-SN-{tag}")),
        category: Set("delivery".to_string()),
        weight: Set(Decimal::ONE),
        max_score: Set(100),
        status: Set(master_data::ACTIVE.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    model
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("seed 指标失败: {e}"))
        .id
}

/// 评估记录种子（supplier/indicator 父行必须先于本行存在——真库 FK 实证）
async fn seed_record(
    db: &sea_orm::DatabaseConnection,
    supplier_id: i32,
    indicator_id: i32,
    score: i64,
) {
    // 与 create_evaluation_record 同式：weighted = score * weight / max_score，
    // 夹具 weight=1.00、max_score=100 ⇒ average_score 还原为 score 原值
    let score = Decimal::from(score);
    let weighted = score / Decimal::from(100);
    let model = supplier_evaluation_record::ActiveModel {
        supplier_id: Set(supplier_id),
        evaluation_period: Set("2026-Q1".to_string()),
        indicator_id: Set(indicator_id),
        score: Set(score),
        max_score: Set(Some(100)),
        weighted_score: Set(Some(weighted)),
        evaluation_date: Set(Some(
            NaiveDate::from_ymd_opt(2026, 1, 15).expect("夹具日期常量"),
        )),
        created_at: Set(Utc::now()),
        ..Default::default()
    };
    model
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("seed 评估记录失败: {e}"));
}

/// 出参单行形状锁：键集恰好相等 + supplier_name 键必在 + average_score 为 JSON 字符串
fn assert_score_exit_shape(row: &Value) {
    let obj = row
        .as_object()
        .unwrap_or_else(|| panic!("评分/排名出参必须是 JSON 对象，实际: {row}"));
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys, EXPECTED_KEYS,
        "出参键集必须与 SupplierScoreResponse 字段集恰好相等，实际: {row}"
    );
    assert!(
        obj["supplier_name"].is_string() || obj["supplier_name"].is_null(),
        "supplier_name 只允许真实名称或 null（禁止数字/对象等冒充形态），实际: {row}"
    );
    assert!(
        obj["average_score"].is_string(),
        "average_score 是 Decimal 的 JSON 字符串线形（与 EvaluationRecord.score 同形态），\
         不得漂移成 JSON number，实际: {:?}",
        obj["average_score"]
    );
    assert!(obj["supplier_id"].is_number() && obj["total_records"].is_number());
}

// =========================================================
// 1) service 层真库出参（rankings / score 两个出口）
// =========================================================

/// 排名榜逐行锁：supplier_name 逐行回显"本行"供应商在 suppliers 表的真名，
/// 排序（分数降序）在回填后保持；键集与线形同锁。
#[tokio::test]
async fn rankings_rows_carry_real_supplier_names() {
    let db = live_db().await;
    let sup_a = seed_supplier(&db, SUP_A_NAME, "RA").await;
    let sup_b = seed_supplier(&db, SUP_B_NAME, "RB").await;
    let indicator = seed_indicator(&db, "R").await;
    seed_record(&db, sup_a, indicator, 80).await;
    seed_record(&db, sup_b, indicator, 90).await;

    let svc = SupplierEvaluationService::new(Arc::new(db));
    let rankings = svc
        .get_supplier_rankings(10)
        .await
        .expect("rankings 查询失败");
    assert_eq!(rankings.len(), 2, "两个供应商各一条记录应双双上榜");

    let value = serde_json::to_value(&rankings).expect("SupplierScoreResponse 必须可序列化");
    let rows = value.as_array().expect("rankings 出参应为数组");
    for row in rows {
        assert_score_exit_shape(row);
    }
    // 90 分者在前：名称必须与**本行** supplier_id 对应的真实行一致（证明逐行取自
    // suppliers，而非缓存、拼装或 id 冒充）
    assert_eq!(rows[0]["supplier_id"], json!(sup_b));
    assert_eq!(rows[0]["supplier_name"], json!(SUP_B_NAME));
    assert_eq!(rows[1]["supplier_id"], json!(sup_a));
    assert_eq!(rows[1]["supplier_name"], json!(SUP_A_NAME));
}

/// 单供应商评分出口：与排名榜同口径回填真实名称，键集不缺 supplier_name
#[tokio::test]
async fn score_exit_carries_real_supplier_name() {
    let db = live_db().await;
    let sup = seed_supplier(&db, SUP_A_NAME, "SA").await;
    let indicator = seed_indicator(&db, "S").await;
    seed_record(&db, sup, indicator, 75).await;

    let svc = SupplierEvaluationService::new(Arc::new(db));
    let score = svc.get_supplier_score(sup).await.expect("score 查询失败");
    let value = serde_json::to_value(&score).expect("SupplierScoreResponse 必须可序列化");
    assert_score_exit_shape(&value);
    assert_eq!(value["supplier_name"], json!(SUP_A_NAME));
    assert_eq!(value["supplier_id"], json!(sup));
}

// =========================================================
// 2) HTTP 层信封与 data 形状（真实 handler + 真实 PostgreSQL）
// =========================================================

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 9,
        username: "supname_shape_fixture_user".to_string(),
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

fn build_app(state: AppState) -> Router {
    Router::new()
        .route(
            "/supplier-evaluations/rankings",
            get(supplier_evaluation_handler::get_rankings),
        )
        .route(
            "/supplier-evaluations/suppliers/{supplier_id}/score",
            get(supplier_evaluation_handler::get_supplier_score_by_path),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
}

async fn get_json(app: Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
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

/// GET /rankings → 200 + 成功信封 code=200 + data 逐行含真实 supplier_name
#[tokio::test]
async fn http_rankings_envelope_carries_supplier_name() {
    let db = live_db().await;
    let sup = seed_supplier(&db, SUP_B_NAME, "HR").await;
    let indicator = seed_indicator(&db, "HR").await;
    seed_record(&db, sup, indicator, 88).await;

    let app = build_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });
    let (status, v) = get_json(app, "/supplier-evaluations/rankings?limit=10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["code"], 200,
        "成功信封 code 契约（utils/response.rs::ApiResponse）"
    );
    let rows = v["data"].as_array().expect("data 应为数组").clone();
    assert!(!rows.is_empty());
    for row in &rows {
        assert_score_exit_shape(row);
    }
    assert_eq!(rows[0]["supplier_name"], json!(SUP_B_NAME));
}

/// GET /suppliers/:id/score → 同锁：信封 + 键集 + 真实名称
#[tokio::test]
async fn http_score_envelope_carries_supplier_name() {
    let db = live_db().await;
    let sup = seed_supplier(&db, SUP_A_NAME, "HS").await;
    let indicator = seed_indicator(&db, "HS").await;
    seed_record(&db, sup, indicator, 70).await;

    let app = build_app(AppState {
        db: Arc::new(db.clone()),
        ..Default::default()
    });
    let (status, v) = get_json(app, &format!("/supplier-evaluations/suppliers/{sup}/score")).await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");
    assert_eq!(v["code"], 200);
    assert_score_exit_shape(&v["data"]);
    assert_eq!(v["data"]["supplier_name"], json!(SUP_A_NAME));
}

// =========================================================
// 3) 序列化键必出锁（无 DB）：supplier_name=None 仍出键、值为 null
// =========================================================

#[test]
fn serializes_supplier_name_key_even_when_none() {
    let response = SupplierScoreResponse {
        supplier_id: 7,
        supplier_name: None,
        average_score: Decimal::ZERO,
        total_records: 1,
        rating: "D".to_string(),
        latest_evaluation_date: None,
    };
    let value = serde_json::to_value(&response).expect("SupplierScoreResponse 必须可序列化");
    let obj = value.as_object().expect("出参必须是 JSON 对象");
    assert!(
        obj.contains_key("supplier_name"),
        "supplier_name 为 None 时序列化仍须出键（前端按恒存在键渲染），实际键集: {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj["supplier_name"].is_null(),
        "None 必须序列化为 JSON null"
    );
}

// =========================================================
// 4) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 只保留"代码"：整行注释（//、///、//!）逐行剔除——说明禁词形态的注释文本自身
/// 命中禁词属假判违例（先例 contract_wave3_after_sales_customer_name_test.rs）
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// impl 成员函数窗：anchor 起，到首个 \n    } 收口（4 空格收口防吞并下一函数）
fn extract_impl_fn(src: &str, anchor: &str) -> String {
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("函数结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// rankings 出口锁：回填必须发生在排序/截断**之后**，且整段只有一次批量查询
#[test]
fn source_scan_rankings_backfills_after_truncate_in_single_query() {
    let clean = code_only(include_str!(
        "../src/services/supplier_evaluation_service.rs"
    ));
    let block = extract_impl_fn(&clean, "pub async fn get_supplier_rankings");
    let truncate_pos = block
        .find(".truncate(")
        .expect("rankings 出口必须保留按 limit 截断");
    let fetch_pos = block
        .find("fetch_supplier_names")
        .expect("rankings 出口必须经 fetch_supplier_names 批量回填名称");
    assert!(
        truncate_pos < fetch_pos,
        "名称回填必须在排序/截断之后（只对留榜 id 查询），实际块:\n{block}"
    );
    assert!(
        block.matches("fetch_supplier_names").count() == 1,
        "回填必须是一次批量查询，禁止出现第二次查询调用（逐行富化=N+1 回潮），实际块:\n{block}"
    );
    // 出口窗内禁止逐行主键再查
    assert!(
        !block.contains("find_by_id("),
        "rankings 出口禁止逐行 find_by_id 查名称（N+1），实际块:\n{block}"
    );
}

/// 批量查询实现锁：is_in 单次查询 suppliers、取 supplier_name 真列
#[test]
fn source_scan_fetch_supplier_names_uses_single_is_in_query() {
    let clean = code_only(include_str!(
        "../src/services/supplier_evaluation_service.rs"
    ));
    let block = extract_impl_fn(&clean, "async fn fetch_supplier_names");
    let block_flat: String = block.chars().filter(|c| !c.is_whitespace()).collect();
    for marker in [
        "supplier::Entity::find()",
        "supplier::Column::Id.is_in(",
        "(s.id,s.supplier_name)",
    ] {
        assert!(
            block_flat.contains(marker),
            "fetch_supplier_names 缺批量查询锚点 {marker}，实际块:\n{block}"
        );
    }
}

/// 拼装假名全面禁令：service 内 supplier_name 的取值只允许来自查询回填映射，
/// 不得出现任何 format! 拼装或按 id 造名
#[test]
fn source_scan_no_fabricated_supplier_name() {
    let clean = code_only(include_str!(
        "../src/services/supplier_evaluation_service.rs"
    ));
    assert!(
        !clean.contains("supplier_name: Some(format!"),
        "supplier_name 禁止 Some(format!(..)) 拼装（编码/id 冒充名称）"
    );
    assert!(
        !clean.contains("供应商 #") && !clean.contains("Supplier #"),
        "禁止以 id 拼装的展示名进入出参链路"
    );
}

/// 前端契约锁：SupplierScore 必须声明必出的 supplier_name 与字符串线形的 average_score
/// （先例：wave3 测试对 custom-order.ts 的接口块扫描）
#[test]
fn source_scan_frontend_interface_matches_wire() {
    let api_src = code_only(include_str!(
        "../../frontend/src/api/supplier-evaluation.ts"
    ));
    let i = api_src
        .find("export interface SupplierScore")
        .expect("SupplierScore 锚点丢失");
    let j = api_src[i..]
        .find("\n}")
        .expect("SupplierScore 接口块收口丢失");
    let iface = &api_src[i..i + j + 2];
    assert!(
        iface.contains("supplier_name: string | null;"),
        "supplier_name 必须是必出键（null=外键行缺失），不得回退成可选幽灵键 ?，实际块:\n{iface}"
    );
    assert!(
        iface.contains("average_score: string;"),
        "average_score 线形为 Decimal JSON 字符串，不得声明成 number，实际块:\n{iface}"
    );
}
