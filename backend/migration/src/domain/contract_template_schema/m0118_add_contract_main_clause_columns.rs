//! 两侧合同主表条款与签署与印花税列补齐迁移
//!
//! 功能 给采购合同主表补齐销售侧既有的条款四列、签署三列、签章时间、签章人与印花税列，
//!   并给两侧合同主表补内外销标记与含税标记，消除销售与采购两侧的合同能力不对称。
//! 调用方 SeaORM migrator 链，经 `domain/contract_template_schema` 域注册为顶层链尾条目，
//!   域内最后执行，只依赖 system 与 business 与 v15 建好的两张合同主表。
//! 入参 无，纯 DDL 迁移。
//! 传给谁 补出的列由合同读写腿按实体映射消费；历史行的空值由合同详情面显示为
//!   模板前签约条款未固化。
//! 存什么 条款正文列、电子签列、印花税金额列、内外销标记列、含税标记列。
//! 存哪里 PostgreSQL `public.sales_contracts` 与 `public.purchase_contracts`。
//!
//! 列型口径，全部实测自复用活库 information_schema，不凭注释或迁移文本推断
//! - 销售侧既有的条款四列与签署三列与 signed_by_user_id 与 signed_at 与 stamp_tax_amount
//!   保持原样不动。当年 business 域与 v15 域对同名签署列各写了一套宽度，
//!   ADD COLUMN IF NOT EXISTS 恒为 no-op，先执行的 business 版本生效，
//!   活库实测是 VARCHAR(255) 一套。本迁移按**活库真实形态**给采购侧补同形列，
//!   不照 v15 文本里的 64 与 500 与 TEXT 补，否则两侧同形是假的。
//!   销售侧列型与 v15 意图的落差属既有缺陷，不在本迁移里顺手改型，已单列进回报待裁定。
//! - 新增列一律可空且不写 DEFAULT。历史合同的条款列必须保持 NULL，读侧据此显示
//!   模板前签约条款未固化。给 DEFAULT 会让历史行在新读法下冒充成已采集过的空条款。
//! - 本迁移绝不 UPDATE 也绝不 DELETE 任何业务数据。
//!
//! 幂等与失败可见
//! - 加列走 ADD COLUMN IF NOT EXISTS。
//! - 内外销列的 CHECK 走“先加可空列，再只读探测，最后重建约束”。探测排在加列之后，
//!   列必存在，PL/pgSQL 不会因引用未建列在规划期报错；全新库上探测恒为 0 行。
//!   命中词表外取值即 RAISE EXCEPTION 点名样例，不静默放宽词表，不洗数据。
//!   约束施加前先 DROP CONSTRAINT IF EXISTS，半态库可安全重跑。
//! - up 末尾有收尾自证，补列若被 IF NOT EXISTS 静默跳过会把缺席的列点名出来，
//!   迁移链不会带着缺列往下走。

use super::CONTRACT_MARKET_ALLOWED_SQL;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 一、采购侧补齐销售侧既有的条款与签署与印花税列。列型逐列对齐活库实测形态，
        // 一律可空且无默认。销售侧的同名列不动，只把采购侧抬到同一水平。
        let add_purchase_leg = r#"
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "quality_terms" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "breach_liability" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "dispute_resolution" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "performance_period" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "signed_at" TIMESTAMPTZ;
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "signed_by_user_id" INTEGER;
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "signature_hash" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "signature_image_url" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "signature_certificate" VARCHAR(255);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "stamp_tax_amount" DECIMAL(18,4);
COMMENT ON COLUMN "purchase_contracts"."quality_terms" IS '质量条款文本，可空。空值表示模板前签约未固化条款';
COMMENT ON COLUMN "purchase_contracts"."breach_liability" IS '违约责任条款文本，可空。空值表示模板前签约未固化条款';
COMMENT ON COLUMN "purchase_contracts"."dispute_resolution" IS '争议解决条款文本，可空。空值表示模板前签约未固化条款';
COMMENT ON COLUMN "purchase_contracts"."performance_period" IS '履行期限条款文本，可空。空值表示模板前签约未固化条款';
COMMENT ON COLUMN "purchase_contracts"."signed_at" IS '电子签章时间，可空。空值表示未走电子签';
COMMENT ON COLUMN "purchase_contracts"."signed_by_user_id" IS '签章人用户主键，可空，不建外键，与本表其余引用列口径一致';
COMMENT ON COLUMN "purchase_contracts"."signature_hash" IS '合同内容哈希，可空';
COMMENT ON COLUMN "purchase_contracts"."signature_image_url" IS '电子签章图片地址，可空';
COMMENT ON COLUMN "purchase_contracts"."signature_certificate" IS 'CA 证书内容，可空';
COMMENT ON COLUMN "purchase_contracts"."stamp_tax_amount" IS '印花税金额，可空。空值表示未采集，与 0 元是两回事';
"#;
        manager
            .get_connection()
            .execute_unprepared(add_purchase_leg)
            .await?;

        // 二、两侧都补的内外销与含税列。含税是三态布尔，真为含税、假为不含税、空为未采集，
        // 布尔列的三态由可空性本身承载，不再叠 CHECK 假装成词表。
        let add_both_leg = r#"
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "contract_market" VARCHAR(16);
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "contract_market" VARCHAR(16);
ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "tax_inclusive" BOOLEAN;
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "tax_inclusive" BOOLEAN;
COMMENT ON COLUMN "sales_contracts"."contract_market" IS '内外销标记，取值 domestic 或 export，可空。空值表示模板前签约未采集';
COMMENT ON COLUMN "purchase_contracts"."contract_market" IS '内外销标记，取值 domestic 或 export，可空。空值表示模板前签约未采集';
COMMENT ON COLUMN "sales_contracts"."tax_inclusive" IS '含税标记，可空三态。真为含税，假为不含税，空为未采集';
COMMENT ON COLUMN "purchase_contracts"."tax_inclusive" IS '含税标记，可空三态。真为含税，假为不含税，空为未采集';
"#;
        manager
            .get_connection()
            .execute_unprepared(add_both_leg)
            .await?;

        // 三、内外销列的先探测后约束。列已由上一步加好，历史行恒为空值，探测放行 NULL、
        // 只拦词表外非空值；命中即点名中止，不放宽词表也不 UPDATE 造值。
        for table in ["sales_contracts", "purchase_contracts"] {
            let check_sql = format!(
                r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE "contract_market" IS NOT NULL
        AND "contract_market" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0118 探测 {table}.contract_market 无词表外存量，可施加 CHECK chk_{table}_contract_market。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (
            SELECT DISTINCT "contract_market" AS sample
              FROM "{table}"
             WHERE "contract_market" IS NOT NULL
               AND "contract_market" NOT IN ({allowed})
             LIMIT 10
          ) s;
        RAISE EXCEPTION 'm0118 探测到 {table}.contract_market 存在 % 行词表外取值（样例 %）。本迁移拒绝在带词表外存量的列上施加 CHECK，且不做 UPDATE 造值洗数据。请按内外销权威词表逐行核实更正存量，更正后重跑迁移；若脏值来自旁路写入，先修写入方，不要在迁移里圆场。',
            bad_rows, samples;
    END IF;
END
$$;
ALTER TABLE "{table}" DROP CONSTRAINT IF EXISTS "chk_{table}_contract_market";
ALTER TABLE "{table}" ADD CONSTRAINT "chk_{table}_contract_market"
    CHECK ("contract_market" IS NULL OR "contract_market" IN ({allowed}));
"#,
                table = table,
                allowed = CONTRACT_MARKET_ALLOWED_SQL,
            );
            manager
                .get_connection()
                .execute_unprepared(&check_sql)
                .await?;
        }

        // 四、收尾自证。十二枚列在两侧共 24 处必须全部在场，缺任一枚即点名中止。
        // 判据按列名集合数行数，不依赖本文件上方语句的执行结果，重跑等价。
        let verify_sql = r#"
DO $$
DECLARE
    expected INTEGER;
    actual   INTEGER;
    missing  TEXT;
BEGIN
    SELECT COUNT(*) INTO expected FROM (
        SELECT 'sales_contracts' AS t, c AS col FROM unnest(ARRAY[
            'quality_terms','breach_liability','dispute_resolution','performance_period',
            'signed_at','signed_by_user_id','signature_hash','signature_image_url',
            'signature_certificate','stamp_tax_amount','contract_market','tax_inclusive']) AS c
        UNION ALL
        SELECT 'purchase_contracts', c FROM unnest(ARRAY[
            'quality_terms','breach_liability','dispute_resolution','performance_period',
            'signed_at','signed_by_user_id','signature_hash','signature_image_url',
            'signature_certificate','stamp_tax_amount','contract_market','tax_inclusive']) AS c
    ) e;
    SELECT COUNT(*) INTO actual FROM information_schema.columns
      WHERE table_schema = current_schema()
        AND table_name IN ('sales_contracts','purchase_contracts')
        AND column_name IN (
            'quality_terms','breach_liability','dispute_resolution','performance_period',
            'signed_at','signed_by_user_id','signature_hash','signature_image_url',
            'signature_certificate','stamp_tax_amount','contract_market','tax_inclusive');
    IF actual <> expected THEN
        SELECT string_agg(m.col, ', ') INTO missing
          FROM (
            SELECT e.t || '.' || e.col AS col FROM (
                SELECT 'sales_contracts' AS t, c AS col FROM unnest(ARRAY[
                    'quality_terms','breach_liability','dispute_resolution','performance_period',
                    'signed_at','signed_by_user_id','signature_hash','signature_image_url',
                    'signature_certificate','stamp_tax_amount','contract_market','tax_inclusive']) AS c
                UNION ALL
                SELECT 'purchase_contracts', c FROM unnest(ARRAY[
                    'quality_terms','breach_liability','dispute_resolution','performance_period',
                    'signed_at','signed_by_user_id','signature_hash','signature_image_url',
                    'signature_certificate','stamp_tax_amount','contract_market','tax_inclusive']) AS c
            ) e
             WHERE NOT EXISTS (
                SELECT 1 FROM information_schema.columns i
                 WHERE i.table_schema = current_schema()
                   AND i.table_name = e.t AND i.column_name = e.col)
            LIMIT 20
          ) m;
        RAISE EXCEPTION 'm0118 收尾自证失败。两侧合同主表各应有 12 枚条款与签署与印花税与内外销与含税列，期望 % 列，实得 % 列，缺席 %。必有列被 ADD COLUMN IF NOT EXISTS 跳过或列名拼错，请对照本文件头注的活库实测列型逐项核对后重跑。',
            expected, actual, missing;
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(verify_sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚只撤本迁移补出的列与其上的 CHECK，不触碰销售侧既有列，不删任何数据行。
        // 列上已有非空值时先探测并点名中止，拒绝用删列冒充无损回滚。
        let guard_sql = r#"
DO $$
DECLARE
    filled INTEGER;
BEGIN
    SELECT
        (SELECT COUNT(*) FROM sales_contracts
          WHERE contract_market IS NOT NULL OR tax_inclusive IS NOT NULL)
      + (SELECT COUNT(*) FROM purchase_contracts
          WHERE contract_market IS NOT NULL OR tax_inclusive IS NOT NULL)
      + (SELECT COUNT(*) FROM purchase_contracts
          WHERE quality_terms IS NOT NULL OR breach_liability IS NOT NULL
             OR dispute_resolution IS NOT NULL OR performance_period IS NOT NULL
             OR signed_at IS NOT NULL OR signed_by_user_id IS NOT NULL
             OR signature_hash IS NOT NULL OR signature_image_url IS NOT NULL
             OR signature_certificate IS NOT NULL OR stamp_tax_amount IS NOT NULL)
      INTO filled;
    IF filled > 0 THEN
        RAISE EXCEPTION '回滚前探测到 % 行已写入本迁移补出的列值。DROP COLUMN 会连带删除这些值，本迁移拒绝用删列冒充无损回滚。请先把这些行的值清空，或改用前滚修复，再执行回滚。',
            filled;
    END IF;
END
$$;
"#;
        manager
            .get_connection()
            .execute_unprepared(guard_sql)
            .await?;

        let drop_sql = r#"
ALTER TABLE "sales_contracts" DROP CONSTRAINT IF EXISTS "chk_sales_contracts_contract_market";
ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "contract_market";
ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "tax_inclusive";
ALTER TABLE "purchase_contracts" DROP CONSTRAINT IF EXISTS "chk_purchase_contracts_contract_market";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "contract_market";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "tax_inclusive";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "quality_terms";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "breach_liability";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "dispute_resolution";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "performance_period";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "signed_at";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "signed_by_user_id";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "signature_hash";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "signature_image_url";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "signature_certificate";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "stamp_tax_amount";
"#;
        manager
            .get_connection()
            .execute_unprepared(drop_sql)
            .await?;
        Ok(())
    }
}
