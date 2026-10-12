//! `customers.tier` 客户分层列：加列（可空）→ 按既有评级逐档映射回填 → CHECK 收口
//!
//! ## 六要素
//! - **功能**：给 `customers` 加**客户分层（tier）**列，值域四档 `VIP / GOLD / SILVER / NORMAL`，
//!   可空（NULL=未定档），DB CHECK `chk_customers_tier`；同批按 `customer_credit_ratings.credit_level`
//!   的既有真实数据做**一一对应的单档映射回填**（不做数值分档猜测）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/business` 域 up 链尾、down 链首。
//!   目标表 `customers` 由 system 域 m0001 建（早于本域）；回填参照表
//!   `customer_credit_ratings` 由本域 m0012 建、其单行唯一与外键由本域 m0082 施加，
//!   三者均先于链尾此处——注册位置沿 m0077/m0078/m0082 的 customers 收口先例。
//! - **入参**：无（读取库内 `customer_credit_ratings.credit_level` 既有值）。
//! - **传给谁**：大客户转单二级审批判据（`services/crm/customer_transfer_approval_service.rs`
//!   的 `check_large_customer` 只读本列）；应用层权威词表
//!   `backend/src/constants/customer_tier.rs`，与本约束逐值同源。
//! - **存什么**：客户分层 token（四值之一）或 NULL=未定档。
//! - **存哪里**：PostgreSQL `public.customers.tier` VARCHAR(20)。
//!
//! ## 词表为什么复用既有四值而不另定新词表
//! 本仓**已有**客户等级/分层词表且已双端收口：DB 侧
//! `chk_tier_customer_level`（`color_price_tiers.customer_level`，production 域 m0044）取值
//! `VIP / NORMAL / GOLD / SILVER`；应用侧 `services/color_price_crud_service.rs` 校验同一集合，
//! `utils/price_calculator.rs` 的等级折扣按这四个大写 token 逐字符消费。价格阶梯的
//! `customer_level` 语义本就是"客户等级"，若 customers 另立 `standard/silver/gold/strategic`
//! 即同概念第二套口径（渠道列收口时已裁定分层词必须与既有词表同源），故本列**照抄既有
//! 四值、大小写逐字符一致**，NULL 放行形态也与 `chk_tier_customer_level` 同形。
//!
//! ## 回填规则（每一档都可由既有真实数据复核，禁止造数）
//! 唯一可靠分档依据 = `customer_credit_ratings.credit_level`（该表每客户恒一行由本域 m0082
//! 的全表 UNIQUE 钉死；列虽无 CHECK，但档位词表在评级计算处成文：
//! `services/customer_credit_evaluate.rs` 的 `calculate_rating_and_limit` 固定映射
//! AAA(90分)→100万、AA(80)→50万、A(70)→20万、BBB(60)→10万、BB(50)→5万、B(10)→1万，
//! AAA 最高、B 最低）。分层四档与评级六档均是无序符号的**有序等级**，回填取序位对齐：
//! - `AAA → VIP`（评级最高 ↔ 分层最高档）
//! - `AA → GOLD`（次高 ↔ 次高）
//! - `A → SILVER`（第三 ↔ 第三）
//! - `BBB / BB / B → NORMAL`（评级低三档在分层阶梯上没有专档，统一落最低实档 NORMAL）
//! - **其余形态一律不回填、保持 NULL（未定档）**：无评级行的客户、`credit_level` 为
//!   NULL、或落在六值词表外的自由文本（写入口 `services/customer_credit_limit.rs` 收
//!   自由文本，未知变体归任何一档都是猜测）。NULL 与 NORMAL 语义严格区分：前者=
//!   无依据未定档，后者=有评级依据且落在低三档。
//! 逐行复核 SQL（回滚/审计同一份）：
//! ```text
//! SELECT c.id, r.credit_level, c.tier
//!   FROM customers c JOIN customer_credit_ratings r ON r.customer_id = c.id
//!  WHERE c.tier IS NOT NULL;   -- 每行 tier 都能由同行的 credit_level 按上表复算
//! ```
//! 只读摸底（各档将回填多少行 / 多少行无法定档保持 NULL，活库执行；up 运行时亦经
//! NOTICE 自动打印同一分布，本环境无活库——`TEST_DATABASE_URL` 为空且无 psql，
//! 全新迁移库上 customers 零种子（迁移域内无 `INSERT INTO customers`），计数以活库为准）：
//! ```text
//! -- ① 各档回填量
//! SELECT m.tier, COUNT(*) FROM customers c
//!   JOIN customer_credit_ratings r ON r.customer_id = c.id
//!   JOIN (VALUES ('AAA','VIP'),('AA','GOLD'),('A','SILVER'),
//!                ('BBB','NORMAL'),('BB','NORMAL'),('B','NORMAL')) AS m(lvl, tier)
//!     ON r.credit_level = m.lvl
//!  GROUP BY m.tier ORDER BY m.tier;
//! -- ② 有评级行但 credit_level 落在六值词表外（保持 NULL，交业务更正评级）
//! SELECT COALESCE(r.credit_level,'(NULL)'), COUNT(*)
//!   FROM customer_credit_ratings r
//!   JOIN customers c ON c.id = r.customer_id AND c.tier IS NULL
//!  WHERE r.credit_level IS NULL OR r.credit_level NOT IN
//!        ('AAA','AA','A','BBB','BB','B')
//!  GROUP BY 1;
//! -- ③ 无评级行（保持 NULL）
//! SELECT COUNT(*) FROM customers c
//!  WHERE c.tier IS NULL
//!    AND NOT EXISTS (SELECT 1 FROM customer_credit_ratings r WHERE r.customer_id = c.id);
//! ```
//!
//! ## 步骤顺序与幂等
//! 1. `ADD COLUMN IF NOT EXISTS "tier" VARCHAR(20)`——可空、无 DEFAULT：NULL 是合法形态
//!    （未定档），给默认值等于替所有存量客户伪造一个档；
//! 2. 单档映射回填——`WHERE c.tier IS NULL` 守卫：重跑只补仍为空的行，**不覆盖**业务
//!    此后手填的分层值；六值外评级不命中任何 WHEN/CASE 分支，天然不回填；
//! 3. 只读摸底 NOTICE 打印回填后逐档分布（含未定档合计），供迁移日志复核回填面；
//! 4. CHECK `chk_customers_tier`（`IS NULL OR IN (四值)`）——pg_constraint 存在性探测后
//!    才添加，重跑跳过；存量此刻只可能是 NULL 或四值，约束必然可过；
//! 5. 收尾回读自证：约束存在、contype='c'、convalidated=true、列 is_nullable='YES'、
//!    列无 DEFAULT。任一项不符 RAISE EXCEPTION 中止（本批经 simple query 单隐式事务，
//!    拒绝即整体回滚，不留半态）。
//!
//! ## down 真实可逆
//! 先 `DROP CONSTRAINT IF EXISTS chk_customers_tier`，再 `DROP COLUMN IF EXISTS "tier"`。
//! 列本身即本迁移新增，回滚不触碰任何既有业务数据；回填值完全可由
//! `customer_credit_ratings.credit_level` 按本文件映射重算（评级源数据全程未改），
//! 业务手填值若有存在需在 down 前留档：
//! ```text
//! SELECT id, tier FROM customers WHERE tier IS NOT NULL;
//! ```

use sea_orm_migration::prelude::*;

/// 四值分层词表的 SQL 字面量清单（与 `constants/customer_tier.rs` 的 `ALLOWED` 逐字符
/// 同源；CHECK 唯一取值种子，契约锁从本文件解析此锚点，禁止另写第二套清单）。
const ALLOWED_SQL: &str = "'VIP','GOLD','SILVER','NORMAL'";

/// 评级六档 → 分层四档的单档映射（回填唯一真源，与摸底 SQL 同源；档位序位依据见文件头）。
const RATING_TO_TIER_SQL: &str = "CASE r.\"credit_level\"\
     WHEN 'AAA' THEN 'VIP'\
     WHEN 'AA'  THEN 'GOLD'\
     WHEN 'A'   THEN 'SILVER'\
     WHEN 'BBB' THEN 'NORMAL'\
     WHEN 'BB'  THEN 'NORMAL'\
     WHEN 'B'   THEN 'NORMAL' END";

/// 评级六档词表（`services/customer_credit_evaluate.rs` 的 `calculate_rating_and_limit`
/// 成文映射；回填只认这六个逐字符值，其余不猜）。
const RATING_TOKENS_SQL: &str = "'AAA','AA','A','BBB','BB','B'";

const COLUMN_COMMENT: &str = "客户分层（tier）- VIP/GOLD/SILVER/NORMAL，NULL=未定档；权威词表 backend/src/constants/customer_tier.rs，DB 约束 chk_customers_tier";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 1) 加列：可空、无默认（NULL=未定档是合法形态，给默认值=替存量伪造档位）。
ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS "tier" VARCHAR(20);

-- 2) 单档映射回填：只认评级六值逐字符；只补 tier 仍为 NULL 的行（重跑不覆盖业务手填）。
UPDATE "customers" c
   SET "tier" = {rating_case}
  FROM "customer_credit_ratings" r
 WHERE r."customer_id" = c."id"
   AND c."tier" IS NULL
   AND r."credit_level" IN ({ratings});

-- 3) 只读摸底自证：打印回填后的逐档分布与未定档合计（迁移日志即取证面）。
DO $$
DECLARE
    rec RECORD;
    n_total INTEGER;
    n_unknown INTEGER;
BEGIN
    SELECT COUNT(*) INTO n_total FROM "customers";
    FOR rec IN SELECT COALESCE("tier", '(NULL 未定档)') AS t, COUNT(*) AS n
                  FROM "customers" GROUP BY "tier" ORDER BY "tier"
    LOOP
        RAISE NOTICE 'm0083 摸底：customers.tier = % × % 行', rec.t, rec.n;
    END LOOP;
    SELECT COUNT(*) INTO n_unknown FROM "customers" WHERE "tier" IS NULL;
    RAISE NOTICE 'm0083 摸底汇总：客户共 % 行，其中 % 行无评级行或评级落在六值词表外，保持 NULL（未定档，禁止造数）',
        n_total, n_unknown;
END
$$;

-- 4) CHECK 收口（NULL 放行形态与 chk_tier_customer_level 同形；存在性探测保幂等）。
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = '"customers"'::regclass
                      AND conname = 'chk_customers_tier') THEN
        ALTER TABLE "customers" ADD CONSTRAINT "chk_customers_tier"
            CHECK ("tier" IS NULL OR "tier" IN ({allowed}));
        RAISE NOTICE 'm0083：已添加 CHECK chk_customers_tier（四值+NULL，与 constants/customer_tier.rs ALLOWED 同源）';
    ELSE
        RAISE NOTICE 'm0083：CHECK 已存在（重跑场景），跳过添加；convalidated 交由收尾自证把关';
    END IF;
END
$$;
COMMENT ON COLUMN "customers"."tier" IS '{comment}';

-- 5) 收尾回读自证：以系统目录为准，任一形态不符即拒滚点名。
DO $$
DECLARE
    v_nullable TEXT;
    v_check_ok BOOLEAN;
    v_has_def BOOLEAN;
BEGIN
    SELECT "is_nullable" INTO v_nullable
      FROM information_schema.columns
     WHERE table_schema = 'public' AND table_name = 'customers'
       AND column_name = 'tier';
    IF v_nullable IS NULL THEN
        RAISE EXCEPTION 'm0083：information_schema 中找不到 customers.tier，加列未生效，中止';
    END IF;
    IF v_nullable <> 'YES' THEN
        RAISE EXCEPTION 'm0083 收尾自证失败：is_nullable = %（期望 YES，NULL=未定档是合法形态）', v_nullable;
    END IF;
    SELECT EXISTS (SELECT 1 FROM pg_attrdef d
             JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum
                    WHERE a.attrelid = '"customers"'::regclass AND a.attname = 'tier')
      INTO v_has_def;
    IF v_has_def THEN
        RAISE EXCEPTION 'm0083 收尾自证失败：tier 列带 DEFAULT——未定档必须是 NULL，任何默认档位都是替存量造数';
    END IF;
    SELECT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = '"customers"'::regclass
                      AND conname = 'chk_customers_tier'
                      AND contype = 'c'
                      AND convalidated)
      INTO v_check_ok;
    IF NOT v_check_ok THEN
        RAISE EXCEPTION 'm0083 收尾自证失败：chk_customers_tier 不存在、非 CHECK 或 convalidated=false（NOT VALID 放行脏行，不算收口）';
    END IF;
    RAISE NOTICE 'm0083 收尾自证通过：tier 可空、无 DEFAULT、四值+NULL CHECK 已校验存在';
END
$$;
"#,
            rating_case = RATING_TO_TIER_SQL,
            ratings = RATING_TOKENS_SQL,
            allowed = ALLOWED_SQL,
            comment = COLUMN_COMMENT,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 先撤 CHECK（存在性探测：重跑 down / 未达该步的库不报错），再删列。
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conrelid = '"customers"'::regclass
                  AND conname = 'chk_customers_tier') THEN
        ALTER TABLE "customers" DROP CONSTRAINT "chk_customers_tier";
        RAISE NOTICE 'm0083 down：已移除 CHECK chk_customers_tier';
    ELSE
        RAISE NOTICE 'm0083 down：CHECK 不存在，跳过';
    END IF;
END
$$;
ALTER TABLE "customers" DROP COLUMN IF EXISTS "tier";
DO $$
BEGIN
    RAISE NOTICE 'm0083 down：已删除 tier 列（回填值可由 customer_credit_ratings.credit_level 重算，源数据未动）';
END
$$;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
