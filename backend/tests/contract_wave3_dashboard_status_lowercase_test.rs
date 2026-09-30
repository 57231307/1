//! 仪表盘销售卡片状态词表大小写契约锁(任务 #161,dashboard_service.rs 4 处)
//!
//! 锁定的根因(与已修的 BI sales 聚合范式同源):
//! - 写入方权威词表 `crate::models::status::sales::sales_order` 各状态均为**小写**
//!   (DRAFT="draft"、CANCELLED="cancelled",见 models/status/sales.rs:16/28)。
//! - 修复前 build_dimension_sql 的 customer/product/salesperson 三个维度 SQL 与
//!   query_turnover_rate 销售侧子查询,裸 SQL 用**大写**字面量
//!   `NOT IN ('CANCELLED', 'DRAFT')` 排除。Postgres 大小写敏感 ⇒ 排除门恒不命中,
//!   draft/cancelled 订单被误计入仪表盘分维销售额与周转率分子。
//!
//! 修复形态(照抄 sales.rs 先例:参数绑定引用词表常量):
//! - 4 处状态取值改为 `NOT IN ($1, $2)` 占位,值绑定小写常量(经 so_status 别名);
//!   append_date_filters 的日期参数基址从 $1 后移到 $3(避让 $1/$2)。
//! - 语义逐条核实:4 处均为"排除草稿/取消"(pending 仍计入),非"只算已完成"。
//!
//! 覆盖策略(全部真实 SQL 行为,无 mock):
//! 1. 维度聚合行为锁:复现 customer 维度生产形态谓词(状态 $1/$2 + 日期过滤 $3),
//!    绑定 [cancelled, draft, 起始日],断言仅 pending(1000)入桶、窗口外 pending(500)
//!    被日期参数正确剔除——即基址后移后日期绑定仍与占位符一一对应。
//!    修复前该断言必然红(大写常量入绑,排除门不命中)。
//! 2. 缺陷实证对照:修复前大写字面量谓词→draft/cancelled 全进(合计 6556.25)。
//! 3. 周转率销售侧行为锁:复现 INNER JOIN + NOT IN ($1,$2),断言 sold 只含 pending 明细。
//! 4. 词表同源锁 + 防回潮源码扫描(dashboard_service.rs 无大写 NOT IN 字面量、
//!    引用 so_status 常量、4 处 $1/$2 占位、param_idx=3)。
//!
//! 缓存纪律说明:本测试直连 sqlite、不经 DashboardService 的 5min TTL 内存缓存
//! (dashboard_cache_key 走 AppCache),无陈旧缓存假绿风险。

use bingxi_backend::models::status::sales::sales_order;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        r#"CREATE TABLE sales_orders (
            id INTEGER PRIMARY KEY,
            customer_id INTEGER,
            order_date TEXT NOT NULL,
            total_amount REAL NOT NULL,
            status TEXT NOT NULL
        )"#,
        r#"CREATE TABLE customers (
            id INTEGER PRIMARY KEY,
            customer_name TEXT NOT NULL
        )"#,
        r#"CREATE TABLE sales_order_items (
            id INTEGER PRIMARY KEY,
            order_id INTEGER NOT NULL,
            quantity REAL NOT NULL
        )"#,
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            ddl,
            Vec::<Value>::new(),
        ))
        .await
        .expect("DDL 建表失败");
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO customers (id, customer_name) VALUES ($1, $2)",
        vec![1i64.into(), "云中客户".into()],
    ))
    .await
    .expect("种子客户插入失败");

    // (订单日, 客户, 金额, 状态, 明细数量) —— 窗口 2096-12-01 起:
    // pending 1000(窗内) / draft 2222.5 / cancelled 3333.75 / pending 500(窗外)
    for (day, cust, amount, status, qty) in [
        (
            "2096-12-05",
            1i64,
            1000.00_f64,
            sales_order::PENDING,
            10.0_f64,
        ),
        ("2096-12-06", 1, 2222.50_f64, sales_order::DRAFT, 100.0_f64),
        (
            "2096-12-07",
            1,
            3333.75_f64,
            sales_order::CANCELLED,
            200.0_f64,
        ),
        ("2096-11-01", 1, 500.00_f64, sales_order::PENDING, 5.0_f64),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO sales_orders (customer_id, order_date, total_amount, status) \
             VALUES ($1, $2, $3, $4)",
            vec![cust.into(), day.into(), amount.into(), status.into()],
        ))
        .await
        .expect("种子订单插入失败");
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO sales_order_items (order_id, quantity) \
             SELECT id, $1 FROM sales_orders WHERE order_date = $2",
            vec![qty.into(), day.into()],
        ))
        .await
        .expect("种子明细插入失败");
    }
    db
}

/// 返回 (桶数, 金额合计, 订单数合计)
fn unpack(rows: Vec<sea_orm::QueryResult>) -> (usize, f64, i64) {
    let mut sum = 0.0_f64;
    let mut cnt = 0_i64;
    for r in &rows {
        sum += r
            .try_get::<Option<f64>>("", "total")
            .expect("total 应可解码为 f64")
            .unwrap_or(0.0);
        cnt += r
            .try_get::<Option<i64>>("", "cnt")
            .expect("cnt 应可解码为 i64")
            .unwrap_or(0);
    }
    (rows.len(), sum, cnt)
}

/// 行为锁:复现 build_dimension_sql(customer)+ append_date_filters 修复后形态——
/// 状态占位 $1/$2、日期过滤从 $3 起,绑定顺序 [cancelled, draft, start_date]。
async fn dim_agg_via_bound_params(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let values: Vec<Value> = vec![
        sales_order::CANCELLED.into(),
        sales_order::DRAFT.into(),
        "2096-12-01".into(),
    ];
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            COALESCE(c.customer_name, '未关联客户') as name,
            COALESCE(SUM(s.total_amount), 0) as total,
            COUNT(s.id) as cnt
        FROM sales_orders s
        LEFT JOIN customers c ON c.id = s.customer_id
        WHERE s.status NOT IN ($1, $2)
        AND s.order_date >= $3
        GROUP BY name ORDER BY total DESC LIMIT 20"#,
        values,
    );
    let rows = db.query_all_raw(stmt).await.expect("参数绑定维度聚合失败");
    unpack(rows)
}

/// 缺陷实证对照:复现修复前形态——大写字面量谓词、日期参数 $1。
async fn dim_agg_via_uppercase_literal(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            COALESCE(c.customer_name, '未关联客户') as name,
            COALESCE(SUM(s.total_amount), 0) as total,
            COUNT(s.id) as cnt
        FROM sales_orders s
        LEFT JOIN customers c ON c.id = s.customer_id
        WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
        AND s.order_date >= $1
        GROUP BY name ORDER BY total DESC LIMIT 20"#,
        vec!["2096-12-01".into()],
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("大写字面量维度聚合失败");
    unpack(rows)
}

/// 绑定小写常量+基址后移后,窗内仅 pending(1000)入桶;窗外 pending(500)由 $3
/// 日期参数剔除。修复前此断言必然红(大写常量入绑→draft/cancelled 全进,合计 6556.25)。
#[tokio::test]
async fn dashboard_dimension_agg_excludes_draft_and_cancelled_via_bound_lowercase_status() {
    let db = setup_db().await;
    let (buckets, total, cnt) = dim_agg_via_bound_params(&db).await;
    assert_eq!(buckets, 1, "窗内应仅 1 个客户桶,实际 {buckets}");
    assert_eq!(
        total, 1000.00,
        "分维销售额必须=1000(6556.25=大小写门缺陷,1500=日期参数撞号漏剔窗外单),实际 {total}"
    );
    assert_eq!(cnt, 1, "订单数必须=1,实际 {cnt}");
}

/// 缺陷实证对照:修复前大写字面量谓词对小写落库值恒不命中→窗内三单全进 6556.25。
#[tokio::test]
async fn dashboard_uppercase_literal_predicate_wrongly_includes_all_orders_defect_proof() {
    let db = setup_db().await;
    let (buckets, total, cnt) = dim_agg_via_uppercase_literal(&db).await;
    assert_eq!(buckets, 1, "同一客户聚成 1 桶");
    assert!(
        (total - 6556.25).abs() < 1e-9,
        "原缺陷合计应=6556.25(1000+2222.5+3333.75),实际 {total}"
    );
    assert_eq!(cnt, 3, "原缺陷订单数应=3,实际 {cnt}");
}

/// 周转率销售侧行为锁:复现 query_turnover_rate 修复后形态(INNER JOIN + $1/$2),
/// sold 只计 pending 明细数量(10);draft(100)/cancelled(200) 与窗外单(5) 均剔除。
#[tokio::test]
async fn dashboard_turnover_sold_quantity_excludes_draft_and_cancelled() {
    let db = setup_db().await;
    let values: Vec<Value> = vec![sales_order::CANCELLED.into(), sales_order::DRAFT.into()];
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            (SELECT COALESCE(SUM(si.quantity), 0)
               FROM sales_order_items si
               INNER JOIN sales_orders s ON s.id = si.order_id
                 AND s.status NOT IN ($1, $2)
            ) as total
        "#,
        values,
    );
    let rows = db.query_all_raw(stmt).await.expect("周转率销售侧聚合失败");
    let sold = rows[0]
        .try_get::<Option<f64>>("", "total")
        .expect("total 应可解码为 f64")
        .unwrap_or(0.0);
    assert!(
        (sold - 10.0).abs() < 1e-9,
        "周转率分子应只含 pending 明细数量 10(310=大小写门缺陷,15=日期口径误入),实际 {sold}"
    );
}

/// 词表同源锁:排除门引用 sales_order 权威小写常量(与仪表盘 so_status 别名同源)。
#[test]
fn status_word_table_is_lowercase_single_source() {
    assert_eq!(
        sales_order::DRAFT,
        "draft",
        "写入方 DRAFT 必须小写(models/status/sales.rs:16)"
    );
    assert_eq!(
        sales_order::CANCELLED,
        "cancelled",
        "写入方 CANCELLED 必须小写(models/status/sales.rs:28)"
    );
}

/// 防回潮源码扫描:dashboard_service.rs 4 处排除门已改 so_status 常量绑定
/// ($1/$2),日期过滤基址后移至 $3,且不再含大写状态字面量。
#[test]
fn source_scan_dashboard_uses_bound_constants_and_shifted_date_base() {
    let dash = include_str!("../src/services/dashboard_service.rs").replace('\r', "");
    assert!(
        !dash.contains("NOT IN ('CANCELLED'") && !dash.contains("NOT IN ('DRAFT'"),
        "dashboard_service.rs: SQL 不得再含大写状态字面量,应改绑定参数"
    );
    assert!(
        dash.contains("so_status::CANCELLED") && dash.contains("so_status::DRAFT"),
        "dashboard_service.rs: 排除门须引用词表常量别名 so_status::CANCELLED / so_status::DRAFT"
    );
    assert_eq!(
        dash.matches("NOT IN ($1, $2)").count(),
        4,
        "customer/product/salesperson 三维度 + 周转率销售侧共 4 处应为 NOT IN ($1, $2)"
    );
    assert!(
        dash.contains("let mut param_idx = 3usize;"),
        "append_date_filters 日期基址必须为 $3($1/$2 已被状态绑定占用)"
    );
}
