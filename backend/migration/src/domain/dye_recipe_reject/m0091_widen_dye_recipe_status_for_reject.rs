//! 染色配方 status CHECK 扩 `rejected` 态 + 补拒绝理由专列（v15 域尾四值 CHECK 的后继放宽迁移）
//!
//! ## 六要素
//! - **功能**：把 `dye_recipe.status` 的 CHECK `chk_dye_recipe_status` 取值集由 v15 域尾
//!   建立的四值（draft/pending_approval/approved/disabled）扩为五值（+ `rejected`），
//!   约束名保持不变（同列禁止第二条 status CHECK，且守卫测试按名解析取值集）；
//!   并为 `dye_recipe` 补可空列 `rejected_reason` TEXT（与 sales_orders.rejected_reason
//!   同形：TEXT、可空、无默认，拒绝理由原文由服务端 trim 非空强制后落此列）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/dye_recipe_reject/` 独立域、
//!   lib.rs 全链尾（v15 域尾建立四值 CHECK 后，任何更早位置都会被 v15 重建覆盖回窄集）。
//! - **入参**：无（纯 DDL 迁移，不读取应用数据）。
//! - **传给谁**：后端写入方 `services/dye_recipe_service.rs` 的 reject 路径
//!   （仅 pending_approval 可拒，写 status='rejected' + rejected_reason）；取值集与
//!   权威词表 `models/status/quality_dyeing.rs::dye_recipe::ALL`（同批五值）逐项一致，
//!   守卫锁 `backend/tests/dye_recipe_status_word_list_test.rs` 解析本迁移 up 的
//!   CHECK 取值集与词表双向比对，判据是集合逐项相等而非包含。
//! - **存什么**：pg_constraint 中的 CHECK 定义、information_schema 新列与列注释；
//!   不改写任何数据行。
//! - **存哪里**：PostgreSQL `public.dye_recipe` 表约束与列。
//!
//! ## 幂等与守卫
//! up 为「fail-visible 存量检测 → ADD COLUMN IF NOT EXISTS → DROP CONSTRAINT IF EXISTS
//! → ADD CONSTRAINT（同名五值）→ COMMENT ON COLUMN」。PG 的 ADD CONSTRAINT 无
//! IF NOT EXISTS，故先按名 DROP 再 ADD，重跑等价；存量检测把词表外（NULL 除外——
//! 该列可空且 CHECK 对 NULL 恒成立）取值点名中止，拒绝在带脏值的列上静默重建约束
//! （照 m0064 custom_order 扩集先例，不做 UPDATE 造值归一）。
//! 扩集是放宽：v15 旧四值集是新五值集的真子集，全新链与存量库升级到此均不违反约束，
//! 只有历史漂移行会被守卫抓到。
//! down 反向：先做 rejected 在途行的 fail-visible 拒滚检查（拒绝是真实业务终态，
//! 把该行移出允许集等于吞裁量结果，须人工处置后重跑），再恢复 v15 四值集并
//! DROP COLUMN IF EXISTS rejected_reason。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) fail-visible 存量检测：dye_recipe.status 出现新词表外的非 NULL 取值即中止，
--    拒绝静默洗数据（样例最多 10 个，消息含违规行数与样例值，供人工定位处置）。
DO $$
DECLARE
    bad_row_count INTEGER;
    bad_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_row_count
      FROM "dye_recipe"
     WHERE "status" IS NOT NULL
       AND "status" NOT IN ('draft', 'pending_approval', 'approved', 'rejected', 'disabled');
    IF bad_row_count > 0 THEN
        SELECT string_agg("status", ', ' ORDER BY "status")
          INTO bad_samples
          FROM (
            SELECT DISTINCT "status"
              FROM "dye_recipe"
             WHERE "status" IS NOT NULL
               AND "status" NOT IN ('draft', 'pending_approval', 'approved', 'rejected', 'disabled')
             LIMIT 10
          ) s;
        RAISE EXCEPTION 'dye_recipe.status 存在 % 行取值位于新词表之外（样例：%）。本迁移拒绝在带词表外存量值的列上重建 CHECK，且不做静默归一（禁止 UPDATE 洗数据）：请人工核实上述行的真实业务状态并处置后重跑。',
            bad_row_count, bad_samples;
    END IF;
END
$$;

-- 2) 拒绝理由专列（可空、无默认：历史行未采集理由，NOT NULL 会在加列瞬间逼出造值）
ALTER TABLE "dye_recipe" ADD COLUMN IF NOT EXISTS "rejected_reason" TEXT;
COMMENT ON COLUMN "dye_recipe"."rejected_reason" IS '审批拒绝理由（必填，服务端 trim 非空强制）。写入方：services/dye_recipe_service.rs reject 路径；与审核通过语义分列，拒绝不复用 approved_by/approved_at 存拒绝人';

-- 3) 重建 CHECK（同名、扩集）+ 列注释固化权威词表指向与逐值清单
ALTER TABLE "dye_recipe" DROP CONSTRAINT IF EXISTS "chk_dye_recipe_status";
ALTER TABLE "dye_recipe" ADD CONSTRAINT "chk_dye_recipe_status" CHECK ("status" IN ('draft', 'pending_approval', 'approved', 'rejected', 'disabled'));
COMMENT ON COLUMN "dye_recipe"."status" IS '状态（全小写，取值见 models/status/quality_dyeing.rs::dye_recipe，与 CHECK chk_dye_recipe_status 逐项一致）：draft(草稿) / pending_approval(待审核) / approved(已审核) / rejected(已拒绝) / disabled(已停用)；中文仅前端 i18n 展示层';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚=恢复 v15 域尾四值集 + 删除 rejected_reason 列。
        // fail-visible 拒滚：存在 rejected 行时 ADD CONSTRAINT 会真库校验失败并冒
        // DbErr，先显式检测并给出可操作的报错文案，避免裸约束错误难以定位。
        let sql = r#"
DO $$
DECLARE
    rejected_row_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO rejected_row_count
      FROM "dye_recipe"
     WHERE "status" = 'rejected';
    IF rejected_row_count > 0 THEN
        RAISE EXCEPTION 'dye_recipe.status 存在 % 行 rejected（审批拒绝终态）。回滚将把该值移出 CHECK 允许集，本迁移拒绝回滚吞掉已裁量配方行：请先人工处置上述行（如按业务重走新版本流程）后重跑 down。',
            rejected_row_count;
    END IF;
END
$$;

ALTER TABLE "dye_recipe" DROP CONSTRAINT IF EXISTS "chk_dye_recipe_status";
ALTER TABLE "dye_recipe" ADD CONSTRAINT "chk_dye_recipe_status" CHECK ("status" IN ('draft', 'pending_approval', 'approved', 'disabled'));
COMMENT ON COLUMN "dye_recipe"."status" IS '状态（全小写，取值见 models/status/quality_dyeing.rs::dye_recipe，与 CHECK chk_dye_recipe_status 逐项一致）：draft(草稿) / pending_approval(待审核) / approved(已审核) / disabled(已停用)；中文仅前端 i18n 展示层';
ALTER TABLE "dye_recipe" DROP COLUMN IF EXISTS "rejected_reason";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
