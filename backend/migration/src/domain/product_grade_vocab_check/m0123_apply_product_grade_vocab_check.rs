//! 产品等级列取值域收口，施加 CHECK 与收尾自证（配套 m0122 只读点名）
//!
//! 语义定案。products.product_grade 的权威词表唯一出现在 backend 侧
//! models::status::general::product_grade::ALL_GRADES（优等品、一等品、合格品，
//! GB/T 分等定级三档，中文稳定值落库）。本域施加的 CHECK 取值集与该词表逐项相等，
//! 由本文件常量单源注入。NULL 语义为未录入即合法，本批不加 NOT NULL、不加 DEFAULT，
//! 列保持可空无默认。
//!
//! 列宽漂移不在本批收型。同名列在两处历史加列语句里分别写 VARCHAR(255) 与 VARCHAR(10)，
//! 均为 IF NOT EXISTS，先跑者胜，实际宽度取决于库的建列时机（新库走链首语句得 255，
//! 早期建库走 v15 域语句得 10）。三档词表最长值三个汉字，两型宽都容得下，
//! CHECK 收口不依赖列宽，本批只钉取值不钉宽度，宽度收一交后续批号裁定。
//!
//! 步骤顺序不可倒，每步存在理由如下。
//! 1. 先 DROP CONSTRAINT IF EXISTS，重跑或半态库不报错，幂等前置。
//! 2. 再存量守卫，fail-visible，非空越界残留大于 0 即 RAISE EXCEPTION，消息带表名、
//!    列名、行数与逐值分布，HINT 指向权威词表文件。本迁移不 UPDATE 造值洗数据，脏行交
//!    业务逐行定性后重跑，禁止静默加约束后把脏行留在库里。点名现场见同域 m0122 日志。
//! 3. 才允许 ADD CONSTRAINT，约束名 chk_products_product_grade，二十六字符，
//!    在 PG 六十三上限内。随后 COMMENT ON COLUMN 固化词表与口径指向。
//! 4. 收尾回读自证，读 pg_constraint 与 information_schema，任一项不满足即 RAISE
//!    EXCEPTION 点名。约束必须 contype 为 c 且 convalidated 为真且同名恰一条，
//!    NOT VALID 会放行存量脏行不算收口。列必须 is_nullable 为 YES 且 column_default 为空，
//!    本批保持可空无默认，NULL 语义为未录入，钉这两项是定案的回读印证。
//!
//! 不加 DEFAULT 的理由。给可空枚举列硬造默认会把未录入显示成实档，与「未录入即 NULL」
//! 语义冲突，三档都是真实业务值，用作默认会与合法取值混淆。
//!
//! 事务与失败保护。sea-orm-migration 把每支 up 包在单事务内，PG 的 DDL 具事务性，
//! 守卫或自证 RAISE EXCEPTION 时整条 up 自动回滚，不留半态。
//!
//! 可重入性。DROP 前置使重跑安全，守卫对已达口径现态恒真，ADD CONSTRAINT 前已 DROP 旧约束，
//! 自证对已达形态恒真。
//!
//! down 真实可逆，DROP CONSTRAINT IF EXISTS 即回到可写形态，不回退数据，
//! 不复活旧越界值也不伪造历史行。
//!
//! 域与注册位置。本域排 lib.rs 迁移链尾，晚于 system 域与 v15 域（两域各有一条对该列的
//! 历史加列语句），products 表与该列届时必然存在。注册在同域 up 链 m0122 之后，
//! 先点名后收口，本支失败时日志已有 m0122 现场可比照。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 产品等级三档 SQL 字面量清单，与 models::status::general::product_grade::ALL_GRADES
/// 逐项逐字符一致，与同域 m0122 携带同一份，由纸面锁钉两支迁移逐字相等。
const PRODUCT_GRADE_ALLOWED: &str = "'优等品','一等品','合格品'";

/// 收口目标与口径出处。谓词放行 NULL（未录入合法），非空必须在词表内。
const TABLE: &str = "products";
const COLUMN: &str = "product_grade";
const CONSTRAINT: &str = "chk_products_product_grade";
const VOCAB: &str = "models::status::general::product_grade::ALL_GRADES";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let legal = format!("\"{COLUMN}\" IS NULL OR \"{COLUMN}\" IN ({PRODUCT_GRADE_ALLOWED})");
        let sql = format!(
            r#"
ALTER TABLE "{table}" DROP CONSTRAINT IF EXISTS "{constraint}";

DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE NOT ({legal});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0123 守卫 {table}.{col} 无非空越界存量行，可安全施加 CHECK {constraint}';
    ELSE
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO samples FROM (
            SELECT COALESCE('''' || "{col}" || '''', '(NULL)') || ' 乘 ' || COUNT(*)::text AS fmt
              FROM "{table}"
             WHERE NOT ({legal})
             GROUP BY "{col}"
        ) b;
        RAISE EXCEPTION 'm0123 守卫 {table}.{col} 存在 % 行越界取值（逐值分布 %），拒绝施加 CHECK {constraint}', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值，请按权威口径 {vocab} 逐行核实更正存量，更正后重跑；脏值若来自旁路写入先修写入方，不要在迁移里圆场';
    END IF;
END
$$;

ALTER TABLE "{table}" ADD CONSTRAINT "{constraint}" CHECK ({legal});
COMMENT ON COLUMN "{table}"."{col}" IS '权威口径 {vocab}，取值域与 CHECK {constraint} 逐项相等，可空无默认，NULL 语义为未录入';

DO $$
DECLARE
    n_con       INTEGER;
    v_contype   "char";
    v_validated BOOLEAN;
    v_nullable  TEXT;
    v_default   TEXT;
BEGIN
    SELECT COUNT(*) INTO n_con FROM pg_constraint c
      JOIN pg_class r ON r.oid = c.conrelid
     WHERE r.relname = '{table}' AND c.conname = '{constraint}';
    IF n_con <> 1 THEN
        RAISE EXCEPTION 'm0123 自证 {table}.{col} 的约束 {constraint} 同名条数为 %，要求恰一条', n_con;
    END IF;

    SELECT c.contype, c.convalidated INTO v_contype, v_validated FROM pg_constraint c
      JOIN pg_class r ON r.oid = c.conrelid
     WHERE r.relname = '{table}' AND c.conname = '{constraint}';
    IF v_contype <> 'c' THEN
        RAISE EXCEPTION 'm0123 自证 {table}.{col} 的约束 {constraint} contype 为 %，要求 c 即 CHECK 约束', v_contype;
    END IF;
    IF v_validated IS NOT TRUE THEN
        RAISE EXCEPTION 'm0123 自证 {table}.{col} 的约束 {constraint} 未 convalidated，NOT VALID 会放行存量脏行不算收口';
    END IF;

    SELECT is_nullable, column_default INTO v_nullable, v_default FROM information_schema.columns
     WHERE table_name = '{table}' AND column_name = '{col}';
    IF v_nullable IS DISTINCT FROM 'YES' THEN
        RAISE EXCEPTION 'm0123 自证 {table}.{col} 的 is_nullable 为 %，本批定案保持可空（NULL 即未录入）', v_nullable;
    END IF;
    IF v_default IS NOT NULL THEN
        RAISE EXCEPTION 'm0123 自证 {table}.{col} 带 column_default %，本批不施加任何默认值，硬造默认会把未录入显示成实值', v_default;
    END IF;

    RAISE NOTICE 'm0123 自证 {table}.{col} 约束 {constraint} 已达形态，contype 为 c 且 convalidated 为真且可空无默认';
    RAISE NOTICE 'm0123 收口完成，产品等级取值域已钉进数据库';
END
$$;
"#,
            table = TABLE,
            col = COLUMN,
            constraint = CONSTRAINT,
            legal = legal,
            vocab = VOCAB,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删约束即回到可写形态，不回退任何数据，不复活旧越界值也不伪造历史行。
        let sql = format!(
            "ALTER TABLE \"{table}\" DROP CONSTRAINT IF EXISTS \"{constraint}\";\nDO $$ BEGIN RAISE NOTICE 'm0123 回滚完成，{table}.{col} 的 CHECK 已撤除'; END $$;",
            table = TABLE,
            col = COLUMN,
            constraint = CONSTRAINT,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
