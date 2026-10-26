//! 纺织品安全合规域取值列词表 CHECK 收口（配套 m0109 建表与 m0111 播种）
//!
//! 语义定案：本域全部取值型列的权威词表唯一出现在 backend 侧 models::status::
//! textile_safety 下各模块，取值集合逐列取自身词表，不做跨列并集；本迁移把取值域钉进
//! 数据库，阻止任何绕过服务层入参校验的旁路写入产生词表外脏值。
//!
//! 步骤顺序不可倒，每步存在理由如下。
//! 1. 先逐列 DROP CONSTRAINT IF EXISTS，重跑或半态库不报错，幂等前置。
//! 2. 再逐列存量守卫，fail-visible，词表外残留（NOT NULL 列还含 NULL 行）大于 0 即
//!    RAISE EXCEPTION，消息带表名、列名、行数与逐值分布，HINT 指向权威词表模块。
//!    新库表恒空守卫天然通过；复用库上若 m0109 与 m0110 之间有旁路写入，本守卫照样拦。
//!    本迁移不 UPDATE 造值洗数据，脏行交业务逐行定性后重跑。
//! 3. 才允许逐列 ADD CONSTRAINT，取值集由本文件 ALLOWED 常量单源注入 IN 占位，
//!    不在 ADD CONSTRAINT 里内联第二套清单；可空列 CHECK 形态为（col IS NULL OR col IN
//!    (...)），NULL 是合法初值（建单前置态），空串不是。约束名统一 chk_表_列。随后
//!    COMMENT ON COLUMN 固化词表指向。
//! 4. 收尾回读自证，逐列读 pg_constraint，任一不满足即 RAISE EXCEPTION 点名。约束必须
//!    contype 为 c 且 convalidated 为真且同名恰一条（NOT VALID 会放行存量脏行不算收口）；
//!    同列除本约束外不得存在另一条引用该列的 CHECK（同列多 CHECK 即第二套词表漂移，
//!    m0109 的结构校验 CHECK 只引用数值列与日期列，不与本批词表列重叠）。
//!
//! 豁免标签逗号串列 exemption_tags 与缺项代号逗号串列 rejection_detail 不在本批：逗号串
//! 列无法用 IN 闭合取值域，逐 token 白名单校验在送检服务与报告服务写入侧执行，列注释已言明。
//!
//! 可重入性：DROP 前置使重跑安全，守卫对已达词表内现态恒真，ADD CONSTRAINT 前已 DROP
//! 旧约束，自证对已达形态恒真。
//!
//! down 真实可逆：逐列 DROP CONSTRAINT IF EXISTS 带回滚探测，不回退数据，不复活旧越界值
//! 也不伪造历史行，口径同 wage_energy_vocab_check 的 down。
//!
//! 域与注册位置：本域排 lib.rs 迁移链尾，晚于 system 建 products；注册在同域 up 链
//! m0109 之后、m0111 播种之前，种子行天然受已生效 CHECK 校验。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 以下 SQL 字面量清单与 models::status::textile_safety 下各词表模块逐项逐字符一致，
/// 由纸面锁 textile_safety_status_vocabulary_test 钉相等，守卫、CHECK、列注释三处共用
/// 同一份，禁止另写第二套。
const ITEM_KIND_ALLOWED: &str = "'numeric','grade','sensory','boolean'";
const BASIS_STANDARD_ALLOWED: &str = "'GB18401','GB31701','EXPORT','CUSTOMER'";
const CHECK_KIND_ALLOWED: &str = "'measurable','manual'";
const LIMIT_CATEGORY_ALLOWED: &str = "'GB18401_A','GB18401_B','GB18401_C','any'";
const LIMIT_FORM_ALLOWED: &str = "'final','intermediate','any'";
const LIMIT_TAG_ALLOWED: &str =
    "'none','wet_processing','laundering_fade_non_final','plain_or_bleached','traditional_hand_dyed'";
const COMPARATOR_ALLOWED: &str = "'lte','gte','between','forbidden','must_none','na'";
const END_USER_GROUP_ALLOWED: &str = "'infant','child','adult'";
const CONTACT_TYPE_ALLOWED: &str = "'direct','indirect','infant_use'";
const SUB_CATEGORY_ALLOWED: &str = "'GB18401_A','GB18401_B','GB18401_C'";
const SUB_FORM_ALLOWED: &str = "'final','intermediate'";
const TARGET_MARKET_ALLOWED: &str = "'dom','eu','us','jp','other'";
const COLOR_ROLE_ALLOWED: &str = "'representative','customer_specified','full'";
const SUBMISSION_STATUS_ALLOWED: &str =
    "'draft','sampling','sent','reported','judged','closed','withdrawn'";
const OVERALL_VERDICT_ALLOWED: &str = "'pass','fail','na'";
const ITEM_SOURCE_ALLOWED: &str = "'GB18401','GB31701','EXPORT','CUSTOMER'";
const ACCEPTANCE_STATUS_ALLOWED: &str = "'pending','accepted','rejected'";
const OEKO_CLASS_ALLOWED: &str = "'I','II','III','IV'";
const RESULT_VERDICT_ALLOWED: &str = "'pass','fail','na','pending'";
const MANUAL_CONCLUSION_ALLOWED: &str = "'pass','fail','na'";
const REQ_MARKET_ALLOWED: &str = "'eu','us','jp','other'";
const REQ_TYPE_ALLOWED: &str =
    "'oeko_tex','reach_svhc','cpsia_lead','cpsia_phthalate','cpsia_cert','jp_formaldehyde'";

/// 一列的收口配方：表、列、约束名、该列自身词表、词表模块符号名、可空性
/// （nullable 为 true 时 NULL 是合法初值，守卫与 CHECK 都放行 NULL）。
struct ColumnCheck<'a> {
    table: &'a str,
    column: &'a str,
    constraint: &'a str,
    allowed: &'a str,
    vocab: &'a str,
    nullable: bool,
}

/// 26 列逐列配方，取值集合按各列自身词表，不做跨列并集。
const COLUMN_CHECKS: &[ColumnCheck] = &[
    ColumnCheck {
        table: "textile_test_item",
        column: "item_kind",
        constraint: "chk_textile_test_item_item_kind",
        allowed: ITEM_KIND_ALLOWED,
        vocab: "models::status::textile_item_kind",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_test_item",
        column: "basis_standard",
        constraint: "chk_textile_test_item_basis_standard",
        allowed: BASIS_STANDARD_ALLOWED,
        vocab: "models::status::textile_basis_standard",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_test_item",
        column: "check_kind",
        constraint: "chk_textile_test_item_check_kind",
        allowed: CHECK_KIND_ALLOWED,
        vocab: "models::status::textile_check_kind",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_limit_rule",
        column: "safety_category",
        constraint: "chk_textile_limit_rule_safety_category",
        allowed: LIMIT_CATEGORY_ALLOWED,
        vocab: "models::status::textile_limit_scope::CATEGORIES",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_limit_rule",
        column: "product_form",
        constraint: "chk_textile_limit_rule_product_form",
        allowed: LIMIT_FORM_ALLOWED,
        vocab: "models::status::textile_limit_scope::FORMS",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_limit_rule",
        column: "condition_tag",
        constraint: "chk_textile_limit_rule_condition_tag",
        allowed: LIMIT_TAG_ALLOWED,
        vocab: "models::status::textile_limit_scope::CONDITION_TAGS",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_limit_rule",
        column: "comparator",
        constraint: "chk_textile_limit_rule_comparator",
        allowed: COMPARATOR_ALLOWED,
        vocab: "models::status::textile_comparator",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "end_user_group",
        constraint: "chk_textile_safety_submission_end_user_group",
        allowed: END_USER_GROUP_ALLOWED,
        vocab: "models::status::textile_end_user_group",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "contact_type",
        constraint: "chk_textile_safety_submission_contact_type",
        allowed: CONTACT_TYPE_ALLOWED,
        vocab: "models::status::textile_contact_type",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "safety_category",
        constraint: "chk_textile_safety_submission_safety_category",
        allowed: SUB_CATEGORY_ALLOWED,
        vocab: "models::status::textile_safety_category",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "product_form",
        constraint: "chk_textile_safety_submission_product_form",
        allowed: SUB_FORM_ALLOWED,
        vocab: "models::status::textile_product_form",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "target_market",
        constraint: "chk_textile_safety_submission_target_market",
        allowed: TARGET_MARKET_ALLOWED,
        vocab: "models::status::export_market",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "color_role",
        constraint: "chk_textile_safety_submission_color_role",
        allowed: COLOR_ROLE_ALLOWED,
        vocab: "models::status::sampling_color_role",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "status",
        constraint: "chk_textile_safety_submission_status",
        allowed: SUBMISSION_STATUS_ALLOWED,
        vocab: "models::status::submission_status",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_safety_submission",
        column: "overall_verdict",
        constraint: "chk_textile_safety_submission_overall_verdict",
        allowed: OVERALL_VERDICT_ALLOWED,
        vocab: "models::status::textile_verdict::OVERALL",
        nullable: true,
    },
    ColumnCheck {
        table: "textile_safety_submission_item",
        column: "source",
        constraint: "chk_textile_safety_submission_item_source",
        allowed: ITEM_SOURCE_ALLOWED,
        vocab: "models::status::textile_item_source",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_third_party_report",
        column: "acceptance_status",
        constraint: "chk_textile_third_party_report_acceptance_status",
        allowed: ACCEPTANCE_STATUS_ALLOWED,
        vocab: "models::status::report_acceptance_status",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_third_party_report",
        column: "oeko_tex_class",
        constraint: "chk_textile_third_party_report_oeko_tex_class",
        allowed: OEKO_CLASS_ALLOWED,
        vocab: "models::status::oeko_tex_class",
        nullable: true,
    },
    ColumnCheck {
        table: "textile_report_item_result",
        column: "verdict",
        constraint: "chk_textile_report_item_result_verdict",
        allowed: RESULT_VERDICT_ALLOWED,
        vocab: "models::status::textile_verdict",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_child_safety_check",
        column: "check_kind",
        constraint: "chk_textile_child_safety_check_check_kind",
        allowed: CHECK_KIND_ALLOWED,
        vocab: "models::status::textile_check_kind",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_child_safety_check",
        column: "manual_conclusion",
        constraint: "chk_textile_child_safety_check_manual_conclusion",
        allowed: MANUAL_CONCLUSION_ALLOWED,
        vocab: "models::status::textile_verdict::OVERALL",
        nullable: true,
    },
    ColumnCheck {
        table: "textile_child_safety_check",
        column: "verdict",
        constraint: "chk_textile_child_safety_check_verdict",
        allowed: RESULT_VERDICT_ALLOWED,
        vocab: "models::status::textile_verdict",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_export_market_requirement",
        column: "market",
        constraint: "chk_textile_export_market_requirement_market",
        allowed: REQ_MARKET_ALLOWED,
        vocab: "models::status::export_market 的出口四值子集（内销 dom 不挂出口要求）",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_export_market_requirement",
        column: "requirement_type",
        constraint: "chk_textile_export_market_requirement_requirement_type",
        allowed: REQ_TYPE_ALLOWED,
        vocab: "models::status::export_requirement_type",
        nullable: false,
    },
    ColumnCheck {
        table: "textile_export_market_requirement",
        column: "oeko_tex_class",
        constraint: "chk_textile_export_market_requirement_oeko_tex_class",
        allowed: OEKO_CLASS_ALLOWED,
        vocab: "models::status::oeko_tex_class",
        nullable: true,
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
            let predicate = if spec.nullable {
                format!("\"{col}\" IS NOT NULL AND \"{col}\" NOT IN ({allowed})", col = spec.column, allowed = spec.allowed)
            } else {
                format!("\"{col}\" IS NULL OR \"{col}\" NOT IN ({allowed})", col = spec.column, allowed = spec.allowed)
            };
            let null_note = if spec.nullable { "词表外" } else { "词表外或 NULL" };
            sql.push_str(&format!(
                r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE {predicate};
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0110 守卫 {table}.{col} 无{note}存量行，可安全施加 CHECK {constraint}';
    ELSE
        SELECT string_agg(fmt, ' / ' ORDER BY fmt) INTO samples FROM (
            SELECT COALESCE('''' || "{col}" || '''', '(NULL)') || ' 乘 ' || COUNT(*)::text AS fmt
              FROM "{table}"
             WHERE {predicate}
             GROUP BY "{col}"
        ) b;
        RAISE EXCEPTION 'm0110 守卫 {table}.{col} 存在 % 行{note}取值（逐值分布 %），拒绝施加 CHECK {constraint}', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值，请按权威词表 {vocab} 逐行核实更正存量，更正后重跑；脏值若来自旁路写入先修写入方，不要在迁移里圆场';
    END IF;
END
$$;
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                predicate = predicate,
                note = null_note,
                allowed = spec.allowed,
                vocab = spec.vocab,
            ));
        }

        // 3) 逐列施加 CHECK，取值集由 ALLOWED 常量注入 IN 占位，并固化列注释。
        for spec in COLUMN_CHECKS {
            let expr = if spec.nullable {
                format!("\"{col}\" IS NULL OR \"{col}\" IN ({allowed})", col = spec.column, allowed = spec.allowed)
            } else {
                format!("\"{col}\" IN ({allowed})", col = spec.column, allowed = spec.allowed)
            };
            let null_note = if spec.nullable {
                "NULL 为合法初值"
            } else {
                "NOT NULL"
            };
            sql.push_str(&format!(
                "ALTER TABLE \"{table}\" ADD CONSTRAINT \"{constraint}\" CHECK ({expr});\nCOMMENT ON COLUMN \"{table}\".\"{col}\" IS '权威词表 {vocab}，取值集与 CHECK {constraint} 逐项相等（纸面锁 textile_safety_status_vocabulary_test），{note}';\n",
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                expr = expr,
                vocab = spec.vocab,
                note = null_note,
            ));
        }

        // 4) 收尾回读自证，逐列读系统目录，任一项不满足即拒滚点名。
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
    n_colchecks INTEGER;
BEGIN
    FOR rec IN SELECT * FROM (VALUES {rows}) AS s(tbl, col, con) LOOP
        SELECT COUNT(*) INTO n_con FROM pg_constraint
          WHERE conrelid = rec.tbl::regclass AND conname = rec.con
            AND contype = 'c' AND convalidated;
        IF n_con <> 1 THEN
            RAISE EXCEPTION 'm0110 收尾自证失败，% 上约束 % 不存在或非 CHECK 或 convalidated 为假或同名重复，命中行数 %', rec.tbl, rec.con, n_con;
        END IF;

        SELECT COUNT(*) INTO n_colchecks FROM pg_constraint c
          WHERE c.conrelid = rec.tbl::regclass AND c.contype = 'c'
            AND pg_get_constraintdef(c.oid) LIKE ('%' || rec.col || '%')
            AND c.conname <> rec.con;
        IF n_colchecks <> 0 THEN
            RAISE EXCEPTION 'm0110 收尾自证失败，% 列 % 除本约束外另有 % 条涉及该列的 CHECK，同列多 CHECK 即第二套词表', rec.tbl, rec.col, n_colchecks;
        END IF;
    END LOOP;
    RAISE NOTICE 'm0110 收尾自证通过，26 列 CHECK 均 contype 为 c 且 convalidated 且同列唯一';
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
        // 不复活旧越界值也不伪造历史行，口径同 wage_energy_vocab_check 的 down。
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
