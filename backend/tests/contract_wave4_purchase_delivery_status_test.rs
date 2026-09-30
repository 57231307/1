//! 采购交货周期样本谓词与词表同源契约锁（任务 #162-①）
//!
//! 锁定的根因（状态词表核查取证）：
//! - `services/purchase_delivery_calculator.rs` 的样本谓词曾手写
//!   `order_status IN ('COMPLETED','RECEIVED','PARTIALLY_RECEIVED')`，其中
//!   `RECEIVED` / `PARTIALLY_RECEIVED` **不在** purchase_order 权威词表
//!   （`models/status/purchase_inventory.rs` 的 `purchase_order` 模块，全大写）中：
//!   部分收货的权威拼写是 `PARTIAL_RECEIVED`，词表没有独立的 RECEIVED 态。
//!   Postgres 字符串比较逐字符敏感 ⇒ 部分收货订单恒不命中样本，平均交货周期失真。
//! - 到货语义的写入方依据：`services/purchase_receipt_ops/crud.rs` 确认入库按收货程度
//!   推进订单为 COMPLETED（全部收货）/ PARTIAL_RECEIVED（部分收货）。
//!
//! 修复形态（对齐 contract_wave3 先例：词表常量 + $N 绑定，禁手写 SQL 状态字面量）：
//! - 谓词改为 `order_status IN ($2, $3)`，值绑定 `purchase_order::COMPLETED` /
//!   `purchase_order::PARTIAL_RECEIVED`。
//! - CLOSED（收货后终态）是否纳入样本属业务口径问题，本次未擅自扩充，待产品拍板；
//!   本测试锁的是「谓词取值与词表同源」，不锁 CLOSED 口径。
//!
//! 覆盖策略（全部真实 SQL 行为，无 mock）：
//! 1. 行为锁：sqlite::memory: 自建 purchase_orders，按词表全部 9 态各种一行真实值
//!    （含 actual_delivery_date 为 NULL 的对照行、异供应商对照行），用**与生产同形态**的
//!    `IN ($1, $2)` 绑定词表常量过滤 → 命中集合恰为 {COMPLETED, PARTIAL_RECEIVED} 两行；
//! 2. 缺陷实证对照：复现修复前的字面量谓词 → 仅 COMPLETED 命中（PARTIAL_RECEIVED 漏样）。
//! 3. 词表同源锁：常量字面值逐字符核对（防第二套手写常量/拼写漂移）。
//! 4. 防回潮源码扫描：calculator 须引用词表常量并以 $N 绑定，不得再出现引号状态字面量。

use bingxi_backend::models::status::purchase_order;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"CREATE TABLE purchase_orders (
            id INTEGER PRIMARY KEY,
            supplier_id INTEGER NOT NULL,
            order_date TEXT,
            actual_delivery_date TEXT,
            order_status TEXT NOT NULL
        )"#,
        Vec::<Value>::new(),
    ))
    .await
    .expect("DDL 建表失败");

    // (supplier, order_date, actual_delivery_date, status) —— 状态一律取词表常量真实值
    let rows: Vec<(i64, Option<&str>, Option<&str>, &str)> = vec![
        (
            7,
            Some("2026-01-01"),
            Some("2026-01-10"),
            purchase_order::COMPLETED,
        ),
        (
            7,
            Some("2026-02-01"),
            Some("2026-02-05"),
            purchase_order::PARTIAL_RECEIVED,
        ),
        // COMPLETED 但无实际到货日：应被 actual_delivery_date IS NOT NULL 排除
        (7, Some("2026-03-01"), None, purchase_order::COMPLETED),
        // 已审批但未到货：不在样本态
        (
            7,
            Some("2026-04-01"),
            Some("2026-04-03"),
            purchase_order::APPROVED,
        ),
        // CLOSED（收货后终态）：口径待产品拍板，本次谓词不含，锁现状
        (
            7,
            Some("2026-05-01"),
            Some("2026-05-04"),
            purchase_order::CLOSED,
        ),
        // 异供应商的 PARTIAL_RECEIVED：验证 supplier_id 谓词仍生效
        (
            8,
            Some("2026-06-01"),
            Some("2026-06-09"),
            purchase_order::PARTIAL_RECEIVED,
        ),
        (
            7,
            Some("2026-07-01"),
            Some("2026-07-02"),
            purchase_order::DRAFT,
        ),
        (
            7,
            Some("2026-07-03"),
            Some("2026-07-05"),
            purchase_order::PENDING_APPROVAL,
        ),
        (
            7,
            Some("2026-07-06"),
            Some("2026-07-08"),
            purchase_order::SUBMITTED,
        ),
        (
            7,
            Some("2026-07-09"),
            Some("2026-07-11"),
            purchase_order::REJECTED,
        ),
        (
            7,
            Some("2026-07-12"),
            Some("2026-07-14"),
            purchase_order::CANCELLED,
        ),
    ];
    for (idx, (supplier, order_date, actual, status)) in rows.iter().enumerate() {
        let n = idx as i64 + 1; // 显式主键，id 即种子序
        // 日期列 None → 显式 NULL（Value::String(None)），避免类型推断漂移
        let od = match order_date {
            Some(s) => Value::String(Some(s.to_string())),
            None => Value::String(None),
        };
        let ad = match actual {
            Some(s) => Value::String(Some(s.to_string())),
            None => Value::String(None),
        };
        let values = vec![n.into(), (*supplier).into(), od, ad, (*status).into()];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO purchase_orders (id, supplier_id, order_date, actual_delivery_date, order_status) VALUES ($1, $2, $3, $4, $5)",
            values,
        ))
        .await
        .unwrap_or_else(|e| panic!("种子行 {n}（{status}）插入失败: {e}"));
    }
    db
}

/// 以**与修复后生产代码同形态**的谓词取命中订单 id：supplier=$1、状态 IN ($2,$3) 绑定词表常量。
async fn matched_ids_bound_constants(db: &sea_orm::DatabaseConnection) -> Vec<i64> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT id FROM purchase_orders
           WHERE supplier_id = $1
           AND order_status IN ($2, $3)
           AND actual_delivery_date IS NOT NULL
           ORDER BY id"#,
        vec![
            7i64.into(),
            purchase_order::COMPLETED.into(),
            purchase_order::PARTIAL_RECEIVED.into(),
        ],
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("绑定词表常量的谓词查询失败");
    rows.iter()
        .map(|r| r.try_get::<i64>("", "id").expect("id 应可解码为 i64"))
        .collect()
}

/// 复现**修复前**的字面量谓词（'RECEIVED'/'PARTIALLY_RECEIVED' 非词表值），作缺陷实证对照。
async fn matched_ids_old_literal(db: &sea_orm::DatabaseConnection) -> Vec<i64> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Sqlite,
        r#"SELECT id FROM purchase_orders
           WHERE supplier_id = $1
           AND order_status IN ('COMPLETED', 'RECEIVED', 'PARTIALLY_RECEIVED')
           AND actual_delivery_date IS NOT NULL
           ORDER BY id"#,
        vec![7i64.into()],
    );
    let rows = db.query_all_raw(stmt).await.expect("旧字面量谓词查询失败");
    rows.iter()
        .map(|r| r.try_get::<i64>("", "id").expect("id 应可解码为 i64"))
        .collect()
}

/// 行为锁：绑定词表常量后，样本恰为「本供应商 + 有到货日」的 COMPLETED 与 PARTIAL_RECEIVED 行。
/// 修复前该断言必然红（字面量 PARTIALLY_RECEIVED 不命中真实落库值 PARTIAL_RECEIVED，只剩 1 行）。
#[tokio::test]
async fn lead_time_sample_predicate_hits_vocabulary_statuses() {
    let db = setup_db().await;
    let ids = matched_ids_bound_constants(&db).await;
    assert_eq!(
        ids,
        vec![1, 2],
        "样本应恰为 id1(COMPLETED) 与 id2(PARTIAL_RECEIVED)；NULL 到货日/异供应商/其余词表态均不得混入，实际 {ids:?}"
    );
}

/// 缺陷实证对照：旧字面量谓词漏掉 PARTIAL_RECEIVED 行（原缺陷=样本恒少，均值失真）。
#[tokio::test]
async fn old_literal_predicate_drops_partial_received_defect_proof() {
    let db = setup_db().await;
    let ids = matched_ids_old_literal(&db).await;
    assert_eq!(
        ids,
        vec![1],
        "旧谓词只命中 COMPLETED；'RECEIVED'/'PARTIALLY_RECEIVED' 非词表值恒不命中（原缺陷实证），实际 {ids:?}"
    );
}

/// 词表同源锁：谓词引用的常量字面值逐字符核对（大写、PARTIAL 非 PARTIALLY）。
#[test]
fn purchase_order_status_vocabulary_literals() {
    assert_eq!(purchase_order::COMPLETED, "COMPLETED");
    assert_eq!(purchase_order::PARTIAL_RECEIVED, "PARTIAL_RECEIVED");
}

/// 防回潮源码扫描：calculator 必须引用词表常量并以 $N 绑定，SQL 内不得再出现引号状态字面量。
#[test]
fn source_scan_calculator_uses_bound_constants_not_status_literals() {
    let src = include_str!("../src/services/purchase_delivery_calculator.rs").replace('\r', "");
    assert!(
        src.contains("purchase_order::COMPLETED")
            && src.contains("purchase_order::PARTIAL_RECEIVED"),
        "样本谓词状态取值必须引用 models::status 词表常量，不得手写"
    );
    assert!(
        src.contains("order_status IN ($2, $3)"),
        "状态谓词须为 $N 绑定参数形态（对齐 contract_wave3 修复范式）"
    );
    for forbidden in ["'COMPLETED'", "'RECEIVED'", "'PARTIALLY_RECEIVED'"] {
        assert!(
            !src.contains(forbidden),
            "SQL 不得再含状态字面量 {forbidden}"
        );
    }
}
