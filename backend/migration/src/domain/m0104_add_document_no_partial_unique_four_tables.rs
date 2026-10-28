//! 单据号四列「部分唯一索引」DB 兜底（取号-插入同临界区缺陷族的先决约束）
//!
//! 覆盖四列（v15 域建表 DDL 实测，均为裸 `VARCHAR(255) NOT NULL` 列，无任何 UNIQUE
//! 约束或索引）：`production_recipe.recipe_no`、`production_recipe_addition.addition_no`、
//! `process_wage_rate.rate_no`、`wage_record.record_no`。
//! 对照：`cost_collections.collection_no` 已由 `domain/production/m0063_add_document_no_unique_constraints`
//! 补齐全表 UNIQUE（约束名 `uq_cost_collections_collection_no`），不在本迁移范围。
//!
//! 为什么要补这层约束（缺陷族前提）：单据号生成器自开事务形态（`generate_no`）
//! 提交即释放前缀级 advisory 锁，取号与调用方 INSERT 之间存在并发窗口，两次并发
//! 建单会取到同一单号；窗口撞号与旁路/人工写入的重复号，最终都必须由数据库唯一
//! 约束暴露为 SQLSTATE 23505 才能被拒绝——无约束时不报错，**静默落重复单号**，
//! 比撞约束报错严重一级。服务侧取号改「同一事务」形态以本约束为先决：先补约束、
//! 再收临界区，顺序不可反。
//!
//! 为什么是**部分索引**（`WHERE "is_deleted" = false`）而非全表 UNIQUE（照 m0068
//! 化学品族既定判据）：四表的读路径与判重口径都只查未软删行——
//! `services/wage_ops/rate.rs::get_by_no`、`record.rs::get_by_no`、
//! `services/production_recipe_ops/recipe_crud.rs::get_by_id/list/get_by_work_order`、
//! `services/production_recipe_ops/addition.rs::get_by_id/list_by_recipe` 全部带
//! `Column::IsDeleted.eq(false)`；删除通道（`delete()`/软删置位）置
//! `is_deleted = true` 而非物理删行，既定语义 = **软删后单号可复用**。
//! 建全表 UNIQUE 会把软删历史行永久占号，与该读路径语义漂移，故禁止。
//! 探测占用另说：`utils/number_generator.rs` 的 `allocate_no` 基数与探测含软删行
//! （软删行仍占号段、不会被再次发出），与部分索引谓词不冲突。
//!
//! 存量重复处置策略（照 m0063/m0068/m0082 fail-visible 先例，禁止静默洗数据）：
//! 每条索引建前先 `SELECT <no>, COUNT(*) ... WHERE "is_deleted" = false
//! GROUP BY <no> HAVING COUNT(*) > 1` 只读探测（探测口径、索引谓词、应用读路径
//! 三者同源），发现任何重复未删行即 RAISE EXCEPTION 中止迁移（消息含表名、列名、
//! 重复单号个数与最多 10 个样例号），交数据治理人工裁定处置后重跑。本迁移绝不
//! UPDATE/DELETE/改名洗数据——建议口径（保留最早 id 行、其余置 is_deleted=true
//! 并在 remarks 留痕）只写在 RAISE 消息与本文档，不自动化。
//! 生产库与测试库均实测无重复未删行（2026-10-25 复核），守卫属防线非常开。
//!
//! 幂等性：up 为「只读探测 → CREATE UNIQUE INDEX IF NOT EXISTS」，重跑等价；
//! down 显式 `DROP INDEX IF EXISTS` 四行（不留空 down），仅删索引本身、不触碰
//! 任何数据行，回滚后回到「依赖应用层无判重」的现状薄弱态与本迁移前一致。
//! 存量重复与否由守卫运行时判定，本文件不预设结论：测试库实测在
//! `backend/tests/contract_docno_partial_unique_four_tables_test.rs` 活体用例中
//! 对同一条只读探测 SQL 断言计数为 0（命中重复时迁移与用例都会点名，属真红非误伤）。
//!
//! 域位置与注册：置于全链尾为新的顶层条目（`domain/customers_nullable_columns_norm.rs`
//! 同款单文件域）——四张目标表均由 v15 域建表，链尾执行时表必然已存在，无跨域
//! 尾调问题；并入已应用域会被台账整体跳过，复用库上永不生效。
//!
//! 契约锁：`backend/tests/contract_docno_partial_unique_four_tables_test.rs`
//! 锁定「四条部分唯一索引形态 + WHERE 谓词与读路径判重同源 + 重复号插入被拒 +
//! 软删后同号可复用 + up/down 对称」，防后续漂移。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 每表一对：① fail-visible 存量重复探测（仅未删行，只读；命中即 RAISE 中止）
        //          ② 幂等部分唯一索引创建（谓词 = 读路径/探测同口径 is_deleted = false）
        let sql = r#"
-- 1) production_recipe.recipe_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "recipe_no"
          FROM "production_recipe"
         WHERE "is_deleted" = false
         GROUP BY "recipe_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("recipe_no", ', ' ORDER BY "recipe_no")
          INTO dup_no_samples
          FROM (
            SELECT "recipe_no"
              FROM "production_recipe"
             WHERE "is_deleted" = false
             GROUP BY "recipe_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 production_recipe 的列 recipe_no 存在 % 个未删重复单号（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_production_recipe_recipe_no_active"
    ON "production_recipe" ("recipe_no") WHERE "is_deleted" = false;

-- 2) production_recipe_addition.addition_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "addition_no"
          FROM "production_recipe_addition"
         WHERE "is_deleted" = false
         GROUP BY "addition_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("addition_no", ', ' ORDER BY "addition_no")
          INTO dup_no_samples
          FROM (
            SELECT "addition_no"
              FROM "production_recipe_addition"
             WHERE "is_deleted" = false
             GROUP BY "addition_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 production_recipe_addition 的列 addition_no 存在 % 个未删重复单号（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_production_recipe_addition_addition_no_active"
    ON "production_recipe_addition" ("addition_no") WHERE "is_deleted" = false;

-- 3) process_wage_rate.rate_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "rate_no"
          FROM "process_wage_rate"
         WHERE "is_deleted" = false
         GROUP BY "rate_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("rate_no", ', ' ORDER BY "rate_no")
          INTO dup_no_samples
          FROM (
            SELECT "rate_no"
              FROM "process_wage_rate"
             WHERE "is_deleted" = false
             GROUP BY "rate_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 process_wage_rate 的列 rate_no 存在 % 个未删重复单号（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_process_wage_rate_rate_no_active"
    ON "process_wage_rate" ("rate_no") WHERE "is_deleted" = false;

-- 4) wage_record.record_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "record_no"
          FROM "wage_record"
         WHERE "is_deleted" = false
         GROUP BY "record_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("record_no", ', ' ORDER BY "record_no")
          INTO dup_no_samples
          FROM (
            SELECT "record_no"
              FROM "wage_record"
             WHERE "is_deleted" = false
             GROUP BY "record_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 wage_record 的列 record_no 存在 % 个未删重复单号（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_wage_record_record_no_active"
    ON "wage_record" ("record_no") WHERE "is_deleted" = false;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚只删索引本身（不触碰任何数据行）。删除后并发撞号保护退回
        // "依赖应用层（且本族应用层原无判重）"的薄弱态，与本迁移之前的现状一致。
        let sql = r#"
DROP INDEX IF EXISTS "uq_production_recipe_recipe_no_active";
DROP INDEX IF EXISTS "uq_production_recipe_addition_addition_no_active";
DROP INDEX IF EXISTS "uq_process_wage_rate_rate_no_active";
DROP INDEX IF EXISTS "uq_wage_record_record_no_active";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
