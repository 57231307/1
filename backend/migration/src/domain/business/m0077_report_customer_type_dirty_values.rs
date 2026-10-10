//! customers.customer_type 存量脏值只读点名（收口前置报告，配套 m0078）
//!
//! 为什么单独存在一支只读迁移：`customer_type` 的写入口历史上各写一套（内联白名单、
//! 直落、导入大写 token），存量库里混有 RETAIL/POTENTIAL/normal/vip/NULL 及可能的词表
//! 外未知值。m0078 的"回填后自证"会在未知残留存在时拒绝收口——那只是结果，不是现场。
//! 本迁移把**回填前的逐值现状**以 RAISE NOTICE 逐条摊进迁移日志，使 CI/人工能核对
//! m0078 的映射口径是否覆盖了真实分布、未知值是否需要另裁；日志本身成为可追溯的
//! 点名报告。
//!
//! 口径来源（逐字符对齐，勿在本文件另立词表）：
//! - 权威五值词表：`backend/src/constants/customer_type.rs` 的 `ALLOWED`
//!   （retail/wholesale/distributor/manufacturer/other）；
//! - 已定案映射（m0078 执行）：RETAIL→retail、POTENTIAL→other、normal→other、
//!   vip→other、NULL→other；其余任何不在五值内的值**无定案映射**，落入本迁移
//!   第三类点名，m0078 遇其残留将拒滚等待人工。
//!
//! 严格只读：up 仅 SELECT count + RAISE NOTICE，无 UPDATE/DELETE/DDL，不 RAISE
//! EXCEPTION（此步目的就是把现状摊给人看，抛错会中断 CI 迁移链，与"点名"相悖）。
//! 因此可任意重跑、任意时点重放，无回滚面。
//!
//! 域与注册位置：customers 由 system/m0001_initial_schema.rs:324-339 建表（列定义
//! :332 `VARCHAR(20)` 可空无 CHECK），business 域晚于 system 执行（lib.rs 域序），
//! 故点名可落 business；注册在 business up 链尾（m0072 之后）、且**必须先于 m0078**。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 五值词表的 SQL 字面量清单（与 constants/customer_type.rs ALLOWED 同源逐字符一致）。
const ALLOWED_SQL: &str = "'retail','wholesale','distributor','manufacturer','other'";

/// 已定案映射的四个已知脏值 token → 回填目标（与 m0078 的 UPDATE 一一对应）。
const KNOWN_TOKENS_SQL: &str =
    "('RETAIL','retail'),('POTENTIAL','other'),('normal','other'),('vip','other')";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
DO $$
DECLARE
    rec RECORD;
    total_rows INTEGER;
    null_rows INTEGER;
    mapped_rows INTEGER := 0;
    unknown_rows INTEGER := 0;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "customers";
    RAISE NOTICE 'm0077 点名（只读，不改数据）：customers 共 % 行；customer_type 权威五值词表 = retail/wholesale/distributor/manufacturer/other，以下为逐值现状', total_rows;

    -- 1) 已定案映射的四个已知脏值：逐值一条 NOTICE（含行数与原值），0 行也报——
    --    "报 0"本身就是"该脏值在此库不存在"的证据，日志才可核对。
    FOR rec IN
        SELECT m.token, m.target, COUNT(c."id") AS cnt
          FROM (VALUES {known_tokens}) AS m(token, target)
          LEFT JOIN "customers" c ON c."customer_type" = m.token
         GROUP BY m.token, m.target
         ORDER BY m.token
    LOOP
        RAISE NOTICE 'm0077 点名：原值 ''%'' 行数 % → m0078 回填口径 ''%''（已定案映射）', rec.token, rec.cnt, rec.target;
        mapped_rows := mapped_rows + rec.cnt;
    END LOOP;

    -- 2) NULL 行（已定案：NULL→other）
    SELECT COUNT(*) INTO null_rows FROM "customers" WHERE "customer_type" IS NULL;
    RAISE NOTICE 'm0077 点名：原值 NULL 行数 % → m0078 回填口径 ''other''（已定案映射）', null_rows;

    -- 3) 五值之外、且非上述四个已知 token 的未知脏值：逐值一条 NOTICE。
    --    这些值没有定案映射，m0078 的"回填后自证"会在它们残留时拒绝加 CHECK，
    --    此处先把现场点名出来，供人工在 m0078 失败时直接对号裁定。
    --    排除条件复用同一份 {known_tokens} 清单（与第一段点名同源，不另写 token 列表）。
    FOR rec IN
        SELECT "customer_type" AS token, COUNT(*) AS cnt
          FROM "customers"
         WHERE "customer_type" IS NOT NULL
           AND "customer_type" NOT IN ({allowed})
           AND "customer_type" NOT IN (SELECT k.token FROM (VALUES {known_tokens}) AS k(token, target))
         GROUP BY "customer_type"
         ORDER BY COUNT(*) DESC, "customer_type"
    LOOP
        RAISE NOTICE 'm0077 点名：未知脏值 ''%'' 行数 % —— 不在五值词表、也无定案映射；m0078 遇其残留将 RAISE EXCEPTION 拒绝收口，需人工先定口径', rec.token, rec.cnt;
        unknown_rows := unknown_rows + rec.cnt;
    END LOOP;

    RAISE NOTICE 'm0077 汇总：已知脏值 % 行 + NULL % 行 + 未知脏值 % 行；合规五值行 % 行。本迁移未做任何写入。',
        mapped_rows, null_rows, unknown_rows,
        total_rows - mapped_rows - null_rows - unknown_rows;
END
$$;
"#,
            allowed = ALLOWED_SQL,
            known_tokens = KNOWN_TOKENS_SQL,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 显式 no-op 且这是正确形态（非偷懒）：up 是纯只读迁移——只有 SELECT count
        // 聚合与 RAISE NOTICE，不含任何 UPDATE/DELETE/DDL，对库内数据与结构零改变，
        // 因此不存在任何需要回滚的副作用。迁移日志里留下的"某时点逐值快照"是历史
        // 事实，由迁移记账表与日志系统本身保留，down 无事可做、也不应抹除任何痕迹。
        Ok(())
    }
}
