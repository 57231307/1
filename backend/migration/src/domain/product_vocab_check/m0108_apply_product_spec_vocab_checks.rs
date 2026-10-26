//! 产品主档规格扩展列取值域收口，施加 CHECK 与收尾自证（配套 m0107 只读点名）
//!
//! 语义定案。本批四列的权威词表唯一出现在 backend 侧 models::status::general::
//! color_fastness（色牢度九档，GB 18401 定案含半级），数值三列的边界口径由本文件与 m0107
//! 各携带同一份。色牢度取值形态 1 与 1-2 与 2 与 2-3 与 3 与 3-4 与 4 与 4-5 与 5，
//! 半级是标准内的合法档位不是脏值，收口必须一并放行。
//!
//! 本批不收口的列与理由，组织结构 structure 与后整理 finish 与用途 usage 三列是既有自由文本列
//! （usage 为 m0106 新列但列注释定案为自由文本），产品类型 product_type 的取值域至今只落在
//! 应用层入参校验。这四列的存量分布无人裁定，先约束即盲门，任何越界拒绝都会把整条迁移链
//! 与全库初始化一起打断。它们的分布由同域 m0107 摊进日志，是否收口交后续批号决定。
//! 耐唾液项目只对 A 类考核、本色漂白与扎染蜡染等传统手工着色产品不考核色牢度，这两条是
//! 跨表与安全类别联动的条件判据，CHECK 表达不了，留在应用层与检验判定侧，不在本批伪装成
//! 数据库约束。
//!
//! 步骤顺序不可倒，每步存在理由如下。
//! 1. 先逐列 DROP CONSTRAINT IF EXISTS，重跑或半态库不报错，幂等前置。
//! 2. 再逐列存量守卫，fail-visible，非空越界残留大于 0 即 RAISE EXCEPTION，消息带表名、
//!    列名、行数与逐值分布，HINT 指向权威词表文件。本迁移不 UPDATE 造值洗数据，脏行交
//!    业务逐行定性后重跑，禁止静默加约束后把脏行留在库里。
//! 3. 才允许逐列 ADD CONSTRAINT，取值集由本文件常量单源注入，约束名统一 chk_表_列，
//!    最长 chk_products_width_needle_included_cm 三十八字符，在 PG 六十三上限内。
//!    随后 COMMENT ON COLUMN 固化词表与口径指向。
//! 4. 收尾回读自证，逐列读 pg_constraint 与 information_schema，任一项不满足即 RAISE
//!    EXCEPTION 点名。约束必须 contype 为 c 且 convalidated 为真且同名恰一条，
//!    NOT VALID 会放行存量脏行不算收口。列必须 is_nullable 为 YES 且 column_default 为空，
//!    本批四列全部保持可空无默认，NULL 语义为未录入，钉这两项是定案的回读印证。
//!
//! 不加 DEFAULT 的理由。给可空数值列硬造 0 默认会把未录入显示成 0，与「不以 0 兜底掩盖缺值」
//! 口径冲突，且氨纶含量 0 与克重公差 0 都是真实业务值，用作默认会与合法取值混淆。
//!
//! 事务与失败保护。sea-orm-migration 把每支 up 包在单事务内，PG 的 DDL 具事务性，
//! 守卫或自证 RAISE EXCEPTION 时整条 up 自动回滚，不留半态。
//!
//! 可重入性。DROP 前置使重跑安全，守卫对已达口径现态恒真，ADD CONSTRAINT 前已 DROP 旧约束，
//! 自证对已达形态恒真。
//!
//! down 真实可逆，逐列 DROP CONSTRAINT IF EXISTS 带回滚探测，不回退数据。删约束即回到
//! 可写形态，不复活旧越界值也不伪造历史行。
//!
//! 域与注册位置。本域排 lib.rs 迁移链尾，晚于 product_spec_extension_columns 域与本域 m0107，
//! 目标列与点名现场届时必然存在。注册在同域 up 链 m0107 之后，先点名后收口，
//! 本支失败时日志已有 m0107 现场可比照。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 色牢度九档 SQL 字面量清单，与 models::status::general::color_fastness::ALL_GRADES
/// 逐项逐字符一致，与同域 m0107 携带同一份，由纸面锁钉两支迁移逐字相等。
const COLOR_FASTNESS_ALLOWED: &str = "'1','1-2','2','2-3','3','3-4','4','4-5','5'";

/// 一列的收口配方。谓词模板里 {col} 换成本列名，{allowed} 换成本列词表清单。
struct ColumnCheck<'a> {
    table: &'a str,
    column: &'a str,
    constraint: &'a str,
    /// CHECK 表达式模板，NULL 一律放行（未录入合法）
    predicate: &'a str,
    /// 该列词表清单，数值列无词表时为空串
    allowed: &'a str,
    /// 口径出处符号名，写进列注释与 HINT 指向
    vocab: &'a str,
}

/// 四列收口清单，取值域与同域 m0107 的点名清单逐字相等。
const COLUMN_CHECKS: &[ColumnCheck] = &[
    ColumnCheck {
        table: "product_colors",
        column: "color_fastness_grade",
        constraint: "chk_product_colors_color_fastness_grade",
        predicate: "\"{col}\" IS NULL OR \"{col}\" IN ({allowed})",
        allowed: COLOR_FASTNESS_ALLOWED,
        vocab: "models::status::general::color_fastness::ALL_GRADES",
    },
    ColumnCheck {
        table: "products",
        column: "spandex_percent",
        constraint: "chk_products_spandex_percent",
        predicate: "\"{col}\" IS NULL OR (\"{col}\" >= 0 AND \"{col}\" <= 100)",
        allowed: "",
        vocab: "产品氨纶含量百分比 0 到 100 闭区间",
    },
    ColumnCheck {
        table: "products",
        column: "weight_tolerance",
        constraint: "chk_products_weight_tolerance",
        predicate: "\"{col}\" IS NULL OR \"{col}\" >= 0",
        allowed: "",
        vocab: "产品克重公差非负（目标值存 gram_weight，本列记允差幅度）",
    },
    ColumnCheck {
        table: "products",
        column: "width_needle_included_cm",
        constraint: "chk_products_width_needle_included_cm",
        predicate: "\"{col}\" IS NULL OR \"{col}\" > 0",
        allowed: "",
        vocab: "产品带针幅宽严格为正（单位 cm，含针边，不参与克重换算）",
    },
];

/// 把谓词模板落成本列可执行的 SQL 片段。
fn render_predicate(spec: &ColumnCheck) -> String {
    spec.predicate
        .replace("{col}", spec.column)
        .replace("{allowed}", spec.allowed)
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut sql = String::new();

        // 1. 幂等前置，逐列删旧约束。
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\n",
                spec.table, spec.constraint
            ));
        }

        // 2. 逐列存量守卫，fail-visible，严禁 UPDATE 造值洗数据。空值合法故只数非空越界行。
        for spec in COLUMN_CHECKS {
            let legal = render_predicate(spec);
            sql.push_str(&format!(
                r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE NOT ({legal});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0108 守卫 {table}.{col} 无非空越界存量行，可安全施加 CHECK {constraint}';
    ELSE
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO samples FROM (
            SELECT COALESCE('''' || "{col}" || '''', '(NULL)') || ' 乘 ' || COUNT(*)::text AS fmt
              FROM "{table}"
             WHERE NOT ({legal})
             GROUP BY "{col}"
        ) b;
        RAISE EXCEPTION 'm0108 守卫 {table}.{col} 存在 % 行越界取值（逐值分布 %），拒绝施加 CHECK {constraint}', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值，请按权威口径 {vocab} 逐行核实更正存量，更正后重跑；脏值若来自旁路写入先修写入方，不要在迁移里圆场';
    END IF;
END
$$;
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                legal = legal,
                vocab = spec.vocab,
            ));
        }

        // 3. 逐列施加 CHECK，取值集由本文件常量单源注入，并固化列注释。
        for spec in COLUMN_CHECKS {
            let legal = render_predicate(spec);
            sql.push_str(&format!(
                r#"
ALTER TABLE "{table}" ADD CONSTRAINT "{constraint}" CHECK ({legal});
COMMENT ON COLUMN "{table}"."{col}" IS '权威口径 {vocab}，取值域与 CHECK {constraint} 逐项相等，可空无默认，NULL 语义为未录入';
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                legal = legal,
                vocab = spec.vocab,
            ));
        }

        // 4. 收尾回读自证，逐列读系统目录，任一项不满足即拒滚点名。
        let mut values_rows = String::new();
        for (idx, spec) in COLUMN_CHECKS.iter().enumerate() {
            if idx > 0 {
                values_rows.push(',');
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
    rec         RECORD;
    n_con       INTEGER;
    v_contype   "char";
    v_validated BOOLEAN;
    v_nullable  TEXT;
    v_default   TEXT;
BEGIN
    FOR rec IN SELECT * FROM (VALUES {rows}) AS t(tbl, col, con) LOOP
        SELECT COUNT(*) INTO n_con FROM pg_constraint c
          JOIN pg_class r ON r.oid = c.conrelid
         WHERE r.relname = rec.tbl AND c.conname = rec.con;
        IF n_con <> 1 THEN
            RAISE EXCEPTION 'm0108 自证 %.% 的约束 % 同名条数为 %，要求恰一条', rec.tbl, rec.col, rec.con, n_con;
        END IF;

        SELECT c.contype, c.convalidated INTO v_contype, v_validated FROM pg_constraint c
          JOIN pg_class r ON r.oid = c.conrelid
         WHERE r.relname = rec.tbl AND c.conname = rec.con;
        IF v_contype <> 'c' THEN
            RAISE EXCEPTION 'm0108 自证 %.% 的约束 % contype 为 %，要求 c 即 CHECK 约束', rec.tbl, rec.col, rec.con, v_contype;
        END IF;
        IF v_validated IS NOT TRUE THEN
            RAISE EXCEPTION 'm0108 自证 %.% 的约束 % 未 convalidated，NOT VALID 会放行存量脏行不算收口', rec.tbl, rec.col, rec.con;
        END IF;

        SELECT is_nullable, column_default INTO v_nullable, v_default FROM information_schema.columns
         WHERE table_name = rec.tbl AND column_name = rec.col;
        IF v_nullable IS DISTINCT FROM 'YES' THEN
            RAISE EXCEPTION 'm0108 自证 %.% 的 is_nullable 为 %，本批定案保持可空（NULL 即未录入）', rec.tbl, rec.col, v_nullable;
        END IF;
        IF v_default IS NOT NULL THEN
            RAISE EXCEPTION 'm0108 自证 %.% 带 column_default %，本批不施加任何默认值，硬造默认会把未录入显示成实值', rec.tbl, rec.col, v_default;
        END IF;

        RAISE NOTICE 'm0108 自证 %.% 约束 % 已达形态，contype 为 c 且 convalidated 为真且可空无默认', rec.tbl, rec.col, rec.con;
    END LOOP;
    RAISE NOTICE 'm0108 收口完成，四列取值域已全部钉进数据库';
END
$$;
"#,
            rows = values_rows,
        ));

        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逐列删约束即回到可写形态，不回退任何数据，不复活旧越界值也不伪造历史行。
        let mut sql = String::new();
        for spec in COLUMN_CHECKS {
            sql.push_str(&format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\n",
                spec.table, spec.constraint
            ));
        }
        sql.push_str("DO $$ BEGIN RAISE NOTICE 'm0108 回滚完成，四列 CHECK 已撤除'; END $$;");
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
