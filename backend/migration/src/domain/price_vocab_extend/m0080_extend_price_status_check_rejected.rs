//! 价目两表 status CHECK 扩 `rejected` 态（m_price_vocab_check 的后继收紧迁移）
//!
//! ## 六要素
//! - **功能**：把 `sales_prices.status` / `purchase_prices.status` 两条 CHECK 的取值集
//!   各扩一个 `rejected` 终态（拒绝动作落库需要），**约束名保持不变**
//!   （`chk_sales_price_status` / `chk_purchase_price_status`，parity 契约锁按名查找，
//!   且同列禁止第二条 status CHECK）。
//! - **调用方**：SeaORM migrator 链，注册在 `m_price_vocab_check` 之后、`m_price_fk`
//!   之前（链序「补列/回填先于 CHECK」：本迁移依赖的加列由 m0079 先行，词表守卫
//!   依赖的默认值收敛与 'ACTIVE' 回填由前驱 m_price_vocab_check 先行完成）。
//! - **入参**：无（纯约束重建，不读取应用之外的数据）。
//! - **传给谁**：价目两域 approve/reject 写入路径（services/sales_price_service.rs、
//!   purchase_price_service.rs）的 status 写入受本约束兜底；与后端权威词表
//!   `models/status/sales.rs::price_approval`（同批新增 REJECTED="rejected"）及
//!   契约锁 `backend/tests/contract_wave8_price_status_parity_test.rs` 三端同源，
//!   取值集必须与词表逐项相等（判据是集合相等而非包含，拆批必红）。
//! - **存什么**：pg_constraint 中的 CHECK 定义与列注释；不改写任何数据行。
//! - **存哪里**：PostgreSQL `public.sales_prices` / `public.purchase_prices` 表约束。
//!
//! ## 取值集（按各表写入方全集分钉，不取并集）
//! - `chk_sales_price_status`    → ('pending','approved','rejected')
//! - `chk_purchase_price_status` → ('pending','approved','rejected','inactive')
//!   `inactive` 是采购侧真实写入态（价目页 PUT 透传 status 落库，见
//!   price_vocab_check/mod.rs 头注释取证），必须保留；销售侧无 inactive 写入方，
//!   不纳入——取并集会旁路出销售价目非法落 inactive。
//!
//! ## 存量安全性论证 + fail-visible 守卫（口径照 price_vocab_check「拒绝造值」范式）
//! 扩集是**放宽**：前驱 m_price_vocab_check 已把两表收敛到旧集
//! （分别 (pending,approved) / (pending,approved,inactive)），旧集是新集真子集，
//! 全新链跑到此恒安全。但存量库可能出现约束缺失/漂移（NOT VALID、被手改）而带
//! 词表外脏值——此类行属未知业务态，禁止借扩集之机静默洗进约束。守卫先逐表统计
//! 新词表外存量并采样例值，非 0 即 RAISE EXCEPTION 点名表名/值/行数中止，
//! 交业务侧核实更正后重跑；本迁移不加约束、不做任何 UPDATE 造值。
//!
//! ## 幂等与回退
//! up 为「DROP IF EXISTS → 守卫 → ADD 同名约束」，重跑等价。
//! down 恢复 m_price_vocab_check 原窄集，并带 `rejected` 在途行的 fail-visible
//! 拒滚检查：reject 端点上线后 `rejected` 是真实终态（R-27：本批无 rejected→draft
//! 回退出边），把该类行移出允许集等于吞单据，须人工处置后重跑 down。
//!
//! ## 上线前只读摸底（存量库；与守卫同源，先摸再跑）
//! ```text
//! -- 1) 两表现行 CHECK 原文与活性（预期：按名各一条、convalidated=true）
//! SELECT c.conrelid::regclass AS tbl, c.conname, c.convalidated,
//!        pg_get_constraintdef(c.oid) AS def
//!   FROM pg_constraint c
//!  WHERE c.contype = 'c'
//!    AND c.conrelid IN ('public.sales_prices'::regclass,
//!                       'public.purchase_prices'::regclass);
//! -- 同表 status 列出现第二条 CHECK ⇒ 先人工收口，禁止带两条进本迁移。
//!
//! -- 2) 新词表外存量（守卫会按同样判据拒绝，摸出脏值先交业务定性）
//! SELECT 'sales_prices' AS tbl, COALESCE(status,'<NULL>') AS val, COUNT(*) AS n
//!   FROM sales_prices
//!  WHERE status IS NULL OR status NOT IN ('pending','approved','rejected')
//!  GROUP BY 2
//! UNION ALL
//! SELECT 'purchase_prices', COALESCE(status,'<NULL>'), COUNT(*)
//!   FROM purchase_prices
//!  WHERE status IS NULL OR status NOT IN ('pending','approved','rejected','inactive')
//!  GROUP BY 2;
//! ```

use sea_orm_migration::prelude::*;

/// 销售价目新取值集：权威词表 price_approval（+rejected）去掉 inactive（销售侧无写入方）。
const SALES_ALLOWED_STATUS_VALUES: &str = "'pending','approved','rejected'";

/// 采购价目新取值集：权威词表 price_approval 全值集（写入方含停用透传 inactive）。
const PURCHASE_ALLOWED_STATUS_VALUES: &str = "'pending','approved','rejected','inactive'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 幂等前置：按名删旧约束（约束名重建后不变）。
        let drop_old = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "chk_sales_price_status";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "chk_purchase_price_status";
"#;
        manager
            .get_connection()
            .execute_unprepared(drop_old)
            .await?;

        // 2) fail-visible 存量守卫（严禁 UPDATE 造值圆场）：逐表统计新词表外残留行
        //    并采样例值，非 0 即点名中止，本迁移不施加新约束。
        let guard = format!(
            r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "sales_prices"
      WHERE "status" IS NULL OR "status" NOT IN ({SALES_ALLOWED_STATUS_VALUES});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_vocab_extend：sales_prices.status 无新词表外存量行，可安全重建 CHECK chk_sales_price_status。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("status", '<NULL>') AS sample
                  FROM "sales_prices"
                 WHERE "status" IS NULL OR "status" NOT IN ({SALES_ALLOWED_STATUS_VALUES})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_vocab_extend：sales_prices.status 存在 % 行新词表外取值（样例值：%），拒绝施加 CHECK chk_sales_price_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/sales.rs::price_approval（pending/approved/rejected）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;

    SELECT COUNT(*) INTO bad_rows FROM "purchase_prices"
      WHERE "status" IS NULL OR "status" NOT IN ({PURCHASE_ALLOWED_STATUS_VALUES});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_vocab_extend：purchase_prices.status 无新词表外存量行，可安全重建 CHECK chk_purchase_price_status。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("status", '<NULL>') AS sample
                  FROM "purchase_prices"
                 WHERE "status" IS NULL OR "status" NOT IN ({PURCHASE_ALLOWED_STATUS_VALUES})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_vocab_extend：purchase_prices.status 存在 % 行新词表外取值（样例值：%），拒绝施加 CHECK chk_purchase_price_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/sales.rs::price_approval（pending/approved/rejected/inactive）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#
        );
        manager.get_connection().execute_unprepared(&guard).await?;

        // 3) 重建 CHECK（同名、扩集）+ 列注释固化权威词表指向与契约锁。
        let apply = format!(
            r#"
ALTER TABLE "sales_prices"    ADD CONSTRAINT "chk_sales_price_status"    CHECK ("status" IN ({SALES_ALLOWED_STATUS_VALUES}));
ALTER TABLE "purchase_prices" ADD CONSTRAINT "chk_purchase_price_status" CHECK ("status" IN ({PURCHASE_ALLOWED_STATUS_VALUES}));
COMMENT ON COLUMN "sales_prices"."status"    IS '销售价目审批态：权威词表 models/status/sales.rs::price_approval（含 rejected 终态），取值集与 CHECK chk_sales_price_status 逐项相等（契约锁 contract_wave8_price_status_parity_test）';
COMMENT ON COLUMN "purchase_prices"."status" IS '采购价目状态（含停用 inactive）：权威词表 models/status/sales.rs::price_approval（含 rejected 终态），取值集与 CHECK chk_purchase_price_status 逐项相等（同一契约锁）';
"#
        );
        manager.get_connection().execute_unprepared(&apply).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚=恢复 m_price_vocab_check 原窄集。fail-visible 拒滚：rejected 行是
        // reject 动作的真实终态（本批无 rejected→draft 出边），移出允许集会吞在途
        // 裁量结果；不做静默 UPDATE 归一，须业务侧处置（如确属误拒，按前向新行
        // 重走价目版本语义）后重跑 down。
        let guard = r#"
DO $$
DECLARE
    rejected_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO rejected_rows FROM "sales_prices" WHERE "status" = 'rejected';
    IF rejected_rows > 0 THEN
        RAISE EXCEPTION 'sales_prices.status 存在 % 行 rejected（审批拒绝终态）。回滚将把该值移出 CHECK 允许集，本迁移拒绝回滚吞掉已裁量价目行：请业务侧逐行处置后重跑 down。', rejected_rows;
    END IF;
    SELECT COUNT(*) INTO rejected_rows FROM "purchase_prices" WHERE "status" = 'rejected';
    IF rejected_rows > 0 THEN
        RAISE EXCEPTION 'purchase_prices.status 存在 % 行 rejected（审批拒绝终态）。回滚将把该值移出 CHECK 允许集，本迁移拒绝回滚吞掉已裁量价目行：请业务侧逐行处置后重跑 down。', rejected_rows;
    END IF;
END;
$$;
"#;
        manager.get_connection().execute_unprepared(guard).await?;

        let restore = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "chk_sales_price_status";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "chk_purchase_price_status";
ALTER TABLE "sales_prices"    ADD CONSTRAINT "chk_sales_price_status"    CHECK ("status" IN ('pending','approved'));
ALTER TABLE "purchase_prices" ADD CONSTRAINT "chk_purchase_price_status" CHECK ("status" IN ('pending','approved','inactive'));
COMMENT ON COLUMN "sales_prices"."status"    IS '销售价目审批态：权威词表 models/status/sales.rs::price_approval，取值集与 CHECK chk_sales_price_status 逐项相等（契约锁 contract_wave8_price_status_parity_test）';
COMMENT ON COLUMN "purchase_prices"."status" IS '采购价目状态（含停用 inactive）：权威词表 models/status/sales.rs::price_approval，取值集与 CHECK chk_purchase_price_status 逐项相等（同一契约锁）';
"#;
        manager.get_connection().execute_unprepared(restore).await?;
        Ok(())
    }
}
