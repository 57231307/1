//! 契约波次 6 · AP 状态字面量与报表口径对齐 AR 先例（任务 #191）
//!
//! 锁定的根因（与 commit a256bbb6 的 AR 修复同族）：
//! 1. **AP 报表/余额/账龄口径分裂**：AP 发票创建即写入 `common::STATUS_DRAFT`
//!    （写入方 `services/ap_invoice_ops/crud.rs:75`），而 `services/ap_report_service.rs`
//!    各聚合门只写 `invoice_status <> CANCELLED`（或未写门）⇒ 草稿应付被计入
//!    总额/未付/逾期/月初月末余额；`services/ap_invoice_ops/report.rs` 账龄与余额、
//!    `services/ap_reconciliation_ops/crud.rs|auto.rs` 对账口径同样缺 DRAFT 门。
//!    修复形态：一律 `invoice_status NOT IN ($k, $k+1)` 参数化绑定写入方词表常量
//!    （SQL 层）或 `is_not_in([CANCELLED, DRAFT])`（SeaORM 层），与 AR 先例逐字同构。
//! 2. **裸字符串字面量比较/过滤**：`ap_report_service.rs` CASE WHEN 里的
//!    `'PAID'`/`'PARTIAL_PAID'`/`'CANCELLED'`、`ap_verification_service.rs` 的
//!    `"CONFIRMED"` 比较与 `"PARTIAL_PAID"`/`"AUDITED"` 写入、
//!    `ap_reconciliation_ops/crud.rs` 的 `"CANCELLED"`、
//!    `ap_reconciliation_ops/auto.rs` 与 `handlers/ap_reconciliation_handler.rs` 的
//!    `"FAILED"` ——全部改绑该列写入方权威常量：
//!    `status::general::payment::{PAYMENT_CONFIRMED, PAYMENT_PAID, PAYMENT_PARTIAL_PAID}`、
//!    `status::general::common::{STATUS_DRAFT, STATUS_CANCELLED}`、
//!    `status::ap_invoice::INVOICE_AUDITED`（ap_invoice_ops/crud.rs:226 写入）、
//!    `status::general::reconcile_result::FAILED`（ap_reconciliation_ops/auto.rs 写入）。
//!    不新造常量、不跨域借用。
//!
//! 覆盖策略（分层）：
//! - sqlite 行为锁：调用生产 builder 得 (sql, params) 后在同构表执行，逐值断言
//!   草稿/已取消不计入；修复前旧谓词（只排 CANCELLED）作缺陷实证对照。
//! - 生产服务行为锁：`ApInvoiceService::get_balance_summary/get_aging_analysis`
//!   （纯 SeaORM 查询，sqlite 可真跑）验证 DRAFT/CANCELLED 剔除。
//! - $N 占位一致性锁：账龄 SQL（CURRENT_DATE 日期算术为 PG 语义，sqlite 不可执行）
//!   断言占位符与参数序列一一对应、常量落在正确槽位。
//! - 禁回潮源码扫描（include_str!）：AP 域文件不得再出现裸状态字面量，且必须
//!   引用写入方常量。
//! - `#[ignore]` 真库锁：统计/日报在真实 PG 上只计入 AUDITED；缺 `TEST_DATABASE_URL`
//!   显式 panic，禁止条件跳过。

use std::sync::Arc;

use bingxi_backend::models::ap_invoice;
use bingxi_backend::models::status::ap_invoice as ap_invoice_status;
use bingxi_backend::models::status::common::{STATUS_CANCELLED, STATUS_DRAFT};
use bingxi_backend::models::status::general::payment::{
    PAYMENT_CONFIRMED, PAYMENT_PAID, PAYMENT_PARTIAL_PAID, PAYMENT_REGISTERED,
};
use bingxi_backend::services::ap_invoice_service::ApInvoiceService;
use bingxi_backend::services::ap_report_service::ApReportService;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, QueryResult, Statement, Value,
};

// ===========================================================================
// 公共辅助（与 contract_wave5 先例同构）
// ===========================================================================

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("测试基准日期必须合法")
}

async fn sqlite_db() -> sea_orm::DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

async fn exec_ddl(db: &sea_orm::DatabaseConnection, sql: &'static str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

async fn run_all(
    db: &sea_orm::DatabaseConnection,
    sql: String,
    params: Vec<Value>,
) -> Vec<QueryResult> {
    db.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        params,
    ))
    .await
    .expect("sqlite 真实 SQL 执行失败")
}

fn col_i64(row: &QueryResult, idx: usize) -> i64 {
    row.try_get_by_index::<Option<i64>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 i64: {e}"))
        .unwrap_or_else(|| panic!("第 {idx} 列聚合结果不应为 NULL"))
}

fn col_f64(row: &QueryResult, idx: usize) -> f64 {
    row.try_get_by_index::<Option<f64>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 f64: {e}"))
        .unwrap_or_else(|| panic!("第 {idx} 列聚合结果不应为 NULL"))
}

fn col_str(row: &QueryResult, idx: usize) -> String {
    row.try_get_by_index::<String>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 String: {e}"))
}

/// 提取 SQL 中全部 `$N` 数字占位符
fn dollar_placeholders(sql: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let bytes = sql.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            let mut j = i + 1;
            let mut num = String::new();
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                num.push(bytes[j] as char);
                j += 1;
            }
            if !num.is_empty() {
                out.push(num.parse::<u32>().expect("占位符数字应可解析"));
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// $N 基址一致性锁：占位符必须恰好铺满 1..=params.len()，无空洞、无越界、无闲置参数
fn assert_placeholders_match_params(sql: &str, params: &[Value], ctx: &str) {
    let ph = dollar_placeholders(sql);
    assert!(
        !ph.is_empty(),
        "{ctx}: SQL 中未发现任何 $N 占位符，参数化绑定丢失"
    );
    let max_n = *ph.iter().max().expect("非空占位符列表必有最大值") as usize;
    let min_n = *ph.iter().min().expect("非空占位符列表必有最小值") as usize;
    assert_eq!(
        (min_n, max_n),
        (1, params.len()),
        "{ctx}: 占位符范围 ${min_n}..${max_n} 与参数个数 {} 不符——\
         存在 $N 撞号/漏绑/多绑（SQL 与参数序列必须一一对应）",
        params.len()
    );
    for n in 1..=max_n {
        assert!(
            ph.contains(&(n as u32)),
            "{ctx}: 占位符 ${n} 缺号——SQL 文本与参数顺序出现空洞"
        );
    }
}

fn expect_status_value(v: &Value, want: &str, ctx: &str) {
    match v {
        Value::String(Some(s)) => assert_eq!(s.as_str(), want, "{ctx}: 状态绑定值与词表常量不符"),
        other => panic!("{ctx}: 期望字符串状态参数，实际 {other:?}"),
    }
}

/// 与 `ap_report_service.rs` 报表 SQL 触达列同构的 sqlite 表（金额 REAL 保证 f64 精确断言）
async fn setup_ap_invoices(db: &sea_orm::DatabaseConnection) {
    exec_ddl(
        db,
        r#"CREATE TABLE ap_invoice (
            id INTEGER PRIMARY KEY,
            invoice_no TEXT NOT NULL,
            supplier_id INTEGER NOT NULL,
            invoice_type TEXT NOT NULL,
            invoice_date TEXT NOT NULL,
            due_date TEXT NOT NULL,
            amount REAL NOT NULL,
            paid_amount REAL NOT NULL,
            unpaid_amount REAL NOT NULL,
            invoice_status TEXT NOT NULL
        )"#,
    )
    .await;
    // 供应商 77：DRAFT/AUDITED/PARTIAL_PAID/PAID/CANCELLED 各一张，
    // 到期日 2096-12-15 早于统计基准日 2096-12-31（逾期口径五张同构，差异只在状态）
    let rows: [(&str, &str, &str, f64, f64, f64); 5] = [
        ("W6-D", "2096-12-01", STATUS_DRAFT, 2222.50, 0.00, 2222.50),
        (
            "W6-A",
            "2096-12-01",
            ap_invoice_status::INVOICE_AUDITED,
            1000.00,
            0.00,
            1000.00,
        ),
        (
            "W6-P",
            "2096-12-01",
            PAYMENT_PARTIAL_PAID,
            500.00,
            200.00,
            300.00,
        ),
        ("W6-F", "2096-12-02", PAYMENT_PAID, 400.00, 400.00, 0.00),
        (
            "W6-C",
            "2096-12-01",
            STATUS_CANCELLED,
            3333.75,
            0.00,
            3333.75,
        ),
    ];
    for (no, inv_day, status, amount, paid, unpaid) in rows {
        let itype = if no == "W6-F" { "EXPENSE" } else { "PURCHASE" };
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO ap_invoice (invoice_no, supplier_id, invoice_type, invoice_date, \
             due_date, amount, paid_amount, unpaid_amount, invoice_status) \
             VALUES ($1, 77, $2, $3, '2096-12-15', $4, $5, $6, $7)",
            vec![
                no.into(),
                itype.into(),
                inv_day.into(),
                amount.into(),
                paid.into(),
                unpaid.into(),
                status.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子应付单 {no} 插入失败: {e}"));
    }
}

async fn setup_ap_payments(db: &sea_orm::DatabaseConnection) {
    exec_ddl(
        db,
        r#"CREATE TABLE ap_payment (
            id INTEGER PRIMARY KEY,
            supplier_id INTEGER NOT NULL,
            payment_date TEXT NOT NULL,
            payment_amount REAL NOT NULL,
            payment_status TEXT NOT NULL
        )"#,
    )
    .await;
    // 同日两张：CONFIRMED 600 计入、REGISTERED 900 不计入（写入方 ap_payment_service.rs:104/253）
    for (no, status, amount) in [
        ("W6-PC", PAYMENT_CONFIRMED, 600.00_f64),
        ("W6-PR", PAYMENT_REGISTERED, 900.00_f64),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO ap_payment (supplier_id, payment_date, payment_amount, payment_status) \
             VALUES (77, '2096-12-15', $1, $2)",
            vec![amount.into(), status.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子付款单 {no} 插入失败: {e}"));
    }
}

// ===========================================================================
// 1) 统计主聚合：生产 builder + sqlite 行为锁（草稿与取消一律不计入）
// ===========================================================================

#[tokio::test]
async fn ap_main_aggregate_builder_excludes_draft_and_cancelled_on_sqlite() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) = ApReportService::build_main_aggregate_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        Some(77),
        date(2096, 12, 31),
    );
    assert_eq!(
        params.len(),
        8,
        "应绑 [start, end, CANCELLED, DRAFT, PAID, PARTIAL_PAID, supplier, today]"
    );
    expect_status_value(&params[2], STATUS_CANCELLED, "主聚合 params[2]");
    expect_status_value(&params[3], STATUS_DRAFT, "主聚合 params[3]");
    expect_status_value(&params[4], PAYMENT_PAID, "主聚合 params[4]");
    expect_status_value(&params[5], PAYMENT_PARTIAL_PAID, "主聚合 params[5]");
    assert!(
        sql.contains("invoice_status NOT IN ($3, $4)")
            && sql.contains("invoice_status = $5")
            && sql.contains("invoice_status = $6")
            && sql.contains("supplier_id = $7")
            && sql.contains("due_date < $8"),
        "主聚合排除门/分桶/日期占位基址必须按参数序顺延，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "主聚合（供应商过滤）");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(rows.len(), 1, "聚合查询应恰返回一行");
    let row = &rows[0];
    assert_eq!(
        col_i64(row, 0),
        3,
        "total_invoice_count 必须=3（=4 即草稿被计入=修复前口径，=5 即取消也被计入）"
    );
    assert_eq!(
        col_f64(row, 1),
        1900.00,
        "total_invoice_amount 只含 AUDITED/PARTIAL/PAID"
    );
    assert_eq!(col_f64(row, 2), 200.00, "total_paid_amount 只含门内三张");
    assert_eq!(col_f64(row, 3), 1300.00, "total_unpaid_amount 只含门内三张");
    assert_eq!(
        col_i64(row, 4),
        1,
        "paid_invoice_count 绑定常量后只命中 PAID"
    );
    assert_eq!(
        col_i64(row, 5),
        1,
        "partial_paid_count 绑定常量后只命中 PARTIAL_PAID"
    );
    assert_eq!(col_i64(row, 6), 2, "unpaid_count = AUDITED + PARTIAL_PAID");
    assert_eq!(
        col_i64(row, 7),
        2,
        "overdue_count 只含门内未付清两张（草稿同条件但门外）"
    );
    assert_eq!(col_f64(row, 8), 1300.00, "overdue_amount 只含门内两张");
}

#[tokio::test]
async fn ap_main_aggregate_builder_no_filter_binds_gate_and_shifts_today() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) = ApReportService::build_main_aggregate_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        None,
        date(2096, 12, 31),
    );
    assert_eq!(
        params.len(),
        7,
        "无供应商过滤时应绑 [start, end, CANCELLED, DRAFT, PAID, PARTIAL_PAID, today]"
    );
    expect_status_value(&params[3], STATUS_DRAFT, "主聚合(无过滤) params[3]");
    assert!(
        sql.contains("due_date < $7"),
        "无供应商时 today 占位应回落到 $7，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "主聚合（无过滤）");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(col_i64(&rows[0], 0), 3, "无过滤同样只计入门内三张");
}

/// 缺陷实证对照：修复前口径（只排 CANCELLED、无 DRAFT 门）在同一数据上把草稿计入统计
#[tokio::test]
async fn ap_legacy_cancelled_only_gate_wrongly_counts_draft_defect_proof() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let rows = run_all(
        &db,
        r#"SELECT COUNT(*) AS cnt, COALESCE(SUM(amount), 0) AS total
           FROM ap_invoice WHERE invoice_status <> $1"#
            .to_string(),
        vec![STATUS_CANCELLED.into()],
    )
    .await;
    assert_eq!(
        col_i64(&rows[0], 0),
        4,
        "旧门应把 DRAFT 与三张有效单一并计入"
    );
    assert_eq!(
        col_f64(&rows[0], 1),
        4122.50,
        "旧门合计=2222.50+1000+500+400"
    );
}

// ===========================================================================
// 2) 状态/类型分布聚合：DRAFT/CANCELLED 补门（与主聚合同口径）
// ===========================================================================

#[tokio::test]
async fn ap_statistics_by_status_builder_excludes_draft_and_cancelled_rows() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) = ApReportService::build_statistics_by_status_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        Some(77),
    );
    assert_eq!(
        params.len(),
        5,
        "应绑 [start, end, CANCELLED, DRAFT, supplier]"
    );
    expect_status_value(&params[3], STATUS_DRAFT, "by_status params[3]");
    assert!(
        sql.contains("invoice_status NOT IN ($3, $4)"),
        "by_status 排除门缺失，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "按状态聚合");

    let rows = run_all(&db, sql, params).await;
    let statuses: Vec<String> = rows.iter().map(|r| col_str(r, 0)).collect();
    assert!(
        !statuses.contains(&STATUS_DRAFT.to_string()),
        "状态分布不应再出现 DRAFT 行"
    );
    assert!(
        !statuses.contains(&STATUS_CANCELLED.to_string()),
        "状态分布不应再出现 CANCELLED 行"
    );
    assert_eq!(
        statuses.len(),
        3,
        "只应剩 AUDITED/PARTIAL_PAID/PAID 三个状态桶"
    );
}

#[tokio::test]
async fn ap_statistics_by_type_builder_gate_excludes_draft_amount() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) = ApReportService::build_statistics_by_type_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        None,
    );
    expect_status_value(&params[3], STATUS_DRAFT, "by_type params[3]");
    assert!(
        sql.contains("invoice_status NOT IN ($3, $4)"),
        "by_type 排除门缺失，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "按类型聚合");

    let rows = run_all(&db, sql, params).await;
    // PURCHASE 桶 = AUDITED 1000 + PARTIAL 500 = 1500（DRAFT 2222.50 与 CANCELLED 3333.75 被剔除）
    let purchase = rows
        .iter()
        .find(|r| col_str(r, 0) == "PURCHASE")
        .expect("应存在 PURCHASE 桶");
    assert_eq!(col_i64(purchase, 1), 2, "PURCHASE 桶只应剩门内两张");
    assert_eq!(col_f64(purchase, 2), 1500.00, "PURCHASE 金额不含草稿");
}

// ===========================================================================
// 3) 日报三聚合：新增/到期原本无门（补门），付款状态裸字面量改绑常量
// ===========================================================================

#[tokio::test]
async fn ap_daily_new_and_due_builders_gate_draft_and_cancelled() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) =
        ApReportService::build_daily_new_invoice_sql_and_params(date(2096, 12, 1), Some(77));
    expect_status_value(&params[1], STATUS_CANCELLED, "日报新增 params[1]");
    expect_status_value(&params[2], STATUS_DRAFT, "日报新增 params[2]");
    assert!(
        sql.contains("invoice_status NOT IN ($2, $3)"),
        "日报新增门缺失，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "日报新增");
    let rows = run_all(&db, sql, params).await;
    assert_eq!(
        col_i64(&rows[0], 0),
        2,
        "12-01 新增只应剩 AUDITED+PARTIAL（草稿/取消剔除）"
    );
    assert_eq!(col_f64(&rows[0], 1), 1500.00, "12-01 新增金额不含草稿");

    let (sql2, params2) =
        ApReportService::build_daily_due_invoice_sql_and_params(date(2096, 12, 15), None);
    expect_status_value(&params2[1], STATUS_CANCELLED, "日报到期 params[1]");
    expect_status_value(&params2[2], STATUS_DRAFT, "日报到期 params[2]");
    assert!(
        sql2.contains("invoice_status NOT IN ($2, $3)"),
        "日报到期门缺失，实际 SQL:\n{sql2}"
    );
    assert_placeholders_match_params(&sql2, &params2, "日报到期");
    let rows2 = run_all(&db, sql2, params2).await;
    assert_eq!(col_i64(&rows2[0], 0), 2, "到期未付只应剩 AUDITED+PARTIAL");
    assert_eq!(
        col_f64(&rows2[0], 1),
        1300.00,
        "到期未付金额不含草稿/已取消"
    );
}

#[tokio::test]
async fn ap_daily_payment_builder_binds_writer_confirmed_constant() {
    let db = sqlite_db().await;
    setup_ap_payments(&db).await;

    let (sql, params) =
        ApReportService::build_daily_payment_sql_and_params(date(2096, 12, 15), Some(77));
    assert_eq!(params.len(), 3, "应绑 [date, PAYMENT_CONFIRMED, supplier]");
    expect_status_value(&params[1], PAYMENT_CONFIRMED, "日报付款 params[1]");
    assert!(
        sql.contains("payment_status = $2"),
        "付款状态应仍为参数绑定而非内联字面量，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "日报付款");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(
        col_i64(&rows[0], 0),
        1,
        "只计 CONFIRMED 一张（REGISTERED 未确认不计）"
    );
    assert_eq!(col_f64(&rows[0], 1), 600.00, "付款金额只含已确认");
}

// ===========================================================================
// 4) 月报月初/月末余额：补 DRAFT 门（余额类）
// ===========================================================================

#[tokio::test]
async fn ap_balance_builder_excludes_draft_from_opening_closing_balance() {
    let db = sqlite_db().await;
    setup_ap_invoices(&db).await;

    let (sql, params) =
        ApReportService::build_balance_sql_and_params(Some(77), date(2096, 12, 31), "<=");
    assert_eq!(
        params.len(),
        4,
        "应绑 [CANCELLED, DRAFT, boundary, supplier]"
    );
    expect_status_value(&params[0], STATUS_CANCELLED, "余额 params[0]");
    expect_status_value(&params[1], STATUS_DRAFT, "余额 params[1]");
    assert!(
        sql.contains("invoice_status NOT IN ($1, $2)") && sql.contains("invoice_date <= $3"),
        "余额门/边界日占位不符，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "余额聚合（月末）");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(
        col_f64(&rows[0], 0),
        1300.00,
        "未付余额只含 AUDITED+PARTIAL，草稿不得计入"
    );
}

// ===========================================================================
// 5) 账龄 SQL：CURRENT_DATE 减法为 PG 专属语义，sqlite 只锁 $N 与常量槽位
// ===========================================================================

#[tokio::test]
async fn ap_aging_builders_bind_all_status_constants_with_shifted_placeholders() {
    for builder in [
        ApReportService::build_aging_overdue_sql_and_params
            as fn(NaiveDate, Option<i32>) -> (String, Vec<Value>),
        ApReportService::build_aging_not_due_sql_and_params,
    ] {
        // 无过滤形态：[today, PAID, CANCELLED, DRAFT]
        let (sql, params) = builder(date(2096, 12, 31), None);
        assert_eq!(params.len(), 4);
        expect_status_value(&params[1], PAYMENT_PAID, "账龄 params[1]");
        expect_status_value(&params[2], STATUS_CANCELLED, "账龄 params[2]");
        expect_status_value(&params[3], STATUS_DRAFT, "账龄 params[3]");
        assert!(
            sql.contains("invoice_status NOT IN ($2, $3, $4)"),
            "账龄排除门必须 NOT IN 三态（PAID/CANCELLED/DRAFT），实际 SQL:\n{sql}"
        );
        assert_placeholders_match_params(&sql, &params, "账龄（无过滤）");

        // 供应商过滤形态：supplier 顺延为 $5
        let (sql2, params2) = builder(date(2096, 12, 31), Some(77));
        assert_eq!(params2.len(), 5);
        assert!(
            sql2.contains("supplier_id = $5"),
            "账龄 supplier 占位必须顺延至 $5，实际 SQL:\n{sql2}"
        );
        assert_placeholders_match_params(&sql2, &params2, "账龄（供应商过滤）");
    }
}

// ===========================================================================
// 6) 生产服务行为锁：ApInvoiceService 余额汇总/账龄分析（纯 SeaORM，sqlite 真跑）
// ===========================================================================

/// 与 `models/ap_invoice.rs` 全列同构的 sqlite 表（Entity 全列 SELECT 解码需要）
async fn setup_full_ap_invoice_table(db: &sea_orm::DatabaseConnection) {
    exec_ddl(
        db,
        r#"CREATE TABLE ap_invoice (
            id INTEGER PRIMARY KEY,
            invoice_no TEXT NOT NULL,
            supplier_id INTEGER NOT NULL,
            invoice_type TEXT NOT NULL,
            source_type TEXT,
            source_id INTEGER,
            invoice_date TEXT NOT NULL,
            due_date TEXT NOT NULL,
            payment_terms INTEGER NOT NULL,
            amount NUMERIC NOT NULL,
            paid_amount NUMERIC NOT NULL,
            unpaid_amount NUMERIC NOT NULL,
            invoice_status TEXT NOT NULL,
            currency TEXT NOT NULL,
            exchange_rate NUMERIC NOT NULL,
            amount_foreign NUMERIC,
            tax_amount NUMERIC NOT NULL,
            notes TEXT,
            attachment_urls TEXT,
            created_by INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_by INTEGER,
            updated_at TEXT NOT NULL,
            approved_by INTEGER,
            approved_at TEXT,
            cancelled_by INTEGER,
            cancelled_at TEXT,
            cancelled_reason TEXT
        )"#,
    )
    .await;
}

async fn seed_invoice(
    db: &sea_orm::DatabaseConnection,
    no: &str,
    status: &str,
    amount: Decimal,
    paid: Decimal,
    unpaid: Decimal,
) {
    let now = Utc::now();
    ap_invoice::ActiveModel {
        invoice_no: Set(no.to_string()),
        supplier_id: Set(77),
        invoice_type: Set("PURCHASE".to_string()),
        invoice_date: Set(date(2096, 12, 1)),
        due_date: Set(date(2000, 1, 31)), // 远超 180 天 → 账龄确定性落入最老桶
        payment_terms: Set(30),
        amount: Set(amount),
        paid_amount: Set(paid),
        unpaid_amount: Set(unpaid),
        invoice_status: Set(status.to_string()),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        tax_amount: Set(Decimal::ZERO),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子应付单 {no} 插入失败: {e}"));
}

#[tokio::test]
async fn ap_invoice_service_balance_summary_excludes_draft_and_cancelled() {
    let db = sqlite_db().await;
    setup_full_ap_invoice_table(&db).await;
    seed_invoice(
        &db,
        "W6E-D",
        STATUS_DRAFT,
        dec!(2222.50),
        Decimal::ZERO,
        dec!(2222.50),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-A",
        ap_invoice_status::INVOICE_AUDITED,
        dec!(1000.00),
        Decimal::ZERO,
        dec!(1000.00),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-P",
        PAYMENT_PARTIAL_PAID,
        dec!(500.00),
        dec!(200.00),
        dec!(300.00),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-F",
        PAYMENT_PAID,
        dec!(400.00),
        dec!(400.00),
        Decimal::ZERO,
    )
    .await;
    seed_invoice(
        &db,
        "W6E-C",
        STATUS_CANCELLED,
        dec!(3333.75),
        Decimal::ZERO,
        dec!(3333.75),
    )
    .await;

    let svc = ApInvoiceService::new(Arc::new(db));
    let summary = svc
        .get_balance_summary(Some(77))
        .await
        .expect("get_balance_summary sqlite 执行失败");
    assert_eq!(
        summary.invoice_count, 3,
        "余额汇总只应剩 AUDITED/PARTIAL/PAID 三张"
    );
    assert_eq!(
        summary.total_invoice_amount,
        dec!(1900.00),
        "草稿 2222.50 不得计入应付总额"
    );
    assert_eq!(summary.total_paid_amount, dec!(600.00));
    assert_eq!(
        summary.total_unpaid_amount,
        dec!(1300.00),
        "草稿/已取消未付不得抬高等待付款"
    );
}

#[tokio::test]
async fn ap_invoice_service_aging_analysis_excludes_draft_rows() {
    let db = sqlite_db().await;
    setup_full_ap_invoice_table(&db).await;
    seed_invoice(
        &db,
        "W6E-D",
        STATUS_DRAFT,
        dec!(2222.50),
        Decimal::ZERO,
        dec!(2222.50),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-A",
        ap_invoice_status::INVOICE_AUDITED,
        dec!(1000.00),
        Decimal::ZERO,
        dec!(1000.00),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-P",
        PAYMENT_PARTIAL_PAID,
        dec!(500.00),
        dec!(200.00),
        dec!(300.00),
    )
    .await;
    seed_invoice(
        &db,
        "W6E-C",
        STATUS_CANCELLED,
        dec!(3333.75),
        Decimal::ZERO,
        dec!(3333.75),
    )
    .await;

    let svc = ApInvoiceService::new(Arc::new(db));
    let buckets = svc
        .get_aging_analysis(Some(77))
        .await
        .expect("get_aging_analysis sqlite 执行失败");
    assert_eq!(buckets.len(), 1, "所有门内行同落最老桶");
    assert_eq!(buckets[0].aging_bucket, "逾期 180 天以上");
    assert_eq!(
        buckets[0].invoice_count, 2,
        "账龄只应剩 AUDITED+PARTIAL（=3 即草稿被计入）"
    );
    assert_eq!(buckets[0].total_amount, dec!(1300.00));
}

// ===========================================================================
// 7) 禁回潮源码扫描（include_str!）：AP 域不得再出现裸状态字面量
// ===========================================================================

fn forbidden(src: &str, needles: &[&str], ctx: &str) {
    for n in needles {
        assert!(
            !src.contains(n),
            "{ctx}: 出现被禁的裸状态字面量 {n}——比较/门必须改绑写入方词表常量"
        );
    }
}

fn required(src: &str, needles: &[&str], ctx: &str) {
    for n in needles {
        assert!(src.contains(n), "{ctx}: 应包含 {n}（修复形态回潮丢失）");
    }
}

#[test]
fn source_scan_ap_report_service_no_bare_status_literals_and_gates_bound() {
    let src = include_str!("../src/services/ap_report_service.rs").replace('\r', "");
    forbidden(
        &src,
        &[
            "'PAID'",
            "'PARTIAL_PAID'",
            "'CANCELLED'",
            "'DRAFT'",
            "'CONFIRMED'",
            "\"CANCELLED\"",
            "\"DRAFT\"",
            "\"CONFIRMED\"",
            "\"PAID\"",
            "\"PARTIAL_PAID\"",
        ],
        "ap_report_service.rs",
    );
    required(
        &src,
        &[
            "common::STATUS_DRAFT",
            "common::STATUS_CANCELLED",
            "payment::PAYMENT_PAID",
            "payment::PAYMENT_PARTIAL_PAID",
            "payment::PAYMENT_CONFIRMED",
            "invoice_status NOT IN ($3, $4)",
            "invoice_status NOT IN ($2, $3)",
            "invoice_status NOT IN ($1, $2)",
            "invoice_status NOT IN ($2, $3, $4)",
        ],
        "ap_report_service.rs",
    );
}

#[test]
fn source_scan_ap_invoice_ops_report_uses_seaorm_not_in_with_draft() {
    let src = include_str!("../src/services/ap_invoice_ops/report.rs").replace('\r', "");
    forbidden(
        &src,
        &["\"DRAFT\"", "\"CANCELLED\"", "\"PAID\"", ".ne("],
        "ap_invoice_ops/report.rs",
    );
    required(
        &src,
        &[
            "is_not_in",
            "crate::models::status::common::STATUS_DRAFT",
            "crate::models::status::common::STATUS_CANCELLED",
            "crate::models::status::payment::PAYMENT_PAID",
        ],
        "ap_invoice_ops/report.rs",
    );
}

#[test]
fn source_scan_ap_verification_binds_writer_constants() {
    let src = include_str!("../src/services/ap_verification_service.rs").replace('\r', "");
    forbidden(
        &src,
        &[
            "\"CONFIRMED\"",
            "\"PARTIAL_PAID\"",
            "\"AUDITED\"",
            "\"CANCELLED\"",
            "\"DRAFT\"",
        ],
        "ap_verification_service.rs",
    );
    required(
        &src,
        &[
            "payment::PAYMENT_CONFIRMED",
            "payment::PAYMENT_PARTIAL_PAID",
            "ap_invoice::INVOICE_AUDITED",
            "is_not_in",
            "common::STATUS_DRAFT",
            "common::STATUS_CANCELLED",
        ],
        "ap_verification_service.rs",
    );
}

#[test]
fn source_scan_ap_reconciliation_crud_auto_handler_use_constants() {
    let crud = include_str!("../src/services/ap_reconciliation_ops/crud.rs").replace('\r', "");
    forbidden(
        &crud,
        &["\"CANCELLED\"", "\"DRAFT\""],
        "ap_reconciliation_ops/crud.rs",
    );
    required(
        &crud,
        &[
            "is_not_in",
            "common::STATUS_DRAFT",
            "common::STATUS_CANCELLED",
            "payment::PAYMENT_CONFIRMED",
        ],
        "ap_reconciliation_ops/crud.rs",
    );

    let auto = include_str!("../src/services/ap_reconciliation_ops/auto.rs").replace('\r', "");
    forbidden(
        &auto,
        &["\"FAILED\"", "\"CANCELLED\"", "\"DRAFT\"", "\"CONFIRMED\""],
        "ap_reconciliation_ops/auto.rs",
    );
    required(
        &auto,
        &[
            "reconcile_result::FAILED",
            "general::common::STATUS_DRAFT",
            "general::common::STATUS_CANCELLED",
            "general::payment::PAYMENT_CONFIRMED",
            "is_not_in",
        ],
        "ap_reconciliation_ops/auto.rs",
    );

    let handler = include_str!("../src/handlers/ap_reconciliation_handler.rs").replace('\r', "");
    forbidden(
        &handler,
        &["\"FAILED\""],
        "handlers/ap_reconciliation_handler.rs",
    );
    required(
        &handler,
        &["reconcile_result::FAILED"],
        "handlers/ap_reconciliation_handler.rs",
    );
}

#[test]
fn source_scan_ap_vocabulary_source_stays_in_writer_tables() {
    // AP 状态词表唯一来源核查：写入方模型注释声明的五值词表不因修复被英文化/新增第二套常量
    let status_finance = include_str!("../src/models/status/finance.rs").replace('\r', "");
    required(
        &status_finance,
        &[
            "pub const INVOICE_AUDITED: &str = \"AUDITED\";",
            "pub const APPROVAL_APPROVING: &str = \"APPROVING\";",
        ],
        "models/status/finance.rs",
    );
    let status_general = include_str!("../src/models/status/general.rs").replace('\r', "");
    required(
        &status_general,
        &[
            "pub const STATUS_DRAFT: &str = \"DRAFT\";",
            "pub const STATUS_CANCELLED: &str = \"CANCELLED\";",
            "pub const PAYMENT_PAID: &str = \"PAID\";",
            "pub const PAYMENT_PARTIAL_PAID: &str = \"PARTIAL_PAID\";",
            "pub const PAYMENT_CONFIRMED: &str = \"CONFIRMED\";",
        ],
        "models/status/general.rs",
    );
}

// ===========================================================================
// 8) #[ignore] 真库锁（PG：占位语义 + 端到端报表值）
// ===========================================================================

/// 需要真实 Postgres（`TEST_DATABASE_URL`，已迁移库）：给既有供应商种
/// DRAFT/AUDITED/CANCELLED 三张 2096-12 窗口的应付单（未来窗口天然隔离既有数据），
/// 统计/日报只应计入 AUDITED。运行：
/// `cargo test --test contract_wave6_ap_status_literal_parity_test -- --ignored --nocapture`
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移的真实 Postgres（sqlite 不支持的 PG 占位/日期语义在此锁）"]
async fn ap_reports_on_real_db_count_only_audited() {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL 未设置——真库行为锁必须显式失败，禁止条件跳过假绿");
    let db = Arc::new(
        sea_orm::Database::connect(&url)
            .await
            .expect("连接 TEST_DATABASE_URL 失败"),
    );

    let sid_row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM suppliers ORDER BY id LIMIT 1".to_string(),
        ))
        .await
        .expect("查询 suppliers 失败")
        .expect("已迁移库应存在至少一个供应商（seed 数据）");
    let supplier_id: i32 = sid_row.try_get_by_index(0).expect("supplier id 应为 i32");

    let seed_prefix = "W6APPARITY-";
    let seed_pattern = format!("{seed_prefix}%");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM ap_invoice WHERE invoice_no LIKE $1",
        vec![seed_pattern.clone().into()],
    ))
    .await
    .expect("清理残留种子失败");

    let approved_amt = dec!(1000.00);
    let seeds: [(&str, &str, Decimal); 3] = [
        ("DRAFT", STATUS_DRAFT, dec!(2222.50)),
        ("AUDITED", ap_invoice_status::INVOICE_AUDITED, approved_amt),
        ("CANCELLED", STATUS_CANCELLED, dec!(3333.75)),
    ];
    for (suffix, status, amount) in seeds {
        ap_invoice::ActiveModel {
            invoice_no: Set(format!("{seed_prefix}{suffix}")),
            supplier_id: Set(supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("MANUAL".to_string())),
            source_id: Set(None),
            invoice_date: Set(date(2096, 12, 5)),
            due_date: Set(date(2096, 12, 15)),
            payment_terms: Set(30),
            amount: Set(amount),
            paid_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(amount),
            invoice_status: Set(status.to_string()),
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::ONE),
            tax_amount: Set(Decimal::ZERO),
            created_by: Set(1),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .expect("真库种子应付单插入失败");
    }

    let svc = ApReportService::new(db.clone());

    // 1) 统计报表（未来窗口隔离既有数据；主聚合/按状态/按类型全部走修复后的 NOT IN 门）
    let stats = svc
        .get_statistics_report(Some(supplier_id), date(2096, 12, 1), date(2096, 12, 31))
        .await
        .expect("get_statistics_report 真库执行失败");
    assert_eq!(stats.total_invoice_count, 1, "统计只应计入 AUDITED 一张");
    assert_eq!(stats.total_invoice_amount, approved_amt);
    assert_eq!(stats.total_unpaid_amount, approved_amt);
    assert_eq!(
        stats.overdue_count, 0,
        "基准窗口内到期日晚于今天（2096 未到期）"
    );
    assert_eq!(
        stats
            .by_status
            .iter()
            .map(|s| s.status.as_str())
            .collect::<Vec<_>>(),
        vec![ap_invoice_status::INVOICE_AUDITED],
        "状态分布不应出现 DRAFT/CANCELLED"
    );

    // 2) 日报（新增聚合补门后：12-05 只应剩 AUDITED）
    let daily = svc
        .get_daily_report(date(2096, 12, 5), Some(supplier_id))
        .await
        .expect("get_daily_report 真库执行失败");
    assert_eq!(daily.new_invoice_count, 1, "日报新增只应计入 AUDITED");
    assert_eq!(daily.new_invoice_amount, approved_amt);

    // 清理，保持幂等
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM ap_invoice WHERE invoice_no LIKE $1",
        vec![seed_pattern.into()],
    ))
    .await
    .expect("清理种子失败");
}
