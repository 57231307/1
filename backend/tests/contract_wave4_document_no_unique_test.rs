//! 任务 wave4 契约锁（B1/B4）：单据号列数据库 UNIQUE 兜底**真实存在**且**真实拒绝重号**
//!
//! 锁定三层事实：
//! 1. **迁移集合逐表锁定**：`migration/src/domain/production/m0063_add_document_no_unique_constraints.rs`
//!    中，9 张表的单号列**逐表**存在「fail-visible 重复检测（GROUP BY + HAVING COUNT(*) > 1
//!    + RAISE EXCEPTION）→ CREATE UNIQUE INDEX IF NOT EXISTS」语句对，且 down 有对应
//!    DROP INDEX 回滚；任何一处缺失即逐表测试点名判红（禁止整批一句断言）。
//! 2. **禁止静默去重**：m0063 全文不得出现冲突忽略 / 删除保一 / 取任一行的去重手段
//!    （ON CONFLICT DO NOTHING、DELETE FROM、WHERE id NOT IN、ROW_NUMBER、DISTINCT ON、ctid）。
//!    发现存量重复只能通过 RAISE EXCEPTION 中止迁移、交人工处置。
//! 3. **注册位置真实**：m0063 在 production 域声明（pub(crate) mod），up/down 由 v15 域
//!    在全部目标表建表之后/回滚最先处调用（m0058 先例）——只声明不调用不算注册。
//!
//! 另附 B4 行为锁：
//! - `number_generator.rs` 取号基数为 max(seq)+1（into_tuple 投影真实单号），
//!   不再出现 `count + 1`；`AppError::internal` 收口为 0 处（DbErr→database，
//!   AppError 原样透传）；advisory lock 仍在且前提已注释声明。
//! - **真实行为断言（路线一：已迁移 PostgreSQL 真表）**：在迁移 m0009 建出的真实
//!   `purchase_inspection` 表上（唯一索引 `uq_purchase_inspection_inspection_no`
//!   由 m0063 落库，不自建任何 DDL），走真实实体 `ActiveModel::insert` 写入同一单号，
//!   第二次必须被**数据库本身**拒绝，且错误经 `sql_err()` 分类为
//!   `SqlErr::UniqueConstraintViolation`——这正是 `insert_with_no_retry`
//!   「23505→保存点重试」的匹配分支（PG 上由 SQLSTATE 23505 直接进入该变体，
//!   与生产链路完全同语义；旧 sqlite 同构表方案仅验证了 sea-orm 的字面量映射，
//!   无法证明真库约束真实存在，路线一改造后此风险点由本用例直接覆盖）。
//!   同时用真实注册表函数 `is_document_no_taken` 验证同列查重从 false 翻转为 true。

use std::path::PathBuf;

use bingxi_backend::models::purchase_inspection;
use bingxi_backend::utils::number_generator::is_document_no_taken;
use chrono::{NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait, PaginatorTrait, SqlErr};

mod test_common;
use test_common::setup_test_db;

fn read(rel: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
}

const M0063_PATH: &str =
    "migration/src/domain/production/m0063_add_document_no_unique_constraints.rs";
const PRODUCTION_MOD_PATH: &str = "migration/src/domain/production/mod.rs";
const V15_MOD_PATH: &str = "migration/src/domain/v15/mod.rs";
const NUMBER_GENERATOR_PATH: &str = "src/utils/number_generator.rs";

fn m0063_src() -> String {
    read(M0063_PATH)
}

/// 截取 m0063 中某张表的守卫块：从逐表标记行 `-- N) <table>.<column>` 之后，
/// 到下一个 `-- ` 标记（或 up() SQL 串结束）为止——保证逐表断言只看自己的语句，
/// 不让 A 表的 GROUP BY 蒙混替 B 表过关。
fn guard_block<'a>(src: &'a str, table: &str, column: &str) -> &'a str {
    let marker = format!(") {table}.{column}");
    let start = src.find(&marker).unwrap_or_else(|| {
        panic!("m0063 缺少逐表守卫块标记 '-- N) {table}.{column}'，该表未被覆盖")
    });
    let rest = &src[start + marker.len()..];
    let end = rest
        .find("\n-- ")
        .or_else(|| rest.find("\n\"#"))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// 逐表锁定：fail-visible 重复检测语句 + 唯一索引创建语句 + down 回滚语句。
fn assert_table_guarded(table: &str, column: &str, index: &str) {
    let src = m0063_src();
    let block = guard_block(&src, table, column);

    // ① 重复检测：对目标列 GROUP BY、对目标表 FROM、HAVING 判定重复
    assert!(
        block.contains(&format!("FROM \"{table}\"")),
        "{table}.{column}: 守卫块缺少对表 {table} 的重复检测查询"
    );
    assert!(
        block.contains(&format!("GROUP BY \"{column}\"")),
        "{table}.{column}: 守卫块缺少 GROUP BY \"{column}\" 重复探测"
    );
    assert!(
        block.contains("HAVING COUNT(*) > 1"),
        "{table}.{column}: 守卫块缺少 HAVING COUNT(*) > 1 重复判定"
    );
    // ② 发现重复必须显式中止迁移，错误消息含表名与重复号个数（fail-visible）
    assert!(
        block.contains(&format!(
            "RAISE EXCEPTION '表 {table} 的列 {column} 存在 % 个重复单号"
        )),
        "{table}.{column}: 缺少 RAISE EXCEPTION 显式中止（消息须含表名/列名/重复号个数）"
    );
    // ③ 唯一索引：幂等创建语句，索引名与列逐字锁定
    assert!(
        block.contains(&format!("CREATE UNIQUE INDEX IF NOT EXISTS \"{index}\"")),
        "{table}.{column}: 缺少 CREATE UNIQUE INDEX IF NOT EXISTS \"{index}\""
    );
    assert!(
        block.contains(&format!("ON \"{table}\" (\"{column}\")")),
        "{table}.{column}: 唯一索引未落在 ON \"{table}\" (\"{column}\") 上"
    );
    // ④ 回滚：down() 有对应 DROP INDEX（只删约束不碰数据）
    assert!(
        src.contains(&format!("DROP INDEX IF EXISTS \"{index}\";")),
        "{table}.{column}: down() 缺少回滚语句 DROP INDEX IF EXISTS \"{index}\""
    );
}

// =========================================================
// 9 张表逐表断言（每表一个独立 #[test]，缺失即点名判红）
// =========================================================

#[test]
fn purchase_inspection_inspection_no_is_guarded() {
    assert_table_guarded(
        "purchase_inspection",
        "inspection_no",
        "uq_purchase_inspection_inspection_no",
    );
}

#[test]
fn purchase_return_return_no_is_guarded() {
    assert_table_guarded(
        "purchase_return",
        "return_no",
        "uq_purchase_return_return_no",
    );
}

#[test]
fn outsourcing_order_order_no_is_guarded() {
    assert_table_guarded(
        "outsourcing_order",
        "order_no",
        "uq_outsourcing_order_order_no",
    );
}

#[test]
fn outsourcing_receipt_receipt_no_is_guarded() {
    assert_table_guarded(
        "outsourcing_receipt",
        "receipt_no",
        "uq_outsourcing_receipt_receipt_no",
    );
}

#[test]
fn ar_invoices_invoice_no_is_guarded() {
    assert_table_guarded("ar_invoices", "invoice_no", "uq_ar_invoices_invoice_no");
}

#[test]
fn ar_reconciliations_reconciliation_no_is_guarded() {
    assert_table_guarded(
        "ar_reconciliations",
        "reconciliation_no",
        "uq_ar_reconciliations_reconciliation_no",
    );
}

#[test]
fn ar_collections_collection_no_is_guarded() {
    assert_table_guarded(
        "ar_collections",
        "collection_no",
        "uq_ar_collections_collection_no",
    );
}

#[test]
fn cost_collections_collection_no_is_guarded() {
    assert_table_guarded(
        "cost_collections",
        "collection_no",
        "uq_cost_collections_collection_no",
    );
}

#[test]
fn finance_invoices_invoice_no_is_guarded() {
    assert_table_guarded(
        "finance_invoices",
        "invoice_no",
        "uq_finance_invoices_invoice_no",
    );
}

// =========================================================
// 整批一致性 + 禁止静默去重 + 注册位置
// =========================================================

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// 9 对守卫语句数量互证：检测块、索引创建、回滚三者各 9，防"多建/漏建"漂移。
#[test]
fn m0063_guards_are_exactly_nine_per_kind() {
    let src = m0063_src();
    assert_eq!(
        count_occurrences(&src, "RAISE EXCEPTION '表"),
        9,
        "fail-visible 重复检测块应恰为 9（每表一块）"
    );
    assert_eq!(
        count_occurrences(&src, "CREATE UNIQUE INDEX IF NOT EXISTS"),
        9,
        "唯一索引创建语句应恰为 9"
    );
    assert_eq!(
        count_occurrences(&src, "DROP INDEX IF EXISTS"),
        9,
        "down 回滚语句应恰为 9"
    );
}

/// 红线锁定：迁移内不得以任何兜底方式吞掉存量重复——只允许 RAISE EXCEPTION 中止。
#[test]
fn m0063_contains_no_silent_dedup_escape_hatch() {
    let src = m0063_src().to_uppercase();
    for forbidden in [
        "ON CONFLICT DO NOTHING",
        "DELETE FROM",
        " WHERE ID NOT IN",
        "ROW_NUMBER",
        "DISTINCT ON",
        "CTID",
    ] {
        assert!(
            !src.contains(forbidden),
            "m0063 出现静默去重/兜底吞数据手段: {forbidden}"
        );
    }
}

/// 注册真实性：production 域声明模块；v15 域在全部建表后调用 up()、down() 逆序最先回滚。
/// 只在 mod.rs 里 `mod` 却不调用 = 未注册（迁移集合里根本不会执行）。
#[test]
fn m0063_is_registered_to_execute_after_all_target_tables() {
    let production_mod = read(PRODUCTION_MOD_PATH);
    assert!(
        production_mod.contains("pub(crate) mod m0063_add_document_no_unique_constraints;"),
        "m0063 未在 production 域 mod.rs 声明"
    );

    let v15_mod = read(V15_MOD_PATH);
    let up_call = "crate::domain::production::m0063_add_document_no_unique_constraints::Migration";
    assert_eq!(
        count_occurrences(&v15_mod, up_call),
        2,
        "v15 域应恰好调用 m0063 的 Migration 两次（up 一次 + down 一次）"
    );
    let up_pos = v15_mod
        .find(&format!("{up_call}\n            .up(manager)"))
        .expect("v15 up() 未调用 m0063（或调用写法与 m0058 后置注册范式不一致）");
    let down_pos = v15_mod
        .find(&format!("{up_call}\n            .down(manager)"))
        .expect("v15 down() 未调用 m0063 回滚");
    let down_fn = v15_mod
        .find("async fn down")
        .expect("v15 域缺少 down() 定义");
    assert!(
        up_pos < down_fn && down_fn < down_pos,
        "m0063 的 up 调用必须位于 up() 内、down 调用必须位于 down() 内（逆序对称）"
    );
}

// =========================================================
// B4：取号基数与错误收口的源码锁 + 真实行为断言
// =========================================================

/// 基数改判：真实投影当日单号解析 max(seq)+1（into_tuple），旧的 `count + 1`
/// 基数与 `.count(conn)` 调用不得回潮；错误收口：`AppError::internal` 清零、
/// advisory lock 语句仍在（不许删锁是任务红线）。
#[test]
fn number_generator_base_is_segment_aligned_and_errors_are_classified() {
    let src = read(NUMBER_GENERATOR_PATH);
    assert!(
        src.contains(".into_tuple::<(String,)>()"),
        "allocate_no 应经 select_only+into_tuple 投影真实单号解析序号基数"
    );
    assert!(
        src.contains("max_seq.map_or(1, |m| m.saturating_add(1))"),
        "取号基数应为 max(seq)+1（与真实号段同源）"
    );
    assert!(
        !src.contains("let mut seq = count + 1") && !src.contains(".count(conn)"),
        "旧的 count+1 基数（软删行计入导致跳号）不得回潮"
    );
    assert!(
        !src.contains("AppError::internal("),
        "number_generator 内 AppError::internal 应全部收口（DbErr→database / AppError 透传）"
    );
    assert!(
        src.contains("SELECT pg_advisory_xact_lock($1)"),
        "advisory lock 不许删除"
    );
    assert!(
        src.contains("跨进程") && src.contains("DefaultHasher"),
        "advisory lock key 的跨进程/跨版本前提必须显式声明在注释中"
    );
}

fn inspection_with(id: i32, inspection_no: &str) -> purchase_inspection::ActiveModel {
    purchase_inspection::ActiveModel {
        id: Set(id),
        inspection_no: Set(inspection_no.to_string()),
        supplier_id: Set(7),
        inspection_date: Set(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
}

/// 真实行为断言（真库）：迁移 m0009 建表 + m0063 唯一索引的**生产同构真表**上，
/// 同一单据号的第二次写入必须被数据库拒绝，且错误可被 `sql_err()` 分类为
/// SqlErr::UniqueConstraintViolation（= PG SQLSTATE 23505，`insert_with_no_retry`
/// 保存点重试的触发端）。若本用例在真库上报「不拒绝」或分类不符，说明 m0063 的
/// 唯一索引在真实库上缺失/未生效——属「疑迁移/表结构缺口」，判红交回，不放行。
/// 并用真实取号注册表 `is_document_no_taken`（number_generator 本体，非复刻）
/// 验证同列查重状态随写入翻转。
#[tokio::test]
async fn second_write_of_same_document_no_is_rejected_on_unique_column() {
    let db = setup_test_db().await;

    let dup_no = "PI20261001001";
    assert!(
        !is_document_no_taken(&db, "purchase_inspection", dup_no)
            .await
            .expect("真实注册表查重查询失败"),
        "初始状态该单号不应已存在"
    );

    inspection_with(1, dup_no)
        .insert(&db)
        .await
        .expect("首条记录写入应成功（真表 purchase_inspection）");
    assert!(
        is_document_no_taken(&db, "purchase_inspection", dup_no)
            .await
            .expect("真实注册表查重查询失败"),
        "写入后 is_document_no_taken（number_generator 注册表，同列）必须报占用"
    );

    let err = inspection_with(2, dup_no)
        .insert(&db)
        .await
        .expect_err("重复单号第二次写入必须被真库上 m0063 的唯一索引拒绝");
    assert!(
        matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))),
        "第二次写入的错误必须分类为 SqlErr::UniqueConstraintViolation（insert_with_no_retry 的 23505 重试触发端），实得: {err}"
    );

    let rows = purchase_inspection::Entity::find()
        .count(&db)
        .await
        .expect("计数查询失败");
    assert_eq!(
        rows, 1,
        "重号确未落库：真表内仍只有一行（对照无约束现状的静默双落库）"
    );
}
