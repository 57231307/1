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
//! 覆盖策略(全部真实 SQL 行为,无 mock;路线一:真库 PostgreSQL,复现生产谓词
//! 形态,不经 DashboardService 的 5min TTL 内存缓存,无陈旧缓存假绿风险):
//! 1. 行为锁:复现修复后谓词(status NOT IN ($1,$2) 绑小写常量 + 日期过滤 $3),
//!    断言窗内日桶仅 pending(1000/1 单)、窗外 pending(500)被 $3 日期参数剔除。
//!    失败签名:6556.25=大小写门漏剔;1500=日期占位符撞号漏剔窗外;7056.25=双缺陷。
//! 2. 大写字面量对照组(缺陷实证因果锁):`NOT IN ('CANCELLED','DRAFT')` 对小写落库
//!    值恒不命中 ⇒ 窗内三单全进 6556.25/3 单——证明"改词表洗绿"不可掩盖大小写因果。
//! 3. 词表同源锁:DRAFT/CANCELLED 值为小写。
//! 4. 防回潮源码扫描:query_daily_sales_amounts 函数体必须含 is_not_in 且引用
//!    so_status 常量、不得出现任何状态字符串字面量;dashboard_service.rs 全文
//!    不得再含大写 NOT IN 字面量。
//!
//! 真库口径说明:生产 total_amount 列为 DECIMAL,聚合出参显式 `::float8` 后按 f64
//! 解码(.00/.25/.50/.75 求和均为二进制精确小数,== 精确断言不受影响);order_date
//! 为 TIMESTAMPTZ,日期下限参数按 PG 语义 `$n::timestamptz` 绑定,谓词形态不变。

use bingxi_backend::models::status::sales::sales_order;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

mod test_common;
use test_common::setup_test_db;

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = setup_test_db().await;

    // FK 前置:sales_orders.customer_id → customers(真表 NOT NULL)
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        // 波0 夹具补 NULL 地雷：customers.customer_type 模型非 Option 而 DDL 可空，
        // 省略该列的种下行按模型读出即 SeaORM 类型错；补真实白名单值（唯一词表模块）。
        format!(
            "INSERT INTO customers (id, customer_code, customer_name, customer_type) VALUES ($1, $2, $3, '{}')",
            bingxi_backend::constants::customer_type::RETAIL
        ),
        vec![
            1i32.into(),
            "W3D-DASH-C1".to_string().into(),
            "日销卡片锁客户".to_string().into(),
        ],
    ))
    .await
    .expect("种子客户插入失败(FK 前置)");

    // (订单日, 金额, 状态) —— 窗口 2096-12-01 起:
    // pending 1000(窗内) / draft 2222.5 / cancelled 3333.75 / pending 500(窗外)
    for (day, amount, status) in [
        ("2096-12-05", Decimal::new(100000, 2), sales_order::PENDING),
        ("2096-12-06", Decimal::new(222250, 2), sales_order::DRAFT),
        (
            "2096-12-07",
            Decimal::new(333375, 2),
            sales_order::CANCELLED,
        ),
        ("2096-11-01", Decimal::new(50000, 2), sales_order::PENDING),
    ] {
        let values: Vec<Value> = vec![
            format!("W3D-{day}").into(),
            day.to_string().into(),
            amount.into(),
            status.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status)
               VALUES ($1, 1, $2::timestamptz, $3, $4)"#,
            values,
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
        DbBackend::Postgres,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0)::float8 as amount,
            COUNT(*)::bigint as cnt
        FROM sales_orders s
        WHERE s.status NOT IN ($1, $2)
        AND s.order_date >= $3::timestamptz
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
        DbBackend::Postgres,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0)::float8 as amount,
            COUNT(*)::bigint as cnt
        FROM sales_orders s
        WHERE s.status NOT IN ('CANCELLED', 'DRAFT')
        AND s.order_date >= $1::timestamptz
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
        DbBackend::Postgres,
        r#"SELECT
            s.order_date as order_day,
            COALESCE(SUM(s.total_amount), 0)::float8 as amount,
            COUNT(*)::bigint as cnt
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

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// #4671 B1①：禁词/必备项都不得被说明性注释命中（"这里以前是 draft 字面量"之类）；
/// 按行剥的理由同 `contract_wave5_outsource_issue_guard_test.rs::code_only` 先例。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 符号定位取函数体：从签名的**代码**（剥注释后）出现处起，到下一个任意缩进的
/// fn 符号之前为止。替代"紧邻下家函数名"这一边界 needle——同文件重构/改名/调序
/// 会移动边界甚至吞进相邻函数（#4671 B1 判责：窗边界必须锚在被锁符号自身上）。
/// 全部偏移在按行拼接文本上产生，find 返回的字节索引天然落在 char boundary，
/// 杜绝多字节字符劈窗 panic（B1③）。
fn fn_body(code_src: &str, signature: &str) -> String {
    let anchor = code_src.find(signature).unwrap_or_else(|| {
        panic!("待锁符号不存在: {signature}（若确已改名/删除，请连同本锁一起更新判据）")
    });
    let tail = &code_src[anchor..];
    let next = [
        "\npub async fn ",
        "\npub fn ",
        "\nasync fn ",
        "\nfn ",
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

/// 防回潮源码扫描:query_daily_sales_amounts 函数体必须挂上 is_not_in 排除门,
/// 且引用 so_status 词表常量、不含任何状态字符串字面量;dashboard_service.rs 全文
/// 不得出现大写 NOT IN 状态字面量(含修复前"零谓词"形态的回潮会被此锁直接拦下)。
/// 判据全部落在剥注释文本上；函数窗由 `query_daily_sales_amounts` 符号定位。
#[test]
fn source_scan_daily_sales_gate_uses_bound_constants() {
    let dash = include_str!("../src/services/dashboard_service.rs").replace('\r', "");
    let code = code_only(&dash);
    let daily_fn = fn_body(&code, "async fn query_daily_sales_amounts");

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
        !code.contains("NOT IN ('CANCELLED'") && !code.contains("NOT IN ('DRAFT'"),
        "dashboard_service.rs 全文 SQL 不得含大写 NOT IN 状态字面量"
    );
}
