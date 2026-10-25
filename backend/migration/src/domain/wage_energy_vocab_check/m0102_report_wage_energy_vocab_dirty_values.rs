//! 工资与能耗状态列存量取值只读点名（收口前置报告，配套 m0103）
//!
//! 为什么单独存在一支只读迁移：process_wage_rate、wage_record、energy_meter、
//! energy_consumption_record、energy_allocation_rule、energy_allocation_record 六张表的十二个
//! 取值列建表原文均为 VARCHAR(255) NOT NULL 且无 CHECK，词表唯一防护历史上只落在应用层，
//! 脏值分布完全未知。m0103 在施加 CHECK 前会逐列守卫拒绝词表外残留，但那只是结果不是现场。
//! 本迁移把十二列在收口前的逐值现状以 RAISE NOTICE 摊进迁移日志，使 CI 与人工能核对
//! 词表口径是否覆盖真实分布，日志本身成为可追溯的点名报告。
//!
//! 口径来源，逐字符对齐，勿在本文件另立词表。八份权威词表取自
//! backend 侧 models::status::wage_energy_chemical_business 下的
//! wage_rate_status、wage_record_status、energy_meter_status、energy_record_status、
//! energy_recording_method、energy_type、energy_rule_status、energy_allocation_basis。
//! 每列取其自身词表，不做跨列并集，取值集合与本域 m0103 的收口清单逐字相等，由纸面锁
//! wage_energy_status_vocabulary_test 与活库锁 contract_wave16_wage_energy_vocab_check_test 锁定。
//!
//! 严格只读：up 仅 SELECT 聚合与 RAISE NOTICE，无 UPDATE、无 DELETE、无 DDL、不 RAISE
//! EXCEPTION，本步目的就是把现状摊给人看，抛错会中断迁移链与点名相悖。因此可任意重跑，
//! 无回滚面，down 是显式 no-op。
//!
//! 词表值命中计数用 unnest 与 LEFT JOIN，词表内每个 token 都出一条 NOTICE，命中 0 行也报，
//! 报 0 本身就是该值在此库不存在的证据。词表外与 NULL 另起一段逐值点名，为 m0103 的守卫拒绝留现场。
//!
//! 域与注册位置：本域排在 lib.rs 迁移链尾，晚于 migration/src/domain/v15 建表域，
//! 六张目标表届时必然存在。点名支必须先于收口支 m0103，注册在同域 up 链首位。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 以下八份 SQL 字面量清单与 models::status 下八个词表模块逐项逐字符一致。
/// 守卫、CHECK、点名三处共用同一份，禁止另写第二套。
const WAGE_RATE_STATUS_ALLOWED: &str = "'draft','active','disabled'";
const WAGE_RECORD_STATUS_ALLOWED: &str = "'draft','confirmed','paid','cancelled'";
const ENERGY_METER_STATUS_ALLOWED: &str = "'active','inactive','maintenance'";
const ENERGY_RECORD_STATUS_ALLOWED: &str = "'draft','confirmed','cancelled'";
const ENERGY_RECORDING_METHOD_ALLOWED: &str = "'manual','iot','auto_calc'";
const ENERGY_TYPE_ALLOWED: &str = "'water','electricity','steam','gas','compressed_air'";
const ENERGY_RULE_STATUS_ALLOWED: &str = "'draft','active','disabled'";
const ENERGY_ALLOCATION_BASIS_ALLOWED: &str =
    "'by_duration','by_output','by_equipment','by_workshop'";

/// 一列的点名配方，表、列、该列自身词表。
struct ColReport<'a> {
    table: &'a str,
    column: &'a str,
    allowed: &'a str,
}

/// 十二列逐列配方，取值集合按各列自身词表，不做跨列并集。
const COL_REPORTS: &[ColReport] = &[
    ColReport {
        table: "process_wage_rate",
        column: "status",
        allowed: WAGE_RATE_STATUS_ALLOWED,
    },
    ColReport {
        table: "wage_record",
        column: "status",
        allowed: WAGE_RECORD_STATUS_ALLOWED,
    },
    ColReport {
        table: "energy_meter",
        column: "status",
        allowed: ENERGY_METER_STATUS_ALLOWED,
    },
    ColReport {
        table: "energy_consumption_record",
        column: "status",
        allowed: ENERGY_RECORD_STATUS_ALLOWED,
    },
    ColReport {
        table: "energy_consumption_record",
        column: "recording_method",
        allowed: ENERGY_RECORDING_METHOD_ALLOWED,
    },
    ColReport {
        table: "energy_consumption_record",
        column: "meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_rule",
        column: "status",
        allowed: ENERGY_RULE_STATUS_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_rule",
        column: "allocation_basis",
        allowed: ENERGY_ALLOCATION_BASIS_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_rule",
        column: "meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_record",
        column: "status",
        allowed: ENERGY_RECORD_STATUS_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_record",
        column: "allocation_basis",
        allowed: ENERGY_ALLOCATION_BASIS_ALLOWED,
    },
    ColReport {
        table: "energy_allocation_record",
        column: "meter_type",
        allowed: ENERGY_TYPE_ALLOWED,
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut sql = String::new();
        for spec in COL_REPORTS {
            // 每列一个只读 DO 块。三段点名全走 SELECT 聚合与 RAISE NOTICE。
            let block = format!(
                r#"
DO $$
DECLARE
    rec RECORD;
    total_rows INTEGER;
    bad_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "{table}";
    RAISE NOTICE 'm0102 点名 {table}.{col} 全表 % 行，词表清单 {allowed}，以下逐值分账', total_rows;

    FOR rec IN
        SELECT vocab.val AS val, COUNT(matched."{col}") AS cnt
          FROM (SELECT unnest(ARRAY[{allowed}]) AS val) vocab
          LEFT JOIN "{table}" matched ON matched."{col}" = vocab.val
         GROUP BY vocab.val
         ORDER BY vocab.val
    LOOP
        RAISE NOTICE 'm0102 点名 {table}.{col} 词表值 % 命中 % 行', rec.val, rec.cnt;
    END LOOP;

    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE "{col}" IS NULL OR "{col}" NOT IN ({allowed});
    RAISE NOTICE 'm0102 点名 {table}.{col} 词表外或 NULL 合计 % 行', bad_rows;

    FOR rec IN
        SELECT COALESCE("{col}", '<NULL>') AS badval, COUNT(*) AS cnt
          FROM "{table}"
         WHERE "{col}" IS NULL OR "{col}" NOT IN ({allowed})
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0102 点名 {table}.{col} 词表外值 % 行数 %，本值无定案映射，m0103 将拒绝施加 CHECK 交人工定性', rec.badval, rec.cnt;
    END LOOP;
END
$$;
"#,
                table = spec.table,
                col = spec.column,
                allowed = spec.allowed,
            );
            sql.push_str(&block);
        }
        sql.push_str(
            r#"DO $$ BEGIN RAISE NOTICE 'm0102 点名完成，十二列现状已全部写入迁移日志，本迁移未做任何写入'; END $$;"#,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 显式 no-op 且这是正确形态非偷懒：up 是纯只读迁移，只有 SELECT 聚合与
        // RAISE NOTICE，不含任何 UPDATE、DELETE、DDL，对库内数据与结构零改变，
        // 因此不存在任何需要回滚的副作用。迁移日志里的逐值快照是历史事实，
        // 由台账与日志系统本身保留，down 无事可做也不应抹除任何痕迹。
        Ok(())
    }
}
