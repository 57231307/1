//! 工资与能耗状态列值域收口，施加词表 CHECK 与收尾自证（配套 m0102 只读点名）
//!
//! 语义定案：本批十二列的权威词表唯一出现在 backend 侧 models::status::
//! wage_energy_chemical_business 下八个模块，取值集合逐列取自身词表，不做跨列并集。
//! 十二列建表原文均为 VARCHAR(255) NOT NULL 且无 DEFAULT 无 CHECK，历史上词表只在应用层
//! 防护，本迁移把取值域钉进数据库，阻止任何绕过服务层入参校验的旁路写入产生词表外脏值。
//!
//! 步骤顺序不可倒，每步存在理由如下。
//! 1. 先逐列 DROP CONSTRAINT IF EXISTS，重跑或半态库不报错，幂等前置。
//! 2. 再逐列存量守卫，fail-visible，词表外非 NULL 残留或 NULL 行大于 0 即 RAISE EXCEPTION，
//!    消息带表名、列名、行数与逐值分布，HINT 指向权威词表文件。本迁移不 UPDATE 造值洗数据，
//!    脏行交业务逐行定性后重跑，禁止静默加约束后把脏行留在库里。
//! 3. 才允许逐列 ADD CONSTRAINT，取值集由本文件八份 ALLOWED 常量单源注入 CHECK 的
//!    IN 占位，约束名统一 chk_表_列，最长 chk_energy_consumption_record_recording_method
//!    为四十六字符，在 PG 六十三上限内。随后 COMMENT ON COLUMN 固化词表指向。
//! 4. 收尾回读自证，逐列读 pg_constraint 与 information_schema，任一项不满足即 RAISE
//!    EXCEPTION 点名。约束必须 contype 为 c 且 convalidated 为真且同名恰一条，NOT VALID
//!    会放行存量脏行不算收口。列必须 is_nullable 为 NO 且 column_default 为空，
//!    本批不施加任何 SET DEFAULT，钉 column_default 为空是裁定第三问结论的回读印证。
//!
//! 不加 DEFAULT 的理由，本族与 export_inspection_vocab_check 同形，建表原文无 DEFAULT，
//! 给 NOT NULL 列补 DB 默认会让旁路裸 SQL 省略该列时静默落出合法但未核算的行，比撞 23502 更坏，
//! 且 energy_meter 建行默认是 active 非 draft，硬造 draft 默认会制造错误初始态。
//!
//! 事务与失败保护，sea-orm-migration 把每支 up 包在单事务内，PG 的 DDL 具事务性，
//! 守卫或自证 RAISE EXCEPTION 时整条 up 自动回滚，不留半态。本支所有语句合成一条
//! simple query 一次执行，进一步保证同事务原子性。
//!
//! 可重入性，DROP 前置使重跑安全，守卫对已达词表内现态恒真，ADD CONSTRAINT 前已 DROP
//! 旧约束，自证对已达形态恒真。
//!
//! down 真实可逆，逐列 DROP CONSTRAINT IF EXISTS 带回滚探测，不回退数据。删约束即回到
//! 可写状态，不复活旧越界值也不伪造历史行，口径同 price_vocab_check 与 export_inspection_vocab_check 的 down。
//!
//! 域与注册位置，本域排 lib.rs 迁移链尾，晚于 migration/src/domain/v15 建表域，六张目标表
//! 届时必然存在。注册在同域 up 链 m0102 之后，先点名后收口，m0103 失败时日志已有 m0102 现场。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 八份 SQL 字面量清单，与 models::status 下八个词表模块逐项逐字符一致。
/// 守卫、CHECK、列注释、点名共用同一份，禁止另写第二套。与 m0102 携带同一份，
/// 由纸面锁钉两支迁移逐字相等。
const WAGE_RATE_STATUS_ALLOWED: &str = "'draft','active','disabled'";
const WAGE_RECORD_STATUS_ALLOWED: &str = "'draft','confirmed','paid','cancelled'";
const ENERGY_METER_STATUS_ALLOWED: &str = "'active','inactive','maintenance'";
const ENERGY_RECORD_STATUS_ALLOWED: &str = "'draft','confirmed','cancelled'";
const ENERGY_RECORDING_METHOD_ALLOWED: &str = "'manual','iot','auto_calc'";
const ENERGY_TYPE_ALLOWED: &str = "'water','electricity','steam','gas','compressed_air'";
const ENERGY_RULE_STATUS_ALLOWED: &str = "'draft','active','disabled'";
const ENERGY_ALLOCATION_BASIS_ALLOWED: &str =
    "'by_duration','by_output','by_equipment','by_workshop'";

/// 一列的收口配方，表、列、约束名、该列自身词表、词表模块符号名供列注释与 HINT 指向。
struct ColumnCheck<'a> {
    table: &'a str,
    column: &'a str,
    constraint: &'a str,
    allowed: &'a str,
    vocab: &'a str,
}

/// 十二列逐列配方，取值集合按各列自身词表，不做跨列并集。全部 NOT NULL，
/// CHECK 直接写列 IN (...) 不留 NULL 放行。
const COLUMN_CHECKS: &[ColumnCheck] = &[
    ColumnCheck {
        table: "process_wage_rate",
        column: "status",
        constraint: "chk_process_wage_rate_status",
        allowed: WAGE_RATE_STATUS_ALLOWED,
        vocab: "models::status::wage_rate_status",
    },
    ColumnCheck {
        table: "wage_record",
        column: "status",
        constraint: "chk_wage_record_status",
        allowed: WAGE_RECORD_STATUS_ALLOWED,
        vocab: "models::status::wage_record_status",
    },
    ColumnCheck {
        table: "energy_meter",
        column: "status",
        constraint: "chk_energy_meter_status",
        allowed: ENERGY_METER_STATUS_ALLOWED,
        vocab: "models::status::energy_meter_status",
    },
    ColumnCheck {
        table: "energy_consumption_record",
        column: "status",
        constraint: "chk_energy_consumption_record_status",
        allowed: ENERGY_RECORD_STATUS_ALLOWED,
        vocab: "models::status::energy_record_status",
    },
    ColumnCheck {
        table: "energy_consumption_record",
        column: "recording_method",
        constraint: "chk_energy_consumption_record_recording_method",
        allowed: ENERGY_RECORDING_METHOD_ALLOWED,
        vocab: "models::status::energy_recording_method",
    },
    ColumnCheck {
        table: "energy_consumption_record",
        column: "meter_type",
        constraint: "chk_energy_consumption_record_meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
        vocab: "models::status::energy_type",
    },
    ColumnCheck {
        table: "energy_allocation_rule",
        column: "status",
        constraint: "chk_energy_allocation_rule_status",
        allowed: ENERGY_RULE_STATUS_ALLOWED,
        vocab: "models::status::energy_rule_status",
    },
    ColumnCheck {
        table: "energy_allocation_rule",
        column: "allocation_basis",
        constraint: "chk_energy_allocation_rule_allocation_basis",
        allowed: ENERGY_ALLOCATION_BASIS_ALLOWED,
        vocab: "models::status::energy_allocation_basis",
    },
    ColumnCheck {
        table: "energy_allocation_rule",
        column: "meter_type",
        constraint: "chk_energy_allocation_rule_meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
        vocab: "models::status::energy_type",
    },
    ColumnCheck {
        table: "energy_allocation_record",
        column: "status",
        constraint: "chk_energy_allocation_record_status",
        allowed: ENERGY_RECORD_STATUS_ALLOWED,
        vocab: "models::status::energy_record_status",
    },
    ColumnCheck {
        table: "energy_allocation_record",
        column: "allocation_basis",
        constraint: "chk_energy_allocation_record_allocation_basis",
        allowed: ENERGY_ALLOCATION_BASIS_ALLOWED,
        vocab: "models::status::energy_allocation_basis",
    },
    ColumnCheck {
        table: "energy_allocation_record",
        column: "meter_type",
        constraint: "chk_energy_allocation_record_meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
        vocab: "models::status::energy_type",
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut sql = String::new();

        // 1) 幂等前置，逐列删旧约束。
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\n",
                spec.table, spec.constraint
            ));
        }

        // 2) 逐列存量守卫，fail-visible，严禁 UPDATE 造值洗数据。
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE "{col}" IS NULL OR "{col}" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0103 守卫 {table}.{col} 无词表外存量行，可安全施加 CHECK {constraint}';
    ELSE
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO samples FROM (
            SELECT COALESCE('''' || "{col}" || '''', '(NULL)') || ' 乘 ' || COUNT(*)::text AS fmt
              FROM "{table}"
             WHERE "{col}" IS NULL OR "{col}" NOT IN ({allowed})
             GROUP BY "{col}"
        ) b;
        RAISE EXCEPTION 'm0103 守卫 {table}.{col} 存在 % 行词表外或 NULL 取值（逐值分布 %），拒绝施加 CHECK {constraint}', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值，请按权威词表 {vocab} 逐行核实更正存量，更正后重跑；脏值若来自旁路写入先修写入方，不要在迁移里圆场';
    END IF;
END
$$;
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                allowed = spec.allowed,
                vocab = spec.vocab,
            ));
        }

        // 3) 逐列施加 CHECK，取值集由 ALLOWED 常量注入 IN 占位，并固化列注释。
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                r#"
ALTER TABLE "{table}" ADD CONSTRAINT "{constraint}" CHECK ("{col}" IN ({allowed}));
COMMENT ON COLUMN "{table}"."{col}" IS '权威词表 {vocab}，取值集与 CHECK {constraint} 逐项相等（纸面锁 wage_energy_status_vocabulary_test 与活库锁 contract_wave16_wage_energy_vocab_check_test），NOT NULL 无 DEFAULT';
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                allowed = spec.allowed,
                vocab = spec.vocab,
            ));
        }

        // 4) 收尾回读自证，逐列读系统目录，任一项不满足即拒滚点名。
        let mut values_rows = String::new();
        for (idx, spec) in COLUMN_CHECKS.iter().enumerate() {
            if idx > 0 {
                values_rows.push_str(",");
            }
            values_rows.push_str(&format!(
                "('{}','{}','{}')",
                spec.table, spec.column, spec.constraint
            ));
        }
        sql.push_str(&format!(
            r#"
DO $$
DECLARE
    rec        RECORD;
    n_con      INTEGER;
    n_colchecks INTEGER;
    v_nullable TEXT;
    v_default  TEXT;
BEGIN
    FOR rec IN SELECT * FROM (VALUES {rows}) AS s(tbl, col, con) LOOP
        SELECT COUNT(*) INTO n_con FROM pg_constraint
          WHERE conrelid = rec.tbl::regclass AND conname = rec.con
            AND contype = 'c' AND convalidated;
        IF n_con <> 1 THEN
            RAISE EXCEPTION 'm0103 收尾自证失败，% 上约束 % 不存在或非 CHECK 或 convalidated 为假或同名重复，命中行数 %', rec.tbl, rec.con, n_con;
        END IF;

        SELECT COUNT(*) INTO n_colchecks FROM pg_constraint c
          WHERE c.conrelid = rec.tbl::regclass AND c.contype = 'c'
            AND pg_get_constraintdef(c.oid) LIKE ('%' || rec.col || '%')
            AND c.conname <> rec.con;
        IF n_colchecks <> 0 THEN
            RAISE EXCEPTION 'm0103 收尾自证失败，% 列 % 除本约束外另有 % 条涉及该列的 CHECK，同列多 CHECK 即第二套词表', rec.tbl, rec.col, n_colchecks;
        END IF;

        SELECT is_nullable, COALESCE(column_default, '<NONE>') INTO v_nullable, v_default
          FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = rec.tbl AND column_name = rec.col;
        IF v_nullable IS NULL THEN
            RAISE EXCEPTION 'm0103 收尾自证失败，information_schema 找不到 %.%，与本域建表漂移', rec.tbl, rec.col;
        END IF;
        IF v_nullable <> 'NO' THEN
            RAISE EXCEPTION 'm0103 收尾自证失败，%.% 的 is_nullable 为 %（期望 NO），NOT NULL 未真实生效', rec.tbl, rec.col, v_nullable;
        END IF;
        IF v_default <> '<NONE>' THEN
            RAISE EXCEPTION 'm0103 收尾自证失败，%.% 存在 column_default %（本批十二列约定无 DEFAULT，默认值收敛不属本迁移职责）', rec.tbl, rec.col, v_default;
        END IF;
    END LOOP;
    RAISE NOTICE 'm0103 收尾自证通过，十二列 CHECK 均 contype 为 c 且 convalidated 且同列唯一，且列 NOT NULL 且 column_default 为空';
END
$$;
"#,
            rows = values_rows,
        ));

        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除约束，不回退数据，删约束即回到可写状态，
        // 不复活旧越界值也不伪造历史行，口径同 export_inspection_vocab_check 的 down。
        let mut sql = String::new();
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\n",
                spec.table, spec.constraint
            ));
        }
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
