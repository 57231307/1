//! purchase_inspection.inspection_result 存量取值只读点名（收口前置报告，配套 m0113b）
//!
//! 为什么单独存在一支只读迁移：purchase_inspection.inspection_result 建表原文为
//! VARCHAR(32) NULL 且全仓零 CHECK，词表防护历史上只落在应用层
//! （backend/src/models/status/purchase_inventory.rs 的 purchase_inspection_result
//! 白名单，完成质检强校验）。写入口虽已收口，旁路裸 SQL 与旧版本残留无从自证，
//! 脏值分布必须先摊开再收口，日志本身成为可追溯的点名报告。
//!
//! 口径来源，逐字符对齐，勿在本文件另立词表。权威词表取自 backend 侧
//! models::status::purchase_inventory::purchase_inspection_result，合法落库 token 为
//! pass / fail / partial；完成质检前该列为 NULL，NULL 是合法初值、空串不是合法取值。
//! 与通用质检记录域 quality_inspection_records.inspection_result 的中文词表分属两张表
//! 两套词表，本支只钉 purchase_inspection 这一列，禁止跨域借用。
//!
//! 严格只读：up 仅 SELECT 聚合与 RAISE NOTICE，无 UPDATE、无 DELETE、无 DDL、不 RAISE
//! EXCEPTION，本步目的就是把现状摊给人看，抛错会中断迁移链与点名相悖。因此可任意重跑，
//! 无回滚面，down 是显式 no-op。
//!
//! 词表值命中计数用 unnest 与 LEFT JOIN，词表内每个 token 都出一条 NOTICE，命中 0 行也报，
//! 报 0 本身就是该值在此库不存在的证据。词表外与空串另起一段逐值点名，NULL 单列计数
//! （合法初值不列为越界），为 m0113b 的守卫拒绝留现场。
//!
//! 域与注册位置：本域排在 lib.rs 迁移链尾，晚于建 purchase_inspection 的域。点名支必须
//! 先于收口支 m0113b，注册在同域 up 链首位。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 与 models::status::purchase_inventory::purchase_inspection_result::ALL 逐字符一致，
/// 守卫、CHECK、点名三处共用同一份（m0113a 与 m0113b 各携带一份，纸面锁钉两支相等）。
const PURCHASE_INSPECTION_RESULT_ALLOWED: &str = "'pass','fail','partial'";

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
    bad_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "purchase_inspection";
    RAISE NOTICE 'm0113a 点名 purchase_inspection.inspection_result 全表 % 行，词表清单 pass,fail,partial，以下逐值分账', total_rows;

    FOR rec IN
        SELECT vocab.val AS val, COUNT(matched."inspection_result") AS cnt
          FROM (SELECT unnest(ARRAY[{allowed}]) AS val) vocab
          LEFT JOIN "purchase_inspection" matched ON matched."inspection_result" = vocab.val
         GROUP BY vocab.val
         ORDER BY vocab.val
    LOOP
        RAISE NOTICE 'm0113a 点名 purchase_inspection.inspection_result 词表值 % 命中 % 行', rec.val, rec.cnt;
    END LOOP;

    SELECT COUNT(*) INTO null_rows FROM "purchase_inspection" WHERE "inspection_result" IS NULL;
    RAISE NOTICE 'm0113a 点名 purchase_inspection.inspection_result NULL（合法初值，完成质检前置态） % 行', null_rows;

    SELECT COUNT(*) INTO bad_rows FROM "purchase_inspection"
      WHERE "inspection_result" IS NOT NULL AND "inspection_result" NOT IN ({allowed});
    RAISE NOTICE 'm0113a 点名 purchase_inspection.inspection_result 词表外（含空串）合计 % 行', bad_rows;

    FOR rec IN
        SELECT '''' || "inspection_result" || '''' AS badval, COUNT(*) AS cnt
          FROM "purchase_inspection"
         WHERE "inspection_result" IS NOT NULL AND "inspection_result" NOT IN ({allowed})
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0113a 点名 purchase_inspection.inspection_result 词表外值 % 行数 %，本值无定案映射，m0113b 将拒绝施加 CHECK 交人工定性', rec.badval, rec.cnt;
    END LOOP;
END
$$;
DO $$ BEGIN RAISE NOTICE 'm0113a 点名完成，现状已全部写入迁移日志，本迁移未做任何写入'; END $$;
"#,
            allowed = PURCHASE_INSPECTION_RESULT_ALLOWED,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 显式 no-op 且这是正确形态非偷懒：up 是纯只读迁移，只有 SELECT 聚合与
        // RAISE NOTICE，不含任何 UPDATE、DELETE、DDL，对库内数据与结构零改变，
        // 因此不存在任何需要回滚的副作用。迁移日志里的逐值快照是历史事实，
        // 由台账与日志系统本身保留，down 无事可做也不应抹除任何痕迹。
        Ok(())
    }
}
