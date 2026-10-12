//! 化学品主数据三表编码列「部分唯一索引」DB 兜底（Wave-I §④ 收口）
//!
//! 背景：`chemical_category.category_code` / `chemical_master.chemical_code` /
//! `chemical_lot.lot_no` 三列建表时仅 `VARCHAR(255) NOT NULL`、无任何 UNIQUE/索引
//! （证据：`domain/v15/mod.rs:2457/:2499/:2472` 建表 DDL），应用层判重
//! （`services/chemical_ops/category.rs:50-60`、`master.rs::check_chemical_code_uniqueness`
//! :101-107、`lot.rs:54-61`）均为「先查后插」，查重-插入竞态窗口可落重复未删行。
//! 后端已在三服务 INSERT 路径加 `SqlErr::UniqueConstraintViolation` 竞态兜底降级
//! （Wave-I §②），其生效前提就是本迁移的 DB 约束。
//!
//! 为什么必须是**部分索引**（`WHERE "is_deleted" = false`）而非全表 UNIQUE：
//! 三处应用层判重口径都只查未软删行（`Column::IsDeleted.eq(false)`，
//! category.rs:56 / master.rs:107 / lot.rs:59），即本表族既定语义 =
//! **软删后同码可复用**；e2e `70-chemical-contract-gaps.spec.ts` 70-01/70-02 对该
//! 复用语义有明确断言。建全表 UNIQUE 会打红该断言、破坏既定语义，故禁止。
//!
//! 列名/可空性实测核对（不猜）：
//! - `chemical_category`：`category_code` VARCHAR(255) NOT NULL、`is_deleted`
//!   BOOLEAN NOT NULL（v15/mod.rs:2459/:2466；实体 `models/chemical_category.rs:20/:34`）
//! - `chemical_master`：`chemical_code` VARCHAR(255) NOT NULL、`is_deleted`
//!   BOOLEAN NOT NULL（v15/mod.rs:2501/:2541；实体 `models/chemical_master.rs:22/:111`）
//! - `chemical_lot`：`lot_no` VARCHAR(255) NOT NULL、`is_deleted`
//!   BOOLEAN NOT NULL（v15/mod.rs:2474/:2492；实体 `models/chemical_lot.rs:22/:64`）
//!   三列均 NOT NULL，无 NULL 绕过唯一性的问题；软删列名三表一致为 `is_deleted`。
//!
//! 存量重复处置策略（照 m0063/m0064/m0067 fail-visible 先例，禁止静默洗数据）：
//! 每个索引建前先 `SELECT <code>, COUNT(*) ... WHERE "is_deleted" = false
//! GROUP BY <code> HAVING COUNT(*) > 1` 只读探测（探测口径与索引谓词、应用判重
//! 三者同源），发现任何重复未删行即 RAISE EXCEPTION 中止迁移（消息含表名、列名、
//! 重复码个数与最多 10 个样例码），交数据治理人工裁定处置后重跑。本迁移绝不
//! UPDATE/DELETE/改名洗数据——推荐的处置口径（保留最早 id、其余置 is_deleted=true
//! 并在 remarks 留痕）只写在 RAISE 消息与本文档中供人工执行，不自动化。
//!
//! 幂等性：up 为「只读探测 → CREATE UNIQUE INDEX IF NOT EXISTS」，重跑等价；
//! down 显式 `DROP INDEX IF EXISTS` 三行（不留空 down，教训见 rls_dept 登记），
//! 仅删索引本身、不触碰任何数据行，回滚后回到「依赖应用层判重」的现状。
//!
//! 域内注册位置：三表均在本 v15 域内建表（:2457/:2472/:2499），而 production 域
//! 早于 v15 执行，直接注册本域 up() 会因 "relation ... does not exist" 中断迁移链。
//! 照 m0058/m0063/m0065 先例：文件留在 production 域（m00NN 命名序列），
//! up/down 由 `domain/v15/mod.rs` 在其全部建表完成后调用（见该文件末段）。
//!
//! 契约锁：`backend/tests/contract_wave7_chemical_partial_unique_index_test.rs`
//! 锁定「三条部分唯一索引形态 + WHERE 谓词 + 探测 fail-closed + down 显式回滚 +
//! v15 注册在场」，防止后续一边改一边忘。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 每表一对：① fail-closed 存量重复探测（仅未删行，只读；命中即 RAISE 中止）
        //          ② 幂等部分唯一索引创建（谓词 = 判重/探测同口径 is_deleted = false）
        let sql = r#"
-- 1) chemical_category.category_code
DO $$
DECLARE
    dup_code_count INTEGER;
    dup_code_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_code_count
      FROM (
        SELECT "category_code"
          FROM "chemical_category"
         WHERE "is_deleted" = false
         GROUP BY "category_code"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_code_count > 0 THEN
        SELECT string_agg("category_code", ', ' ORDER BY "category_code")
          INTO dup_code_samples
          FROM (
            SELECT "category_code"
              FROM "chemical_category"
             WHERE "is_deleted" = false
             GROUP BY "category_code"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 chemical_category 的列 category_code 存在 % 个未删重复编码（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_code_count, dup_code_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_category_code_active"
    ON "chemical_category" ("category_code") WHERE "is_deleted" = false;

-- 2) chemical_master.chemical_code
DO $$
DECLARE
    dup_code_count INTEGER;
    dup_code_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_code_count
      FROM (
        SELECT "chemical_code"
          FROM "chemical_master"
         WHERE "is_deleted" = false
         GROUP BY "chemical_code"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_code_count > 0 THEN
        SELECT string_agg("chemical_code", ', ' ORDER BY "chemical_code")
          INTO dup_code_samples
          FROM (
            SELECT "chemical_code"
              FROM "chemical_master"
             WHERE "is_deleted" = false
             GROUP BY "chemical_code"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 chemical_master 的列 chemical_code 存在 % 个未删重复编码（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_code_count, dup_code_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_master_code_active"
    ON "chemical_master" ("chemical_code") WHERE "is_deleted" = false;

-- 3) chemical_lot.lot_no
DO $$
DECLARE
    dup_code_count INTEGER;
    dup_code_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_code_count
      FROM (
        SELECT "lot_no"
          FROM "chemical_lot"
         WHERE "is_deleted" = false
         GROUP BY "lot_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_code_count > 0 THEN
        SELECT string_agg("lot_no", ', ' ORDER BY "lot_no")
          INTO dup_code_samples
          FROM (
            SELECT "lot_no"
              FROM "chemical_lot"
             WHERE "is_deleted" = false
             GROUP BY "lot_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 chemical_lot 的列 lot_no 存在 % 个未删重复批号（样例：%）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重/软删洗数据：请人工处置上述重复（建议保留最早 id 行、其余置 is_deleted=true 并在 remarks 留痕，口径由数据治理裁定）后重跑。',
            dup_code_count, dup_code_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_lot_no_active"
    ON "chemical_lot" ("lot_no") WHERE "is_deleted" = false;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚只删索引本身（不触碰任何数据行）。删除后竞态保护退回
        // "依赖应用层判重"的薄弱态，与本迁移之前的现状一致。
        let sql = r#"
DROP INDEX IF EXISTS "uk_chemical_category_code_active";
DROP INDEX IF EXISTS "uk_chemical_master_code_active";
DROP INDEX IF EXISTS "uk_chemical_lot_no_active";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
