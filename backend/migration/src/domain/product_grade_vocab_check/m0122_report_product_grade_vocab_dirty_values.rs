//! 产品等级列收口前存量取值只读点名（配套 m0123）
//!
//! 为什么单独存在一支只读迁移。products.product_grade 建列于历史批次且无取值收口，
//! 表单侧此前是自由文本录入，库里可能存在词表外残留（含历史空串）。m0123 施加 CHECK 前
//! 会逐值守卫拒绝越界残留，但那只是结果不是现场。本迁移把该列收口前的现状以 RAISE NOTICE
//! 摊进迁移日志，使 CI 与人工能核对 m0123 的取值域口径是否覆盖真实分布，日志本身成为
//! 可追溯的点名报告。
//!
//! 口径来源，逐字符对齐，勿在本文件另立词表。三档词表取自 backend 侧
//! models::status::general::product_grade::ALL_GRADES（优等品 一等品 合格品，
//! GB/T 分等定级三档）。NULL 语义为未录入即合法，本批不加 NOT NULL、不加 DEFAULT。
//! 本清单与本域 m0123 施加的 CHECK 表达式逐字相等，两支各携带同一份，由纸面锁钉彼此
//! 相等，禁止任一侧单独改动。
//!
//! 列宽漂移一并摊出。同名列在两处历史加列语句里分别写 VARCHAR(255) 与 VARCHAR(10)，
//! 均为 IF NOT EXISTS，先跑者胜、后跑者静默空转，实际宽度取决于库的建列时机。本迁移回读
//! information_schema 打印当前列的真实宽度与基数分布，为宽度是否收一提供现场，只点名不改型。
//!
//! 严格只读。up 仅 SELECT 聚合与 RAISE NOTICE，无 UPDATE、无 DELETE、无 DDL、不 RAISE
//! EXCEPTION，本步目的就是把现状摊给人看，抛错会中断迁移链与点名相悖。因此可任意重跑，
//! 无回滚面，down 是显式 no-op。
//!
//! 域与注册位置。本域排 lib.rs 迁移链尾，晚于 system 域与 v15 域（两域各有一条对该列的
//! 历史加列语句），products 表与该列届时必然存在。点名支必须先于收口支 m0123，
//! 注册在同域 up 链首位。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 产品等级三档 SQL 字面量清单，与 models::status::general::product_grade::ALL_GRADES
/// 逐项逐字符一致。守卫、点名与本域 m0123 的 CHECK 共用同一份形态，禁止另写第二套。
const PRODUCT_GRADE_ALLOWED: &str = "'优等品','一等品','合格品'";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let legal =
            format!("\"product_grade\" IS NULL OR \"product_grade\" IN ({PRODUCT_GRADE_ALLOWED})");
        let sql = format!(
            r#"
DO $$
DECLARE
    rec RECORD;
    total_rows INTEGER;
    bad_rows INTEGER;
    col_width TEXT;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "products";
    RAISE NOTICE 'm0122 点名 products.product_grade 全表 % 行，收口口径 空值或 优等品 一等品 合格品', total_rows;

    SELECT column_name || ' ' || data_type || '(' || character_maximum_length || ')'
      INTO col_width FROM information_schema.columns
     WHERE table_name = 'products' AND column_name = 'product_grade';
    RAISE NOTICE 'm0122 点名 products.product_grade 当前列宽 %（两处历史加列语句宽度不同，先跑者胜，本迁移不改型只摊出）', col_width;

    SELECT COUNT(*) INTO bad_rows FROM "products"
      WHERE NOT ({legal});
    RAISE NOTICE 'm0122 点名 products.product_grade 越界行合计 % 行，空值按未录入计为合法', bad_rows;

    -- 越界行按谓词定义必非空（空值已判合法并被 WHERE 排除），故逐值点名不做空值兜底
    FOR rec IN
        SELECT "product_grade"::text AS badval, COUNT(*) AS cnt
          FROM "products"
         WHERE NOT ({legal})
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0122 点名 products.product_grade 越界值 % 行数 %，本值无定案映射，m0123 将拒绝施加 CHECK 交人工定性', rec.badval, rec.cnt;
    END LOOP;

    FOR rec IN
        SELECT "product_grade"::text AS val, COUNT(*) AS cnt
          FROM "products"
         WHERE "product_grade" IS NOT NULL
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0122 点名 products.product_grade 已录入值 % 行数 %', rec.val, rec.cnt;
    END LOOP;
END
$$;
"#,
            legal = legal,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 显式 no-op 且这是正确形态非偷懒。up 是纯只读迁移，只有 SELECT 聚合与
        // RAISE NOTICE，不含任何 UPDATE、DELETE、DDL，对库内数据与结构零改变，
        // 因此不存在任何需要回滚的副作用。迁移日志里的逐值快照是历史事实，
        // 由台账与日志系统本身保留，down 无事可做也不应抹除任何痕迹。
        Ok(())
    }
}
