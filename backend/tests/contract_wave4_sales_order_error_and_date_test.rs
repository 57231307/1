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
//! 覆盖策略（沿用先例 contract_wave2_reservation_error_mapping_test.rs：真实行为 + 源码扫描双保险；
//! 路线一：统一真库 PostgreSQL）：
//! - 日期下推：真实 sales_orders（迁移产出表）种两条不同日期单据+明细，调用真实 service，
//!   传区间只回窗口内那条，金额与计数精确断言（不许只断 200）；无区间回全量；
//!   非法日期 400 VALIDATION_ERROR。
//! - 状态门拒绝：reject/cancel 状态门在事务内 lock_exclusive 之后——真库通道上真实执行；
//!   订单由**用例自种子**（自己插销售订单+明细，用真实 ID 喂服务），不再依赖任何
//!   CI 播种的 TEST_SEED_* 环境变量（#4669 实证：分片 job 只迁移不播种，env 恒 NotPresent）。
//!   经真实 handler（含 `?` 映射）返回，断言 4xx + code=BUSINESS_ERROR + 底层拒绝原因非“服务器内部错误”。
//! - 源码扫描防回潮锁（CI 必跑）：A3 六站点 + 导出站点的 internal 强转文案零命中；
//!   AppError::internal 仅收缩棘轮；报价词表同源锁（禁 Expr::cust 手写状态字面量）。
//!
//! TEST_DATABASE_URL 缺失/指 sqlite 时 setup_test_db 夹具直接 panic（不静默 skip、不回退）。

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
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::Value as JsonValue;
use std::sync::Arc;

mod test_common;
use test_common::setup_test_db;

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
// 1) 统计日期区间真实下推（真库 sales_orders，CI 必跑，无 mock）
// =========================================================

/// FK 前置：customers(10)/products(9)——sales_orders.customer_id 与
/// sales_order_items.product_id 在真表上都带 FK（迁移 m0001），必须用例自种
async fn seed_fk_prerequisites(db: &sea_orm::DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id, customer_code, customer_name) VALUES ($1, $2, $3)",
        vec![
            10i32.into(),
            "W4SO-C10".to_string().into(),
            "统计锁客户".to_string().into(),
        ],
    ))
    .await
    .expect("种子客户插入失败（sales_orders FK 前置）");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO products (id, code, name) VALUES ($1, $2, $3)",
        vec![
            9i32.into(),
            "W4SO-P9".to_string().into(),
            "统计锁产品".to_string().into(),
        ],
    ))
    .await
    .expect("种子产品插入失败（sales_order_items FK 前置）");
}

/// 在真实 sales_orders 上种一张单 + 一条同额明细，RETURNING 真实主键 id。
/// order_date 为 TIMESTAMPTZ：常量日期以 ::timestamptz 文本参数落库；
/// 明细列（quantity/unit_price/subtotal）NOT NULL，按单据金额给足。
async fn seed_order(
    db: &sea_orm::DatabaseConnection,
    order_no: &str,
    day: &str,
    amount: Decimal,
    status: &str,
) -> i32 {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status)
               VALUES ($1, 10, $2::timestamptz, $3, $4) RETURNING id"#,
            vec![
                order_no.to_string().into(),
                day.to_string().into(),
                amount.into(),
                status.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子销售订单 {order_no} 插入失败: {e}"))
        .unwrap_or_else(|| panic!("种子销售订单 {order_no} 的 RETURNING id 未回一行"));
    let id: i32 = row
        .try_get_by_index(0)
        .expect("RETURNING id 应可解码为 i32");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO sales_order_items (order_id, product_id, quantity, unit_price, subtotal)
           VALUES ($1, 9, 1, $2, $2)"#,
        vec![id.into(), amount.into()],
    ))
    .await
    .unwrap_or_else(|e| panic!("种子销售订单 {order_no} 明细插入失败: {e}"));
    id
}

/// 统计夹具：真库 + 两条不同日期单据（Jan 10 → 100.0 目标窗口内；Jun 20 → 200.0 窗口外）。
/// get_order_statistics 的 SeaORM 查询只 SELECT COUNT(id) / SUM(total_amount)，
/// WHERE 引用 order_date、customer_id——全部走迁移产出的真实列型。
async fn setup_stats_db() -> sea_orm::DatabaseConnection {
    let db = setup_test_db().await;
    seed_fk_prerequisites(&db).await;
    seed_order(
        &db,
        "W4SO-STAT-1",
        "2026-01-10",
        Decimal::new(10000, 2),
        "pending",
    )
    .await;
    seed_order(
        &db,
        "W4SO-STAT-2",
        "2026-06-20",
        Decimal::new(20000, 2),
        "pending",
    )
    .await;
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
// 2) 真库状态门 400 透传（reject/cancel 状态门在事务内 lock_exclusive 之后，
//    路线一真库通道真实执行；订单用例自种子，不依赖 TEST_SEED_* 播种）
// =========================================================

async fn live_state() -> AppState {
    let db = setup_test_db().await;
    seed_fk_prerequisites(&db).await;
    AppState {
        db: Arc::new(db),
        ..Default::default()
    }
}

/// 对不处于 pending 的订单（本用例自种子 completed 单）经真实 handler 调 reject →
/// 4xx + code=BUSINESS_ERROR，底层拒绝原因文案非“服务器内部错误”
/// （修复前被 map_err(internal) 压成 500）。
#[tokio::test]
async fn reject_order_handler_maps_state_gate_to_4xx_business_not_500() {
    let state = live_state().await;
    let id = seed_order(
        state.db.as_ref(),
        "W4SO-REJECT-GATE",
        "2026-02-01",
        Decimal::new(10000, 2),
        "completed",
    )
    .await;

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

/// 真库：对不可取消状态订单（本用例自种子 completed 单）经真实 handler 调 cancel →
/// 4xx + code=BUSINESS_ERROR。
#[tokio::test]
async fn cancel_order_handler_maps_state_gate_to_4xx_business_not_500() {
    let state = live_state().await;
    let id = seed_order(
        state.db.as_ref(),
        "W4SO-CANCEL-GATE",
        "2026-02-02",
        Decimal::new(10000, 2),
        "completed",
    )
    .await;

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
