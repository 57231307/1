//! 夹具"并行互清窗口"活体复现对 —— 诊断用，**不是回归锁**。
//!
//! ## 它证明什么
//! 集成测试夹具 `setup_test_db()` 每次调用都会对 public 下**全部业务表**执行
//! `TRUNCATE … RESTART IDENTITY CASCADE`（见 `src/services/test_common.rs`，
//! `tests/test_common/mod.rs` 只是再导出 shim）。这是对整库的破坏性清理，
//! 只要两个集成测试**并发**跑在同一只 PostgreSQL 上，就会出现：
//! - 用例 B（victim）setup 后写入并即将断言自己的业务行；
//! - 用例 A（clearer）在其间调用 `setup_test_db()`，一条全局 TRUNCATE 把
//!   B 刚写的行一起清空 → B 的"存在性"断言突然失败。
//! 这正是"上次绿、这次红"的随机红根因。
//!
//! ## 如何用它拿到红/绿证据
//! 本文件两个用例都打了 `#[ignore]`：默认的 10 分片跑不到它们；
//! CI 的 ignored 真库 job 以 `--test-threads=1` 串行跑本档 → **恒绿**。
//! 要复现红，需用 `.config/nextest.toml` 里的 `repro-race` 档（刻意不串行化）并行跑：
//! ```text
//! cargo nextest run -P repro-race --run-ignored only \
//!   --test fixture_truncate_race_repro --test-threads 4
//! ```
//! 预期 `repro_race_victim` 稳定 FAIL。二者对照即证明：并行互清窗口真实存在，
//! 串行化（default 档的 db-integration 组）能消除它。
//! 由于本地禁止 `cargo test/build`，红/绿的活体切换只能在受控并行环境（CI/手工）复核。
//!
//! ## 约定
//! 用 id 落在项目私有段 992x（不复用他用例的 id）；哨兵 code 用运行期语义名，
//! 不新增硬编码 `TEST_` 前缀值（规避反模式）。

mod test_common;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::time::Duration;
use test_common::setup_test_db;

/// victim 用例的哨兵行主键（私有段 992x，避免与其他用例碰撞）。
const VICTIM_ID: i64 = 992_101;
/// clearer 用例的哨兵行主键。
const CLEARER_ID: i64 = 992_102;

/// 向业务表 `work_centers` 写入一行哨兵数据（id/code/name 三列，无 FK 父依赖）。
async fn seed_work_center(db: &DatabaseConnection, id: i64, code: &str) {
    // id 为受控整数常量、code 为本文件内部常量，非外部输入；直接内联避免与列类型不符。
    let sql = format!(
        "INSERT INTO work_centers (id, code, name) VALUES ({id}, '{code}', '{code}-哨兵行')"
    );
    // sea-orm 2.0.2 契约（registry 源码 database/connection.rs:23，参照 tests/bi_analysis_test.rs:48）：
    // execute_raw 按值收 Statement。这里先以 &str 构造语句（Into<String> 不移动 sql），
    // 保留 sql 供失败 panic 打印原文。
    let stmt = Statement::from_string(DbBackend::Postgres, sql.as_str());
    db.execute_raw(stmt)
        .await
        .unwrap_or_else(|e| panic!("种子 work_centers id={id} 写入失败: {e}\nSQL: {sql}"));
}

/// 统计指定 id 的 `work_centers` 行是否仍存在（0 = 已被并行的全局 TRUNCATE 清掉）。
async fn count_work_center(db: &DatabaseConnection, id: i64) -> i64 {
    // sea-orm 2.0.2 契约（registry 源码 database/connection.rs:36 + 本仓既有先例
    // tests/contract_wave4_ar_report_truth_test.rs:686）：泛型 query_one 收 &impl StatementBuilder
    // （SchemaBuilder 族），预构造 Statement 必须走 query_one_raw（按值）。
    let stmt = Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT COUNT(*) AS cnt FROM work_centers WHERE id = {id}"),
    );
    let row = db
        .query_one_raw(stmt)
        .await
        .unwrap_or_else(|e| panic!("count 查询失败（id={id}）: {e}"))
        .unwrap_or_else(|| panic!("count 查询应返回单行聚合结果（id={id}）"));
    row.try_get::<i64>("", "cnt")
        .unwrap_or_else(|e| panic!("COUNT(*) 应解码为 i64: {e}"))
}

/// 受害者：setup → 写自己的哨兵行 → 停留在断言窗口 → 断言行仍在。
///
/// 串行下恒绿；并行（repro-race 档）下 `clearer` 的 setup_test_db 全局 TRUNCATE 会命中此窗口。
#[tokio::test]
#[ignore = "互清窗口活体复现对：仅 repro-race 并行档下预期红，勿纳入默认回归"]
async fn repro_race_victim() {
    let db = setup_test_db().await;
    seed_work_center(&db, VICTIM_ID, "wc-race-victim").await;

    // 关键窗口：这段等待期间，若并行的 clearer 调用 setup_test_db()，
    // 其对全部业务表的 TRUNCATE 会把上面刚写入的 VICTIM 行清空。
    tokio::time::sleep(Duration::from_millis(600)).await;

    let cnt = count_work_center(&db, VICTIM_ID).await;
    assert_eq!(
        cnt, 1,
        "命中夹具并行互清窗口：victim 的哨兵行(id={VICTIM_ID})被并行 clearer 的 \
         setup_test_db 全局 TRUNCATE 清空（实测 count={cnt}，期望 1）。串行/带 serial group 时不应出现。"
    );
}

/// 清场者：先让出时间窗，再调用 setup_test_db() 触发全局 TRUNCATE，
/// 使清理动作恰好落在 victim 的断言窗口内，把互清从"偶发"抬升为"高确定"。
#[tokio::test]
#[ignore = "互清窗口活体复现对：仅 repro-race 并行档下预期让 victim 红，勿纳入默认回归"]
async fn repro_race_clearer() {
    // 让 victim 先完成 setup + 写行；随后本用例的 setup_test_db TRUNCATE 命中其窗口。
    tokio::time::sleep(Duration::from_millis(150)).await;
    let db = setup_test_db().await; // 全局 TRUNCATE 业务表（含 work_centers）
    seed_work_center(&db, CLEARER_ID, "wc-race-clearer").await;
}
