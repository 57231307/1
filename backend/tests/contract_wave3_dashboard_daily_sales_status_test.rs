//! 仪表盘日/周/月销售卡片状态排除门契约锁(任务 #164,
//! dashboard_service.rs::query_daily_sales_amounts)
//!
//! 锁定的根因(与同文件分维/周转率及 BI sales 聚合先例同源):
//! - 写入方权威词表 `crate::models::status::sales::sales_order` 各状态均为**小写**
//!   (DRAFT="draft"、CANCELLED="cancelled",见 models/status/sales.rs:16/28)。
//! - 修复前 query_daily_sales_amounts(日/周/月销售卡片唯一取数入口)完全没有
//!   状态谓词 ⇒ draft/cancelled 订单金额一并计入卡片趋势,与 BI/利润分析口径不同源。
//!
//! 修复形态:
//! - SeaORM builder 上追加 `Column::Status.is_not_in([so_status::CANCELLED,
//!   so_status::DRAFT])`,取值引用词表常量、经参数绑定(Value::String 占位符)下推,
//!   无 format! 拼字面量;本函数为 builder 查询,日期过滤走 gte/lte 参数绑定,
//!   不存在手写 $N 基址,天然无撞号(与 raw SQL 处 $1/$2+日期 $3 的分维形态同源不同体)。
//! - 语义核实:排除门只剔 draft/cancelled,pending 等中间态仍计入(非"只算已完成")。
//! - 卡片为单度量(SalesDataPoint 仅 date+amount,无订单数字段);金额与其日分桶
//!   由同一 WHERE 谓词门控,不存在"只滤金额不滤单数"——本测试同时在复现 SQL 中带
//!   COUNT(*) 锁"同一谓词对金额与行数同步生效"。
//!
//! 覆盖策略(全部真实 SQL 行为,无 mock;复现生产谓词形态,直连 sqlite::memory:,
//! 不经 DashboardService 的 5min TTL 内存缓存,无陈旧缓存假绿风险):
//! 1. 行为锁:复现修复后谓词(status NOT IN ($1,$2) 绑小写常量 + 日期过滤 $3),
//!    断言窗内日桶仅 pending(1000/1 单)、窗外 pending(500)被 $3 日期参数剔除。
//!    失败签名:6556.25=大小写门漏剔;1500=日期占位符撞号漏剔窗外;7056.25=双缺陷。
//! 2. 大写字面量对照组(缺陷实证因果锁):`NOT IN ('CANCELLED','DRAFT')` 对小写落库
//!    值恒不命中 ⇒ 窗内三单全进 6556.25/3 单——证明"改词表洗绿"不可掩盖大小写因果。
//! 3. 词表同源锁:DRAFT/CANCELLED 值为小写。
//! 4. 防回潮源码扫描:query_daily_sales_amounts 函数体必须含 is_not_in 且引用
//!    so_status 常量、不得出现任何状态字符串字面量;dashboard_service.rs 全文
//!    不得再含大写 NOT IN 字面量。

use bingxi_backend::models::status::sales::sales_order;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE sales_orders (
            id INTEGER PRIMARY KEY,
            order_date TEXT NOT NULL,
            total_amount REAL NOT NULL,
            status TEXT NOT NULL
        )"#,
        Vec::<Value>::new(),
    ))
    .await
    .expect("DDL 建表失败");

    // (订单日, 金额, 状态) —— 窗口 2096-12-01 起:
    // pending 1000(窗内) / draft 2222.5 / cancelled 3333.75 / pending 500(窗外)
    for (day, amount, status) in [
        ("2096-12-05", 1000.00_f64, sales_order::PENDING),
        ("2096-12-06", 2222.50_f64, sales_order::DRAFT),
        ("2096-12-07", 3333.75_f64, sales_order::CANCELLED),
        ("2096-11-01", 500.00_f64, sales_order::PENDING),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO sales_orders (order_date, total_amount, status) \
             VALUES ($1, $2, $3)",
            vec![day.into(), amount.into(), status.into()],
        ))
        .await
        .expect("种子订单插入失败");
    }
    db
}

/// 返回 (日桶数, 金额合计, 订单行数合计)
fn unpack(rows: Vec<sea_orm::QueryResult>) -> (usize, f64, i64) {
    let mut sum = 0.0_f64;
    let mut cnt = 0_i64;
    for r in &rows {
        sum += r
            .try_get::<Option<f64>>("", "amount")
            .expect("amount 应可解码为 f64")
            .unwrap_or(0.0);
        cnt += r
            .try_get::<Option<i64>>("", "cnt")
            .expect("cnt 应可解码为 i64")
            .unwrap_or(0);
    }
    (rows.len(), sum, cnt)
}

/// 行为锁:复现 query_daily_sales_amounts 修复后谓词形态——按日 SUM(total_amount)
/// 分桶,状态排除门 $1/$2 绑定小写词表常量,日期下限 $3;COUNT(*) 与 SUM 共用同一
/// WHERE,证明同一谓词同步门控金额与行数(单桶=单日内订单行数)。
async fn daily_agg_via_bound_params(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let values: Vec<Value> = vec![
        sales_order::CANCELLED.into(),
        sales_order::DRAFT.into(),
        "2096-12-01".into(),
    ];
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0) as amount,
            COUNT(*) as cnt
        FROM sales_orders s
        WHERE s.status NOT IN ($1, $2)
        AND s.order_date >= $3
        GROUP BY s.order_date
        ORDER BY s.order_date ASC"#,
        values,
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("参数绑定日聚合查询失败");
    unpack(rows)
}

/// 缺陷实证对照:复现"大写状态字面量门"形态(大小写敏感 ⇒ 对小写落库值恒不命中),
/// 日期参数 $1。与行为锁同种子数据、同断言点位,差值即大小写因果。
async fn daily_agg_via_uppercase_literal(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0) as amount,
            COUNT(*) as cnt
        FROM sales_orders s
        WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
        AND s.order_date >= $1
        GROUP BY s.order_date
        ORDER BY s.order_date ASC"#,
        vec!["2096-12-01".into()],
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("大写字面量日聚合查询失败");
    unpack(rows)
}

/// 无排除门对照(修复前生产形态=全表聚合):四行全进,证明"缺谓词"本身即是缺陷。
async fn daily_agg_via_no_predicate(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0) as amount,
            COUNT(*) as cnt
        FROM sales_orders s
        GROUP BY s.order_date
        ORDER BY s.order_date ASC"#,
        Vec::<Value>::new(),
    );
    let rows = db.query_all_raw(stmt).await.expect("无谓词日聚合查询失败");
    unpack(rows)
}

/// 绑定小写常量后,窗内仅 pending(1000)入桶、1 单;窗外 pending(500)由 $3
/// 日期参数剔除。修复前(无状态谓词)此断言必然红:4 桶、7056.25、4 单。
#[tokio::test]
async fn dashboard_daily_sales_excludes_draft_and_cancelled_via_bound_lowercase_status() {
    let db = setup_db().await;
    let (buckets, total, cnt) = daily_agg_via_bound_params(&db).await;
    assert_eq!(
        buckets, 1,
        "窗内应仅 1 个日桶(pending 2096-12-05),实际 {buckets}"
    );
    assert_eq!(
        total, 1000.00,
        "日/周/月卡片金额必须=1000(7056.25=无排除门原缺陷,6556.25=大小写门不命中,1500=日期占位符撞号漏剔窗外),实际 {total}"
    );
    assert_eq!(cnt, 1, "同一谓词门控的行数必须=1,实际 {cnt}");
}

/// 缺陷实证对照①:大写状态字面量对小写落库值恒不命中→窗内三单全进 6556.25/3 单。
/// 若有人改词表为大写来"洗绿"本文件断言,此对照将失配(词表同源锁同步爆红)。
#[tokio::test]
async fn dashboard_daily_sales_uppercase_literal_wrongly_includes_draft_and_cancelled() {
    let db = setup_db().await;
    let (buckets, total, cnt) = daily_agg_via_uppercase_literal(&db).await;
    assert_eq!(buckets, 3, "大写字面量门下窗内三单全进,应 3 桶");
    assert!(
        (total - 6556.25).abs() < 1e-9,
        "大写门合计应=6556.25(1000+2222.5+3333.75),实际 {total}"
    );
    assert_eq!(cnt, 3, "大写门订单行数应=3,实际 {cnt}");
}

/// 缺陷实证对照②:修复前生产形态=完全无状态谓词,含 draft/cancelled 与窗外行,
/// 全表 4 桶 7056.25 4 单——与行为锁差值即排除门+窗口门的联合因果。
#[tokio::test]
async fn dashboard_daily_sales_no_predicate_defect_proof_includes_all_status() {
    let db = setup_db().await;
    let (buckets, total, cnt) = daily_agg_via_no_predicate(&db).await;
    assert_eq!(buckets, 4, "无谓词形态四行各成一日桶,应 4 桶");
    assert!(
        (total - 7056.25).abs() < 1e-9,
        "无谓词合计应=7056.25(含 draft/cancelled/窗外 pending),实际 {total}"
    );
    assert_eq!(cnt, 4, "无谓词行数应=4,实际 {cnt}");
}

/// 词表同源锁:排除门引用 sales_order 权威小写常量(与 dashboard_service.rs
/// so_status 别名同源,models/status/sales.rs:16/:28)。
#[test]
fn status_word_table_is_lowercase_single_source_daily_card_gate() {
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

/// 防回潮源码扫描:query_daily_sales_amounts 函数体必须挂上 is_not_in 排除门,
/// 且引用 so_status 词表常量、不含任何状态字符串字面量;dashboard_service.rs 全文
/// 不得出现大写 NOT IN 状态字面量(含修复前"零谓词"形态的回潮会被此锁直接拦下)。
#[test]
fn source_scan_daily_sales_gate_uses_bound_constants() {
    let dash = include_str!("../src/services/dashboard_service.rs").replace('\r', "");
    let fn_start = dash
        .find("async fn query_daily_sales_amounts")
        .expect("dashboard_service.rs 必须存在 query_daily_sales_amounts");
    let rest = &dash[fn_start..];
    let fn_end = rest
        .find("fn aggregate_sales_by_periods")
        .expect("query_daily_sales_amounts 与 aggregate_sales_by_periods 相邻,须能找到函数边界");
    let daily_fn = &rest[..fn_end];

    assert_eq!(
        daily_fn.matches("is_not_in").count(),
        1,
        "query_daily_sales_amounts 必须且只能挂 1 处 is_not_in 排除门"
    );
    assert!(
        daily_fn.contains("so_status::CANCELLED") && daily_fn.contains("so_status::DRAFT"),
        "排除门须引用词表常量 so_status::CANCELLED / so_status::DRAFT,禁止字面量"
    );
    for literal in ["\"draft\"", "\"cancelled\"", "'DRAFT'", "'CANCELLED'"] {
        assert!(
            !daily_fn.contains(literal),
            "query_daily_sales_amounts 不得含状态字符串字面量 {literal},应绑词表常量"
        );
    }
    assert!(
        !dash.contains("NOT IN ('CANCELLED'") && !dash.contains("NOT IN ('DRAFT'"),
        "dashboard_service.rs 全文 SQL 不得含大写 NOT IN 状态字面量"
    );
}
