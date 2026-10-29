//! 产品每卷米数列探测式下线（m0109）
//!
//! 裁定口径（产品档案规格与每匹米数收口工单）：每卷米数只是每匹米数的照抄假字段，
//! 一卷即一匹是现网唯一口径；按卷换算腿已改由每匹米数单源供给，代码面已不再读写本列，
//! 本迁移负责把列形态收掉。迁移绝不 UPDATE、绝不 DELETE 业务数据。
//!
//! 先探测后约束的失败可见形态。DROP 前先探测：若库里存在 meters_per_roll 非空且与
//! meters_per_piece 不同值的行，说明该列携带过独立信息（与「表单恒照抄」的前提冲突），
//! 立即 RAISE EXCEPTION 点名行数与样例 id 并中止，绝不静默 DROP 有信息量的数据；
//! 这类行须由人工把正确值并入每匹米数后重跑本迁移。探测通过（不等的行数为 0，
//! 含两列全空或全同值的所有形态）才执行 DROP。
//!
//! 幂等。列不存在（已下线，或历史库从未跑过 m0059 加列）时打印 NOTICE 直接跳过，
//! 可任意重跑；down 加回可空列（IF NOT EXISTS），只恢复形态不伪造数据。
//!
//! 依赖。目标表 products 由 business 域 m0008 建表、本列由 production 域 m0059 添加，
//! 均早于本域；本域排 lib.rs 迁移链尾，晚于代码面下线无需迁移感知（列可空且无依赖对象，
//! DROP 不牵连索引与约束；CHECK 与默认值从未施加到本列）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"DO $m0109$
DECLARE
    v_col_exists boolean;
    v_dirty_count bigint;
    v_sample_ids text;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'products' AND column_name = 'meters_per_roll'
    ) INTO v_col_exists;

    IF NOT v_col_exists THEN
        RAISE NOTICE 'm0109 跳过：products.meters_per_roll 列不存在（已下线或该库未跑过加列迁移）';
        RETURN;
    END IF;

    SELECT COUNT(*) INTO v_dirty_count
    FROM products
    WHERE meters_per_roll IS NOT NULL
      AND meters_per_piece IS DISTINCT FROM meters_per_roll;

    IF v_dirty_count > 0 THEN
        SELECT string_agg(id::text, ', ') INTO v_sample_ids
        FROM (
            SELECT id FROM products
            WHERE meters_per_roll IS NOT NULL
              AND meters_per_piece IS DISTINCT FROM meters_per_roll
            ORDER BY id
            LIMIT 20
        ) dirty_rows;
        RAISE EXCEPTION
            'm0109 拒绝下线 meters_per_roll：探测到 % 行每卷米数非空且与每匹米数不等，列携带过独立信息（样例 id：%）。本迁移不 UPDATE 业务数据，请先人工把正确值并入每匹米数后重跑', v_dirty_count, v_sample_ids;
    END IF;

    RAISE NOTICE 'm0109 探测通过（每卷米数与每匹米数不等的非空行为 0），下线 products.meters_per_roll';
    EXECUTE 'ALTER TABLE "products" DROP COLUMN "meters_per_roll"';
END
$m0109$;"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "meters_per_roll" DECIMAL(12,4);
COMMENT ON COLUMN "products"."meters_per_roll" IS '每卷米数（已裁定废弃）：一卷即一匹，按卷换算单源用 meters_per_piece；本列仅回退形态恢复空壳，不回填数据';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
