//! 纺织品安全合规域八张表建表迁移
//!
//! ## 六要素
//! - **功能**：创建纺织品安全合规子域八张表——检测项目主数据 `textile_test_item`、
//!   限值判定表 `textile_limit_rule`、送检单 `textile_safety_submission`、送检项目清单
//!   `textile_safety_submission_item`、第三方报告 `textile_third_party_report`、逐项目实测
//!   结果 `textile_report_item_result`、童装加考核查 `textile_child_safety_check`、出口市场
//!   要求登记 `textile_export_market_requirement`。
//! - **调用方**：SeaORM migrator 链，经 `domain/textile_safety` 域注册为顶层链尾条目，
//!   本支在域内首位执行；晚于 system 域建 products，送检单外键届时可引用。
//! - **入参**：无（纯 DDL 迁移）。
//! - **传给谁**：无应用层直调；表由 textile_safety 域的 handler 与 service 按实体映射读写。
//! - **存什么**：见各表列注释；全部取值型列的 CHECK 收口在配套 m0111，不在本迁移内联
//!   词表字面量，避免同一字典在两处漂移。
//! - **存哪里**：PostgreSQL public 模式八张表。
//!
//! ## 形态说明
//! - 本域为新建顶层域，与既有七张检验表（quality_inspection_records、fabric_inspection_record、
//!   fabric_physical_test_record、purchase_inspection、export_inspection、unqualified_products、
//!   quality_standards 族）零交集；对既有数据只做 product_id 外键只读关联，不改它们一列。
//! - 限值判定表按"项目乘安全类别乘产品形态乘条件标志"四维定位一行限值，比较器列表达
//!   不大于、区间、不低于、禁用、必须为无、不考核六种判定形态，三态判定（考核、放宽、
//!   不适用）全部由行数据表达，判定器不硬编码限值。同项目允许并存多行（如可分解致癌
//!   芳香胺的禁用行与 20mg/kg 上限行），唯一键含比较器列，判定取合取。
//! - 中间产品 pH 放宽档以 (product_form=intermediate, condition_tag=wet_processing) 行表达，
//!   最终产品沿用该放宽在矩阵里没有对应行，判定器查不到行即显式报错，不会静默放宽。
//! - 外键方向与删除语义：从表引用主表一律 RESTRICT（送检结果与清单项引用的主数据被删时
//!   显式撞库报错，不静默级联销账）；唯二 CASCADE 是送检单派生的清单行与童装核查行
//!   （草稿送检单删除时其派生行随主行回收，第三方报告与逐项目结果挂报告表，RESTRICT）。
//! - `CREATE TABLE IF NOT EXISTS` 加具名约束：全新库直接建表，复用库上本域作为链尾新顶层
//!   条目必然执行，幂等可重放；本迁移不写任何种子行，种子在配套 m0112。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS textile_test_item (
    id SERIAL PRIMARY KEY,
    item_code VARCHAR(64) NOT NULL,
    item_name VARCHAR(128) NOT NULL,
    item_kind VARCHAR(16) NOT NULL,
    basis_standard VARCHAR(16) NOT NULL,
    check_kind VARCHAR(16) NOT NULL DEFAULT 'measurable',
    method_standard VARCHAR(64),
    unit VARCHAR(32),
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_textile_test_item_code UNIQUE (item_code)
);
COMMENT ON TABLE "textile_test_item" IS '纺织品安全检测项目主数据（甲醛、pH、色牢度、童装加考条款等）';
COMMENT ON COLUMN "textile_test_item"."item_code" IS '项目稳定编码，判定器与种子的查找键，禁改值';
COMMENT ON COLUMN "textile_test_item"."item_kind" IS '测量类型（numeric、grade、sensory、boolean），决定录入校验与限值行比较器形状';
COMMENT ON COLUMN "textile_test_item"."basis_standard" IS '项目出身标准分组（GB18401、GB31701、EXPORT、CUSTOMER），只作分类键，限值一律查 textile_limit_rule';
COMMENT ON COLUMN "textile_test_item"."check_kind" IS '核查方式（measurable 可测值自动判定，manual 只登记条目与人工结论位，绝不伪装自动判定）';
COMMENT ON COLUMN "textile_test_item"."method_standard" IS '检测方法标准号（如 GB/T 5713），来源不确定的留 NULL，由报告行按实测记录方法列承载';
COMMENT ON COLUMN "textile_test_item"."unit" IS '计量单位（mg/kg、%、mm、级），等级与感官型可空';

CREATE TABLE IF NOT EXISTS textile_limit_rule (
    id SERIAL PRIMARY KEY,
    item_id INTEGER NOT NULL,
    safety_category VARCHAR(16) NOT NULL,
    product_form VARCHAR(16) NOT NULL,
    condition_tag VARCHAR(48) NOT NULL DEFAULT 'none',
    comparator VARCHAR(16) NOT NULL,
    limit_low DECIMAL(12,4),
    limit_high DECIMAL(12,4),
    clause_ref VARCHAR(64) NOT NULL DEFAULT '',
    remark TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_textile_limit_rule_item FOREIGN KEY (item_id) REFERENCES textile_test_item (id) ON DELETE RESTRICT,
    CONSTRAINT uq_textile_limit_rule_dims UNIQUE (item_id, safety_category, product_form, condition_tag, comparator),
    CONSTRAINT chk_textile_limit_rule_range_sane CHECK (limit_low IS NULL OR limit_high IS NULL OR limit_low <= limit_high)
);
COMMENT ON TABLE "textile_limit_rule" IS '限值判定表（四维矩阵：项目乘安全类别乘产品形态乘条件标志），判定器唯一限值来源，行数据可维护';
COMMENT ON COLUMN "textile_limit_rule"."safety_category" IS '安全类别维（GB18401_A、GB18401_B、GB18401_C 或通配 any）；通配只对判定表有意义，业务列 CHECK 不含 any';
COMMENT ON COLUMN "textile_limit_rule"."product_form" IS '产品形态维（final、intermediate 或通配 any）';
COMMENT ON COLUMN "textile_limit_rule"."condition_tag" IS '条件标志维（none 无条件，或四枚豁免标签之一；wet_processing 触发 pH 放宽档，其余三枚触发色牢度不考核）';
COMMENT ON COLUMN "textile_limit_rule"."comparator" IS '比较器（lte 不大于、gte 不低于、between 区间、forbidden 禁用、must_none 必须为无、na 不考核）';
COMMENT ON COLUMN "textile_limit_rule"."limit_low" IS '下限（between 与 gte 消费；等级型存半级编码数值，3-4 级下限记 3.5）';
COMMENT ON COLUMN "textile_limit_rule"."limit_high" IS '上限（between 与 lte 消费；外层面料燃烧性能 1 级为最佳档，用 lte 1 表达须达 1 级）';
COMMENT ON COLUMN "textile_limit_rule"."clause_ref" IS '依据标准号（如 GB 18401-2010），只存标准号不臆造条款号，细则文字进 remark';
CREATE INDEX IF NOT EXISTS idx_textile_limit_rule_item ON textile_limit_rule (item_id);

CREATE TABLE IF NOT EXISTS textile_safety_submission (
    id SERIAL PRIMARY KEY,
    submission_no VARCHAR(40) NOT NULL,
    product_id INTEGER NOT NULL,
    batch_no VARCHAR(64),
    color_no VARCHAR(64) NOT NULL,
    process_no VARCHAR(128) NOT NULL,
    color_role VARCHAR(24) NOT NULL DEFAULT 'full',
    end_user_group VARCHAR(16) NOT NULL,
    contact_type VARCHAR(16) NOT NULL,
    safety_category VARCHAR(16) NOT NULL,
    product_form VARCHAR(16) NOT NULL,
    target_market VARCHAR(8) NOT NULL DEFAULT 'dom',
    exemption_tags VARCHAR(160),
    status VARCHAR(16) NOT NULL DEFAULT 'draft',
    overall_verdict VARCHAR(8),
    remarks TEXT,
    created_by INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_textile_safety_submission_no UNIQUE (submission_no),
    CONSTRAINT fk_textile_safety_submission_product FOREIGN KEY (product_id) REFERENCES products (id) ON DELETE RESTRICT
);
COMMENT ON TABLE "textile_safety_submission" IS '送检单（取样批次口径为同品种同色号同工艺，按人群、接触方式与市场推检测项目集）';
COMMENT ON COLUMN "textile_safety_submission"."product_id" IS '品种维（外键只读引用 products，不改产品表一列；产品侧安全类别与目标市场字段写入归产品车道）';
COMMENT ON COLUMN "textile_safety_submission"."color_no" IS '色号维，与 process_no 共同构成同批判定；浅色系列多以最深色号代表送检';
COMMENT ON COLUMN "textile_safety_submission"."process_no" IS '工艺维（同工艺才算同批）';
COMMENT ON COLUMN "textile_safety_submission"."color_role" IS '色号在本批中的送检代表性（representative 代表色、customer_specified 客户指定、full 全检）';
COMMENT ON COLUMN "textile_safety_submission"."safety_category" IS '送检申报安全类别稳定码（GB18401_A、GB18401_B、GB18401_C），判定器分派键；展示文本经词表 display_label 换算，不入库';
COMMENT ON COLUMN "textile_safety_submission"."target_market" IS '目标市场码（dom、eu、us、jp、other）';
COMMENT ON COLUMN "textile_safety_submission"."exemption_tags" IS '豁免标签集，英文逗号分隔无空格（取值域为四枚标签；逗号串列无法用 CHECK 闭合，逐token白名单校验在送检服务写入侧）';
COMMENT ON COLUMN "textile_safety_submission"."status" IS '送检单状态机（draft、sampling、sent、reported、judged、closed、withdrawn）';
COMMENT ON COLUMN "textile_safety_submission"."overall_verdict" IS '整单结论（pass、fail、na），判定完成前恒 NULL，pending 不是合法整单结论';
CREATE INDEX IF NOT EXISTS idx_textile_safety_submission_product ON textile_safety_submission (product_id);
CREATE INDEX IF NOT EXISTS idx_textile_safety_submission_status ON textile_safety_submission (status);

CREATE TABLE IF NOT EXISTS textile_safety_submission_item (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    source VARCHAR(16) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_textile_submission_item_submission FOREIGN KEY (submission_id) REFERENCES textile_safety_submission (id) ON DELETE CASCADE,
    CONSTRAINT fk_textile_submission_item_item FOREIGN KEY (item_id) REFERENCES textile_test_item (id) ON DELETE RESTRICT,
    CONSTRAINT uq_textile_submission_item UNIQUE (submission_id, item_id)
);
COMMENT ON TABLE "textile_safety_submission_item" IS '送检项目清单（按人群、接触方式与市场推出的应检项目集，重生成冲掉自动项、保留客户点名项）';
COMMENT ON COLUMN "textile_safety_submission_item"."source" IS '清单行归因（GB18401、GB31701、EXPORT、CUSTOMER），CUSTOMER 行不被自动重生成冲掉';
CREATE INDEX IF NOT EXISTS idx_textile_submission_item_item ON textile_safety_submission_item (item_id);

CREATE TABLE IF NOT EXISTS textile_third_party_report (
    id SERIAL PRIMARY KEY,
    report_no VARCHAR(64) NOT NULL,
    submission_id INTEGER NOT NULL,
    inspection_agency VARCHAR(128) NOT NULL,
    has_cma BOOLEAN NOT NULL DEFAULT false,
    has_cnas BOOLEAN NOT NULL DEFAULT false,
    sample_name VARCHAR(128),
    material_composition VARCHAR(255),
    color_no VARCHAR(64),
    process_no VARCHAR(128),
    test_basis VARCHAR(128),
    conclusion VARCHAR(255),
    oeko_tex_class VARCHAR(8),
    issue_date DATE NOT NULL,
    valid_until DATE,
    acceptance_status VARCHAR(16) NOT NULL DEFAULT 'pending',
    rejection_detail VARCHAR(255) NOT NULL DEFAULT '',
    report_file_url VARCHAR(512),
    created_by INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_textile_third_party_report_no UNIQUE (report_no),
    CONSTRAINT fk_textile_report_submission FOREIGN KEY (submission_id) REFERENCES textile_safety_submission (id) ON DELETE RESTRICT,
    CONSTRAINT chk_textile_report_valid_window CHECK (valid_until IS NULL OR valid_until >= issue_date)
);
COMMENT ON TABLE "textile_third_party_report" IS '第三方检测报告登记（完整性门：CMA、CNAS 资质位、六项要素、有效期、按批次一致性，缺项受理侧点名拒绝）';
COMMENT ON COLUMN "textile_third_party_report"."has_cma" IS '报告是否带 CMA 资质认定标志（完整性门要素，缺位受理拒绝并在 rejection_detail 点名 cma）';
COMMENT ON COLUMN "textile_third_party_report"."has_cnas" IS '报告是否带 CNAS 认可标志（同上，点名 cnas）';
COMMENT ON COLUMN "textile_third_party_report"."test_basis" IS '检验依据原文形态（如 "GB 18401-2010 B类"），受理门做存在性与格式校验，不回写产品侧 execution_standard 列';
COMMENT ON COLUMN "textile_third_party_report"."color_no" IS '报告侧色号，与送检单色号不一致时受理门点名 batch_mismatch（按批次口径）';
COMMENT ON COLUMN "textile_third_party_report"."process_no" IS '报告侧工艺，与送检单工艺不一致时受理门点名 batch_mismatch';
COMMENT ON COLUMN "textile_third_party_report"."oeko_tex_class" IS 'OEKO-TEX STANDARD 100 分级位（I、II、III、IV），出口与报价订单侧只读消费';
COMMENT ON COLUMN "textile_third_party_report"."acceptance_status" IS '受理态（pending、accepted、rejected），rejected 时 rejection_detail 逐条点名缺项代号';
COMMENT ON COLUMN "textile_third_party_report"."rejection_detail" IS '缺项代号逗号串（取值域 report_element_code 词表，逗号串列无法 CHECK，逐token校验在服务写入侧）';
CREATE INDEX IF NOT EXISTS idx_textile_third_party_report_submission ON textile_third_party_report (submission_id);

CREATE TABLE IF NOT EXISTS textile_report_item_result (
    id SERIAL PRIMARY KEY,
    report_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    measured_value DECIMAL(12,4),
    measured_text VARCHAR(64),
    measured_unit VARCHAR(32),
    test_method VARCHAR(64),
    verdict VARCHAR(8) NOT NULL DEFAULT 'pending',
    judged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_textile_report_result_report FOREIGN KEY (report_id) REFERENCES textile_third_party_report (id) ON DELETE CASCADE,
    CONSTRAINT fk_textile_report_result_item FOREIGN KEY (item_id) REFERENCES textile_test_item (id) ON DELETE RESTRICT,
    CONSTRAINT uq_textile_report_result UNIQUE (report_id, item_id)
);
COMMENT ON TABLE "textile_report_item_result" IS '第三方报告逐项目实测结果（结构化承载：存值、单位、检测方法，判定器按限值矩阵行出三态）';
COMMENT ON COLUMN "textile_report_item_result"."measured_value" IS '数值与等级型实测值；等级型存半级编码数值（3-4 级记 3.5），编码表在词表 textile_fastness_grade';
COMMENT ON COLUMN "textile_report_item_result"."measured_text" IS '感官与布尔证据型 flag（none、detected）与方法补充说明文本；must_none 与 forbidden 行消费 flag';
COMMENT ON COLUMN "textile_report_item_result"."measured_unit" IS '实测单位快照（与项目主数据单位可不一致，以报告为准，判定器按数值判定不换算单位）';
COMMENT ON COLUMN "textile_report_item_result"."test_method" IS '报告所载检测方法（标准号或方法名），满足逐项目方法要素承载';
COMMENT ON COLUMN "textile_report_item_result"."verdict" IS '逐项目判定三态（pass、fail、na）加待判定 pending；重录实测值会复位 pending';
CREATE INDEX IF NOT EXISTS idx_textile_report_result_item ON textile_report_item_result (item_id);

CREATE TABLE IF NOT EXISTS textile_child_safety_check (
    id SERIAL PRIMARY KEY,
    submission_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    check_kind VARCHAR(16) NOT NULL,
    measured_value DECIMAL(12,4),
    measured_unit VARCHAR(32),
    measured_flag VARCHAR(16),
    check_desc VARCHAR(255),
    manual_conclusion VARCHAR(8),
    verdict VARCHAR(8) NOT NULL DEFAULT 'pending',
    judged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_textile_child_check_submission FOREIGN KEY (submission_id) REFERENCES textile_safety_submission (id) ON DELETE CASCADE,
    CONSTRAINT fk_textile_child_check_item FOREIGN KEY (item_id) REFERENCES textile_test_item (id) ON DELETE RESTRICT,
    CONSTRAINT uq_textile_child_check UNIQUE (submission_id, item_id)
);
COMMENT ON TABLE "textile_child_safety_check" IS 'GB 31701 童装加考核查登记与判定（绳带、锐利物、附件、标签位置；可测项自动判定，条款项只有人工结论位，绝不伪装自动判定）';
COMMENT ON COLUMN "textile_child_safety_check"."check_kind" IS '核查方式快照（measurable 有实测值走判定器，manual 仅登记条目与人工结论位）';
COMMENT ON COLUMN "textile_child_safety_check"."measured_value" IS '可测项实测值（铅、镉、邻苯、燃烧性能、腰部绳带伸出长度 mm 等）';
COMMENT ON COLUMN "textile_child_safety_check"."measured_flag" IS 'boolean 与感官可测项的实测 flag（none、detected），must_none 判定行消费本列（金属针残留、头颈部绳带）';
COMMENT ON COLUMN "textile_child_safety_check"."check_desc" IS '核查描述（绳带位置、附件形态、标签缝制位置等现场记录文字）';
COMMENT ON COLUMN "textile_child_safety_check"."manual_conclusion" IS '人工结论位（pass、fail、na），measurable 行为 NULL 由判定器写 verdict；条款项没有本列取值则整单判定拒绝完成，不静默放行';
COMMENT ON COLUMN "textile_child_safety_check"."verdict" IS '核查判定态（measurable 行由判定器产出，manual 行在整单判定搬运 manual_conclusion）';

CREATE TABLE IF NOT EXISTS textile_export_market_requirement (
    id SERIAL PRIMARY KEY,
    requirement_code VARCHAR(64) NOT NULL,
    market VARCHAR(8) NOT NULL,
    requirement_type VARCHAR(32) NOT NULL,
    oeko_tex_class VARCHAR(8),
    standard_ref VARCHAR(128) NOT NULL DEFAULT '',
    detail TEXT NOT NULL,
    mandatory BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_textile_export_requirement_code UNIQUE (requirement_code)
);
COMMENT ON TABLE "textile_export_market_requirement" IS '出口市场合规要求登记（OEKO-TEX 分级、REACH SVHC、CPSIA、日本自主管控；登记位可被报价与订单侧只读消费，限值细则不反向流入判定矩阵）';
COMMENT ON COLUMN "textile_export_market_requirement"."market" IS '登记所属出口市场（eu、us、jp、other，内销 dom 不挂出口要求，故取值域不含 dom）';
COMMENT ON COLUMN "textile_export_market_requirement"."requirement_type" IS '要求类型（oeko_tex、reach_svhc、cpsia_lead、cpsia_phthalate、cpsia_cert、jp_formaldehyde）';
COMMENT ON COLUMN "textile_export_market_requirement"."standard_ref" IS '依据法规或标准名号（OEKO-TEX STANDARD 100、REACH、CPSIA 等）';
COMMENT ON COLUMN "textile_export_market_requirement"."detail" IS '要求细则文字；法规数值以官方现行文本为准，本表是登记位不是判定源';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序删表：先删引用方（结果、清单、核查、报告、要求、限值行），再删被引用的
        // 送检单与项目主数据。表内若已有本域之外的引用残留，DROP 会显式报错而不是静默。
        let sql = r#"
DROP TABLE IF EXISTS textile_report_item_result;
DROP TABLE IF EXISTS textile_child_safety_check;
DROP TABLE IF EXISTS textile_safety_submission_item;
DROP TABLE IF EXISTS textile_third_party_report;
DROP TABLE IF EXISTS textile_safety_submission;
DROP TABLE IF EXISTS textile_export_market_requirement;
DROP TABLE IF EXISTS textile_limit_rule;
DROP TABLE IF EXISTS textile_test_item;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
