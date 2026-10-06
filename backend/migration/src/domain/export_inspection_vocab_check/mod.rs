//! 出口商检结论词表 DB CHECK 约束迁移
//!
//! 将本域权威词表 `models/status/export_inspection.rs::export_inspection_result`
//! 钉入数据库，阻止任何绕过服务层入参校验的写入产生词表外脏值（CHECK 与入参校验同一字典，
//! 避免两处维护漂移）：
//! - `export_inspection.result` VARCHAR(255) NOT NULL：('pending','pass','fail')
//!   建单动作恒落 pending（service create），登记结果动作写 pass/fail（service update_result
//!   依词表强校验），全仓仅这两个写入方，取值集与词表 `ALL` 逐元素相等。
//!
//! 约束名 `chk_export_inspection_result` 与词表模块头注释、契约测试成文约定一致。
//! 词表变更必须同时动 `models/status/export_inspection.rs` 与本约束，取值集逐项相等由契约测试
//! `backend/tests/contract_wave11_export_inspection_result_check_test.rs` 锁定；
//! 两侧任一侧单独增删值都判红（词表加一值而 CHECK 未加 ⇒ 写入撞 23514 冒裸 500；
//! CHECK 比词表宽 ⇒ 旁路脚本可写出业务永不产生的脏 token）。
//!
//! 存量守卫（fail-visible，严禁 UPDATE 造值洗数据）：本表结论列历史上无任何写入口、
//! 预期为空，但仍在加约束前统计词表外残留行数——非 0 即点名样例值并中止，不静默加约束后
//! 把脏行留在库里（若线上确有空表假设之外的存量，交业务定性后重跑，不在迁移里圆场）。
//!
//! 列 NOT NULL（建表 v15/mod.rs 原文），CHECK 直接写 "result" IN (...) 不留 NULL 放行；
//! result 无历史词表外默认值（建表无 DEFAULT），故不需回填/默认值收敛。
//!
//! 幂等：先 DROP CONSTRAINT IF EXISTS 再 ADD，可安全重跑。注册在迁移链尾、晚于 v15 域
//! （本表建表在 v15），全部建表类迁移先行完成后才施加 CHECK，全新库与存量库均不违反 CHECK。
//!
//! 域与注册位置：export_inspection 由 v15 域建表；lib.rs 域序 …→v15→rls_dept→…→本域，
//! 本域晚于 v15 执行，目标表届时必然存在（口径同 price_vocab_check 后置建表域）。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_export_inspection_vocab_check"
    }
}

/// 结论词表的 SQL 字面量清单（与 models/status/export_inspection.rs::ALL 同源逐字符一致；
/// 守卫、CHECK、列注释三处共用此一份，禁止另写一套）。
const ALLOWED_SQL: &str = "'pending','pass','fail'";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 幂等前置：删旧约束（重跑/半态库不报错）。
        let drop_old = r#"ALTER TABLE "export_inspection" DROP CONSTRAINT IF EXISTS "chk_export_inspection_result";"#;
        manager
            .get_connection()
            .execute_unprepared(drop_old)
            .await?;

        // 2) 存量守卫（fail-visible）：统计词表外残留行数并采样例值；非 0 即点名中止。
        let guard = format!(
            r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "export_inspection"
      WHERE "result" IS NULL OR "result" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_export_inspection_vocab_check：export_inspection.result 无词表外存量行，可安全施加 CHECK chk_export_inspection_result。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("result", '<NULL>') AS sample
                  FROM "export_inspection"
                 WHERE "result" IS NULL OR "result" NOT IN ({allowed})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_export_inspection_vocab_check：export_inspection.result 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_export_inspection_result。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/export_inspection.rs::export_inspection_result（pending/pass/fail）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#,
            allowed = ALLOWED_SQL,
        );
        manager.get_connection().execute_unprepared(&guard).await?;

        // 3) 施加 CHECK 并以列注释固化权威词表指向（列 NOT NULL，不设 NULL 放行）。
        let apply = format!(
            r#"
ALTER TABLE "export_inspection" ADD CONSTRAINT "chk_export_inspection_result" CHECK ("result" IN ({allowed}));
COMMENT ON COLUMN "export_inspection"."result" IS '出口商检结论：权威词表 models/status/export_inspection.rs::export_inspection_result，取值集与 CHECK chk_export_inspection_result 逐项相等（契约锁 contract_wave11_export_inspection_result_check_test）';
"#,
            allowed = ALLOWED_SQL,
        );
        manager.get_connection().execute_unprepared(&apply).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除约束，不回退数据：删约束即回到可写状态（口径同 price_vocab_check 的 down，
        // 不复活旧越界值，也不伪造历史行）。
        let sql = r#"ALTER TABLE "export_inspection" DROP CONSTRAINT IF EXISTS "chk_export_inspection_result";"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
