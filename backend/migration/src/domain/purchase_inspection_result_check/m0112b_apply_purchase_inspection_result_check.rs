//! purchase_inspection.inspection_result 值域收口，施加词表 CHECK 与收尾自证（配套 m0112a 只读点名）
//!
//! 语义定案：本列权威词表唯一出现在 backend 侧 models::status::purchase_inventory::
//! purchase_inspection_result（pass、fail、partial 三值），完成质检入口已按该白名单强校验；
//! 本迁移把取值域钉进数据库，阻止任何绕过服务层入参校验的旁路写入产生词表外脏值。
//!
//! 步骤顺序不可倒，每步存在理由如下。
//! 1. 先 DROP CONSTRAINT IF EXISTS，重跑或半态库不报错，幂等前置。
//! 2. 再存量守卫，fail-visible，词表外（含空串）残留大于 0 即 RAISE EXCEPTION，消息带
//!    行数与逐值分布，HINT 指向权威词表文件。本迁移不 UPDATE 造值洗数据，脏行交业务
//!    逐行定性后重跑；NULL 是合法初值（完成质检前置态），不在守卫拒绝之列。
//! 3. 才施加 CHECK，形态为（inspection_result IS NULL OR inspection_result IN (...)），
//!    取值集由 ALLOWED 常量单源注入 IN 占位，不在 ADD CONSTRAINT 里内联第二套清单。
//!    约束名 chk_purchase_inspection_inspection_result，随后 COMMENT ON COLUMN 固化词表指向。
//! 4. 收尾回读自证：约束必须 contype 为 c 且 convalidated 为真且同名恰一条（NOT VALID
//!    会放行存量脏行不算收口）；同列除本约束外不得存在另一条引用该列的 CHECK。列可空性
//!    必须维持 is_nullable 为 YES（NULL 合法初值的回读印证，本批不动 NOT NULL 与默认值）。
//!
//! 跨车道说明：本表无其他在途车道拥有，purchase 收货与出入库车道的文件不在本迁移触达面内；
//! 只补 DB 约束，不改应用层白名单校验，两侧取值集由纸面锁钉逐字相等。
//!
//! 可重入性：DROP 前置使重跑安全，守卫对已达词表内现态恒真，自证对已达形态恒真。
//!
//! down 真实可逆：DROP CONSTRAINT IF EXISTS 带回滚探测，不回退数据，不复活旧越界值也不
//! 伪造历史行，口径同 wage_energy_vocab_check 的 down。
//!
//! 域与注册位置：本域排 lib.rs 迁移链尾，注册在同域 up 链 m0112a 之后，先点名后收口，
//! 收口失败时日志已有 m0112a 现场。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 与 models::status::purchase_inventory::purchase_inspection_result::ALL 逐字符一致，
/// 与 m0112a 携带同一份，由纸面锁钉两支迁移逐字相等。
const PURCHASE_INSPECTION_RESULT_ALLOWED: &str = "'pass','fail','partial'";

const TABLE: &str = "purchase_inspection";
const COLUMN: &str = "inspection_result";
const CONSTRAINT: &str = "chk_purchase_inspection_inspection_result";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
ALTER TABLE "{table}" DROP CONSTRAINT IF EXISTS "{constraint}";

DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE "{col}" IS NOT NULL AND "{col}" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0112b 守卫 {table}.{col} 无词表外存量行，可安全施加 CHECK {constraint}';
    ELSE
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO samples FROM (
            SELECT '''' || "{col}" || '''' || ' 乘 ' || COUNT(*)::text AS fmt
              FROM "{table}"
             WHERE "{col}" IS NOT NULL AND "{col}" NOT IN ({allowed})
             GROUP BY "{col}"
        ) b;
        RAISE EXCEPTION 'm0112b 守卫 {table}.{col} 存在 % 行词表外取值（逐值分布 %），拒绝施加 CHECK {constraint}', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值，请按权威词表 models::status::purchase_inventory::purchase_inspection_result 逐行核实更正存量，更正后重跑；脏值若来自旁路写入先修写入方，不要在迁移里圆场';
    END IF;
END
$$;

ALTER TABLE "{table}" ADD CONSTRAINT "{constraint}" CHECK ("{col}" IS NULL OR "{col}" IN ({allowed}));
COMMENT ON COLUMN "{table}"."{col}" IS '权威词表 models::status::purchase_inventory::purchase_inspection_result，取值集与 CHECK {constraint} 逐项相等（纸面锁 textile_safety_status_vocabulary_test），NULL 为完成质检前合法初值';

DO $$
DECLARE
    n_con       INTEGER;
    n_colchecks INTEGER;
    v_nullable  TEXT;
BEGIN
    SELECT COUNT(*) INTO n_con FROM pg_constraint
      WHERE conrelid = '{table}'::regclass AND conname = '{constraint}'
        AND contype = 'c' AND convalidated;
    IF n_con <> 1 THEN
        RAISE EXCEPTION 'm0112b 收尾自证失败，{table} 上约束 {constraint} 不存在或非 CHECK 或 convalidated 为假或同名重复，命中行数 %', n_con;
    END IF;

    SELECT COUNT(*) INTO n_colchecks FROM pg_constraint c
      WHERE c.conrelid = '{table}'::regclass AND c.contype = 'c'
        AND pg_get_constraintdef(c.oid) LIKE '%{col}%'
        AND c.conname <> '{constraint}';
    IF n_colchecks <> 0 THEN
        RAISE EXCEPTION 'm0112b 收尾自证失败，{table}.{col} 除本约束外另有 % 条涉及该列的 CHECK，同列多 CHECK 即第二套词表', n_colchecks;
    END IF;

    SELECT is_nullable INTO v_nullable FROM information_schema.columns
     WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{col}';
    IF v_nullable <> 'YES' THEN
        RAISE EXCEPTION 'm0112b 收尾自证失败，{table}.{col} 的 is_nullable 为 %（期望 YES，NULL 是本列合法初值，本批不改可空性）', v_nullable;
    END IF;
    RAISE NOTICE 'm0112b 收尾自证通过，CHECK 已生效且同列唯一，列可空性维持 YES';
END
$$;
"#,
            table = TABLE,
            col = COLUMN,
            constraint = CONSTRAINT,
            allowed = PURCHASE_INSPECTION_RESULT_ALLOWED,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除约束，不回退数据，删约束即回到可写状态，
        // 不复活旧越界值也不伪造历史行，口径同 wage_energy_vocab_check 的 down。
        let sql = format!(
            "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\n",
            TABLE, CONSTRAINT
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
