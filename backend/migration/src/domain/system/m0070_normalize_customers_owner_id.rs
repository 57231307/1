//! customers.owner_id 真实列形态归一（run #4671 W1 · "ADD COLUMN IF NOT EXISTS
//! 被当改列约束用"族，判责原文 ci4671-triage.md §2.3 A1 簇 / §⑤ W1）
//!
//! 根因（全部 file:line 实测）：
//! - `domain/system/mod.rs:229` = `ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS
//!   "owner_id" INTEGER;`（可空、无默认）—— 这是该列的**生效定义**（system 域先执行）；
//! - `domain/finance/mod.rs:109` = `... ADD COLUMN IF NOT EXISTS "owner_id" INTEGER
//!   NOT NULL DEFAULT 0`，因列已存在被 `IF NOT EXISTS` 吃成**恒 no-op**，其 NOT NULL
//!   DEFAULT 0 从未生效（域执行序 `migration/src/lib.rs:36-40` system→…→finance）；
//! - 模型 `src/models/customer.rs:97` 是非 Option `owner_id: i32` ⇒ 写入路径不设值即
//!   落 NULL ⇒ 回读报 `Missing value for column 'owner_id'`（#4671 p4/p5/p6 三条红，
//!   `contract_wave2_after_sales_create_test.rs:188` 等）。
//!
//! 比测试红更重的后果 B（静默数据可见性缺陷）：`domain/finance/mod.rs:116-131` 的
//! RLS policy `customers_isolation` 判据 `owner_id = current_setting(...)::int
//! OR owner_id = 0` 对 NULL 行两侧均判 unknown ⇒ 公海/历史客户行对所有人不可见。
//! 本迁移必须落在 system 域 up 链尾（finance 域建 policy 之前），policy 生效后列即
//! NOT NULL DEFAULT 0，NULL 盲区闭合。
//!
//! 修法纪律（本仓红线）：
//! - **只新建迁移，绝不修改已应用迁移**：`system/mod.rs:229` 的裸 `INTEGER` 一行保持
//!   原样不动（历史迁移被 `_seaorm_` 记账，改写文本对已部署库永不重放，只会制造
//!   历史漂移）；新迁移在域 up 链尾（晚于该 inline 补列块）做真实归一。
//! - 回填前先做 fail-visible 存量探测：`owner_id IS NULL AND owner_assigned_at
//!   IS NOT NULL` 的行是"已指派却丢失归属人"，自动归 0 会把受保护客户静默降回
//!   公海、暴露给任意领取 ⇒ 拒绝归一，RAISE EXCEPTION 点名 id 样例，人工按
//!   crm_assignment 记录/审计轨迹回填后重跑。绝不猜归属、绝不 COALESCE 洗数据。
//! - 双列皆空（owner_id 与 owner_assigned_at 均 NULL）= 未分配，归 0 是列的
//!   **成文语义**（`finance/mod.rs:110` COMMENT 与 `customer.rs:95-97` 文档注释
//!   "0 表示未分配/公海客户"），不是发明默认值。
//!
//! 幂等性：探测条件在 NOT NULL 生效后恒空、UPDATE 无匹配行、SET DEFAULT/SET NOT
//! NULL 重放等价、COMMENT 覆盖同文 ⇒ up 可重跑。
//!
//! down 真实可逆（本仓纪律，教训见 rls_dept 空 down 登记）：恢复"可空、无默认"的
//! 修复前形态。数据无损性论证：up 只把 NULL 归 0（0 在旧形态同样可表达），down 后
//! 不存在被吞掉的取值形态，无需拒滚探测；但回滚即重新打开 RLS NULL 盲区与
//! "Missing value for column" 写入缺陷（0 行在 down 后新写入会退回 NULL），故 down
//! 仅用于"整链回退演练"；若库中已依赖 NOT NULL 约束保数据完整性，回滚前须人工
//! 处置规则：`SELECT id FROM customers WHERE owner_id IS NULL` 应为 0 行（恒成立），
//! 人工自行评估是否保留 policy 或补回本迁移。
//!
//! 契约锁：`backend/tests/contract_wave7_column_shape_lock_test.rs` 以
//! information_schema 活库断言本列（is_nullable=NO / column_default='0'）与同族
//! 数组列形态，永久钉死"迁移文本写了 NOT NULL 但实际列可空"这类静默失效。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) fail-visible 存量探测（照 m0063/m0067/m0068 先例，只读，命中即中止）：
--    owner_assigned_at 已填而 owner_id 为 NULL ⇒ "已指派却丢失归属人"，
--    不允许自动归 0（静默降公海 = 把受保护客户暴露给任意领取），人工回填后重跑。
DO $$
DECLARE
    bad_row_count INTEGER;
    bad_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_row_count
      FROM "customers"
     WHERE "owner_id" IS NULL AND "owner_assigned_at" IS NOT NULL;
    IF bad_row_count > 0 THEN
        SELECT string_agg("id"::text, ', ' ORDER BY "id")
          INTO bad_samples
          FROM (
            SELECT "id" FROM "customers"
             WHERE "owner_id" IS NULL AND "owner_assigned_at" IS NOT NULL
             ORDER BY "id" LIMIT 10
          ) s;
        RAISE EXCEPTION 'customers.owner_id 存在 % 行 owner_id 为 NULL 但 owner_assigned_at 已填（样例客户 id：%）。此类行是"已指派却丢失归属人"，本迁移拒绝自动归 0（静默降公海会把受保护客户暴露给任意领取），且不做任何猜测性 UPDATE 洗数据：请人工依据 crm_assignment 指派记录/审计轨迹恢复 owner_id 后重跑本迁移。',
            bad_row_count, bad_samples;
    END IF;
END
$$;

-- 2) 存量回填：owner_id 与 owner_assigned_at 皆空 = 未分配，按列成文语义归 0（公海）。
--    （步骤 1 已排除"指派信息残缺"的异常行，此处只触及语义明确为公海的行。）
UPDATE "customers" SET "owner_id" = 0 WHERE "owner_id" IS NULL;

-- 3) 列约束归一：落实 finance/mod.rs:109 被 IF NOT EXISTS 吞掉的原始意图
--    （NOT NULL DEFAULT 0），使 RLS policy 的 owner_id=0 公海判据不再对 NULL 判 unknown。
ALTER TABLE "customers" ALTER COLUMN "owner_id" SET DEFAULT 0;
ALTER TABLE "customers" ALTER COLUMN "owner_id" SET NOT NULL;
COMMENT ON COLUMN "customers"."owner_id" IS '客户归属人 ID（0=公海客户，对所有用户可见）';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 恢复修复前形态（可空、无默认）。up 只把 NULL 归 0、未收窄任何可表达取值，
        // 故回滚不吞数据、无需拒滚探测（论证见文件头）；回滚会重新打开 RLS NULL
        // 盲区，仅用于整链回退演练。
        let sql = r#"
ALTER TABLE "customers" ALTER COLUMN "owner_id" DROP NOT NULL;
ALTER TABLE "customers" ALTER COLUMN "owner_id" DROP DEFAULT;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
