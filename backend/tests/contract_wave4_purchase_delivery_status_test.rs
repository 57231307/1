//! 采购交货周期样本谓词与词表同源契约锁（任务 #162-①）
//!
//! 表结构唯一来源 = backend/migration（路线一，#4669 判责）：本文件不自建 DDL，
//! 全部读写打已迁移 PostgreSQL 的真表 `purchase_orders`（`test_common::setup_test_db()`）。
//!
//! 锁定的根因（状态词表核查取证）：
//! - `services/purchase_delivery_calculator.rs` 的样本谓词曾手写
//!   `order_status IN ('COMPLETED','RECEIVED','PARTIALLY_RECEIVED')`，其中
//!   `RECEIVED` / `PARTIALLY_RECEIVED` **不在** purchase_order 权威词表
//!   （`models/status/purchase_inventory.rs` 的 `purchase_order` 模块，全大写）中：
//!   部分收货的权威拼写是 `PARTIAL_RECEIVED`，词表没有独立的 RECEIVED 态。
//!   Postgres 字符串比较逐字符敏感 ⇒ 部分收货的订单恒不命中样本，平均交货周期失真。
//! - 到货语义的写入方依据：`services/purchase_receipt_ops/crud.rs` 确认入库按收货程度
//!   推进订单为 COMPLETED（全部收货）/ PARTIAL_RECEIVED（部分收货）。
//! - 销售发货链的出库四维现口径 = 缸号/色号/批次/**匹号**（第四维不再是幅宽/款号，
//!   `services/inv/fabric_class.rs`）；本文件是采购交期样本谓词，不涉及四维，
//!   状态取值一律按 `models/status/purchase_inventory.rs::purchase_order` 权威词表核对。
//!
//! 修复形态（对齐 contract_wave3 先例：词表常量 + $N 绑定，禁手写 SQL 状态字面量）：
//! - 谓词改为 `order_status IN ($2, $3)`，值绑定 `purchase_order::COMPLETED` /
//!   `purchase_order::PARTIAL_RECEIVED`。
//! - CLOSED（收货后终态）是否纳入样本属业务口径问题，本次未擅自扩充，待产品拍板；
//!   本测试锁的是「谓词取值与词表同源」，不锁 CLOSED 口径。
//!
//! 覆盖策略（全部真实 SQL 行为，无 mock）：
//! 1. 行为锁：真表 purchase_orders，按词表全部 9 态各种一行真实值
//!    （含 actual_delivery_date 为 NULL 的对照行、异供应商对照行），用**与生产同形态**的
//!    `IN ($1, $2)` 绑定词表常量过滤 → 命中集合恰为 {COMPLETED, PARTIAL_RECEIVED} 两行；
//! 2. 缺陷实证对照：复现修复前的字面量谓词 → 仅 COMPLETED 命中（PARTIAL_RECEIVED 漏样）。
//! 3. 词表同源锁：常量字面值逐字符核对（防第二套手写常量/拼写漂移）。
//! 4. 防回潮源码扫描：calculator 须引用词表常量并以 $N 绑定，不得再出现引号状态字面量。
//! 5. 决策定案 #7 接入锁：`actual_delivery_date` 由收货确认回写后，原被
//!    `IS NOT NULL` 整批排除的单据必须进入平均交期样本（回写本体断言见
//!    `contract_wave5_purchase_actual_delivery_writeback_test.rs`）。
//!
//! FK 前置（裁定 R1）：`purchase_orders.supplier_id` FK→`suppliers`。`suppliers` 是
//! 迁移种子参照表（不清空、m0015 播种两个演示供应商），本文件按其稳定业务键
//! supplier_code 反查真实 id 作种子供应商，不新插、更不删参照行。

mod test_common;

use bingxi_backend::models::status::purchase_order;
use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use std::str::FromStr;

/// 演示供应商稳定业务键（backend/migration/src/domain/business/
/// m0015_seed_supplier_product_catalog.rs 播种，SEALED 参照表不参与清空）。
const SUPPLIER_MAIN: &str = "SUP-DEMO-FAB-01";
const SUPPLIER_OTHER: &str = "SUP-DEMO-FAB-02";

async fn id_of_supplier(db: &sea_orm::DatabaseConnection, code: &str) -> i32 {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM suppliers WHERE supplier_code = $1",
            vec![code.to_string().into()],
        ))
        .await
        .expect("查询参照供应商失败")
        .unwrap_or_else(|| panic!("迁移种子供应商 {code} 必须存在（suppliers 为参照表）"));
    row.try_get::<i32>("", "id")
        .expect("suppliers.id 应解码为 i32")
}

/// 真表 purchase_orders 的 NOT NULL 无默认列：order_no(UNIQUE)/supplier_id(FK)/
/// order_date（其余列有默认或可空：order_status DEFAULT 'DRAFT' v15:4466-4467、
/// created_at/updated_at DEFAULT CURRENT_TIMESTAMP）。种子只写谓词触及的列 +
/// 满足约束的最小真实值；夹具 TRUNCATE RESTART IDENTITY 后显式 id=1..11 稳定。
async fn setup_db() -> (sea_orm::DatabaseConnection, i32, i32) {
    let db = test_common::setup_test_db().await;
    let sup_main = id_of_supplier(&db, SUPPLIER_MAIN).await;
    let sup_other = id_of_supplier(&db, SUPPLIER_OTHER).await;

    // (order_no, supplier, order_date, actual_delivery_date, status)
    // —— 状态一律取词表常量真实值（全大写权威拼写）
    let rows: Vec<(&str, i32, &str, Option<&str>, &str)> = vec![
        (
            "PO-DELIV-01",
            sup_main,
            "2026-01-01",
            Some("2026-01-10"),
            purchase_order::COMPLETED,
        ),
        (
            "PO-DELIV-02",
            sup_main,
            "2026-02-01",
            Some("2026-02-05"),
            purchase_order::PARTIAL_RECEIVED,
        ),
        // COMPLETED 但无实际到货日：应被 actual_delivery_date IS NOT NULL 排除
        (
            "PO-DELIV-03",
            sup_main,
            "2026-03-01",
            None,
            purchase_order::COMPLETED,
        ),
        // 已审批但未到货：不在样本态
        (
            "PO-DELIV-04",
            sup_main,
            "2026-04-01",
            Some("2026-04-03"),
            purchase_order::APPROVED,
        ),
        // CLOSED（收货后终态）：口径待产品拍板，本次谓词不含，锁现状
        (
            "PO-DELIV-05",
            sup_main,
            "2026-05-01",
            Some("2026-05-04"),
            purchase_order::CLOSED,
        ),
        // 异供应商的 PARTIAL_RECEIVED：验证 supplier_id 谓词仍生效
        (
            "PO-DELIV-06",
            sup_other,
            "2026-06-01",
            Some("2026-06-09"),
            purchase_order::PARTIAL_RECEIVED,
        ),
        (
            "PO-DELIV-07",
            sup_main,
            "2026-07-01",
            Some("2026-07-02"),
            purchase_order::DRAFT,
        ),
        (
            "PO-DELIV-08",
            sup_main,
            "2026-07-03",
            Some("2026-07-05"),
            purchase_order::PENDING_APPROVAL,
        ),
        (
            "PO-DELIV-09",
            sup_main,
            "2026-07-06",
            Some("2026-07-08"),
            purchase_order::SUBMITTED,
        ),
        (
            "PO-DELIV-10",
            sup_main,
            "2026-07-09",
            Some("2026-07-11"),
            purchase_order::REJECTED,
        ),
        (
            "PO-DELIV-11",
            sup_main,
            "2026-07-12",
            Some("2026-07-14"),
            purchase_order::CANCELLED,
        ),
    ];
    for (idx, (no, supplier, order_date, actual, status)) in rows.iter().enumerate() {
        let n = idx as i32 + 1; // SERIAL 从 1 起（夹具 RESTART IDENTITY），id 即种子序
        // DATE 真列按类型绑定：sea-query 1.0.2 的变体名是 ChronoDate(Option<NaiveDate>)
        // （未装箱），写 `Value::Date(..)` 会 E0599；None 即显式 NULL，不靠文本亲和。
        let od = Value::ChronoDate(Some(date(order_date)));
        let ad = Value::ChronoDate(actual.map(date));
        let values: Vec<Value> = vec![
            n.into(),
            no.to_string().into(),
            (*supplier).into(),
            od,
            ad,
            (*status).into(),
        ];
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO purchase_orders \
             (id, order_no, supplier_id, order_date, actual_delivery_date, order_status) \
             VALUES ($1, $2, $3, $4, $5, $6)",
            values,
        ))
        .await
        .unwrap_or_else(|e| panic!("种子行 {n}（{status}）插入失败: {e}"));
    }
    (db, sup_main, sup_other)
}

fn date(s: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::from_str(s).unwrap_or_else(|_| panic!("种子日期非法: {s}"))
}

/// 以**与修复后生产代码同形态**的谓词取命中订单 id：supplier=$1、状态 IN ($2,$3) 绑定词表常量。
async fn matched_ids_bound_constants(db: &sea_orm::DatabaseConnection, supplier: i32) -> Vec<i32> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT id FROM purchase_orders
           WHERE supplier_id = $1
           AND order_status IN ($2, $3)
           AND actual_delivery_date IS NOT NULL
           ORDER BY id"#,
        vec![
            supplier.into(),
            purchase_order::COMPLETED.into(),
            purchase_order::PARTIAL_RECEIVED.into(),
        ],
    );
    let rows = db
        .query_all_raw(stmt)
        .await
        .expect("绑定词表常量的谓词查询失败");
    rows.iter()
        .map(|r| r.try_get::<i32>("", "id").expect("id 应可解码为 i32"))
        .collect()
}

/// 复现**修复前**的字面量谓词（'RECEIVED'/'PARTIALLY_RECEIVED' 非词表值），作缺陷实证对照。
async fn matched_ids_old_literal(db: &sea_orm::DatabaseConnection, supplier: i32) -> Vec<i32> {
    let stmt = Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT id FROM purchase_orders
           WHERE supplier_id = $1
           AND order_status IN ('COMPLETED', 'RECEIVED', 'PARTIALLY_RECEIVED')
           AND actual_delivery_date IS NOT NULL
           ORDER BY id"#,
        vec![supplier.into()],
    );
    let rows = db.query_all_raw(stmt).await.expect("旧字面量谓词查询失败");
    rows.iter()
        .map(|r| r.try_get::<i32>("", "id").expect("id 应可解码为 i32"))
        .collect()
}

/// 行为锁：绑定词表常量后，样本恰为「本供应商 + 有到货日」的 COMPLETED 与 PARTIAL_RECEIVED 行。
/// 修复前该断言必然红（字面量 PARTIALLY_RECEIVED 不命中真实落库值 PARTIAL_RECEIVED，只剩 1 行）。
#[tokio::test]
async fn lead_time_sample_predicate_hits_vocabulary_statuses() {
    let (db, sup_main, _sup_other) = setup_db().await;
    let ids = matched_ids_bound_constants(&db, sup_main).await;
    assert_eq!(
        ids,
        vec![1, 2],
        "样本应恰为 id1(COMPLETED) 与 id2(PARTIAL_RECEIVED)；NULL 到货日/异供应商/其余词表态均不得混入，实际 {ids:?}"
    );
}

/// 缺陷实证对照：旧字面量谓词漏掉 PARTIAL_RECEIVED 行（原缺陷=样本恒少，均值失真）。
#[tokio::test]
async fn old_literal_predicate_drops_partial_received_defect_proof() {
    let (db, sup_main, _sup_other) = setup_db().await;
    let ids = matched_ids_old_literal(&db, sup_main).await;
    assert_eq!(
        ids,
        vec![1],
        "旧谓词只命中 COMPLETED；'RECEIVED'/'PARTIALLY_RECEIVED' 非词表值恒不命中（原缺陷实证），实际 {ids:?}"
    );
}

/// 决策定案 #7 接入断言：`actual_delivery_date` 由收货确认回写后，原本被
/// `actual_delivery_date IS NOT NULL` 整批排除的单据应进入平均交期样本。
/// 种子行 id3（COMPLETED + 到货日 NULL，即回写落地前的恒空现状）在被写入
/// 与其同形态的确认回写 UPDATE 后必须命中谓词——锁定「回写 → 样本非空」链路，
/// 防止列再次退化为永不为真的幽灵字段。回写本体（含取最大日期语义）由
/// `contract_wave5_purchase_actual_delivery_writeback_test.rs` 真实调用服务断言。
#[tokio::test]
async fn write_back_after_confirm_bring_po_into_lead_time_sample() {
    let (db, sup_main, _sup_other) = setup_db().await;
    // 回写前：NULL 到货日的 COMPLETED 行被排除（现状=该列从无写入点）
    let before = matched_ids_bound_constants(&db, sup_main).await;
    assert_eq!(
        before,
        vec![1, 2],
        "未回写时 id3 必须被 actual_delivery_date IS NOT NULL 排除，实际 {before:?}"
    );
    // 模拟生产回写事务内的落库形态（purchase_receipt_private.rs::
    // write_back_actual_delivery_date：SET actual_delivery_date = 已确认收货最大 receipt_date）
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE purchase_orders SET actual_delivery_date = $2 WHERE id = $1",
        vec![3i32.into(), Value::ChronoDate(Some(date("2026-03-08")))],
    ))
    .await
    .expect("回写形态 UPDATE 执行失败");
    let after = matched_ids_bound_constants(&db, sup_main).await;
    assert_eq!(
        after,
        vec![1, 2, 3],
        "回写到货日后该单必须进入平均交期样本（数值真实化），实际 {after:?}"
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
