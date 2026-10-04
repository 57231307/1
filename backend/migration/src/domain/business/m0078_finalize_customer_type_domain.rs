//! customers.customer_type 值域收口：备份 → 定案回填 → 残留自证 → DEFAULT/NOT NULL/CHECK
//!
//! 语义定案：本列是**渠道**，权威词表唯一出现在 `backend/src/constants/customer_type.rs`
//! 的 `ALLOWED`（retail/wholesale/distributor/manufacturer/other）。收口前库里混有
//! RETAIL/POTENTIAL/normal/vip/NULL 等历史写入口留下的脏值，列本身可空、无默认、无
//! CHECK（生效定义 = system/m0001_initial_schema.rs:332 `VARCHAR(20)`；finance/mod.rs:534
//! 的 `ADD COLUMN IF NOT EXISTS VARCHAR(255)` 因列早已存在是恒 no-op，从未生效）。
//! CHECK 词表与应用校验同源同一五值，是为了避免两侧各自漂移：词表变更必须同时动
//! constants 与本约束，本文件不引入第二套取值。
//!
//! 步骤顺序不可倒（每步的存在理由）：
//! 1. 先建备份列 `customer_type_pre_domain_fix VARCHAR(20)`（与源列同宽，复制无损）并
//!    整列快照原值——回填会抹掉原值形态（RETAIL→retail 等），无快照则 down 无法真实
//!    还原、事后也无法复核映射正确性；
//! 2. 再按定案映射逐值 UPDATE（精确逐字符匹配，不做大小写/trim 归一——归一会把词表外
//!    变体静默吞进映射，未知变体必须落到步骤 3 被点名拒绝）；
//! 3. 回填后自证：残留（NULL 或五值外）非 0 即 RAISE EXCEPTION 并列出残留分布——
//!    **回填没干净就不许进 CHECK**，禁止静默加约束后让脏行留在库里；本批 SQL 经
//!    simple query 单隐式事务执行，该拒错连带回滚步骤 1/2，不留半态；
//! 4. 才允许 SET DEFAULT 'other' / SET NOT NULL / ADD CONSTRAINT
//!    chk_customers_customer_type（约束名与 constants/customer_type.rs 注释成文约定一致）；
//! 5. 收尾回读自证：information_schema 断言 is_nullable='NO' 且 column_default 为
//!    'other'，pg_constraint 断言 CHECK 存在且 **convalidated**（NOT VALID 约束放行
//!    脏行，不算收口完成）。
//!
//! 可重入性：备份列存在即跳过再复制（防重跑把已回填值覆盖成首快照）；五值 UPDATE
//! 重跑 0 命中无害；ADD CONSTRAINT 前探测 pg_constraint 跳过；收尾断言对已达形态
//! 恒真。NOT NULL 的写入面注意：DEFAULT 只在 INSERT 省略列时生效，显式写 NULL 会被
//! NOT NULL 拒绝——这与"NULL 不再是合法存储形态"的定案语义一致，实体/DTO 侧同步
//! （NOT NULL 列不得再按 Option 缺省写 NULL）由后端按本文件口径处理。
//!
//! down 真实可逆（禁止空实现）：按约束→NOT NULL→DEFAULT→备份列还原原值（含 NULL）
//! →删备份列→恢复旧列注释的顺序逐步回退，每步带存在性探测以便重复执行；备份列缺失
//! 时不静默吞掉而是 NOTICE 点名"无法还原、保留五值现态待人工处置"。
//!
//! 域与注册位置：customers 由 system 域 m0001 建表，business 域晚于 system（lib.rs
//! 域序 system→business→…→finance），本收口必须**先于** finance 域那段恒 no-op 的
//! ADD COLUMN 之后再无人改写本列形态；v15 的 customer_pool_rules.customer_type 是
//! 公海规则作用域值、非本列口径（constants 文件头已点名两回事）。注册在 business
//! up 链尾、m0077（只读点名）之后：先点名后收口，m0078 失败时日志已有现场。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 五值词表的 SQL 字面量清单（与 constants/customer_type.rs ALLOWED 同源逐字符一致；
/// 自证、CHECK、m0077 三处共用此一份，禁止另写一套）。
const ALLOWED_SQL: &str = "'retail','wholesale','distributor','manufacturer','other'";

/// 列注释旧文（system/m0001:344 原文，down 恢复用）与新文（收口语义，up 写入）。
const COLUMN_COMMENT_OLD: &str = "客户类型 - 批发/零售";
const COLUMN_COMMENT_NEW: &str = "客户类型（渠道）- retail/wholesale/distributor/manufacturer/other；权威词表 backend/src/constants/customer_type.rs，DB 侧约束 chk_customers_customer_type";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 1) 备份列：先整列快照原值再回填。存在性守卫 = 重跑时保留**首次**快照
--    （列已在却再复制，会把首快照污染成本迁移回填后的值，回滚面即失真）。
DO $$
DECLARE
    has_backup BOOLEAN;
BEGIN
    SELECT EXISTS (SELECT 1 FROM information_schema.columns
                    WHERE table_schema = 'public' AND table_name = 'customers'
                      AND column_name = 'customer_type_pre_domain_fix')
      INTO has_backup;
    IF has_backup THEN
        RAISE NOTICE 'm0078：备份列 customer_type_pre_domain_fix 已存在（重跑场景），保留首次快照，跳过再复制';
    ELSE
        ALTER TABLE "customers" ADD COLUMN "customer_type_pre_domain_fix" VARCHAR(20);
        UPDATE "customers" SET "customer_type_pre_domain_fix" = "customer_type";
        RAISE NOTICE 'm0078：备份列已建并整列复制原值（与源列同宽 VARCHAR(20)，复制无损）';
    END IF;
END
$$;

-- 2) 定案映射回填（逐值精确匹配、逐字符；NULL→other 同口径）。
UPDATE "customers" SET "customer_type" = 'retail' WHERE "customer_type" = 'RETAIL';
UPDATE "customers" SET "customer_type" = 'other'  WHERE "customer_type" = 'POTENTIAL';
UPDATE "customers" SET "customer_type" = 'other'  WHERE "customer_type" = 'normal';
UPDATE "customers" SET "customer_type" = 'other'  WHERE "customer_type" = 'vip';
UPDATE "customers" SET "customer_type" = 'other'  WHERE "customer_type" IS NULL;

-- 3) 回填后自证：残留非 0 即拒绝进入约束收口，并逐值点名残留分布
--    （未知残留无定案映射，绝不猜测归一；现场已由 m0077 事前点名过，可对照裁定）。
DO $$
DECLARE
    bad_count INTEGER;
    bad_list TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_count FROM "customers"
     WHERE "customer_type" IS NULL
        OR "customer_type" NOT IN ({allowed});
    IF bad_count > 0 THEN
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO bad_list FROM (
            SELECT COALESCE('''' || "customer_type" || '''', '(NULL)') || ' × ' || COUNT(*)::text AS fmt
              FROM "customers"
             WHERE "customer_type" IS NULL
                OR "customer_type" NOT IN ({allowed})
             GROUP BY "customer_type"
        ) b;
        RAISE EXCEPTION 'm0078：回填后仍有 % 行 customer_type 落在五值词表之外（残留分布：%）。拒绝执行 DEFAULT/NOT NULL/CHECK——不允许静默加约束后让脏行留在库里。此残留值不在定案映射内，请人工定口径处置后重跑本迁移（整批单事务，本次拒绝已回滚全部改动，库内无半态）。', bad_count, bad_list;
    END IF;
    RAISE NOTICE 'm0078：回填后自证通过，customer_type 全部落在五值词表内';
END
$$;

-- 4) 约束收口（顺序：先默认值再 NOT NULL，最后 CHECK；均在自证通过后执行）。
ALTER TABLE "customers" ALTER COLUMN "customer_type" SET DEFAULT 'other';
ALTER TABLE "customers" ALTER COLUMN "customer_type" SET NOT NULL;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = '"customers"'::regclass
                      AND conname = 'chk_customers_customer_type') THEN
        ALTER TABLE "customers" ADD CONSTRAINT "chk_customers_customer_type"
            CHECK ("customer_type" IN ({allowed}));
        RAISE NOTICE 'm0078：已添加 CHECK chk_customers_customer_type（五值，与 constants/customer_type.rs ALLOWED 同源）';
    ELSE
        RAISE NOTICE 'm0078：CHECK 已存在（重跑场景），跳过添加；convalidated 交由收尾自证把关';
    END IF;
END
$$;
COMMENT ON COLUMN "customers"."customer_type" IS '{comment_new}';

-- 5) 收尾回读自证：约束的真实生效形态以系统目录为准，任一项不满足即拒滚点名。
DO $$
DECLARE
    v_nullable TEXT;
    v_default TEXT;
    v_check_ok BOOLEAN;
BEGIN
    SELECT "is_nullable", "column_default" INTO v_nullable, v_default
      FROM information_schema.columns
     WHERE table_schema = 'public' AND table_name = 'customers'
       AND column_name = 'customer_type';
    IF v_nullable IS NULL THEN
        RAISE EXCEPTION 'm0078：information_schema 中找不到 customers.customer_type，与本域建表迁移漂移，中止';
    END IF;
    IF v_nullable <> 'NO' THEN
        RAISE EXCEPTION 'm0078 收尾自证失败：is_nullable = %（期望 NO），NOT NULL 未真实生效', v_nullable;
    END IF;
    IF v_default IS NULL OR (v_default <> '''other''' AND v_default <> '''other''::character varying') THEN
        RAISE EXCEPTION 'm0078 收尾自证失败：column_default = %（期望默认 ''other''，含 varchar 类型标注），DEFAULT 未真实生效', v_default;
    END IF;
    SELECT EXISTS (SELECT 1 FROM pg_constraint
                    WHERE conrelid = '"customers"'::regclass
                      AND conname = 'chk_customers_customer_type'
                      AND contype = 'c'
                      AND convalidated)
      INTO v_check_ok;
    IF NOT v_check_ok THEN
        RAISE EXCEPTION 'm0078 收尾自证失败：chk_customers_customer_type 不存在、非 CHECK、或 convalidated=false（NOT VALID 约束会放行存量脏行，不算收口完成），请人工核查 pg_constraint';
    END IF;
    RAISE NOTICE 'm0078 收尾自证通过：customer_type 已 NOT NULL、DEFAULT ''other''、五值 CHECK 校验存在';
END
$$;
"#,
            allowed = ALLOWED_SQL,
            comment_new = COLUMN_COMMENT_NEW,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 1) 撤 CHECK（存在性探测：重跑 down / 未达该步的库不报错）。
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_constraint
                WHERE conrelid = '"customers"'::regclass
                  AND conname = 'chk_customers_customer_type') THEN
        ALTER TABLE "customers" DROP CONSTRAINT "chk_customers_customer_type";
        RAISE NOTICE 'm0078 down：已移除 CHECK chk_customers_customer_type';
    ELSE
        RAISE NOTICE 'm0078 down：CHECK 不存在，跳过';
    END IF;
END
$$;

-- 2) 撤 NOT NULL 与 DEFAULT（各自探测目录现状；必须先于还原——还原要写回 NULL）。
DO $$
DECLARE
    attnot BOOLEAN;
    has_def BOOLEAN;
BEGIN
    SELECT attnotnull INTO attnot FROM pg_attribute
     WHERE attrelid = '"customers"'::regclass AND attname = 'customer_type';
    SELECT EXISTS (SELECT 1 FROM pg_attrdef d
             JOIN pg_attribute a ON a.attrelid = d.adrelid AND a.attnum = d.adnum
                    WHERE a.attrelid = '"customers"'::regclass AND a.attname = 'customer_type')
      INTO has_def;
    IF attnot THEN
        ALTER TABLE "customers" ALTER COLUMN "customer_type" DROP NOT NULL;
    END IF;
    IF has_def THEN
        ALTER TABLE "customers" ALTER COLUMN "customer_type" DROP DEFAULT;
    END IF;
    RAISE NOTICE 'm0078 down：NOT NULL/DEFAULT 探测式撤除完成（回退前 attnotnull=%, has_default=%）', attnot, has_def;
END
$$;

-- 3) 用备份列还原原值（含 NULL 与全部脏值形态），再删备份列。
--    up 之后新插入的行备份列为 NULL：还原成 NULL 恰与"收口前可空"旧形态语义一致。
--    备份列缺失（up 从未跑到、或 down 重跑）时不静默：NOTICE 点名无法还原的现状。
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public' AND table_name = 'customers'
                  AND column_name = 'customer_type_pre_domain_fix') THEN
        UPDATE "customers" SET "customer_type" = "customer_type_pre_domain_fix";
        ALTER TABLE "customers" DROP COLUMN "customer_type_pre_domain_fix";
        RAISE NOTICE 'm0078 down：已用备份列还原原值（含 NULL/脏值）并删除备份列';
    ELSE
        RAISE NOTICE 'm0078 down：备份列不存在，无法还原原值——现库保留五值回填结果，回滚是否可接受由人工判定';
    END IF;
END
$$;

-- 4) 列注释恢复为收口前原文（system/m0001:344）。
COMMENT ON COLUMN "customers"."customer_type" IS '{comment_old}';
"#,
            comment_old = COLUMN_COMMENT_OLD,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
