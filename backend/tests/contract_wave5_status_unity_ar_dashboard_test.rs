//! 契约波次 5 · AR 报表 & 仪表盘库存状态口径统一契约锁（决策定案 + 词表核查 A1）
//!
//! 锁定的两条根因：
//! 1. **AR 报表口径分裂**：`services/ar_ops/report.rs` 的 8 处过滤门只写
//!    `status <> CANCELLED`，而 AR 发票创建写入的就是 `common::STATUS_DRAFT`
//!    （services/ar/inv.rs:222）⇒ 草稿应收被计入 total/unpaid/逾期额/回款率分母，
//!    与 BI（`bi_analysis_ops/*` 用 `NOT IN (CANCELLED, DRAFT)`）和仪表盘口径自相矛盾。
//!    同族裸 `"CANCELLED"` 字面量：`ar/vfy_ops/aging.rs`（SeaORM 账龄）、
//!    `fund_management_service.rs` 现金流预测应收侧。
//!    修复形态：8 处一律 `status NOT IN ($k, $k+1)`，值绑定
//!    `crate::models::status::common::{STATUS_CANCELLED, STATUS_DRAFT}`；
//!    SeaORM 侧改 `is_not_in([CANCELLED, DRAFT])`。
//! 2. **仪表盘 4 个库存聚合恒 0**：`dashboard_service.rs` 裸 SQL
//!    `WHERE s.stock_status = 'active'` 过滤**中文词表列**
//!    （`inventory_stock_status` 权威取值 正常/报废/已删除），PG 比较区分大小写且
//!    中英不同 ⇒ 在库价值/品类价值/库龄/周转率分母恒空。修复形态：4 处改绑定
//!    `inventory_stock_status::NORMAL` 常量（周转率处基址避让 $1/$2 用 $3）。
//!
//! 覆盖策略（路线一真库化， 判责；表结构唯一来源 = backend/migration）
//! - 真 PG 行为锁：调用**生产同源 builder** 得到 (sql, params) 后在真实 ar_invoices /
//!   inventory_stocks 表上执行——统计/日报表逐值断言只含 APPROVED（含全过滤参数形态，
//!   锁 $N 基址位移不撞号）；修复前旧谓词（只排 CANCELLED）作缺陷实证对照（草稿被误计入）。
//!   金额列按真表 DECIMAL 以 Decimal 解码逐值断言（sqlite TEXT 亲和失真通道已废除）。
//! - 仪表盘库存门真库行为锁：绑定中文常量命中「正常」行；旧英文小写字面量
//!   谓词恒零命中作缺陷实证对照。
//! - $N 占位一致性锁：月报表/账龄报表（to_char、CURRENT_DATE 减法为 PG 语义，
//!   行为锁在末尾真库用例真跑，本组先锁 builder 纯函数形态）逐分支断言 SQL 占位符与
//!   参数序列一一对应且状态常量落在正确槽位。
//! - 禁回潮源码扫描（include_str!）：report.rs / dashboard_service.rs 不得再出现
//!   `'active'`、`'CANCELLED'`、`'DRAFT'`、`"CANCELLED"`、`"DRAFT"` 裸状态字面量；
//!   同族收口点（vfy aging / fund）锁常量引用形态不回退。
//! - 中文词表保护：dashboard 库存门必须引用 `inventory_stock_status::NORMAL`，
//!   词表值必须保持中文，不得被「顺手英文化」。
//! - 四类报表真库锁（原 #[ignore]，路线一转正真跑）：在真实 PG 种
//!   DRAFT/APPROVED/CANCELLED 三张发票（父行 customers 按裁定 R1 自插），
//!   逐值断言全部只含 APPROVED。

mod test_common;

use std::str::FromStr;
use std::sync::Arc;

use bingxi_backend::models::status::common::{STATUS_APPROVED, STATUS_CANCELLED, STATUS_DRAFT};
use bingxi_backend::models::status::purchase_inventory::inventory_stock_status;
use bingxi_backend::models::{ar_invoice, customer, product, warehouse};
use bingxi_backend::services::ar_service::ArService;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, QueryResult, Statement, Value,
};

// ===========================================================================
// 公共辅助
// ===========================================================================

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("测试基准日期必须合法")
}

async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

/// ar_invoices.customer_id 有真外键 fk_ar_invoices_customer→customers（m0012:238），
/// customers 属被清空且不播种的业务表 ⇒ 按裁定 R1 自插合法父行。
/// 显式 ID=77：与本文件既有 builder 过滤参数 Some(77) 对齐（TRUNCATE RESTART IDENTITY
/// 后真表为空，显式 ID 不与任何种子冲突）。
async fn seed_customer_77(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();
    customer::ActiveModel {
        id: Set(77),
        customer_code: Set("W5-DU-CUS77".to_string()),
        customer_name: Set("波5口径统一夹具客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(0), // customers.owner_id 无外键（DEFAULT 0=未分配），非本用例锁面对
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("自插 customers 父行失败（裁定 R1）");
}

/// 真表种子：同一客户 77、三个不同开票日：DRAFT 2222.50 / APPROVED 1000.00 / CANCELLED 3333.75，
/// 到期日均早于基准日 2096-12-31（逾期口径三张同构，差异只在状态）。
/// 金额按真表 DECIMAL 列以 Decimal 绑定（sqlite TEXT 亲和失真族即本用例集的 根因）。
async fn setup_ar_invoices(db: &sea_orm::DatabaseConnection) {
    seed_customer_77(db).await;
    for (id, no, inv_day, amount, status) in [
        (1i32, "W5-D", 5, dec!(2222.50), STATUS_DRAFT),
        (2, "W5-A", 6, dec!(1000.00), STATUS_APPROVED),
        (3, "W5-C", 7, dec!(3333.75), STATUS_CANCELLED),
    ] {
        ar_invoice::ActiveModel {
            id: Set(id),
            invoice_no: Set(no.to_string()),
            invoice_date: Set(date(2096, 12, inv_day)),
            due_date: Set(date(2096, 11, inv_day)),
            customer_id: Set(77),
            invoice_amount: Set(amount),
            received_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(amount),
            status: Set(status.to_string()),
            approval_status: Set(STATUS_APPROVED.to_string()),
            created_by: Set(1), // ar_invoices.created_by 无库级外键（m0012 仅 supplier/customer FK）
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("种子 AR 发票 {no} 插入失败: {e}"));
    }
}

async fn run_all(
    db: &sea_orm::DatabaseConnection,
    sql: String,
    params: Vec<Value>,
) -> Vec<QueryResult> {
    db.query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        params,
    ))
    .await
    .expect("真库 PG SQL 执行失败")
}

fn col_i64(row: &QueryResult, idx: usize) -> i64 {
    row.try_get_by_index::<Option<i64>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 i64: {e}"))
        .unwrap_or_else(|| panic!("第 {idx} 列聚合结果不应为 NULL"))
}

/// 真表金额列为 DECIMAL：聚合结果按 Decimal 解码逐值断言
/// （原 f64 解码是 sqlite REAL 同构表时代的形态，真库下必 ColumnDecode—— 判责原文
/// 「第 0 列应可解码为 f64: mismatched types DECIMAL」即此族）
fn col_decimal(row: &QueryResult, idx: usize) -> Decimal {
    row.try_get_by_index::<Option<Decimal>>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 Decimal: {e}"))
        .unwrap_or_else(|| panic!("第 {idx} 列聚合结果不应为 NULL"))
}

fn col_date(row: &QueryResult, idx: usize) -> NaiveDate {
    row.try_get_by_index::<NaiveDate>(idx)
        .unwrap_or_else(|e| panic!("第 {idx} 列应可解码为 DATE: {e}"))
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
/// （本仓刚踩过 $N 撞号的雷——排除门多绑一个参数后，后续日期/客户参数若未顺延即错位）
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
        "{ctx}: 占位符范围 $ {min_n}..${max_n} 与参数个数 {} 不符——\
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
        // sea-query 1.x：Value::String(Option<String>)
        Value::String(Some(s)) => assert_eq!(s.as_str(), want, "{ctx}: 状态绑定值与词表常量不符"),
        other => panic!("{ctx}: 期望字符串状态参数，实际 {other:?}"),
    }
}

// ===========================================================================
// 1) AR 报表真库行为锁：草稿与取消状态均不计入（聚合只含 APPROVED）
// ===========================================================================

/// 统计报表：调用生产 builder `build_statistics_sql_and_params`，返回的 (sql, params)
/// 原样在真实 PG 表上执行 → 六项聚合值必须只含 APPROVED。
/// （today 参数绑定 NaiveDate、`$N` 顺延由 builder 内部 `params.len()+1` 保证。）
#[tokio::test]
async fn ar_statistics_via_production_builder_counts_only_approved() {
    let db = live_db().await;
    setup_ar_invoices(&db).await;

    let (sql, params) =
        ArService::build_statistics_sql_and_params(None, None, None, date(2096, 12, 31));
    assert_eq!(
        params.len(),
        3,
        "无附加过滤时应绑 [CANCELLED, DRAFT, today]"
    );
    expect_status_value(&params[0], STATUS_CANCELLED, "统计 params[0]");
    expect_status_value(&params[1], STATUS_DRAFT, "统计 params[1]");
    assert_placeholders_match_params(&sql, &params, "统计报表（无过滤）");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(rows.len(), 1, "聚合查询应恰返回一行");
    let row = &rows[0];
    assert_eq!(
        col_i64(row, 0),
        1,
        "total_invoices 必须=1（=2 即草稿被计入=修复前口径，=3 即取消也被计入）"
    );
    assert_eq!(
        col_decimal(row, 1),
        dec!(1000.00),
        "total_amount 只含 APPROVED"
    );
    assert_eq!(
        col_decimal(row, 2),
        Decimal::ZERO,
        "paid_amount 只含 APPROVED"
    );
    assert_eq!(
        col_decimal(row, 3),
        dec!(1000.00),
        "unpaid_amount 只含 APPROVED"
    );
    assert_eq!(
        col_i64(row, 4),
        1,
        "overdue_count 只含 APPROVED（另两张同为逾期但状态门外）"
    );
    assert_eq!(
        col_decimal(row, 5),
        dec!(1000.00),
        "overdue_amount 只含 APPROVED"
    );
}

/// 统计报表（customer_id + 起止日期全过滤）：锁排除门占位 $1/$2 与后续参数的基址顺延。
/// 窗口 [2096-12-05, 2096-12-06] 覆盖 DRAFT+APPROVED 两张——若状态参数与日期参数撞号
/// （如 NOT IN 用了 $4/$5 而日期用了 $1/$2），金额/条数必然偏离逐值断言。
#[tokio::test]
async fn ar_statistics_full_filter_placeholders_and_values_are_approved_only() {
    let db = live_db().await;
    setup_ar_invoices(&db).await;

    let (sql, params) = ArService::build_statistics_sql_and_params(
        Some(date(2096, 12, 5)),
        Some(date(2096, 12, 6)),
        Some(77),
        date(2096, 12, 31),
    );
    assert_eq!(
        params.len(),
        6,
        "应绑 [CANCELLED, DRAFT, cid, start, end, today]"
    );
    expect_status_value(&params[0], STATUS_CANCELLED, "统计(过滤) params[0]");
    expect_status_value(&params[1], STATUS_DRAFT, "统计(过滤) params[1]");
    assert!(
        sql.contains("status NOT IN ($1, $2)")
            && sql.contains("customer_id = $3")
            && sql.contains("invoice_date >= $4")
            && sql.contains("invoice_date <= $5"),
        "统计(过滤) 占位基址必须按参数序顺延，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "统计报表（全过滤）");

    let rows = run_all(&db, sql, params).await;
    assert_eq!(rows.len(), 1, "聚合查询应恰返回一行");
    let row = &rows[0];
    assert_eq!(
        col_i64(row, 0),
        1,
        "窗内 DRAFT 必须被状态门剔除，仅剩 APPROVED"
    );
    assert_eq!(
        col_decimal(row, 1),
        dec!(1000.00),
        "窗内 total_amount 只含 APPROVED"
    );
    assert_eq!(col_i64(row, 4), 1, "窗内 overdue_count 只含 APPROVED");
    assert_eq!(
        col_decimal(row, 5),
        dec!(1000.00),
        "窗内 overdue_amount 只含 APPROVED"
    );
}

/// 缺陷实证对照：修复前口径（只排 CANCELLED）在同一数据上把草稿计入统计
/// （total_invoices=2、total=3222.50）——锁「旧门必错」的因果，防有人回退判定。
#[tokio::test]
async fn ar_legacy_cancelled_only_gate_wrongly_counts_draft_defect_proof() {
    let db = live_db().await;
    setup_ar_invoices(&db).await;

    let values: Vec<Value> = vec![STATUS_CANCELLED.into()];
    let rows = run_all(
        &db,
        r#"SELECT COUNT(*) AS cnt, COALESCE(SUM(invoice_amount), 0) AS total
           FROM ar_invoices WHERE status <> $1"#
            .to_string(),
        values,
    )
    .await;
    assert_eq!(col_i64(&rows[0], 0), 2, "旧门应把 DRAFT+APPROVED 一起计入");
    assert_eq!(
        col_decimal(&rows[0], 1),
        dec!(3222.50),
        "旧门合计=2222.50+1000.00"
    );
}

/// 日报表：生产 builder `build_daily_sql_and_params`（无过滤 + 全过滤两形态）
/// 在真实 PG 执行 → 只出 APPROVED 一个日桶，条数/金额逐值断言。
#[tokio::test]
async fn ar_daily_via_production_builder_buckets_only_approved() {
    let db = live_db().await;
    setup_ar_invoices(&db).await;

    let (sql, params) = ArService::build_daily_sql_and_params(None, None, None);
    assert_placeholders_match_params(&sql, &params, "日报表（无过滤）");
    let rows = run_all(&db, sql, params).await;
    assert_eq!(rows.len(), 1, "日桶必须只有 APPROVED 的 2096-12-06 一个");
    assert_eq!(col_date(&rows[0], 0), date(2096, 12, 6));
    assert_eq!(col_i64(&rows[0], 1), 1, "invoice_count 只含 APPROVED");
    assert_eq!(
        col_decimal(&rows[0], 2),
        dec!(1000.00),
        "invoice_amount 只含 APPROVED"
    );
    assert_eq!(
        col_decimal(&rows[0], 4),
        dec!(1000.00),
        "unpaid_amount 只含 APPROVED"
    );

    // 全过滤形态：窗口盖住 DRAFT+APPROVED 两日 + customer_id → 桶仍只剩 APPROVED
    let (sql, params) = ArService::build_daily_sql_and_params(
        Some(date(2096, 12, 5)),
        Some(date(2096, 12, 6)),
        Some(77),
    );
    assert!(
        sql.contains("status NOT IN ($1, $2)")
            && sql.contains("customer_id = $3")
            && sql.contains("invoice_date >= $4")
            && sql.contains("invoice_date <= $5"),
        "日报表全过滤占位基址必须顺延，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "日报表（全过滤）");
    let rows = run_all(&db, sql, params).await;
    assert_eq!(
        rows.len(),
        1,
        "窗内 DRAFT 日被状态门剔除，只剩 APPROVED 日桶"
    );
    assert_eq!(col_date(&rows[0], 0), date(2096, 12, 6));
    assert_eq!(col_decimal(&rows[0], 2), dec!(1000.00));
}

/// 月报表：`to_char` 为 PG 语义（真库行为锁见本文件末尾转正用例）→
/// 此处锁生产 builder 的占位基址与常量绑定槽位。
#[tokio::test]
async fn ar_monthly_builder_binds_status_constants_with_shifted_placeholders() {
    let (sql, params) = ArService::build_monthly_sql_and_params(
        Some(date(2096, 12, 5)),
        Some(date(2096, 12, 31)),
        Some(77),
    );
    assert_eq!(params.len(), 5, "应绑 [CANCELLED, DRAFT, cid, start, end]");
    expect_status_value(&params[0], STATUS_CANCELLED, "月报 params[0]");
    expect_status_value(&params[1], STATUS_DRAFT, "月报 params[1]");
    assert!(
        sql.contains("status NOT IN ($1, $2)")
            && sql.contains("customer_id = $3")
            && sql.contains("invoice_date >= $4")
            && sql.contains("invoice_date <= $5"),
        "月报表占位基址必须按参数序顺延，实际 SQL:\n{sql}"
    );
    assert_placeholders_match_params(&sql, &params, "月报表（全过滤）");
    assert!(
        sql.contains("to_char(invoice_date, 'YYYY-MM')"),
        "月报表分桶表达式不应被顺手改写（本次只统一状态口径）"
    );
}

/// 账龄报表：`CURRENT_DATE - due_date` 为 PG 日期算术（真库行为锁见文件末尾转正用例）
/// → 对生产 builder 四个分支逐一锁：$1=today、$2/$3=排除门常量、$4/$5=客户/业务员，
/// 参数序列与占位一一对应。
#[tokio::test]
async fn ar_aging_builder_all_branches_bind_constants_with_shifted_placeholders() {
    type FilterPair = (Option<i32>, Option<i32>);
    let today = date(2096, 12, 31);
    let cases: [(&str, FilterPair, usize, &[&str]); 4] = [
        (
            "客户+业务员",
            (Some(77), Some(9)),
            5,
            &["customer_id = $4", "salesperson_id = $5"],
        ),
        ("仅客户", (Some(77), None), 4, &["customer_id = $4"]),
        ("仅业务员", (None, Some(9)), 4, &["salesperson_id = $4"]),
        ("无过滤", (None, None), 3, &[]),
    ];
    for (name, (cid, sid), want_params, extras) in cases {
        let (sql, params) = ArService::build_aging_sql_and_params(cid, today, sid);
        assert_eq!(params.len(), want_params, "账龄分支「{name}」参数个数不符");
        expect_status_value(&params[1], STATUS_CANCELLED, "账龄「{name}」params[1]");
        expect_status_value(&params[2], STATUS_DRAFT, "账龄「{name}」params[2]");
        assert!(
            sql.contains("status NOT IN ($2, $3)"),
            "账龄「{name}」排除门必须为 NOT IN ($2, $3)（$1 已被基准日占用），实际:\n{sql}"
        );
        for frag in extras {
            assert!(
                sql.contains(frag),
                "账龄「{name}」缺少顺延后的条件 `{frag}`"
            );
        }
        assert_placeholders_match_params(sql, &params, &format!("账龄「{name}」"));
    }
}

// ===========================================================================
// 2) 仪表盘库存门真库行为锁：中文词表列上的门必须绑定中文常量才命中「正常」行
// ===========================================================================

/// 真表 inventory_stocks 的 product_id/warehouse_id 有真外键（fk_inventory_product/
/// fk_inventory_warehouse，m0001:630-631）⇒ 按裁定 R1 自插合法父行（products/warehouses
/// 属被清空的业务表），库存行本身按真表列直插（quantity_meters DECIMAL、
/// stock_status VARCHAR 存中文权威词表值）。
async fn setup_inventory_stocks(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();
    product::ActiveModel {
        id: Set(1),
        name: Set("波5口径统一夹具产品".to_string()),
        code: Set("W5DUP1".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("成品布".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("自插 products 父行失败（裁定 R1）");
    warehouse::ActiveModel {
        id: Set(1),
        warehouse_code: Set("W5DUWH1".to_string()),
        name: Set("波5口径统一夹具仓库".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("自插 warehouses 父行失败（裁定 R1）");

    // 三种中文权威取值各一行——正常 100 / 报废 500 / 已删除 700
    for (qty, status) in [
        (dec!(100), inventory_stock_status::NORMAL),
        (dec!(500), inventory_stock_status::SCRAPPED),
        (dec!(700), inventory_stock_status::DELETED),
    ] {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO inventory_stocks (product_id, warehouse_id, quantity, \
             quantity_meters, stock_status) VALUES ($1, $2, $3, $4, $5)",
            vec![
                1i32.into(),
                1i32.into(),
                Value::Decimal(Some(Decimal::ZERO)),
                Value::Decimal(Some(qty)),
                status.into(),
            ],
        ))
        .await
        .unwrap_or_else(|e| panic!("种子库存行插入失败: {e}"));
    }
}

/// 行为锁：修复后形态 `WHERE s.stock_status = $1` 绑定中文常量 → 聚合只含「正常」行。
#[tokio::test]
async fn dashboard_inventory_gate_bound_chinese_constant_hits_normal_rows() {
    let db = live_db().await;
    setup_inventory_stocks(&db).await;

    let values: Vec<Value> = vec![inventory_stock_status::NORMAL.into()];
    let rows = run_all(
        &db,
        "SELECT COALESCE(SUM(quantity_meters), 0) AS total FROM inventory_stocks \
         WHERE stock_status = $1"
            .to_string(),
        values,
    )
    .await;
    assert_eq!(
        col_decimal(&rows[0], 0),
        dec!(100),
        "库存门绑定 inventory_stock_status::NORMAL 后必须命中「正常」行（0 即中英错配恒空）"
    );
}

/// 缺陷实证对照：修复前英文小写裸字面量谓词对中文落库值恒零命中 → 聚合恒空。
#[tokio::test]
async fn dashboard_legacy_english_literal_gate_returns_zero_defect_proof() {
    let db = live_db().await;
    setup_inventory_stocks(&db).await;

    let rows = run_all(
        &db,
        "SELECT COALESCE(SUM(quantity_meters), 0) AS total FROM inventory_stocks \
         WHERE stock_status = 'active'"
            .to_string(),
        Vec::new(),
    )
    .await;
    assert_eq!(
        col_decimal(&rows[0], 0),
        Decimal::ZERO,
        "旧英文小写字面量对中文词表列必须恒零命中（此处 0 即「页面看起来就是没数据」的根因实证）"
    );
}

// ===========================================================================
// 3) 禁回潮源码扫描（include_str!）
// ===========================================================================

#[test]
fn source_scan_ar_report_and_dashboard_have_no_bare_status_literals() {
    let report = include_str!("../src/services/ar_ops/report.rs").replace('\r', "");
    let dash = include_str!("../src/services/dashboard_service.rs").replace('\r', "");

    for (name, src) in [
        ("ar_ops/report.rs", &report),
        ("dashboard_service.rs", &dash),
    ] {
        for needle in [
            "'active'",
            "'CANCELLED'",
            "'DRAFT'",
            "\"CANCELLED\"",
            "\"DRAFT\"",
        ] {
            assert!(
                !src.contains(needle),
                "{name}: 不得再出现裸状态字面量 {needle:?}——状态值只能引用/绑定 \
                 models::status 词表常量（SQL 用 $N 参数化，SeaORM 用常量）"
            );
        }
    }

    // report.rs：8 处排除门全部为参数化 NOT IN 形态。
    // 统计/日/月三个 builder 经 format! 模板生成（运行期形态由真库行为锁断言），
    // 源码形态 = "status NOT IN (${}, ${})"；账龄四分支 + 业务员账龄为静态 SQL = ($2, $3)。
    assert_eq!(
        report.matches("status NOT IN (${}, ${})").count(),
        3,
        "统计/日/月三个 builder 的排除门应为 NOT IN 模板（两枚顺延占位）"
    );
    assert_eq!(
        report.matches("status NOT IN ($2, $3)").count(),
        5,
        "账龄四分支 + 业务员账龄的排除门应为 NOT IN ($2, $3)（$1 已被基准日占用）"
    );
    assert!(
        !report.contains("status <> $"),
        "report.rs 不得残留单值 `<>` 排除门（只排 CANCELLED 即口径回潮）"
    );
    assert!(
        report.contains("crate::models::status::common::STATUS_CANCELLED")
            && report.contains("crate::models::status::common::STATUS_DRAFT"),
        "report.rs 排除门取值必须引用写入方词表 common 常量"
    );
}

#[test]
fn source_scan_same_family_aging_and_fund_use_bound_constants() {
    let vfy = include_str!("../src/services/ar/vfy_ops/aging.rs").replace('\r', "");
    assert!(
        !vfy.contains(".ne(\"CANCELLED\")")
            && !vfy.contains("'CANCELLED'")
            && !vfy.contains("'DRAFT'"),
        "ar/vfy_ops/aging.rs 不得残留裸 \"CANCELLED\" 字面量或单值 ne 门"
    );
    assert!(
        vfy.contains("common::STATUS_CANCELLED") && vfy.contains("common::STATUS_DRAFT"),
        "ar/vfy_ops/aging.rs 账龄门必须以 is_not_in 绑定 common 词表两常量"
    );

    let fund = include_str!("../src/services/fund_management_service.rs").replace('\r', "");
    assert!(
        !fund.contains("Status.ne(\"CANCELLED\")"),
        "fund_management_service.rs 现金流预测应收侧不得残留 SeaORM 裸 \"CANCELLED\" 门"
    );
    assert!(
        fund.contains("crate::models::status::common::STATUS_CANCELLED")
            && fund.contains("crate::models::status::common::STATUS_DRAFT"),
        "fund_management_service.rs 应收流入门必须绑定 common 词表两常量（口径与 AR 报表同源）"
    );
}

/// 中文词表保护：dashboard 库存门必须引用 `inventory_stock_status::NORMAL`，
/// 词表值必须保持中文——不得被「顺手英文化」，也不得混入其他状态域常量。
#[test]
fn source_scan_dashboard_inventory_gate_protects_chinese_vocabulary() {
    assert_eq!(
        inventory_stock_status::NORMAL,
        "正常",
        "库存台账状态权威值必须为中文"
    );
    assert_eq!(inventory_stock_status::SCRAPPED, "报废");
    assert_eq!(inventory_stock_status::DELETED, "已删除");

    let dash = include_str!("../src/services/dashboard_service.rs").replace('\r', "");
    assert!(
        !dash.contains("stock_status = '"),
        "dashboard 库存门不得残留任何 SQL 字符串字面量形态（中英皆禁）"
    );
    assert_eq!(
        dash.matches("WHERE s.stock_status = $1").count(),
        3,
        "仓库价值/品类价值/库龄三处库存门应为参数化 `= $1`"
    );
    assert_eq!(
        dash.matches("WHERE stock_status = $3").count(),
        1,
        "周转率分母库存门应为 `= $3`（$1/$2 已被销售侧排除门占用——基址避让防撞号）"
    );
    assert_eq!(
        dash.matches("inventory_stock_status::NORMAL.into()")
            .count(),
        4,
        "四处库存门必须全部绑定中文词表常量 NORMAL"
    );
    assert_eq!(
        dash.matches("StockStatus.eq(inventory_stock_status::NORMAL)")
            .count(),
        6,
        "SeaORM 侧 6 处库存门（既有正确实现）必须保持引用 NORMAL，不得被改写"
    );
    assert!(
        !dash.contains("StockStatus.eq(master_data") && !dash.contains("stock_status = $2"),
        "库存门禁止混用其他状态域常量或错位占位（$2 未在本域出现）"
    );
}

// ===========================================================================
// 4) 真库（PG）行为锁：四类报表同库三态发票逐值断言（路线一转正真跑，原 #[ignore]）
// ===========================================================================

fn dec_of(v: &serde_json::Value, key: &str) -> Decimal {
    let raw = v[key]
        .as_str()
        .unwrap_or_else(|| panic!("报表响应键 `{key}` 应为金额字符串，实际 {v}"));
    Decimal::from_str(raw).unwrap_or_else(|e| panic!("键 `{key}` 金额 {raw:?} 无法解析: {e}"))
}

/// 真库（公共夹具 setup_test_db：已迁移 PostgreSQL + 清空业务表）：
/// 自插 customers 父行（裁定 R1，原「查既有客户」依赖已被 TRUNCATE 废除）后种
/// DRAFT/APPROVED/CANCELLED 三张 AR 发票 → 统计/日/月/账龄四类结果金额与条数
/// **都只含 APPROVED**（逐值断言）。
///
/// 隔离策略：开票日固定在远未来 2096-12 窗口 + 自插客户 77 + 哨兵 salesperson_id；
/// 业务表每次清空 ⇒ 用例互不串库，无需按单号先删后插。
#[tokio::test]
async fn ar_four_reports_on_real_db_count_only_approved() {
    let db = Arc::new(test_common::setup_test_db().await);
    let svc = ArService::new(db.clone());

    // 裁定 R1：ar_invoices.customer_id → customers 真外键，父行自插（ID=77）
    seed_customer_77(&db).await;
    let customer_id = 77;
    // 哨兵业务员 id（salesperson_id 列为无 FK 的裸 INTEGER，见 m0012 + business/mod.rs:51）：
    // 取远超真实用户量级的固定值，账龄端按它隔离，不受库内既有数据污染
    let sid: i32 = 2_096_120_001;

    let approved_amt = dec!(1000.00);
    let seeds: [(&str, &str, Decimal); 3] = [
        ("DRAFT", STATUS_DRAFT, dec!(2222.50)),
        ("APPROVED", STATUS_APPROVED, approved_amt),
        ("CANCELLED", STATUS_CANCELLED, dec!(3333.75)),
    ];
    for (suffix, status, amount) in seeds {
        ar_invoice::ActiveModel {
            invoice_no: Set(format!("W5STATUS-UNITY-{suffix}")),
            invoice_date: Set(date(2096, 12, 6)),
            due_date: Set(date(2000, 1, 31)), // 远超 90 天 → 账龄必入 90+ 桶（确定性分桶）
            customer_id: Set(customer_id),
            invoice_amount: Set(amount),
            received_amount: Set(Decimal::ZERO),
            unpaid_amount: Set(amount),
            status: Set(status.to_string()),
            approval_status: Set(STATUS_APPROVED.to_string()),
            salesperson_id: Set(Some(sid)),
            created_by: Set(1),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .expect("真库种子发票插入失败");
    }

    // 1) 统计报表：仅 APPROVED
    let stats = svc
        .get_statistics_report(
            Some(date(2096, 12, 1)),
            Some(date(2096, 12, 31)),
            Some(customer_id),
        )
        .await
        .expect("get_statistics_report 真库执行失败");
    assert_eq!(
        stats["total_invoices"].as_i64(),
        Some(1),
        "统计只应计入 APPROVED 一张"
    );
    assert_eq!(dec_of(&stats, "total_amount"), approved_amt);
    assert_eq!(dec_of(&stats, "unpaid_amount"), approved_amt);
    assert_eq!(stats["overdue_count"].as_i64(), Some(1));
    assert_eq!(dec_of(&stats, "overdue_amount"), approved_amt);

    // 2) 日报表
    let daily = svc
        .get_daily_report(
            Some(date(2096, 12, 1)),
            Some(date(2096, 12, 31)),
            Some(customer_id),
        )
        .await
        .expect("get_daily_report 真库执行失败");
    let daily_rows = daily.as_array().expect("日报表应为数组");
    assert_eq!(daily_rows.len(), 1, "日桶只应剩 APPROVED 的 2096-12-06");
    assert_eq!(daily_rows[0]["date"], "2096-12-06");
    assert_eq!(daily_rows[0]["invoice_count"].as_i64(), Some(1));
    assert_eq!(dec_of(&daily_rows[0], "invoice_amount"), approved_amt);

    // 3) 月报表（to_char——PG 专属语义，sqlite 时代锁不到的正是这里，真库直接真跑）
    let monthly = svc
        .get_monthly_report(
            Some(date(2096, 12, 1)),
            Some(date(2096, 12, 31)),
            Some(customer_id),
        )
        .await
        .expect("get_monthly_report 真库执行失败");
    let monthly_rows = monthly.as_array().expect("月报表应为数组");
    assert_eq!(monthly_rows.len(), 1);
    assert_eq!(monthly_rows[0]["month"], "2096-12");
    assert_eq!(monthly_rows[0]["invoice_count"].as_i64(), Some(1));
    assert_eq!(dec_of(&monthly_rows[0], "invoice_amount"), approved_amt);

    // 4) 账龄报表（CURRENT_DATE - due_date——另一处 PG 专属语义；按哨兵业务员隔离）
    let aging = svc
        .get_aging_report(None, Some(date(2096, 12, 31)), Some(sid))
        .await
        .expect("get_aging_report 真库执行失败");
    assert_eq!(
        aging["invoice_count"].as_i64(),
        Some(1),
        "账龄只应计入 APPROVED"
    );
    assert_eq!(dec_of(&aging, "bucket_90_plus"), approved_amt);
    assert_eq!(dec_of(&aging, "total_overdue"), approved_amt);
    assert_eq!(dec_of(&aging, "not_due"), Decimal::ZERO);
}
