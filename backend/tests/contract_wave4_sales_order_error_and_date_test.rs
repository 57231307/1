//! Wave4 销售订单错误映射 + 统计日期区间下推契约锁
//!
//! 锁定本波修复（普查 A3 与日期口径缺陷）：
//! 1. `handlers/sales_order_handler.rs` 对 `SalesService::reject_order / cancel_order /
//!    get_order_deliveries / create_delivery / cancel_delivery / get_order_statistics /
//!    export_orders_to_xlsx`（均返回 `Result<_, AppError>`）不再
//!    `.map_err(|e| AppError::internal(..))` 把 400 业务拒绝/404/参数校验压成 500，
//!    改 `?` 透传，保留 service 给的 status/code/文案（用户点“拒绝/取消/发货”看到的
//!    “请求错误”根因）。
//! 2. `services/so/order_query.rs::get_order_statistics` 的 `start_date`/`end_date`
//!    此前被解出为 `_start_date`/`_end_date` 后丢弃、从不进 WHERE ⇒ 日期筛选完全不生效。
//!    修复后按 `build_orders_query`（本文件已生效实现，order_query.rs:173-185）同源口径
//!    下推：`OrderDate >= start(当日 00:00 含) AND OrderDate < end(次日 00:00 不含)`。
//!
//! 覆盖策略（沿用先例 contract_wave2_reservation_error_mapping_test.rs：真实行为 + 源码扫描双保险）：
//! - 日期下推：sqlite::memory: 两条不同日期单据，调用真实 service，传区间只回窗口内那条，
//!   金额与计数精确断言（不许只断 200）；无区间回全量；非法日期 400 VALIDATION_ERROR。
//! - 状态门拒绝：lock_exclusive 在 sqlite 方言不支持，400 透传形态以 #[ignore] 活库用例固化，
//!   经真实 handler（含 `?` 映射）返回，断言 4xx + code=BUSINESS_ERROR + 底层拒绝原因非“服务器内部错误”。
//! - 源码扫描防回潮锁（CI 必跑）：A3 六站点 + 导出站点的 internal 强转文案零命中；
//!   AppError::internal 仅收缩棘轮；报价词表同源锁（禁 Expr::cust 手写状态字面量）。
//!
//! 活库用例的 TEST_DATABASE_URL 缺失时明确 panic（不静默 skip）。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::so::order::SalesService;
use bingxi_backend::utils::error::AppError;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use serde_json::Value as JsonValue;
use std::sync::Arc;

// =========================================================
// 通用工具
// =========================================================

/// 从统计结果 JSON 中稳健取整数（兼容 i64 / f64 / 字符串编码）。
fn as_count(v: &JsonValue, key: &str) -> i64 {
    match &v[key] {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                i
            } else {
                n.as_f64().unwrap_or(0.0) as i64
            }
        }
        JsonValue::String(s) => s.parse::<f64>().unwrap_or(0.0) as i64,
        other => panic!("字段 {key} 无法解码为整数，实际: {other}"),
    }
}

/// 从统计结果 JSON 中稳健取金额（兼容 f64 / 字符串编码）。
fn as_amount(v: &JsonValue, key: &str) -> f64 {
    match &v[key] {
        JsonValue::Number(n) => n.as_f64().unwrap_or(0.0),
        JsonValue::String(s) => s
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("金额字段无法解析: {s}")),
        other => panic!("字段 {key} 无法解码为金额，实际: {other}"),
    }
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave4_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

// =========================================================
// 1) 统计日期区间真实下推（sqlite::memory:，CI 必跑，无 mock）
// =========================================================

/// 与统计查询引用列逐一对应的 sqlite 最小 DDL。
/// get_order_statistics 的 SeaORM 查询只 SELECT COUNT(id) / SUM(total_amount)，
/// WHERE 只引用 order_date、customer_id，故仅建这些列即可。
async fn setup_stats_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE sales_orders (
            id INTEGER PRIMARY KEY,
            customer_id INTEGER,
            order_date TEXT NOT NULL,
            total_amount REAL NOT NULL
        )"#,
        Vec::<Value>::new(),
    ))
    .await
    .expect("DDL 建 sales_orders 表失败");

    // 两条不同日期单据：Jan 10 → 100.0（目标窗口内）；Jun 20 → 200.0（窗口外）。
    // order_date 存 ISO 日期串，与 SeaORM 绑定的 DateTime 前缀逐位比较在“日/月”位即分胜负，
    // 不受时间/时区后缀格式差异影响。
    for (id, day, amount) in [(1_i64, "2026-01-10", 100.0_f64), (2, "2026-06-20", 200.0)] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO sales_orders (id, customer_id, order_date, total_amount) \
             VALUES ($1, $2, $3, $4)",
            vec![id.into(), Value::Int(Some(10)), day.into(), amount.into()],
        ))
        .await
        .expect("种子销售订单插入失败");
    }
    db
}

fn stats_service(db: sea_orm::DatabaseConnection) -> SalesService {
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    SalesService::new(Arc::new(db), search_client)
}

/// 核心回归锁：传日期区间只回窗口内那条——计数与金额必须精确等于窗口内单据，
/// 修复前（_start_date/_end_date 收下即丢）必然返回全量 2 单 / 300.0。
#[tokio::test]
async fn order_statistics_date_range_pushdown_returns_only_in_window_row() {
    let svc = stats_service(setup_stats_db().await);
    let stats = svc
        .get_order_statistics(serde_json::json!({
            "start_date": "2026-01-01",
            "end_date": "2026-01-31"
        }))
        .await
        .expect("统计查询应成功");

    let count = as_count(&stats, "total_orders");
    let amount = as_amount(&stats, "total_amount");
    assert_eq!(
        count, 1,
        "窗口内只应有 Jan-10 一条，实际 {count}（修复前日期未下推会得全量 2）"
    );
    assert!(
        (amount - 100.0).abs() < 1e-6,
        "窗口内金额合计必须=100.0（修复前日期未下推会得 300.0），实际 {amount}"
    );
}

/// 对照组：不传日期端点时不加界，回全量 2 单 / 300.0（证明过滤确由日期端点驱动，
/// 而非恰好因其他原因漏剔）。
#[tokio::test]
async fn order_statistics_without_date_range_returns_all_rows() {
    let svc = stats_service(setup_stats_db().await);
    let stats = svc
        .get_order_statistics(serde_json::json!({}))
        .await
        .expect("统计查询应成功");
    assert_eq!(as_count(&stats, "total_orders"), 2, "无区间应回全量 2 单");
    assert!(
        (as_amount(&stats, "total_amount") - 300.0).abs() < 1e-6,
        "无区间金额合计=300.0"
    );
}

/// 非法日期格式必须 400 VALIDATION_ERROR（可外显原因，仅涉及用户自己提交的字段），
/// 不得静默套默认值吞掉筛选、也不得 500。
#[tokio::test]
async fn order_statistics_invalid_date_returns_validation_error_not_500() {
    let svc = stats_service(setup_stats_db().await);
    let err = svc
        .get_order_statistics(serde_json::json!({ "start_date": "2026-13-99" }))
        .await
        .expect_err("非法日期必须被拒绝");
    assert_eq!(err.error_code(), "VALIDATION_ERROR", "实际: {err:?}");
    let resp = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "非法日期必须 400，不得 500"
    );
}

// =========================================================
// 2) 活库（PostgreSQL）状态门 400 透传（lock_exclusive，sqlite 不支持 → #[ignore]）
// =========================================================

async fn live_db() -> sea_orm::DatabaseConnection {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "活库用例须设置 TEST_DATABASE_URL 指向已跑完迁移的 PostgreSQL（缺失即失败，不静默 skip）",
    );
    sea_orm::Database::connect(&url)
        .await
        .expect("活库连接失败：TEST_DATABASE_URL 无法建立连接")
}

fn live_state() -> AppState {
    AppState::default()
}

/// 活库：对不处于 pending 的订单经真实 handler 调 reject → 4xx + code=BUSINESS_ERROR，
/// 底层拒绝原因文案非“服务器内部错误”（修复前被 map_err(internal) 压成 500）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL + TEST_SEED_SO_NOT_PENDING_ID（reject 状态门在 lock_exclusive 之后，sqlite 不支持）"]
async fn reject_order_handler_maps_state_gate_to_4xx_business_not_500() {
    let id: i32 = std::env::var("TEST_SEED_SO_NOT_PENDING_ID")
        .expect("须提供指向非 pending 状态且客户存在的销售订单 ID")
        .parse()
        .unwrap();

    let mut state = live_state();
    state.db = Arc::new(live_db().await);

    let result = sales_order_handler::reject_order(
        make_auth(1),
        State(state),
        Path(id),
        Json(sales_order_handler::RejectSalesOrderRequest {
            reason: "契约测试拒绝".to_string(),
        }),
    )
    .await;

    let err = match result {
        Ok(_) => panic!("非 pending 订单不可被拒绝，却返回成功"),
        Err(e) => e,
    };
    // 修复后 service 拒绝原样透传（不再被 internal 包裹）
    assert!(
        !matches!(err, AppError::InternalError(_)),
        "状态门拒绝禁止再被压成 InternalError(500)，实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR", "实际: {err:?}");
    // 真实拒绝原因随 AppError 透传（service 用 business 变体，HTTP body 脱敏属设计如此）
    assert!(
        err.to_string().contains("不允许拒绝"),
        "透传的拒绝原因须保留，实际: {err}"
    );

    let resp = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "拒绝必须 400，不得 500"
    );
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: JsonValue = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_ne!(
        v["message"], "服务器内部错误",
        "出参不得再是 500 的‘服务器内部错误’，实际: {v}"
    );
}

/// 活库：对不可取消状态订单经真实 handler 调 cancel → 4xx + code=BUSINESS_ERROR。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL + TEST_SEED_SO_NOT_CANCELABLE_ID（cancel 状态门在 lock_exclusive 之后，sqlite 不支持）"]
async fn cancel_order_handler_maps_state_gate_to_4xx_business_not_500() {
    let id: i32 = std::env::var("TEST_SEED_SO_NOT_CANCELABLE_ID")
        .expect("须提供指向不可取消状态（如 completed）的销售订单 ID")
        .parse()
        .unwrap();

    let mut state = live_state();
    state.db = Arc::new(live_db().await);

    let result = sales_order_handler::cancel_order(make_auth(1), State(state), Path(id)).await;
    let err = match result {
        Ok(_) => panic!("不可取消状态订单不应取消成功"),
        Err(e) => e,
    };
    assert!(
        !matches!(err, AppError::InternalError(_)),
        "取消状态门拒绝禁止再被压成 InternalError(500)，实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR", "实际: {err:?}");
    assert!(
        err.to_string().contains("当前状态不允许取消"),
        "透传的取消拒绝原因须保留，实际: {err}"
    );
    let resp = err.into_response();
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "取消拒绝必须 400，不得 500"
    );
}

// =========================================================
// 3) 源码扫描防回潮锁（CI 必跑）
// =========================================================

/// A3 六站点 + 导出站点的 internal 强转文案零命中（改 `?` 透传后不得回潮）。
#[test]
fn source_scan_sales_order_handler_no_service_error_coercion() {
    let src = include_str!("../src/handlers/sales_order_handler.rs").replace('\r', "");
    for forbidden in [
        "拒绝订单失败: {}",
        "取消订单失败: {}",
        "获取发货记录失败: {}",
        "创建发货失败: {}",
        "取消发货单失败: {}",
        "获取订单统计失败: {}",
        "导出失败: {}",
    ] {
        assert!(
            !src.contains(forbidden),
            "sales_order_handler 禁止再把 service 的 AppError 强转 500 internal，命中: {forbidden}"
        );
    }
}

/// AppError::internal 收缩棘轮：仅剩合法序列化/不可归类站点，计数不得回潮上升。
/// 本波修复后精确基线为 10（全部为 `serde_json::to_value` 序列化失败）。
#[test]
fn source_scan_sales_order_handler_internal_ratchet_shrink_only() {
    let src = include_str!("../src/handlers/sales_order_handler.rs").replace('\r', "");
    let hits = src.matches("AppError::internal(").count();
    assert!(
        hits <= 10,
        "sales_order_handler AppError::internal 仅可收缩，不得超过基线 10，实际 {hits}（若确需新增须同步下调并说明）"
    );
    // 合法站点只能是“序列化失败”
    for seg in src.split(';') {
        if seg.contains("AppError::internal(") {
            assert!(
                seg.contains("序列化失败"),
                "剩余 AppError::internal 站点须是序列化类，可疑片段: {seg}"
            );
        }
    }
}

/// 报价词表同源锁：list_expired 不得再用 Expr::cust 手写状态字面量，须绑定 quotation 词表常量；
/// 全文件不得出现大写状态字面量硬串。
#[test]
fn source_scan_quotation_status_filter_uses_word_table_constants() {
    let src = include_str!("../src/handlers/quotation_handler.rs").replace('\r', "");
    assert!(
        !src.contains("Expr::cust"),
        "quotation_handler 禁止再用 Expr::cust 手写 SQL 状态字面量（绕过唯一词表来源）"
    );
    assert!(
        !src.contains("status NOT IN"),
        "quotation_handler 禁止残留手写 'status NOT IN (...)' SQL 字面量"
    );
    // list_expired 必须引用词表常量
    assert!(
        src.contains("quotation_status::CANCELLED")
            && src.contains("quotation_ext::EXPIRED")
            && src.contains("quotation_ext::CONVERTED"),
        "过期清单状态过滤必须绑定 quotation/quotation_ext 词表常量"
    );
    // 大写状态字面量硬串（词表同源锁）
    for literal in [
        "'DRAFT'",
        "'PENDING'",
        "'APPROVED'",
        "'REJECTED'",
        "'CANCELLED'",
        "'CONVERTED'",
        "'EXPIRED'",
    ] {
        assert!(
            !src.contains(literal),
            "quotation_handler 不得出现大写状态字面量硬串 {literal}"
        );
    }
}

/// 统计日期下推源码锁：get_order_statistics 内必须对 OrderDate 施加 gte/lt，
/// 且不得再出现把日期解成未使用变量的丢弃形态。
#[test]
fn source_scan_statistics_date_is_pushed_down() {
    let src = include_str!("../src/services/so/order_query.rs").replace('\r', "");
    let start = src
        .find("pub async fn get_order_statistics")
        .expect("get_order_statistics 必须存在");
    let body = &src[start..];
    assert!(
        body.contains("OrderDate.gte") && body.contains("OrderDate.lt"),
        "统计查询必须把日期区间下推到 OrderDate（gte 含 / lt 次日不含）"
    );
    assert!(
        !body.contains("let _start_date") && !body.contains("let _end_date"),
        "禁止再把 start_date/end_date 解成未使用变量丢弃"
    );
}
