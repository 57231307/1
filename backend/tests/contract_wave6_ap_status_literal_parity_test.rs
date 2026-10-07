//! AP 状态字面量与报表口径对齐 AR 先例的契约锁
//!
//! 锁定的根因（AR 域已有同族口径）：
//! 1. **AP 报表/余额/账龄口径分裂风险**：AP 发票创建即写入 `common::STATUS_DRAFT`
//!    （写入方 `services/ap_invoice_ops/crud.rs:75`），聚合门若只排除 CANCELLED（或不写门）
//!    ⇒ 草稿应付被计入总额/未付/逾期/月初月末余额；`services/ap_invoice_ops/report.rs`
//!    账龄与余额、`services/ap_reconciliation_ops/crud.rs|auto.rs` 对账口径同受 DRAFT 门约束。
//!    锁定形态：一律 `invoice_status NOT IN ($k, $k+1)` 参数化绑定写入方词表常量
//!    （SQL 层）或 `is_not_in([CANCELLED, DRAFT])`（SeaORM 层），与 AR 先例逐字同构。
//! 2. **裸状态字面量禁回潮**：`ap_report_service.rs` CASE WHEN 的 PAID/PARTIAL_PAID/
//!    CANCELLED、`ap_verification_service.rs` 的 CONFIRMED 比较与 PARTIAL_PAID/AUDITED
//!    写入、`ap_reconciliation_ops/crud.rs` 的 CANCELLED、
//!    `ap_reconciliation_ops/auto.rs` 与 `handlers/ap_reconciliation_handler.rs` 的
//!    FAILED——各处比较/写入均绑该列写入方权威常量：
//!    `status::general::payment::{PAYMENT_CONFIRMED, PAYMENT_PAID, PAYMENT_PARTIAL_PAID}`、
//!    `status::general::common::{STATUS_DRAFT, STATUS_CANCELLED}`、
//!    `status::ap_invoice::INVOICE_AUDITED`（ap_invoice_ops/crud.rs:226 写入）、
//!    `status::general::reconcile_result::FAILED`（ap_reconciliation_ops/auto.rs 写入）。
//!    不新造常量、不跨域借用。
//!
//! 覆盖策略（分层）：
//! - 真库行为锁：调用生产 builder 得 (sql, params) 后在
//!   **已迁移 PostgreSQL** 的真实 `ap_invoice`/`ap_payment` 表上执行，逐值断言
//!   草稿/已取消不计入；另以旧谓词（只排 CANCELLED）作对照，证明其会把草稿计入统计。
//!   表结构唯一来源 = `backend/migration`，本文件不自建同构 DDL。
//! - 生产服务行为锁：`ApInvoiceService::get_balance_summary/get_aging_analysis`
//!   （纯 SeaORM 查询）验证 DRAFT/CANCELLED 剔除。
//! - $N 占位一致性锁：账龄 SQL（含 CURRENT_DATE 日期算术，PG 语义）只断言 SQL 文本、
//!   占位符与参数序列一一对应、常量落在正确槽位（该段 SQL 不在此执行）；
//!   端到端口径见第 8 节 `#[ignore]` 活库锁。
//! - 禁回潮源码扫描（include_str!）：AP 域文件不得再出现裸状态字面量，且必须
//!   引用写入方常量。
//! - `#[ignore]` 真库锁：统计/日报在真实 PG 上只计入 AUDITED；缺 `TEST_DATABASE_URL`
//!   由夹具直接 panic，禁止条件跳过。
//!
//! 注意：用例名中的 `on_sqlite` 与实际通道不符——当前在真库（PostgreSQL）执行，
//! 语义以本注释与用例体为准（名称保留以维持 CI 用例可追溯）。

mod test_common;

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
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend,
    QueryResult, Statement, Value,
};

// ===========================================================================
// 公共辅助（与 contract_wave5 先例同构）
// ===========================================================================

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("测试基准日期必须合法")
}

/// 真库连接（已迁移 PostgreSQL + 清空业务表）；缺变量/指 sqlite 由夹具直接 panic
async fn live_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

/// 外键父行 `suppliers`：迁移种子参照表（m0015 播种、不参与清空、不删其数据），
/// 取其真实首个 id 作 `ap_invoice.supplier_id`/`ap_payment.supplier_id` 的父行，
/// 不硬编码常量、不指望业务表里已有数据。
async fn seeded_supplier_id(db: &DatabaseConnection) -> i32 {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM suppliers ORDER BY id LIMIT 1".to_string(),
        ))
        .await
        .expect("夹具：读取 suppliers 参照表失败")
        .unwrap_or_else(|| panic!("夹具：迁移未播种 suppliers 参照表（m0015），FK 父行缺失"));
    row.try_get_by_index::<i32>(0)
        .expect("夹具：suppliers.id 应可解码为 i32")
}

async fn run_all(db: &DatabaseConnection, sql: String, params: Vec<Value>) -> Vec<QueryResult> {
    db.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        params,
    ))
    .await
    .unwrap_or_else(|e| panic!("真库 SQL 执行失败: {e}"))
}

fn col_i64(row: &QueryResult, idx: usize) -> i64 {
    row.try_get_by_index::<Option<i64>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 i64: {e}"))
        .unwrap_or_else(|| panic!("第 {idx} 列聚合结果不应为 NULL"))
}

/// 金额列解码：真库 `ap_invoice.amount/paid_amount/unpaid_amount` 是 DECIMAL(18,2)、
/// `COALESCE(SUM(...), 0)` 在 PG 返回 numeric ⇒ 必须按 Decimal 解码（与生产侧
/// `fetch_ap_statistics_*` 的 `try_get_by_index::<Decimal>` 同口径；sqlite 时代按
/// REAL/f64 读是方言失真，正是 解码红的那一族）。
fn col_decimal(row: &QueryResult, idx: usize) -> Decimal {
    row.try_get_by_index::<Option<Decimal>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 Decimal: {e}"))
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

/// 真表 `ap_invoice` 种子（供应商 X：DRAFT/AUDITED/PARTIAL_PAID/PAID/CANCELLED 各一张）。
///
/// 逐列对照 `models/ap_invoice.rs` + 迁移建表语句 `m0012_add_ap_ar_finance_analysis.rs:18`：
/// NOT NULL 且无 DB 默认值的列全部给业务合法值（invoice_no/supplier_id/invoice_type/
/// invoice_date/due_date/amount/created_by）；金额列按 `Decimal` 绑定（真列是
/// DECIMAL(18,2)，原 sqlite 夹具写成 REAL 并绑 f64，即 解码红的根因形态）。
async fn setup_ap_invoices(db: &DatabaseConnection, supplier_id: i32) {
    // 到期日 2096-12-15 早于统计基准日 2096-12-31（逾期口径五张同构，差异只在状态）
    let rows: [(&str, NaiveDate, &str, Decimal, Decimal, Decimal); 5] = [
        (
            "W6-D",
            date(2096, 12, 1),
            STATUS_DRAFT,
            dec!(2222.50),
            Decimal::ZERO,
            dec!(2222.50),
        ),
        (
            "W6-A",
            date(2096, 12, 1),
            ap_invoice_status::INVOICE_AUDITED,
            dec!(1000.00),
            Decimal::ZERO,
            dec!(1000.00),
        ),
        (
            "W6-P",
            date(2096, 12, 1),
            PAYMENT_PARTIAL_PAID,
            dec!(500.00),
            dec!(200.00),
            dec!(300.00),
        ),
        (
            "W6-F",
            date(2096, 12, 2),
            PAYMENT_PAID,
            dec!(400.00),
            dec!(400.00),
            Decimal::ZERO,
        ),
        (
            "W6-C",
            date(2096, 12, 1),
            STATUS_CANCELLED,
            dec!(3333.75),
            Decimal::ZERO,
            dec!(3333.75),
        ),
    ];
    for (no, inv_day, status, amount, paid, unpaid) in rows {
        let itype = if no == "W6-F" { "EXPENSE" } else { "PURCHASE" };
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO ap_invoice (invoice_no, supplier_id, invoice_type, invoice_date, \
             due_date, amount, paid_amount, unpaid_amount, invoice_status, created_by) \
             VALUES ($1, $2, $3, $4, '2096-12-15', $5, $6, $7, $8, $9)",
            vec![
                no.into(),
                supplier_id.into(),
                itype.into(),
                // DATE 列必须按日期类型绑定（PG 不会把 text 参数隐式赋给 date 列,
                // 原 sqlite 通道绑 'YYYY-MM-DD' 字符串是方言纵容）
                inv_day.into(),
                amount.into(),
                paid.into(),
                unpaid.into(),
                status.into(),
                // created_by 为 NOT NULL 列但真库无外键约束（仅 sales_orders.created_by
                // 有 FK），取与报表出参无关的操作人常量，不伪造归属语义
                1i32.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子应付单 {no} 插入失败: {e}"));
    }
}

/// 真表 `ap_payment` 种子（同日两张：CONFIRMED 600 计入、REGISTERED 900 不计入）。
///
/// 逐列对照 `models/ap_payment.rs` + `m0012:108`：NOT NULL 且无默认值的列
/// payment_no/payment_date/supplier_id/payment_method/payment_amount/created_by 全部给值；
/// `payment_method` 取写入方词表（模型列注释 TT/LC/DP/DA/CHECK/CASH）内的合法值。
async fn setup_ap_payments(db: &DatabaseConnection, supplier_id: i32) {
    // 写入方 ap_payment_service.rs:104/253：REGISTERED 登记、CONFIRMED 确认
    for (no, status, amount) in [
        ("W6-PC", PAYMENT_CONFIRMED, dec!(600.00)),
        ("W6-PR", PAYMENT_REGISTERED, dec!(900.00)),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO ap_payment (payment_no, supplier_id, payment_date, payment_method, \
             payment_amount, payment_status, created_by) \
             VALUES ($1, $2, '2096-12-15', 'TT', $3, $4, $5)",
            vec![
                format!("AP{no}").into(),
                supplier_id.into(),
                amount.into(),
                status.into(),
                1i32.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子付款单 {no} 插入失败: {e}"));
    }
}

// ===========================================================================
// 1) 统计主聚合：生产 builder + 真库行为锁（草稿与取消一律不计入）
// ===========================================================================

#[tokio::test]
async fn ap_main_aggregate_builder_excludes_draft_and_cancelled_on_sqlite() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

    let (sql, params) = ApReportService::build_main_aggregate_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        Some(sid),
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
        col_decimal(row, 1),
        dec!(1900.00),
        "total_invoice_amount 只含 AUDITED/PARTIAL/PAID"
    );
    // total_paid_amount = SUM(paid_amount) 在门内三张上 = 0(AUDITED)+200(PARTIAL)+400(PAID)
    // = 600。原期望 200 是把"只统计部分付款那张"误当成该列口径的自身算错
    // （CI 已实测 left: 600.0，SQL 见 ap_report_service.rs:96）；
    // 该列在门内/门外五张上的差异为 0（草稿/取消的 paid_amount 均为 0），
    // 真正区分 DRAFT 门的是 count/总额/未付额三列，下面各自的断言原样保留。
    assert_eq!(
        col_decimal(row, 2),
        dec!(600.00),
        "total_paid_amount 只含门内三张"
    );
    assert_eq!(
        col_decimal(row, 3),
        dec!(1300.00),
        "total_unpaid_amount 只含门内三张"
    );
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
    assert_eq!(
        col_decimal(row, 8),
        dec!(1300.00),
        "overdue_amount 只含门内两张"
    );
}

#[tokio::test]
async fn ap_main_aggregate_builder_no_filter_binds_gate_and_shifts_today() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

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

/// 缺陷实证对照：旧谓词（只排 CANCELLED、无 DRAFT 门）在同一数据上把草稿计入统计
#[tokio::test]
async fn ap_legacy_cancelled_only_gate_wrongly_counts_draft_defect_proof() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

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
        col_decimal(&rows[0], 1),
        dec!(4122.50),
        "旧门合计=2222.50+1000+500+400"
    );
}

// ===========================================================================
// 2) 状态/类型分布聚合：DRAFT/CANCELLED 补门（与主聚合同口径）
// ===========================================================================

#[tokio::test]
async fn ap_statistics_by_status_builder_excludes_draft_and_cancelled_rows() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

    let (sql, params) = ApReportService::build_statistics_by_status_sql_and_params(
        date(2096, 12, 1),
        date(2096, 12, 31),
        Some(sid),
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
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

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
    assert_eq!(
        col_decimal(purchase, 2),
        dec!(1500.00),
        "PURCHASE 金额不含草稿"
    );
}

// ===========================================================================
// 3) 日报三聚合：新增/到期原本无门（补门），付款状态裸字面量改绑常量
// ===========================================================================

#[tokio::test]
async fn ap_daily_new_and_due_builders_gate_draft_and_cancelled() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

    let (sql, params) =
        ApReportService::build_daily_new_invoice_sql_and_params(date(2096, 12, 1), Some(sid));
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
    assert_eq!(
        col_decimal(&rows[0], 1),
        dec!(1500.00),
        "12-01 新增金额不含草稿"
    );

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
        col_decimal(&rows2[0], 1),
        dec!(1300.00),
        "到期未付金额不含草稿/已取消"
    );
}

#[tokio::test]
async fn ap_daily_payment_builder_binds_writer_confirmed_constant() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_payments(&db, sid).await;

    let (sql, params) =
        ApReportService::build_daily_payment_sql_and_params(date(2096, 12, 15), Some(sid));
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
    assert_eq!(col_decimal(&rows[0], 1), dec!(600.00), "付款金额只含已确认");
}

// ===========================================================================
// 4) 月报月初/月末余额：补 DRAFT 门（余额类）
// ===========================================================================

#[tokio::test]
async fn ap_balance_builder_excludes_draft_from_opening_closing_balance() {
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    setup_ap_invoices(&db, sid).await;

    let (sql, params) =
        ApReportService::build_balance_sql_and_params(Some(sid), date(2096, 12, 31), "<=");
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
        col_decimal(&rows[0], 0),
        dec!(1300.00),
        "未付余额只含 AUDITED+PARTIAL，草稿不得计入"
    );
}

// ===========================================================================
// 5) 账龄 SQL：本用例只锁 $N 占位与常量槽位（不执行含 CURRENT_DATE 减法的该段 SQL）；
//    真库端到端的账龄/统计口径见第 8 节的 #[ignore] 活库锁
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

        // 供应商过滤形态：supplier 顺延为 $5（该形态只做 SQL 文本/参数序断言，不触库，
        // 故这里的 77 只是占位实参、不是外键引用）
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
// 6) 生产服务行为锁：ApInvoiceService 余额汇总/账龄分析（纯 SeaORM，真库真跑）
// ===========================================================================

async fn seed_invoice(
    db: &DatabaseConnection,
    supplier_id: i32,
    no: &str,
    status: &str,
    amount: Decimal,
    paid: Decimal,
    unpaid: Decimal,
) {
    let now = Utc::now();
    ap_invoice::ActiveModel {
        invoice_no: Set(no.to_string()),
        supplier_id: Set(supplier_id),
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
        // 与第 1~4 节同一口径：ap_invoice.created_by 真库无外键约束（仅
        // sales_orders.created_by 有 FK），此处为操作人常量，不伪造归属语义
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
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    seed_invoice(
        &db,
        sid,
        "W6E-D",
        STATUS_DRAFT,
        dec!(2222.50),
        Decimal::ZERO,
        dec!(2222.50),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-A",
        ap_invoice_status::INVOICE_AUDITED,
        dec!(1000.00),
        Decimal::ZERO,
        dec!(1000.00),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-P",
        PAYMENT_PARTIAL_PAID,
        dec!(500.00),
        dec!(200.00),
        dec!(300.00),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-F",
        PAYMENT_PAID,
        dec!(400.00),
        dec!(400.00),
        Decimal::ZERO,
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-C",
        STATUS_CANCELLED,
        dec!(3333.75),
        Decimal::ZERO,
        dec!(3333.75),
    )
    .await;

    let svc = ApInvoiceService::new(Arc::new(db));
    let summary = svc
        .get_balance_summary(Some(sid))
        .await
        .expect("get_balance_summary 真库执行失败");
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
    let db = live_db().await;
    let sid = seeded_supplier_id(&db).await;
    seed_invoice(
        &db,
        sid,
        "W6E-D",
        STATUS_DRAFT,
        dec!(2222.50),
        Decimal::ZERO,
        dec!(2222.50),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-A",
        ap_invoice_status::INVOICE_AUDITED,
        dec!(1000.00),
        Decimal::ZERO,
        dec!(1000.00),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-P",
        PAYMENT_PARTIAL_PAID,
        dec!(500.00),
        dec!(200.00),
        dec!(300.00),
    )
    .await;
    seed_invoice(
        &db,
        sid,
        "W6E-C",
        STATUS_CANCELLED,
        dec!(3333.75),
        Decimal::ZERO,
        dec!(3333.75),
    )
    .await;

    let svc = ApInvoiceService::new(Arc::new(db));
    let buckets = svc
        .get_aging_analysis(Some(sid))
        .await
        .expect("get_aging_analysis 真库执行失败");
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

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// 本节的判据是"裸状态字面量不得出现在**比较/门**里"与"必须绑写入方常量"，两者都
/// 只有执行体意义：`ap_report_service.rs:375` 的文档注释里正当写着
/// 「…绑 `payment::PAYMENT_CONFIRMED`，禁止裸 "CONFIRMED" 字面量」，按原文判禁词
/// 会把这句自我约束当成违例——故先剥注释再判禁词。
/// 按行处理而不做字符级扫描：被锁文件里 SQL 多为跨行 raw string/多行参数，
/// 单行引号配平会把代码文本当注释吃掉；宁少剥（行尾尾注释、块注释不动）不可错剥。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn forbidden(src: &str, needles: &[&str], ctx: &str) {
    let code = code_only(src);
    for n in needles {
        assert!(
            !code.contains(n),
            "{ctx}: 出现被禁的裸状态字面量 {n}——比较/门必须改绑写入方词表常量"
        );
    }
}

fn required(src: &str, needles: &[&str], ctx: &str) {
    let code = code_only(src);
    for n in needles {
        assert!(
            code.contains(n),
            "{ctx}: 应包含 {n}（修复形态回潮丢失；注释里复述该片段不算实现）"
        );
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

/// 需要真实 Postgres（`test_common::setup_test_db()` → 已迁移库 + 清空业务表）：
/// 给迁移种子参照表里的既有供应商种 DRAFT/AUDITED/CANCELLED 三张 2096-12 窗口的应付单
/// （未来窗口天然隔离既有数据），统计/日报只应计入 AUDITED。运行：
/// `cargo test --test contract_wave6_ap_status_literal_parity_test -- --ignored --nocapture`
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移的真实 Postgres（PG 占位/日期语义在此锁）"]
async fn ap_reports_on_real_db_count_only_audited() {
    let db = Arc::new(test_common::setup_test_db().await);

    // 外键父行：迁移播种的 suppliers 参照表（不参与清空），取真实 id 而非硬编码
    let supplier_id = seeded_supplier_id(&db).await;

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
