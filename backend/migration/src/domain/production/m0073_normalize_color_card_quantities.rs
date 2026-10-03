//! `color_cards.stock_quantity` / `issued_quantity` 收紧为 NOT NULL DEFAULT 0
//! （run #4671 W1 移交的 SHADOWED 族 · 色卡种子红的迁移半根因）
//!
//! 缺陷事实（三段证据，非推断）：
//! - 列由 production 域先建，形态是**裸可空无默认**：`migration/src/domain/production/mod.rs:432`
//!   （`ADD COLUMN IF NOT EXISTS "issued_quantity" INTEGER`）、`:434`（`"stock_quantity" INTEGER`）；
//! - finance 域随后写的权威形态 `ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS
//!   "stock_quantity" INTEGER NOT NULL DEFAULT 0`（`migration/src/domain/finance/mod.rs:296-298`）
//!   因列已存在（production 域先执行）而被 `IF NOT EXISTS` **整句吞成恒 no-op**，
//!   NOT NULL 与 DEFAULT 从未生效——这正是 W1 穷举器 `C:/Users/57231/scan_out.txt` SHADOWED 段
//!   第 244 行点名的形态，也是 #4671 §2.4-C「色卡种子缺列/缺值」红的**迁移侧根因**
//!   （只补夹具不修这里，下一轮仍红：见判责 §⑤ W4 与 W1 报告 §③）；
//! - 模型与出参却都按非空整数声明：`src/models/color_card.rs:22,24`（`i32`）、
//!   `src/models/color_card_response_dto.rs:32,52`（`i32`）⇒ 库里任何 NULL 行在读取时即
//!   `ColumnNull` 解码失败（列表/详情/发放校验整链 500），发放门的
//!   `issued_qty <= stock_quantity` 比较也失去依据。
//!
//! 为什么方向是"收紧列"而不是"模型改 Option"：
//! - 两列的成文口径就是带默认值的计数列（finance DDL 明文 `NOT NULL DEFAULT 0` +
//!   列注释「V15 P0-F10 库存联动」）；模型侧 `i32` 与该口径一致，改模型 = 反向迁就漂移；
//! - 同库同名语义在 m0070（`customers.owner_id` NOT NULL DEFAULT 0）已确立"计数/归属列
//!   用非空 + 默认"的先例，两列再留可空就是第三套口径。
//!
//! 存量数据策略（fail-visible，照 m0070/m0067 先例）：
//! - up 先只读统计两列各自的 NULL 行数并 RAISE NOTICE 点名（含样例 id），绝不静默回填；
//! - 回填取值 0 不是猜测：**0 = 无库存/未发放**，是默认值本身（不会凭空造出库存或发放量），
//!   与列注释和应用层默认完全一致；不做任何"按发放记录聚合推算 stock_quantity"的推断性洗数据
//!   （那属于业务补录，须另立项，见 PR 正文"委外收回匹实测值补录"同族待决项）。
//! - 表/列不存在视为结构漂移 → fail-visible 中止，不"跳过即通过"。
//!
//! 幂等：NOT NULL/DEFAULT 生效后 NULL 计数恒 0、两条 UPDATE 无匹配行，重跑等价。
//!
//! down 真实可逆（禁止空实现）：撤掉 NOT NULL 与 DEFAULT，回到"可空无默认"的修复前形态。
//! 与 m0070 同一论证：up 只把 NULL 归成该列的默认值 0，未收窄任何可表达取值，
//! 因此回滚不吞业务数据；唯一不可复原的是"哪些行原本是 NULL"这一信息维度（本迁移不假装能还原它）。
//!
//! 活体锁：`backend/tests/contract_wave7_column_shape_lock_test.rs` 的 EXPECTED 增加两列，
//! 按 information_schema 断言 data_type/udt_name/is_nullable/column_default 逐字相符，
//! 终结"迁移文本写了 NOT NULL 但被 IF NOT EXISTS 吃掉"这类静默失效。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) 结构存在性 fail-visible：表或列缺失说明迁移链形态与本迁移前提不符，拒绝静默跳过
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables
                    WHERE table_name = 'color_cards') THEN
        RAISE EXCEPTION 'm0073：表 color_cards 不存在，迁移前提（production 域已建表建列）被破坏，中止。';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'color_cards' AND column_name = 'stock_quantity') THEN
        RAISE EXCEPTION 'm0073：列 color_cards.stock_quantity 不存在（生效 DDL 应在 production/mod.rs:434），中止。';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'color_cards' AND column_name = 'issued_quantity') THEN
        RAISE EXCEPTION 'm0073：列 color_cards.issued_quantity 不存在（生效 DDL 应在 production/mod.rs:432），中止。';
    END IF;
END
$$;

-- 2) 只读统计并点名 NULL 行（回填前可见化，禁止静默归一）
DO $$
DECLARE
    null_stock  INTEGER;
    null_issued INTEGER;
    sample_ids  TEXT;
BEGIN
    SELECT COUNT(*) INTO null_stock  FROM "color_cards" WHERE "stock_quantity"  IS NULL;
    SELECT COUNT(*) INTO null_issued FROM "color_cards" WHERE "issued_quantity" IS NULL;
    SELECT string_agg(id::text, ', ' ORDER BY id)
      INTO sample_ids
      FROM (SELECT "id" FROM "color_cards"
             WHERE "stock_quantity" IS NULL OR "issued_quantity" IS NULL
             LIMIT 10) s;
    RAISE NOTICE 'm0073：color_cards 待回填行数 stock_quantity NULL=% / issued_quantity NULL=%；受影响样例 id：%。回填取值 0 = 无库存/未发放（即该列成文默认值，见 finance/mod.rs:297-298），不凭空造量；需按实盘补录库存的业务另立项。',
        null_stock, null_issued, COALESCE(sample_ids, '无');
END
$$;

-- 3) 回填后收紧（先 UPDATE 再 SET NOT NULL，否则 ALTER 直接失败）
UPDATE "color_cards" SET "stock_quantity" = 0 WHERE "stock_quantity" IS NULL;
UPDATE "color_cards" SET "issued_quantity" = 0 WHERE "issued_quantity" IS NULL;

ALTER TABLE "color_cards" ALTER COLUMN "stock_quantity"  SET DEFAULT 0;
ALTER TABLE "color_cards" ALTER COLUMN "stock_quantity"  SET NOT NULL;
ALTER TABLE "color_cards" ALTER COLUMN "issued_quantity" SET DEFAULT 0;
ALTER TABLE "color_cards" ALTER COLUMN "issued_quantity" SET NOT NULL;

-- 4) 复核收紧结果（不信任"执行过=生效过"，再读一次元数据自证）
DO $$
DECLARE
    still_null INTEGER;
    nn_stock   TEXT;
    nn_issued  TEXT;
BEGIN
    SELECT COUNT(*) INTO still_null FROM "color_cards"
     WHERE "stock_quantity" IS NULL OR "issued_quantity" IS NULL;
    IF still_null > 0 THEN
        RAISE EXCEPTION 'm0073：收紧后仍有 % 行数量为 NULL，NOT NULL 未真实生效，中止。', still_null;
    END IF;
    SELECT is_nullable INTO nn_stock  FROM information_schema.columns
     WHERE table_name = 'color_cards' AND column_name = 'stock_quantity';
    SELECT is_nullable INTO nn_issued FROM information_schema.columns
     WHERE table_name = 'color_cards' AND column_name = 'issued_quantity';
    IF nn_stock <> 'NO' OR nn_issued <> 'NO' THEN
        RAISE EXCEPTION 'm0073：information_schema 复核失败（stock_quantity is_nullable=%，issued_quantity is_nullable=%，期望均为 NO）', nn_stock, nn_issued;
    END IF;
    RAISE NOTICE 'm0073：color_cards 两列已收紧为 NOT NULL DEFAULT 0，与模型 i32 口径同源。';
END
$$;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 回到修复前形态：可空、无默认（撤约束在前、丢默认在后均可，这里两条列各两步）。
-- up 未收窄任何可表达取值（NULL→该列成文默认 0），故本回滚不吞业务数据；
-- 不可复原的只有"哪些行原本为 NULL"这一信息维度，本迁移不假装能还原它。
ALTER TABLE "color_cards" ALTER COLUMN "stock_quantity"  DROP NOT NULL;
ALTER TABLE "color_cards" ALTER COLUMN "stock_quantity"  DROP DEFAULT;
ALTER TABLE "color_cards" ALTER COLUMN "issued_quantity" DROP NOT NULL;
ALTER TABLE "color_cards" ALTER COLUMN "issued_quantity" DROP DEFAULT;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
