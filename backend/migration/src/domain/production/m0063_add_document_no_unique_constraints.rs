//! 单据号列数据库 UNIQUE 兜底：取号器重号保护的真实生效前提（B1）
//!
//! 依据（契约同源，非本文件自创规则）：
//! - `.monkeycode/MEMORY.md:84`（IR，2026-09-17）：单据号由系统自动生成，
//!   后端「传了就用、没传才生成」+ **数据库 UNIQUE 兜底**。
//! - `src/utils/number_generator.rs::insert_with_no_retry` 的重号保护依赖
//!   捕获唯一约束冲突（PostgreSQL SQLSTATE 23505 → `SqlErr::UniqueConstraintViolation`）
//!   后在保存点内重新取号重试。**该列没有 UNIQUE 约束时，旁路写入 / 人工输入
//!   重复号 / 并发撞号全部静默落库，重试保护形同虚设。**
//!
//! 逐表实测核对结论（下列 9 列建表时均未带 UNIQUE，且全部迁移集合中不存在
//! 任何补偿性 `CREATE UNIQUE INDEX` / `ADD CONSTRAINT ... UNIQUE`，证据为建表行号）：
//! - `purchase_inspection.inspection_no`  business/m0009:147
//! - `purchase_return.return_no`          business/m0009:179
//!   （business/m0011:118 带 UNIQUE 的是 `sales_return.return_no`，非本表；
//!    m0009:68 带 UNIQUE 的是 `purchase_receipt.receipt_no`，非 outsourcing_receipt）
//! - `outsourcing_order.order_no`         v15:3229
//! - `outsourcing_receipt.receipt_no`     v15:691
//! - `ar_invoices.invoice_no`             business/m0012:210
//!   （m0012:20 / m0012:142 带 UNIQUE 的是 ap 侧 `ap_invoice.invoice_no` /
//!    `ap_reconciliation.reconciliation_no`，ar 侧三列均无）
//! - `ar_reconciliations.reconciliation_no` business/m0012:273
//!   （sales_crm/mod.rs:433 的 `ADD COLUMN IF NOT EXISTS reconciliation_no`
//!    对已存在列是 no-op，不新增任何约束）
//! - `ar_collections.collection_no`       business/m0012:248
//! - `cost_collections.collection_no`     business/m0012:381
//! - `finance_invoices.invoice_no`        v15:2934
//!
//! 存量重复处置策略（fail-visible，禁止静默去重）：
//! 每个唯一索引之前先做 `SELECT <col>, COUNT(*) ... GROUP BY <col>
//! HAVING COUNT(*) > 1` 重复检测，发现任何重复即 `RAISE EXCEPTION`
//! 中止迁移（消息含表名、列名、重复单号个数与最多 10 个样例号），
//! 由人工对照业务处置存量数据后重跑。绝不使用冲突忽略、任一行保留、
//! 删除保一类的兜底去重把数据问题吞掉——那是把数据问题降级成
//! schema 问题，违反 IR 红线。
//!
//! 软删行语义：这些表多数带 `is_deleted`，唯一索引对软删行同样生效
//! （软删行仍占用单号）。与 `allocate_no` 的 max(seq)+1 取基数同源：
//! 号段中已被占用（含软删）的号不再复用，避免历史号二次出现。
//!
//! 域内注册位置：9 张表中 `outsourcing_order` / `outsourcing_receipt` /
//! `finance_invoices` 在 v15 域内建表，而本域（production）早于 v15 执行，
//! 若注册在本域 up() 会因 "relation ... does not exist" 中断迁移链。
//! 照仓内既有先例（m0058 目标表在 v15 域内建，up/down 后置到 v15 域尾调用，
//! 见 domain/production/mod.rs:31-34 与 domain/v15/mod.rs 末段），
//! 本迁移文件留在 production 域（m00NN 命名序列），up/down 由 domain/v15/mod.rs
//! 在其全部建表完成后调用。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 每表一对：① fail-visible 重复检测（RAISE 异常即中止整个迁移事务）
        //          ② 幂等唯一索引创建（IF NOT EXISTS，可重跑）
        let sql = r#"
-- 1) purchase_inspection.inspection_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "inspection_no"
          FROM "purchase_inspection"
         GROUP BY "inspection_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("inspection_no", ', ' ORDER BY "inspection_no")
          INTO dup_no_samples
          FROM (
            SELECT "inspection_no"
              FROM "purchase_inspection"
             GROUP BY "inspection_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 purchase_inspection 的列 inspection_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_purchase_inspection_inspection_no"
    ON "purchase_inspection" ("inspection_no");

-- 2) purchase_return.return_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "return_no"
          FROM "purchase_return"
         GROUP BY "return_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("return_no", ', ' ORDER BY "return_no")
          INTO dup_no_samples
          FROM (
            SELECT "return_no"
              FROM "purchase_return"
             GROUP BY "return_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 purchase_return 的列 return_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_purchase_return_return_no"
    ON "purchase_return" ("return_no");

-- 3) outsourcing_order.order_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "order_no"
          FROM "outsourcing_order"
         GROUP BY "order_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("order_no", ', ' ORDER BY "order_no")
          INTO dup_no_samples
          FROM (
            SELECT "order_no"
              FROM "outsourcing_order"
             GROUP BY "order_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 outsourcing_order 的列 order_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_outsourcing_order_order_no"
    ON "outsourcing_order" ("order_no");

-- 4) outsourcing_receipt.receipt_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "receipt_no"
          FROM "outsourcing_receipt"
         GROUP BY "receipt_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("receipt_no", ', ' ORDER BY "receipt_no")
          INTO dup_no_samples
          FROM (
            SELECT "receipt_no"
              FROM "outsourcing_receipt"
             GROUP BY "receipt_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 outsourcing_receipt 的列 receipt_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_outsourcing_receipt_receipt_no"
    ON "outsourcing_receipt" ("receipt_no");

-- 5) ar_invoices.invoice_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "invoice_no"
          FROM "ar_invoices"
         GROUP BY "invoice_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("invoice_no", ', ' ORDER BY "invoice_no")
          INTO dup_no_samples
          FROM (
            SELECT "invoice_no"
              FROM "ar_invoices"
             GROUP BY "invoice_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 ar_invoices 的列 invoice_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_ar_invoices_invoice_no"
    ON "ar_invoices" ("invoice_no");

-- 6) ar_reconciliations.reconciliation_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "reconciliation_no"
          FROM "ar_reconciliations"
         GROUP BY "reconciliation_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("reconciliation_no", ', ' ORDER BY "reconciliation_no")
          INTO dup_no_samples
          FROM (
            SELECT "reconciliation_no"
              FROM "ar_reconciliations"
             GROUP BY "reconciliation_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 ar_reconciliations 的列 reconciliation_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_ar_reconciliations_reconciliation_no"
    ON "ar_reconciliations" ("reconciliation_no");

-- 7) ar_collections.collection_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "collection_no"
          FROM "ar_collections"
         GROUP BY "collection_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("collection_no", ', ' ORDER BY "collection_no")
          INTO dup_no_samples
          FROM (
            SELECT "collection_no"
              FROM "ar_collections"
             GROUP BY "collection_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 ar_collections 的列 collection_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_ar_collections_collection_no"
    ON "ar_collections" ("collection_no");

-- 8) cost_collections.collection_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "collection_no"
          FROM "cost_collections"
         GROUP BY "collection_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("collection_no", ', ' ORDER BY "collection_no")
          INTO dup_no_samples
          FROM (
            SELECT "collection_no"
              FROM "cost_collections"
             GROUP BY "collection_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 cost_collections 的列 collection_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_cost_collections_collection_no"
    ON "cost_collections" ("collection_no");

-- 9) finance_invoices.invoice_no
DO $$
DECLARE
    dup_no_count INTEGER;
    dup_no_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_no_count
      FROM (
        SELECT "invoice_no"
          FROM "finance_invoices"
         GROUP BY "invoice_no"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_no_count > 0 THEN
        SELECT string_agg("invoice_no", ', ' ORDER BY "invoice_no")
          INTO dup_no_samples
          FROM (
            SELECT "invoice_no"
              FROM "finance_invoices"
             GROUP BY "invoice_no"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 finance_invoices 的列 invoice_no 存在 % 个重复单号（样例：%）。本迁移拒绝在带存量重号的表上建唯一索引，且不做静默去重：请人工处置上述重复号后重跑。',
            dup_no_count, dup_no_samples;
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_finance_invoices_invoice_no"
    ON "finance_invoices" ("invoice_no");
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚只删索引本身（不触碰任何数据行）。删除后重号保护退回
        // "依赖取号器探测"的薄弱态，与本迁移之前的现状一致。
        let sql = r#"
DROP INDEX IF EXISTS "uq_purchase_inspection_inspection_no";
DROP INDEX IF EXISTS "uq_purchase_return_return_no";
DROP INDEX IF EXISTS "uq_outsourcing_order_order_no";
DROP INDEX IF EXISTS "uq_outsourcing_receipt_receipt_no";
DROP INDEX IF EXISTS "uq_ar_invoices_invoice_no";
DROP INDEX IF EXISTS "uq_ar_reconciliations_reconciliation_no";
DROP INDEX IF EXISTS "uq_ar_collections_collection_no";
DROP INDEX IF EXISTS "uq_cost_collections_collection_no";
DROP INDEX IF EXISTS "uq_finance_invoices_invoice_no";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
