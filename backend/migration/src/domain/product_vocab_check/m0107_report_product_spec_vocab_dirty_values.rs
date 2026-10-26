//! 产品主档规格扩展列收口前存量取值只读点名（配套 m0108）
//!
//! 为什么单独存在一支只读迁移。m0106 为 products 与 product_colors 补了六列，其中五列
//! 由后续 m0108 施加 CHECK 收口。m0108 在施加前会逐列守卫拒绝越界残留，但那只是结果不是现场。
//! 本迁移把这五列在收口前的现状以 RAISE NOTICE 摊进迁移日志，使 CI 与人工能核对
//! m0108 的取值域口径是否覆盖真实分布，日志本身成为可追溯的点名报告。
//!
//! 口径来源，逐字符对齐，勿在本文件另立词表。色牢度九档取自 backend 侧
//! models::status::general::color_fastness::ALL_GRADES（GB 18401 定案含半级 1 与 1-2 与 2
//! 与 2-3 与 3 与 3-4 与 4 与 4-5 与 5）。安全类别三档取自 models::status::general::
//! product_safety_category::ALL_CLASSES（GB 18401-2010 的 A/B/C）。氨纶含量上限 100、克重公差非负、
//! 带针幅为正三条数值口径与本域 m0108 施加的 CHECK 表达式逐字相等，两支各携带同一份清单，
//! 由纸面锁钉彼此相等，禁止任一侧单独改动。
//!
//! 严格只读。up 仅 SELECT 聚合与 RAISE NOTICE，无 UPDATE、无 DELETE、无 DDL、不 RAISE
//! EXCEPTION，本步目的就是把现状摊给人看，抛错会中断迁移链与点名相悖。因此可任意重跑，
//! 无回滚面，down 是显式 no-op。
//!
//! 附带四列自由文本现状点名（产品类型与组织结构与后整理与用途）。这四列本批不收口，
//! 理由写在 m0108 文件头，它们的取值分布至今无人裁定，先约束即盲门。本迁移仍把
//! 分布摊出来，为后续是否收口提供现场，这一段同样只读不抛错。
//!
//! 域与注册位置。本域排 lib.rs 迁移链尾，晚于 product_spec_extension_columns 域（该域
//! 加这五列），也晚于 business 域（该域 m0008 建 products 与 product_colors 两表），
//! 两张目标表与本批新列届时必然存在。点名支必须先于收口支 m0108，注册在同域 up 链首位。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 色牢度九档 SQL 字面量清单，与 models::status::general::color_fastness::ALL_GRADES
/// 逐项逐字符一致。守卫、点名与本域 m0108 的 CHECK 共用同一份形态，禁止另写第二套。
const COLOR_FASTNESS_ALLOWED: &str = "'1','1-2','2','2-3','3','3-4','4','4-5','5'";

/// 安全类别三档 SQL 字面量清单，与 models::status::general::product_safety_category::ALL_CLASSES
/// 逐项逐字符一致（GB 18401-2010 的 A/B/C），与本域 m0108 携带同一份，禁止另写第二套。
const SAFETY_CATEGORY_ALLOWED: &str = "'A','B','C'";

/// 一列的收口配方。谓词模板里 {col} 换成本列名，{allowed} 换成本列词表清单。
/// NULL 一律视为未录入即合法，本批不给任何列加 NOT NULL，也不加 DEFAULT。
struct ColGuard<'a> {
    table: &'a str,
    column: &'a str,
    /// 合法形态谓词模板，必须自带对 NULL 的放行
    predicate: &'a str,
    /// 该列词表清单，数值列无词表时为空串
    allowed: &'a str,
    /// 展示用口径描述，写进 NOTICE，不带 SQL 引号
    domain_display: &'a str,
}

/// 五列收口清单，取值域与本域 m0108 的施加清单逐字相等。
/// 谓词一律写成 `"{col}" IS NULL OR ...` 形态（Postgres 的 IS NULL 是需左操作数的后缀判据，
/// 裸 `IS NULL` 会解析失败），与 m0108 的 ColumnCheck predicate 同形。
const COL_GUARDS: &[ColGuard] = &[
    ColGuard {
        table: "product_colors",
        column: "color_fastness_grade",
        predicate: "\"{col}\" IS NULL OR \"{col}\" IN ({allowed})",
        allowed: COLOR_FASTNESS_ALLOWED,
        domain_display: "色牢度等级 空值或 1 与 1-2 与 2 与 2-3 与 3 与 3-4 与 4 与 4-5 与 5",
    },
    ColGuard {
        table: "products",
        column: "spandex_percent",
        predicate: "\"{col}\" IS NULL OR (\"{col}\" >= 0 AND \"{col}\" <= 100)",
        allowed: "",
        domain_display: "氨纶含量百分比 空值或 0 到 100 闭区间",
    },
    ColGuard {
        table: "products",
        column: "weight_tolerance",
        predicate: "\"{col}\" IS NULL OR \"{col}\" >= 0",
        allowed: "",
        domain_display: "克重公差 空值或非负",
    },
    ColGuard {
        table: "products",
        column: "width_needle_included_cm",
        predicate: "\"{col}\" IS NULL OR \"{col}\" > 0",
        allowed: "",
        domain_display: "带针幅宽 空值或严格为正（单位 cm）",
    },
    ColGuard {
        table: "products",
        column: "safety_category",
        predicate: "\"{col}\" IS NULL OR \"{col}\" IN ({allowed})",
        allowed: SAFETY_CATEGORY_ALLOWED,
        domain_display: "安全类别 空值或 A 与 B 与 C（GB 18401-2010）",
    },
];

/// 本批不收口、仅摊出现场的自由文本列。
const INFO_COLS: &[(&str, &str)] = &[
    ("products", "product_type"),
    ("products", "structure"),
    ("products", "finish"),
    ("products", "usage"),
];

/// 把谓词模板落成本列可执行的 SQL 片段。
fn render_predicate(spec: &ColGuard) -> String {
    spec.predicate
        .replace("{col}", spec.column)
        .replace("{allowed}", spec.allowed)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut sql = String::new();
        for spec in COL_GUARDS {
            let legal = render_predicate(spec);
            // NOTICE 的展示串不带 SQL 单引号，词表清单只用于 IN，
            // 插进 RAISE 的字符串里会提前闭合字面量并让 DO 块解析失败。
            let block = format!(
                r#"
DO $$
DECLARE
    rec RECORD;
    total_rows INTEGER;
    bad_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO total_rows FROM "{table}";
    RAISE NOTICE 'm0107 点名 {table}.{col} 全表 % 行，收口口径 {domain}', total_rows;

    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE NOT ({legal});
    RAISE NOTICE 'm0107 点名 {table}.{col} 越界行合计 % 行，空值按未录入计为合法', bad_rows;

    FOR rec IN
        SELECT COALESCE("{col}"::text, '<未录入>') AS badval, COUNT(*) AS cnt
          FROM "{table}"
         WHERE NOT ({legal})
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0107 点名 {table}.{col} 越界值 % 行数 %，本值无定案映射，m0108 将拒绝施加 CHECK 交人工定性', rec.badval, rec.cnt;
    END LOOP;

    FOR rec IN
        SELECT "{col}"::text AS val, COUNT(*) AS cnt
          FROM "{table}"
         WHERE "{col}" IS NOT NULL
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0107 点名 {table}.{col} 已录入值 % 行数 %', rec.val, rec.cnt;
    END LOOP;
END
$$;
"#,
                table = spec.table,
                col = spec.column,
                legal = legal,
                domain = spec.domain_display,
            );
            sql.push_str(&block);
        }
        for (table, col) in INFO_COLS {
            let block = format!(
                r#"
DO $$
DECLARE
    rec RECORD;
BEGIN
    FOR rec IN
        SELECT COALESCE("{col}"::text, '<NULL>') AS val, COUNT(*) AS cnt
          FROM "{table}"
         GROUP BY 1
         ORDER BY COUNT(*) DESC, 1
    LOOP
        RAISE NOTICE 'm0107 现场 {table}.{col} 取值 % 行数 %，本批不收口，分布摊出供后续裁定', rec.val, rec.cnt;
    END LOOP;
END
$$;
"#,
                table = table,
                col = col,
            );
            sql.push_str(&block);
        }
        sql.push_str(
            "DO $$ BEGIN RAISE NOTICE 'm0107 点名完成，五列收口口径与四列自由文本现状已全部写入迁移日志，本迁移未做任何写入'; END $$;",
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 显式 no-op 且这是正确形态非偷懒。up 是纯只读迁移，只有 SELECT 聚合与
        // RAISE NOTICE，不含任何 UPDATE、DELETE、DDL，对库内数据与结构零改变，
        // 因此不存在任何需要回滚的副作用。迁移日志里的逐值快照是历史事实，
        // 由台账与日志系统本身保留，down 无事可做也不应抹除任何痕迹。
        Ok(())
    }
}
