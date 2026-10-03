//! BI 利润分析/KPI/OLAP 状态词表大小写契约锁(任务 #161,profit.rs 6 处 + olap.rs 1 处)
//!
//! 锁定的根因(与已修的 sales 聚合范式同源):
//! - 写入方权威词表 `crate::models::status::sales::sales_order` 各状态均为**小写**
//!   (DRAFT="draft"、CANCELLED="cancelled",见 models/status/sales.rs:16/28)。
//! - 修复前 profit.rs(profit_analysis/fetch_current_kpi/yoy×2/mom×2 共 6 处)与
//!   olap.rs(execute_pivot_query 1 处)裸 SQL 用**大写**字面量
//!   `NOT IN ('CANCELLED', 'DRAFT')` 排除。Postgres 字符串比较大小写敏感
//!   ⇒ 排除门恒不命中,draft/cancelled 订单被误计入利润/KPI/同比环比/透视矩阵。
//!
//! 修复形态(照抄 sales.rs/drilldown.rs 先例:参数绑定引用词表常量):
//! - 状态取值改为 `NOT IN ($k, $k+1)` 占位符,值绑定 `sales_order::CANCELLED`/
//!   `sales_order::DRAFT`(小写);新增 2 个占位后 data-scope 谓词基址逐处后移。
//! - yoy 双子查询间状态值复用同一占位符 $4/$5(mom 复用 $5/$6),PG 协议允许
//!   $N 重复引用;该编号形态依赖 PG 行为,由源码扫描锁定(真库通道下语义与
//!   生产一致),执行锁只覆盖单谓词形态。
//! - yoy 原有 scope 基址 $3/$4 与 last_year($3) 撞号(Self/Dept 范围下会把
//!   created_by/department_id 绑上年份字符串),修复时已一并移正($6/$7)。
//!
//! 语义逐条核实:7 处均为"排除草稿/取消"(pending 仍计入),与 sales.rs 已修处
//! 一致,非"只算已完成",未改成第二套条件。
//!
//! 缓存纪律说明:本测试直连真库 PostgreSQL(路线一:`test_common::setup_test_db()`，
//! 已迁移库)、经真实谓词聚合,不经 BI service 的 5min TTL
//! 内存缓存(bi_analysis_service.rs:32-48),不受陈旧缓存影响,参数键无需互异避让。
//!
//! 真库口径说明:生产列为 DECIMAL/NUMERIC,聚合出参在 SELECT 侧显式
//! `::float8` 后按 f64 解码(与旧 sqlite REAL 通道等价的精确二进制小数,
//! .00/.50/.75 均可 == 精确断言);日期/状态绑定按 PG 原生类型
//! (`::timestamptz` 字面量、varchar 词表常量)。大小写敏感比较正是 PG 的
//! 生产行为,sqlite 上"缺陷实证"与"修复形态"两锁在 PG 上语义不变。

use bingxi_backend::models::status::sales::sales_order;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DbBackend, FromQueryResult, Statement, Value};

mod test_common;
use test_common::setup_test_db;

#[derive(Debug, FromQueryResult)]
struct ProfitShapeRow {
    total_revenue: Option<f64>,
    total_cost: Option<f64>,
    order_count: Option<i64>,
}

/// 谓词复现查询的 SELECT 列(真库列型为 NUMERIC,统一 ::float8 后按 f64 解码)
const SHAPE_SELECT: &str = r#"SELECT
            COALESCE(SUM(s.total_amount), 0)::float8 as total_revenue,
            COALESCE(SUM((
                SELECT SUM(si.quantity * COALESCE(p.cost_price, 0))
                FROM sales_order_items si
                LEFT JOIN products p ON p.id = si.product_id
                WHERE si.order_id = s.id
            )), 0)::float8 as total_cost,
            COUNT(*)::bigint as order_count
        FROM sales_orders s"#;

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = setup_test_db().await;

    // FK 前置:sales_orders.customer_id → customers(真表 NOT NULL)
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO customers (id, customer_code, customer_name) VALUES ($1, $2, $3)",
        vec![
            1i32.into(),
            "W3B-PROD-C1".to_string().into(),
            "利润口径锁客户".to_string().into(),
        ],
    ))
    .await
    .expect("种子客户插入失败(FK 前置)");

    // 成本用 Decimal(真列 DECIMAL(12,2)):用例金额(.00/.50/.75)均为十进制精确值
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO products (id, code, name, cost_price) VALUES ($1, $2, $3, $4)",
        vec![
            1i32.into(),
            "W3B-PROD-1".to_string().into(),
            "利润口径锁产品".to_string().into(),
            Decimal::new(500, 2).into(),
        ],
    ))
    .await
    .expect("种子产品插入失败");

    // (订单日, 销售额, 状态, 明细数量) —— 成本 = 数量*5:
    // pending 1000(成本50) / draft 2222.5(成本500) / cancelled 3333.75(成本1000)
    for (day, amount, status, qty) in [
        (
            "2096-12-05",
            Decimal::new(100000, 2),
            sales_order::PENDING,
            Decimal::new(1000, 2),
        ),
        (
            "2096-12-06",
            Decimal::new(222250, 2),
            sales_order::DRAFT,
            Decimal::new(10000, 2),
        ),
        (
            "2096-12-07",
            Decimal::new(333375, 2),
            sales_order::CANCELLED,
            Decimal::new(20000, 2),
        ),
    ] {
        // order_date 真列为 TIMESTAMPTZ:测试常量日期以 ::timestamptz 字面量落库
        let order_values: Vec<Value> = vec![
            format!("W3B-{day}").into(),
            day.to_string().into(),
            amount.into(),
            status.into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO sales_orders (order_no, customer_id, order_date, total_amount, status)
              VALUES ($1, 1, $2::timestamptz, $3, $4)"#,
            order_values,
        ))
        .await
        .expect("种子订单插入失败");
        let item_values: Vec<Value> = vec![qty.into(), format!("W3B-{day}").into()];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"INSERT INTO sales_order_items (order_id, quantity, product_id, unit_price, subtotal)
              SELECT id, $1, 1, 0, 0 FROM sales_orders WHERE order_no = $2"#,
            item_values,
        ))
        .await
        .expect("种子明细插入失败");
    }
    db
}

/// 用**与生产一致**的谓词形态(状态取值绑定小写词表常量)复现 profit_analysis /
/// fetch_current_kpi / execute_pivot_query 共用的排除门聚合($1/$2 状态占位)。
/// 修复前该断言必然红:SQL 绑的是大写常量,排除门不命中→三单全进。
#[tokio::test]
async fn profit_shape_excludes_draft_and_cancelled_via_bound_lowercase_status() {
    let db = setup_db().await;
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!("{SHAPE_SELECT} WHERE s.status NOT IN ($1, $2)"),
        vec![sales_order::CANCELLED.into(), sales_order::DRAFT.into()],
    );
    let row: ProfitShapeRow = ProfitShapeRow::find_by_statement(stmt)
        .one(&db)
        .await
        .expect("参数绑定聚合查询失败")
        .expect("聚合应恒返回一行");
    assert_eq!(
        row.order_count,
        Some(1),
        "仅 pending 应入利润口径(draft/cancelled 进计数=排除门失守)"
    );
    assert_eq!(
        row.total_revenue,
        Some(1000.0),
        "绑定小写常量后收入必须=1000(若=6556.25 即大小写门缺陷实证)"
    );
    assert_eq!(
        row.total_cost,
        Some(50.0),
        "成本必须只含 pending 单明细(若=1550 即 draft/cancelled 误入)"
    );
}

/// 缺陷实证对照:修复前的大写字面量谓词对小写落库值恒不命中→三单全进
/// (收入 6556.25、成本 1550、订单数 3)。锁"大写比较=错误"这一因果,
/// 防止有人改写入方词表为大写来洗绿。
#[tokio::test]
async fn uppercase_literal_predicate_wrongly_includes_all_orders_defect_proof() {
    let db = setup_db().await;
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!("{SHAPE_SELECT} WHERE s.status NOT IN ('CANCELLED', 'DRAFT')"),
        Vec::<Value>::new(),
    );
    let row: ProfitShapeRow = ProfitShapeRow::find_by_statement(stmt)
        .one(&db)
        .await
        .expect("大写字面量聚合查询失败")
        .expect("聚合应恒返回一行");
    assert_eq!(row.order_count, Some(3), "原缺陷形态:三单全进");
    assert!(
        (row.total_revenue.unwrap_or_default() - 6556.25).abs() < 1e-9,
        "原缺陷收入合计应=6556.25(1000+2222.5+3333.75)"
    );
    assert_eq!(
        row.total_cost,
        Some(1550.0),
        "原缺陷成本应=1550(50+500+1000)"
    );
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

/// 防回潮源码扫描:profit.rs 7 处(含 olap.rs)不得再出现大写状态字面量,
/// 且 $N 基址须与 values 顺序一致(逐处核对后的目标形态)。
#[test]
fn source_scan_profit_and_olap_use_bound_constants_with_shifted_bases() {
    let profit = include_str!("../src/services/bi_analysis_ops/profit.rs").replace('\r', "");
    let olap = include_str!("../src/services/bi_analysis_ops/olap.rs").replace('\r', "");

    for (name, src) in [
        ("bi_analysis_ops/profit.rs", &profit),
        ("bi_analysis_ops/olap.rs", &olap),
    ] {
        assert!(
            !src.contains("NOT IN ('CANCELLED'") && !src.contains("NOT IN ('DRAFT'"),
            "{name}: SQL 不得再含大写状态字面量,应改绑定参数"
        );
        assert!(
            src.contains("sales_order::CANCELLED") && src.contains("sales_order::DRAFT"),
            "{name}: 排除门须引用词表常量 sales_order::CANCELLED / sales_order::DRAFT"
        );
        assert!(
            src.contains("NOT IN ($"),
            "{name}: 状态谓词须为 $N 绑定参数形态"
        );
    }

    // profit.rs 逐处编号锁:6 处排除门的占位基址(与 values 顺序一一对应)
    assert_eq!(
        profit.matches("NOT IN ($1, $2)").count(),
        2,
        "profit_analysis 与 fetch_current_kpi 应为 NOT IN ($1, $2)(scope 基址 $3 起)"
    );
    assert_eq!(
        profit.matches("NOT IN ($4, $5)").count(),
        2,
        "fetch_yoy_growth 两个子查询应复用 NOT IN ($4, $5)(scope 基址 $6/$7)"
    );
    assert_eq!(
        profit.matches("NOT IN ($5, $6)").count(),
        2,
        "fetch_mom_growth 两个子查询应复用 NOT IN ($5, $6)(scope 基址 $7/$8)"
    );
    assert_eq!(
        profit.matches("sales_order::CANCELLED.into()").count(),
        4,
        "profit.rs 应有 4 组状态绑定(profit/kpi/yoy/mom),values 顺序不得与占位脱钩"
    );

    // olap.rs:pivot 排除门 $1/$2,scope 基址后移到 $3
    assert!(
        olap.contains("WHERE s.status NOT IN ($1, $2)"),
        "olap.rs pivot 应为 NOT IN ($1, $2)"
    );
    assert!(
        olap.contains("self.scope_sql(\"s\", 3)"),
        "olap.rs scope 基址须为 3($1/$2 已被状态占用)"
    );
}
