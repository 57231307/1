//! P3-4 BI 多维分析集成测试
//!
//! V15 Batch 485 P0-06-6 修复：
//! - 原 16 个测试使用过时的静态方法调用 `BiAnalysisService::method(args)`
//!   但 v9 批次 130 重构后所有方法改为实例方法 `&self`
//! - 本批次修复为 `BiAnalysisService::new(db).method(args)` 实例调用
//! - 参数校验类测试（无效输入返回 Err）连真实 PostgreSQL（只连接不触库），
//!   校验在 DB 查询前返回
//! - 需要真实数据的测试标记 #[ignore]，由 CI ignored job 在已迁移库上执行

mod test_common;

use std::sync::Arc;

use bingxi_backend::models::status::sales::sales_order;
use bingxi_backend::services::bi_analysis_service::BiAnalysisService;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

/// 构造测试用 BiAnalysisService（真实 PostgreSQL，只连接不清空）
///
/// 校验类用例只取连接对象，方法在执行 DB 查询前校验参数并返回 Err；
/// 活库用例由 ignored job 在已迁移的 `TEST_DATABASE_URL` 库上真跑。
async fn make_service() -> BiAnalysisService {
    let database: DatabaseConnection = test_common::connect_live_db().await;
    BiAnalysisService::new(Arc::new(database))
}

/// 活库真数据夹具（CI #4672 §A.2 "BI 无 seed" 族）：
/// `setup_test_db()`（已迁移 PG + TRUNCATE 业务表）后种**真实**父行与一张有效销售订单
/// 及其明细，手法逐列照抄本仓 BI 真库种子范式
/// `contract_wave3_bi_profit_status_lowercase_test.rs:59-137`：
/// - customers/products 为 FK 父行（`m0001_initial_schema.rs:631-634`：
///   sales_orders.customer_id → customers、sales_order_items.product_id → products）；
/// - 订单状态取写入方权威小写词表 `sales_order::PENDING`（models/status/sales.rs；
///   profit.rs:53 / :138 排除门仅拒 cancelled/draft，pending 如实计入）；
/// - 收入 1000 > 成本 10×60=600 ⇒ gross_margin=40%>0（成本口径 profit.rs:9
///   "SUM(sales_order_items.quantity * products.cost_price)"）；
/// - order_date 真列 TIMESTAMPTZ，按范式以 ::timestamptz 字面量落库。
async fn make_seeded_service() -> BiAnalysisService {
    let db = test_common::setup_test_db().await;
    exec_seed(&db).await;
    BiAnalysisService::new(Arc::new(db))
}

async fn exec_seed(db: &DatabaseConnection) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id, customer_code, customer_name) VALUES ($1, $2, $3)",
        vec![
            1i32.into(),
            "BI-SEED-C1".to_string().into(),
            "BI 种子客户".to_string().into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("BI 种子 customers 执行失败: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO products (id, code, name, cost_price) VALUES ($1, $2, $3, $4)",
        vec![
            1i32.into(),
            "BI-SEED-P1".to_string().into(),
            "BI 种子产品".to_string().into(),
            Decimal::new(6000, 2).into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("BI 种子 products 执行失败: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status)
           VALUES ($1, 1, $2::timestamptz, $3, $4)"#,
        vec![
            "BI-SEED-O1".to_string().into(),
            "2026-06-15".to_string().into(),
            Decimal::new(100000, 2).into(),
            sales_order::PENDING.into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("BI 种子 sales_orders 执行失败: {e}"));
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"INSERT INTO sales_order_items (order_id, quantity, product_id, unit_price, subtotal)
           SELECT id, $1, 1, $2, $3 FROM sales_orders WHERE order_no = $4"#,
        vec![
            Decimal::new(1000, 2).into(),
            Decimal::new(10000, 2).into(),
            Decimal::new(100000, 2).into(),
            "BI-SEED-O1".to_string().into(),
        ],
    ))
    .await
    .unwrap_or_else(|e| panic!("BI 种子 sales_order_items 执行失败: {e}"));
}

/// 单元测试：drilldown_year_to_month 无效年份返回 Err（参数校验，不依赖 DB）
#[tokio::test]
async fn test_drilldown_year_to_month_invalid() {
    let service = make_service().await;
    // 年份 < 1900 或 > 2999 应返回 Err（校验在 DB 查询前）
    assert!(
        service.drilldown_year_to_month(1800).await.is_err(),
        "1800 年应被拒绝"
    );
    assert!(
        service.drilldown_year_to_month(3000).await.is_err(),
        "3000 年应被拒绝"
    );
}

/// 单元测试：drilldown_year_to_month 有效年份（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库（to_char / EXTRACT 语法）"]
async fn test_drilldown_year_to_month() {
    let service = make_service().await;
    let data = service.drilldown_year_to_month(2026).await.unwrap();
    assert_eq!(data.len(), 12, "12 个月");
}

/// 单元测试：slice 无效维度返回 Err（参数校验，不依赖 DB）
#[tokio::test]
async fn test_slice_invalid_dimension() {
    let service = make_service().await;
    let result = service.slice("invalid", &serde_json::json!({})).await;
    assert!(result.is_err(), "无效维度应返回 Err");
}

/// 单元测试：slice 有效维度（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_slice() {
    let service = make_service().await;
    let result = service
        .slice("customer", &serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(result["dimension"], "customer");
}

/// 单元测试：rollup 无效粒度返回 Err（参数校验，不依赖 DB）
#[tokio::test]
async fn test_rollup_invalid_level() {
    let service = make_service().await;
    let result = service.rollup("invalid", "month").await;
    assert!(result.is_err(), "无效粒度应返回 Err");
}

/// 单元测试：rollup 有效粒度（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_rollup() {
    let service = make_service().await;
    let result = service.rollup("day", "month").await.unwrap();
    assert_eq!(result["from"], "day");
    assert_eq!(result["to"], "month");
}

/// 单元测试：sales_by_time 日期反转返回 Err（参数校验，不依赖 DB）
#[tokio::test]
async fn test_sales_by_time_invalid_dates() {
    let service = make_service().await;
    let result = service
        .sales_by_time(
            chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            "month",
        )
        .await;
    assert!(result.is_err(), "结束日期早于开始日期应返回 Err");
}

/// 单元测试：pivot（需要真实 DB，标记 ignore）
///
/// 度量名按后端权威词表改判（CI #4672 §A.2 定性）：合法度量只有
/// `["total_amount", "order_count", "quantity", "profit_amount"]`
/// （`bi_analysis_ops/olap.rs:135`，聚合表达式实现 `bi_analysis_service.rs:82-102`
/// 逐词列举、无 "amount" 分支），原用例传的 `"amount"` 是**不存在的度量名**，
/// 服务如实返回 ValidationErrorDisplayable("不支持的度量: amount")（olap.rs:136-141）
/// ——这是词表契约锁的正解，不是源码缺陷。断言键同源词表用例的真实出参形状
/// （build_pivot_matrix 出参键为 row_dim/col_dim/measure，olap.rs:255-262；
/// 原断言的 "row"/"col" 键在服务出参里根本不存在）。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_pivot() {
    let service = make_service().await;
    let result = service
        .pivot("time", "product", "total_amount")
        .await
        .unwrap();
    assert_eq!(result["row_dim"], "time");
    assert_eq!(result["col_dim"], "product");
    assert_eq!(result["measure"], "total_amount");
}

/// kpi/利润/透视族的数据前提锁（#4672 反证另一半）：非法度量名 `amount` **不在**
/// 后端权威词表（olap.rs:135），pivot 必须在触库前就返回可外显的校验错误；
/// 防止有人为洗绿在词表里补一个 "amount" 同义分支、把词表契约改成第二套口径。
#[tokio::test]
async fn test_pivot_invalid_measure_rejected_before_db() {
    let service = make_service().await;
    let err = service
        .pivot("time", "product", "amount")
        .await
        .expect_err("词表外度量名必须返回 Err，不得静默换聚合口径");
    assert!(
        matches!(&err, bingxi_backend::utils::error::AppError::ValidationErrorDisplayable(m)
            if m.contains("不支持的度量")),
        "必须命中 validate_pivot_params 的度量词表拒绝（olap.rs:136-141），实际: {err:?}"
    );
}

/// 单元测试：kpi_summary（需要真实 DB，标记 ignore）
///
/// 真值断言走 `make_seeded_service()`：#4672 判责 §A.2 定性为"BI 无 seed"——
/// fetch_current_kpi（bi_analysis_ops/profit.rs:128-165）对 TRUNCATE 后的空
/// sales_orders 聚合恒得 0，断 >0 必红；本仓裁定是**种真实数据**而非放宽断言。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_kpi_summary_returns_valid() {
    let service = make_seeded_service().await;
    let kpi = service.kpi_summary().await.unwrap();
    assert!(kpi.total_sales > 0.0, "total_sales 应大于 0");
    assert!(kpi.order_count > 0, "order_count 应大于 0");
    assert!(kpi.avg_order_value > 0.0, "avg_order_value 应大于 0");
}

/// 单元测试：kpi_summary 无业务 schema 时返回 Err（不标记 ignore，验证非崩溃）
#[tokio::test]
async fn test_kpi_summary_returns_err_without_db() {
    // 负前提交集：连 CI 里不跑迁移的第二只空 schema 库（指错库夹具直接判红）。
    // 库里没有 sales_orders 表，查询应返回 Err（非 panic）。
    let database: DatabaseConnection = test_common::connect_empty_schema_db().await;
    let service = BiAnalysisService::new(Arc::new(database));
    let result = service.kpi_summary().await;
    assert!(result.is_err(), "无真实 DB 时 kpi_summary 应返回 Err");
}

/// 单元测试：sales_by_customer（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_sales_by_customer_limit() {
    let service = make_service().await;
    let result = service.sales_by_customer(2).await.unwrap();
    assert!(result.len() <= 2);
}

/// 单元测试：sales_by_product（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_sales_by_product_limit() {
    let service = make_service().await;
    let result = service.sales_by_product(1).await.unwrap();
    assert!(result.len() <= 1);
}

/// 单元测试：drilldown_month_to_day（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_drilldown_month_to_day() {
    let service = make_service().await;
    let data = service.drilldown_month_to_day(2026, 6).await.unwrap();
    assert_eq!(data.len(), 30);
}

/// 单元测试：profit_analysis（需要真实 DB，标记 ignore）
///
/// 同 test_kpi_summary_returns_valid：真值断言必须种真实数据。种子形态保证
/// 收入(1000) > 成本(明细 10 × cost_price 60 = 600) ⇒ gross_margin=40>0
/// （profit.rs:72-80 的派生口径，成本公式见本文件头 exec_seed 注释）。
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_profit_analysis() {
    let service = make_seeded_service().await;
    let p = service.profit_analysis().await.unwrap();
    assert!(p.total_revenue > 0.0);
    assert!(p.gross_margin > 0.0);
}

/// 单元测试：sales_trend（需要真实 DB，标记 ignore）
#[tokio::test]
#[ignore = "需要 PostgreSQL 测试数据库"]
async fn test_sales_trend() {
    let service = make_service().await;
    let data = service.sales_trend(7).await;
    assert!(data.is_ok());
}

/// 集成测试：端到端（CI 启用，沙箱 OOM 跳过）
#[tokio::test]
#[ignore = "需要 PostgreSQL + ETL 数据 + axum server"]
async fn test_e2e_etl_to_aggregation() {
    // 完整流程：
    // 1. ETL 加载业务库数据到 sales_facts
    // 2. 启动 axum server（in-process）
    // 3. HTTP GET /api/v1/erp/bi/sales/kpi
    // 4. 验证返回 KPI 数据
    // 沙箱 OOM 限制下仅保留 stub
}
