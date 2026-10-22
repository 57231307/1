//! 分页上限口径锁：账龄预警规则、期末报表快照与供应商对照表三条列表链
//!
//! 契约（与全仓主流口径一致，页大小上界 100、页码上界 1000）：
//! - 账龄预警规则列表 handlers/aging_alert_rule_handler.rs：页大小与页码在入口截断，
//!   本链页码基数为 0，下界保持 0（取数偏移由页码乘页大小得出，抬到 1 会跳过首页）。
//! - 期末报表快照列表 handlers/period_report_snapshot_handler.rs：同口径，
//!   本链页码基数为 1，下界取 1（取数偏移由页码减一再乘页大小得出）。
//! - 供应商对照表列表 handlers/sku_mapping_handler.rs：取数侧上限早已存在于
//!   services/sku_mapping_service.rs，本锁钉回显必须等于真实生效值，
//!   不得回显未截断的入参（回显虚高会让分页条与真实返回行数对不上）。
//!
//! 为什么断回显而不是只断源码形态：三条链的出参都是标准分页信封 PaginatedResponse，
//! 其 page 与 page_size 由入口变量直接组装，所以回显值就是生效值；
//! 传超大入参后回显仍等于原值即为缺陷复现。
//!
//! 覆盖策略：真 PostgreSQL（环境变量 TEST_DATABASE_URL）经真实 handler 走 HTTP 断言，
//! 与 tests/contract_wave1_ar_list_paginated_shape_test.rs 同一范式。
//! 前两条链各播种 105 行，用于同时锁住两件事：回显被截断，以及取数真的只取 100 行。

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
use bingxi_backend::handlers::{
    aging_alert_rule_handler, period_report_snapshot_handler, sku_mapping_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{aging_alert_rule, period_report_snapshot};
use chrono::Utc;
use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

/// 播种行数：必须大于页大小上界 100，才能让未截断与已截断产生不同的返回行数
const SEED_ROWS: u64 = 105;

/// 全仓主流分页上界（页码上限与页大小上限）
const PAGE_UPPER: u64 = 1000;
const PAGE_SIZE_UPPER: u64 = 100;

/// 构造测试 AuthContext（与既有 handler 级 HTTP 用例同字段）
fn make_auth(user_id: i32, username: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: username.to_string(),
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

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 由已迁移库构造 AppState（其余字段取默认，列表读路径不触碰）
fn state_of(db: DatabaseConnection) -> AppState {
    AppState {
        db: Arc::new(db),
        ..Default::default()
    }
}

/// 账龄预警规则：播种 n 行（该表无外键与唯一约束，可直插）
async fn seed_aging_rules(db: &DatabaseConnection, n: u64) {
    let models: Vec<aging_alert_rule::ActiveModel> = (1..=n)
        .map(|i| aging_alert_rule::ActiveModel {
            rule_name: Set(format!("分页上限锁规则{i:04}")),
            rule_code: Set(format!("PGSZ-{i:04}")),
            aging_bucket: Set("30_days".to_string()),
            threshold_days: Set(30),
            alert_level: Set("warning".to_string()),
            notify_method: Set("system".to_string()),
            is_active: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        })
        .collect();
    aging_alert_rule::Entity::insert_many(models)
        .exec(db)
        .await
        .unwrap();
}

/// 期末报表快照：播种 n 行（该表 DDL 无外键与唯一约束，period_id 取任意值可直插）
async fn seed_snapshots(db: &DatabaseConnection, n: u64) {
    let models: Vec<period_report_snapshot::ActiveModel> = (1..=n)
        .map(|i| period_report_snapshot::ActiveModel {
            period_id: Set(1),
            report_type: Set("balance_sheet".to_string()),
            report_data: Set(serde_json::json!({ "rows": [] })),
            snapshot_hash: Set(format!("hash-{i:04}")),
            created_by: Set(100),
            created_at: Set(Utc::now()),
            ..Default::default()
        })
        .collect();
    period_report_snapshot::Entity::insert_many(models)
        .exec(db)
        .await
        .unwrap();
}

// ===========================================================================
// 链一：账龄预警规则列表（页码基数 0）
// ===========================================================================

/// 缺省口径不变 + 超大 page_size 截到 100 + 超大 page 截到 1000 且不溢出报错
#[tokio::test]
async fn aging_alert_rules_list_page_size_is_bounded() {
    let db = test_common::setup_test_db().await;
    seed_aging_rules(&db, SEED_ROWS).await;
    let app = Router::new()
        .route(
            "/aging-alert-rules",
            get(aging_alert_rule_handler::list_rules),
        )
        .with_state(state_of(db))
        .layer(from_fn_with_state(
            make_auth(100, "e2e_accountant_100"),
            inject_auth,
        ));

    // 缺省：页大小 20、页码 0（本链基数为 0，缺省即首页，下界不得被抬成 1）
    let (status, v) = get_json(&app, "/aging-alert-rules").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["data"]["page"].as_u64(),
        Some(0),
        "账龄链页码基数为 0，缺省请求应仍回显 0 并返回首页"
    );
    assert_eq!(
        v["data"]["page_size"].as_u64(),
        Some(20),
        "缺省页大小仍为 20，上限加固不得改动缺省值"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 20);
    assert_eq!(v["data"]["total"].as_u64(), Some(SEED_ROWS));

    // 超大页大小：回显与取数都必须等于上界 100
    let (status, v) = get_json(&app, "/aging-alert-rules?page_size=5000").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["data"]["page_size"].as_u64(),
        Some(PAGE_SIZE_UPPER),
        "传 page_size=5000 时实际生效值必须被截到 100"
    );
    assert_eq!(
        v["data"]["items"].as_array().unwrap().len(),
        PAGE_SIZE_UPPER as usize,
        "取数侧必须真的只返回 100 行，否则回显与行数不一致仍是假绿"
    );

    // 超大页码：截到上界、返回 200 空窗，不得因偏移乘法溢出而 500
    let (status, v) = get_json(&app, "/aging-alert-rules?page=99999&page_size=5000").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "超大页码不得让偏移计算溢出成 500，实际={status}"
    );
    assert_eq!(
        v["data"]["page"].as_u64(),
        Some(PAGE_UPPER),
        "传 page=99999 时实际生效值必须被截到 1000"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 0);
}

// ===========================================================================
// 链二：期末报表快照列表（页码基数 1）
// ===========================================================================

/// 缺省口径不变 + 超大 page_size 截到 100 + 超大 page 截到 1000 且不溢出报错
#[tokio::test]
async fn period_report_snapshots_list_page_size_is_bounded() {
    let db = test_common::setup_test_db().await;
    seed_snapshots(&db, SEED_ROWS).await;
    let app = Router::new()
        .route(
            "/period-report-snapshots",
            get(period_report_snapshot_handler::list_snapshots),
        )
        .with_state(state_of(db))
        .layer(from_fn_with_state(
            make_auth(100, "e2e_accountant_100"),
            inject_auth,
        ));

    // 缺省：页大小 20、页码为 1 基首页
    let (status, v) = get_json(&app, "/period-report-snapshots").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["data"]["page"].as_u64(),
        Some(1),
        "快照链页码基数为 1，缺省请求应回显 1 并返回首页"
    );
    assert_eq!(
        v["data"]["page_size"].as_u64(),
        Some(20),
        "缺省页大小仍为 20，上限加固不得改动缺省值"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 20);
    assert_eq!(v["data"]["total"].as_u64(), Some(SEED_ROWS));

    // 超大页大小：回显与取数都必须等于上界 100
    let (status, v) = get_json(&app, "/period-report-snapshots?page=1&page_size=5000").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["data"]["page_size"].as_u64(),
        Some(PAGE_SIZE_UPPER),
        "传 page_size=5000 时实际生效值必须被截到 100"
    );
    assert_eq!(
        v["data"]["items"].as_array().unwrap().len(),
        PAGE_SIZE_UPPER as usize,
        "取数侧必须真的只返回 100 行，否则回显与行数不一致仍是假绿"
    );

    // 显式传 page=0：1 基链的下界是 1，取数仍落首页而非报错或回绕
    let (status, v) = get_json(&app, "/period-report-snapshots?page=0&page_size=10").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["page"].as_u64(), Some(1));
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 10);

    // 超大页码：截到上界、返回 200 空窗，不得因偏移乘法溢出而 500
    let (status, v) = get_json(&app, "/period-report-snapshots?page=99999&page_size=5000").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "超大页码不得让偏移计算溢出成 500，实际={status}"
    );
    assert_eq!(
        v["data"]["page"].as_u64(),
        Some(PAGE_UPPER),
        "传 page=99999 时实际生效值必须被截到 1000"
    );
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 0);
}

// ===========================================================================
// 链三：供应商对照表列表（取数上限既存，本锁钉回显等于生效值）
// ===========================================================================

/// 回显必须等于截断后的真实生效值：入参未经截断就回显会让条数虚高于实际返回行数。
/// 取数侧上限既存且行为不变，故空表即可判定：入参 99999 与 5000 必须回显成 1000 与 100。
#[tokio::test]
async fn sku_mappings_list_echoes_bounded_page_values() {
    let db = test_common::setup_test_db().await;
    let app = Router::new()
        .route(
            "/purchase/sku-mappings",
            get(sku_mapping_handler::list_mappings),
        )
        .with_state(state_of(db))
        .layer(from_fn_with_state(
            make_auth(100, "e2e_purchase_clerk_100"),
            inject_auth,
        ));

    let (status, v) = get_json(&app, "/purchase/sku-mappings?page=99999&page_size=5000").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        v["data"]["page_size"].as_u64(),
        Some(PAGE_SIZE_UPPER),
        "回显必须等于取数侧的生效上限，不得原样回显 5000"
    );
    assert_eq!(
        v["data"]["page"].as_u64(),
        Some(PAGE_UPPER),
        "回显必须等于取数侧的生效上限，不得原样回显 99999"
    );

    // 缺省口径不变：页码 1、页大小 20（与 service 内同一组默认值）
    let (status, v) = get_json(&app, "/purchase/sku-mappings").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["data"]["page"].as_u64(), Some(1));
    assert_eq!(v["data"]["page_size"].as_u64(), Some(20));
}
