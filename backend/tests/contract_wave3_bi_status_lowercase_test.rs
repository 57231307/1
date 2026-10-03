//! BI 报表状态词表大小写契约锁(任务 #159,71-03 判红点对应的后端修复)
//!
//! 锁定的根因(契约断言 + 日志取证两路核实一致):
//! - 写入方权威词表 `crate::models::status::sales::sales_order` 各状态均为**小写**
//!   (DRAFT="draft"、CANCELLED="cancelled",见 sales.rs:16/28),建单/取消实际落库小写值。
//! - 修复前 BI/钻取全部裸 SQL 用**大写**字面量 `NOT IN ('CANCELLED', 'DRAFT')` 排除
//!   (bi_analysis_ops/sales.rs:74/165/188/251/306/355、drilldown.rs:64/156/243/296)。
//!   Postgres 字符串比较大小写敏感 ⇒ 排除门恒不命中,draft/cancelled 订单被误计入
//!   销售额/订单数聚合(真实业务数字错误)。
//!
//! 修复形态(唯一来源 + 参数绑定,顺带收掉注入面):
//! - 10 处 SQL 状态取值改为 `NOT IN ($k, $k+1)` 占位符,值绑定自
//!   `sales_order::CANCELLED` / `sales_order::DRAFT`(小写),不再 `format!` 塞字面量。
//! - 语义逐条核实均为"排除草稿/取消"(业务规则注释 sales.rs:10-14),未改成"只算已完成",
//!   `pending` 仍应计入(71-03 的 1000 元单即为 pending 单)。
//!
//! 覆盖策略(全部真实 SQL 行为,无 mock;路线一:真库 PostgreSQL):
//! 1. 行为锁:在迁移产出的真实 sales_orders 上种 pending(1000)/draft(2222.5)/
//!    cancelled(3333.75) 三单(金额对齐 71-03),用**与生产同形态**的 `NOT IN ($1,$2)`
//!    绑定小写词表常量聚合 → 断言仅 1 日桶、合计 1000、订单数 1;
//!    对照组用**修复前**的大写字面量谓词聚合 → 断言 3 桶、合计 6556.25(即原缺陷实证)。
//!    修复前该"仅 1 桶/1000"断言必然红(SQL 绑的是大写常量,排除门不命中→全进)。
//! 2. 词表同源锁:断言小写常量确实来自 sales_order 词表(禁第二套手写常量),且其字面值
//!    为 "draft"/"cancelled"。
//! 3. 防回潮源码扫描:断言 bi_analysis_ops/sales.rs、drilldown.rs 已不含大写状态 SQL 字面量,
//!    且已改为引用 sales_order::CANCELLED / sales_order::DRAFT、参数占位符形态。
//!
//! 真库口径说明:生产 total_amount 列为 DECIMAL,聚合出参显式 `::float8` 后按 f64
//! 解码(.00/.25/.50/.75 求和均为二进制精确小数,== 精确断言不受影响);
//! order_date 为 TIMESTAMPTZ,桶键即该列本身,谓词与 GROUP BY 形态与生产逐字同构。
//! 缓存纪律说明:本测试直连真库、经真实谓词聚合,不经 BI service 的 5min TTL 内存缓存
//! (bi_analysis_service.rs:32-48),不受陈旧缓存影响;71-03 的活体端点测试由其自身
//! 的 2096-12 独占窗口 + 互异参数键保证,与此处单元锁互补。

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
            "W3C-BI-C1".to_string().into(),
            "BI词表锁客户".to_string().into(),
        ],
    ))
    .await
    .expect("种子客户插入失败(FK 前置)");

    for (day, amount, status) in [
        ("2096-12-05", Decimal::new(100000, 2), sales_order::PENDING),
        ("2096-12-06", Decimal::new(222250, 2), sales_order::DRAFT),
        (
            "2096-12-07",
            Decimal::new(333375, 2),
            sales_order::CANCELLED,
        ),
    ] {
        // order_date 真列为 TIMESTAMPTZ:测试常量日期以 ::timestamptz 字面量落库;
        // status 存小写实际落库值(写入方词表权威形态)
        // 绑定值必须显式标注 Vec<Value>：裸 vec![.. .into()] 在 Postgres 参数位上推不出类型
        let values: Vec<Value> = vec![
            format!("W3C-{day}").into(),
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

/// 返回 (桶数, 金额合计, 订单数合计)
fn unpack(rows: Vec<sea_orm::QueryResult>) -> (usize, f64, i64) {
    let mut sum = 0.0_f64;
    let mut cnt = 0_i64;
    for r in &rows {
        // 聚合列以 Option 解码(对齐 omni_audit_handler 惯用法,规避驱动列类型报告差异)
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

/// 用**与生产一致**的谓词形态(状态取值绑定小写词表常量)做排除聚合。
async fn agg_excluding_via_params(db: &sea_orm::DatabaseConnection) -> (usize, f64, i64) {
    let values: Vec<Value> = vec![sales_order::CANCELLED.into(), sales_order::DRAFT.into()];
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT order_date AS period,
                  COALESCE(SUM(total_amount), 0)::float8 AS total,
                  COUNT(*)::bigint AS cnt
           FROM sales_orders
           WHERE status NOT IN ($1, $2)
           GROUP BY period
           ORDER BY period ASC"#,
        values,
    );
    let rows = db.query_all_raw(stmt).await.expect("参数绑定聚合查询失败");
    unpack(rows)
}

/// 复现**修复前**缺陷谓词:大写 SQL 字面量(无绑定),证明其对小写落库值恒不命中。
async fn agg_excluding_via_uppercase_literal(
    db: &sea_orm::DatabaseConnection,
) -> (usize, f64, i64) {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT order_date AS period,
                  COALESCE(SUM(total_amount), 0)::float8 AS total,
                  COUNT(*)::bigint AS cnt
           FROM sales_orders
           WHERE status NOT IN ('CANCELLED', 'DRAFT')
           GROUP BY period
           ORDER BY period ASC"#,
        Vec::<Value>::new(),
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("大写字面量聚合查询失败");
    unpack(rows)
}

/// 核心行为锁:绑定小写词表常量后,draft/cancelled 被正确排除,仅 pending(1000)成桶。
/// 修复前(SQL 绑大写常量)此断言必然红:三桶全进、合计 6556.25。
#[tokio::test]
async fn bi_aggregation_excludes_draft_and_cancelled_via_bound_lowercase_status() {
    let db = setup_db().await;
    let (buckets, total, cnt) = agg_excluding_via_params(&db).await;
    assert_eq!(
        buckets, 1,
        "仅 pending 应成 1 个日桶(draft/cancelled 进桶=排除门失守),实际桶数 {buckets}"
    );
    assert_eq!(
        total, 1000.00,
        "窗口聚合金额必须=1000(若=6556.25 即大小写门缺陷实证),实际 {total}"
    );
    assert_eq!(cnt, 1, "订单数必须=1,实际 {cnt}");
}

/// 缺陷实证对照:修复前的大写字面量谓词对小写落库值恒不命中→三桶全进、合计 6556.25。
/// 这条锁的是"大写比较=错误"这一因果,防止有人误把写入方词表改成大写来洗绿。
#[tokio::test]
async fn uppercase_literal_predicate_wrongly_includes_all_orders_defect_proof() {
    let db = setup_db().await;
    let (buckets, total, cnt) = agg_excluding_via_uppercase_literal(&db).await;
    assert_eq!(
        buckets, 3,
        "大写字面量谓词不会命中任何小写状态,draft/cancelled/pending 全进(原缺陷形态)"
    );
    assert!(
        (total - 6556.25).abs() < 1e-9,
        "原缺陷合计应=6556.25(1000+2222.5+3333.75),实际 {total}"
    );
    assert_eq!(cnt, 3, "原缺陷订单数应=3,实际 {cnt}");
}

/// 词表同源锁:排除门引用的是 sales_order 权威小写常量,且字面值就是 draft/cancelled。
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

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// #4671 B1① 判责：禁词扫描命中"修复前这里是 NOT IN ('CANCELLED'…)"这类说明
/// 注释即假判违例；必备项扫描命中注释则是假绿。双向都必须在剥注释文本上判定。
/// 按行而非按字符/引号配平的先例与理由同 `contract_wave5_outsource_issue_guard_test.rs`
/// （被扫源码大量跨行 raw string SQL，字符级配平会吃掉真代码）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 防回潮源码扫描:BI 聚合/钻取 SQL 不得再出现大写状态字面量,且须引用词表常量绑定。
/// 全部判据在剥注释文本上执行（正向必备项同样剥——防"只在注释里引用常量"的假绿）。
#[test]
fn source_scan_bi_sql_uses_bound_constants_not_uppercase_literals() {
    let sales = include_str!("../src/services/bi_analysis_ops/sales.rs").replace('\r', "");
    let drill = include_str!("../src/services/bi_analysis_ops/drilldown.rs").replace('\r', "");

    for (name, src) in [
        ("bi_analysis_ops/sales.rs", &sales),
        ("drilldown.rs", &drill),
    ] {
        let code = code_only(src);
        assert!(
            !code.contains("NOT IN ('CANCELLED'") && !code.contains("NOT IN ('DRAFT'"),
            "{name}: SQL 不得再含大写状态字面量,应改绑定参数"
        );
        assert!(
            code.contains("sales_order::CANCELLED") && code.contains("sales_order::DRAFT"),
            "{name}: 排除门须引用词表常量 sales_order::CANCELLED / sales_order::DRAFT"
        );
        // 绑定形态判据去空白后比对：SQL 文本在 raw string 内不被 rustfmt 折行，
        // 但 `NOT IN ($` 与参数编号之间可能因排版换行/对齐而含空白，排版不参与判定。
        let flat: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            flat.contains("NOTIN($"),
            "{name}: 状态谓词须为 $N 绑定参数形态(顺带收掉注入面)"
        );
    }
}
