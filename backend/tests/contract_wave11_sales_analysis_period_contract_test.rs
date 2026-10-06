//! 销售趋势端点 period 契约锁（`GET /api/v1/erp/crm/sales-analysis/trends`，别名 `/trend`）
//!
//! ## 锁定的真实契约（唯一真相 = 源码）
//! - `backend/src/handlers/sales_analysis_handler.rs` 的 `TrendQuery.period` 为
//!   `Option<String>`（与同域 `SalesStatisticQuery.period` / `RankingQuery.period` 同源）；
//! - `backend/src/services/sales_analysis_service.rs::get_trends(Option<&str>)`：
//!   `None` ⇒ **不拼 period 过滤条件**（返回全周期数据），`Some` ⇒ `Period.eq(...)` 等值过滤，
//!   形态与同文件 `get_statistics_list` / `get_rankings` 的 `if let Some` 完全同构；
//! - 空串 `?period=` 由应用边界中间件 `normalize_empty_query_params` 剔除 ⇒ 与「键缺失」
//!   同义（不过滤），**绝不退化成 `WHERE period = ''` 的恒 0 行**。本测试自建路由时显式挂上
//!   同一个中间件，使「缺省不过滤」与「空串不过滤」两条都在真实链路形态下被验证。
//!
//! ## 为什么这三条需要被钉住
//! period 曾为必填 `String`，缺省即 axum `Query` 拒绝 400；放宽为 Option 后，若有人用
//! `unwrap_or_default()` 之类兜底「补一个空串再过滤」，端点仍返回 200 但**恒空数组**——
//! 这是本仓明令禁止的静默假绿形态，只有「真库 + 真实两期数据 + 断言行数与合计值」才抓得住。
//!
//! ## 值域校验为何不在本锁内
//! `sales_statistics.period` 的真实取值域在本仓没有唯一权威定义（证据清单见交付报告），
//! 「非法值应否 400」属待裁项，故本文件**不断言任何非法 period 的状态码或机器码**，
//! 以免把未裁词表固化成第二套手写常量。
//!
//! ## 夹具口径（真库自证）
//! `test_common::setup_test_db()` 打已迁移 PostgreSQL 并 TRUNCATE 业务表
//! （`sales_statistics` 属业务表、非种子参照表 ⇒ 每次从空表起步）。为防「夹具把迁移台账一起
//! 清掉 ⇒ 真库断言空转」的历史事故形态，用例显式自证两件事：迁移台账 `seaql_migrations`
//! 记账行数 > 0，且自插 seed 行数经 Entity 直读确为预期值。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_analysis_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::sales_analysis;
use bingxi_backend::services::sales_analysis_service::SalesAnalysisService;
use bingxi_backend::utils::query_params::normalize_empty_query_params;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tower::ServiceExt;

/// seed 行主键与对应金额（合计 3500；PERIOD_B 两行合计 3000；PERIOD_A 单行 500）
const ROW_B1: (i32, i64) = (11, 1_000);
const ROW_B2: (i32, i64) = (12, 2_000);
const ROW_A1: (i32, i64) = (13, 500);

/// seed 用周期：两个互不相同的「期」，用于证明缺省返回确实是跨期全量
const PERIOD_A: &str = "2025-Q4";
const PERIOD_B: &str = "2026-Q1";

/// 自插三行：PERIOD_B 两行、PERIOD_A 一行，金额互不相同
/// ⇒ 一旦「缺省」被实现成按某个默认值过滤，行数与合计都会当场对不上。
async fn seed_three_rows(db: &sea_orm::DatabaseConnection) {
    let created_at = Utc.with_ymd_and_hms(2026, 1, 15, 8, 0, 0).unwrap();
    let rows: [(i32, i64, &str, &str); 3] = [
        (ROW_B1.0, ROW_B1.1, PERIOD_B, "order"),
        (ROW_B2.0, ROW_B2.1, PERIOD_B, "product"),
        (ROW_A1.0, ROW_A1.1, PERIOD_A, "order"),
    ];
    for (id, raw_amount, period, statistic_type) in rows {
        let amount = Decimal::from(raw_amount);
        let active = sales_analysis::ActiveModel {
            id: Set(id),
            statistic_type: Set(statistic_type.to_string()),
            period: Set(period.to_string()),
            dimension_type: Set("product".to_string()),
            dimension_id: Set(Some(id)),
            dimension_name: Set(Some(format!("趋势夹具维度-{id}"))),
            order_count: Set(1),
            total_amount: Set(amount),
            total_qty: Set(amount),
            total_cost: Set(Decimal::ZERO),
            gross_profit: Set(Decimal::ZERO),
            gross_profit_rate: Set(Decimal::ZERO),
            avg_order_value: Set(amount),
            created_at: Set(created_at),
        };
        active
            .insert(db)
            .await
            .unwrap_or_else(|e| panic!("夹具：自插 sales_statistics 行 id={id} 失败: {e}"));
    }
}

/// 真库自证 1：迁移台账仍可读且记账行数 > 0（台账被误清 ⇒ 一切「已迁移」前提空转，必须点名）
async fn assert_migration_ledger_nonempty(db: &sea_orm::DatabaseConnection) {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT COUNT(*) AS cnt FROM \"seaql_migrations\"".to_string(),
        ))
        .await
        .expect("真库自证：迁移台账 seaql_migrations 必须可读（夹具连的不是已迁移 PostgreSQL？）");
    let count: i64 = row
        .and_then(|r| r.try_get_by_index::<i64>(0).ok())
        .expect("真库自证：迁移台账记账行数解码失败");
    assert!(
        count > 0,
        "真库自证：迁移台账行数为 0——本用例的库不是经迁移记账建起来的，真库断言会在空库上空转"
    );
}

/// 真库自证 2：直读 Entity 确认 seed 行数确为预期（HTTP 断言之前的前置自证）
async fn assert_seeded(db: &sea_orm::DatabaseConnection) {
    let total = sales_analysis::Entity::find()
        .count(db)
        .await
        .expect("夹具：直读 sales_statistics 行数失败");
    assert_eq!(
        total, 3,
        "seed 必须是 3 行（两期），否则后面的行数断言无意义"
    );
    let period_b = sales_analysis::Entity::find()
        .filter(sales_analysis::Column::Period.eq(PERIOD_B))
        .count(db)
        .await
        .expect("夹具：按 period 直读失败");
    assert_eq!(period_b, 2, "PERIOD_B 必须是 2 行，用于区分全量与单期过滤");
}

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 7,
        username: "sales_trend_period_fixture_user".to_string(),
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

/// 只挂本端点的两条真实挂载路径，并复刻应用边界的 query 归一化层
/// （注册在后 = 更外层，故 `normalize_empty_query_params` 先于鉴权与 handler 执行，与生产一致）
fn build_app(state: AppState) -> Router {
    Router::new()
        .route(
            "/sales-analysis/trends",
            get(sales_analysis_handler::get_trends),
        )
        .route(
            "/sales-analysis/trend",
            get(sales_analysis_handler::get_trends),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
        .layer(from_fn(normalize_empty_query_params))
}

/// 发一次 GET，返回（状态码, 成功信封 data 反序列化后的行）
async fn get_rows(
    db: sea_orm::DatabaseConnection,
    uri: &str,
) -> (StatusCode, Vec<sales_analysis::Model>) {
    let app = build_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        panic!(
            "响应体必须是 JSON 信封，uri={uri} 解析失败: {e}，实际: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    if status == StatusCode::OK {
        assert_eq!(
            v["code"], 200,
            "成功信封 code 契约（utils/response.rs::ApiResponse），uri={uri}"
        );
        let rows: Vec<sales_analysis::Model> = serde_json::from_value(v["data"].clone())
            .unwrap_or_else(|e| panic!("data 必须是 sales_analysis::Model 数组: {e}"));
        (status, rows)
    } else {
        (status, Vec::new())
    }
}

// =========================================================
// 1) 缺省（不带 period）= 全周期，真跨两期，不是空数组蒙过
// =========================================================

#[tokio::test]
async fn trends_without_period_returns_all_periods_from_real_db() {
    let db = test_common::setup_test_db().await;
    assert_migration_ledger_nonempty(&db).await;
    seed_three_rows(&db).await;
    assert_seeded(&db).await;

    let (status, rows) = get_rows(db, "/sales-analysis/trends").await;

    assert_eq!(
        status,
        StatusCode::OK,
        "period 已放宽为 Option：缺省请求必须 200（不得回到 Query 拒绝 400）"
    );
    assert_eq!(
        rows.len(),
        3,
        "缺省必须返回全部 3 行；返回 0 行说明缺省被实现成按默认值过滤（兜底掩盖）"
    );
    let periods: Vec<&str> = rows.iter().map(|r| r.period.as_str()).collect();
    let distinct: HashSet<&str> = periods.iter().copied().collect();
    assert!(
        distinct.len() >= 2,
        "缺省返回的行必须真的跨两期以上，实际 period 列: {periods:?}"
    );
    let sum: Decimal = rows.iter().map(|r| r.total_amount).sum();
    assert_eq!(
        sum,
        Decimal::from(3_500),
        "缺省返回的必须是两期全部行（1000+2000+500），实际合计: {sum}"
    );
}

// =========================================================
// 2) 空串 period 与键缺失同义（经真实边界中间件），同样不过滤
// =========================================================

#[tokio::test]
async fn trends_with_empty_period_is_treated_as_absent() {
    let db = test_common::setup_test_db().await;
    assert_migration_ledger_nonempty(&db).await;
    seed_three_rows(&db).await;
    assert_seeded(&db).await;

    let (status, rows) = get_rows(db, "/sales-analysis/trends?period=").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        rows.len(),
        3,
        "`?period=` 被边界中间件剔除后等同缺省；若返回 0 行说明生成了 WHERE period = ''"
    );
}

// =========================================================
// 3) 带合法（有写入侧证据的）period 时结果被过滤到该期；别名同语义
// =========================================================

#[tokio::test]
async fn trends_with_period_filters_to_that_period_only() {
    let db = test_common::setup_test_db().await;
    assert_migration_ledger_nonempty(&db).await;
    seed_three_rows(&db).await;
    assert_seeded(&db).await;

    let uri = format!("/sales-analysis/trends?period={PERIOD_B}");
    let (status, rows) = get_rows(db, &uri).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(rows.len(), 2, "PERIOD_B 只有 2 行，过滤必须真的生效");
    for row in &rows {
        assert_eq!(
            row.period, PERIOD_B,
            "带 period 请求不得混入其他周期的行（等值过滤回归）"
        );
    }
    let sum: Decimal = rows.iter().map(|r| r.total_amount).sum();
    assert_eq!(sum, Decimal::from(3_000), "过滤后的合计只应含该期两行");
}

/// 别名 `/trend` 与 `/trends` 挂同一 handler（routes/crm.rs 两处挂载），
/// 缺省语义必须同样成立——防止只按一条路径改、另一条继续撞旧契约。
#[tokio::test]
async fn trend_alias_shares_the_relaxed_default() {
    let db = test_common::setup_test_db().await;
    assert_migration_ledger_nonempty(&db).await;
    seed_three_rows(&db).await;
    assert_seeded(&db).await;

    let (status, rows) = get_rows(db, "/sales-analysis/trend").await;

    assert_eq!(status, StatusCode::OK, "别名缺省请求同样必须 200");
    assert_eq!(
        rows.len(),
        3,
        "别名 /trend 的缺省语义必须与 /trends 完全一致（同一 handler）"
    );
}

/// service 层新签名直测：`None` 不过滤、`Some` 等值过滤
/// （若有人把参数退回 `&str`，本用例第一行就编译不过）
#[tokio::test]
async fn service_get_trends_accepts_none_and_some() {
    let db = test_common::setup_test_db().await;
    seed_three_rows(&db).await;

    let svc = SalesAnalysisService::new(Arc::new(db));

    let all = svc
        .get_trends(None)
        .await
        .expect("缺省（None）不得报错，且必须是不过滤语义");
    assert_eq!(all.len(), 3, "None ⇒ 全周期 3 行");

    let one = svc
        .get_trends(Some(PERIOD_A))
        .await
        .expect("提供 period 时不得报错");
    assert_eq!(one.len(), 1, "Some(PERIOD_A) ⇒ 该期 1 行");
    assert_eq!(
        one[0].total_amount,
        Decimal::from(ROW_A1.1),
        "单期过滤返回的必须是该期那一行"
    );
}
