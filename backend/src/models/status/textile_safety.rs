#![allow(dead_code)]
//! 纺织品安全合规域状态词表（GB 18401-2010 基本安全、GB 31701-2015 童装加考、第三方报告与送检流程）
//!
//! 本文件是安全合规检验子域全部取值型列的**唯一**权威来源：入参白名单校验、限值判定器的
//! 分派键、DB CHECK 约束、前端选项与回显映射一律引用本 mod 常量，禁止在任何调用点内联字面量。
//!
//! 安全类别与目标市场两枚字段在产品侧由产品车道写入，本域只读消费并在送检单上落同码快照；
//! 存储码一律用带前缀稳定码（`GB18401_A` 而非裸 `A`），裸 A/B/C 字面在本仓已被三套语义占用
//! （`services/quality_inspection_service.rs` 的 A 合格 B 让步 C 返工报废质量分级、
//! `inventory_stock_grade`、`product_grade`），撞名误读风险不可控，故落库与传参只用前缀码；
//! 报告"检验依据"需要国标原文形态（"GB 18401-2010 B类"）时经
//! `textile_safety_category::display_label` 换算展示文本，展示文本永不入库。
//!
//! 与既有词表的关系（不可互抄）：
//! - `quality_inspection_result`（待检/合格/不合格）钉的是通用质检记录 `quality_inspection_records.inspection_result`，
//!   语义是**整单质检结论**；本域的 `textile_verdict` 语义是**单个检测项目对限值的判定三态**（多一个"不考核"），
//!   两者取值集合与用途都不同。
//! - `QUALITY_GRADE_A/B/C`（按合格率分档的质量分级）与 `inventory_stock_grade`（一等品/二等品/等外品）
//!   与 `fabric_grade`（首级/次级）都是**质量档次**；本域 `textile_safety_category` 是 GB 18401 的
//!   **安全类别**（婴幼儿/直接接触/非直接接触），强制门槛而非质量档次，任何一侧改取值都不得顺手动另一侧。
//! - `export_inspection_result`（pending/pass/fail）是出口**商检**放行结论，与本域逐项目判定无关。

/// 国家安全类别（GB 18401-2010 第 5 章，按"贴谁身上"三分，强制门槛而非质量档次）
///
/// 消费方：限值判定器的分派键（`services/textile_safety_limit_judge.rs` 按本列取限值行）、
/// 送检项目清单生成、第三方报告"检验依据"要素拼装。存于送检单与限值行的 `safety_category` 列。
/// 落库码带 `GB18401_` 前缀（判据见文件头），展示文本只出自 `display_label`。
pub mod textile_safety_category {
    /// A 类：婴幼儿用品（36 个月及以下、身高 100cm 及以下；连体衣、内衣、尿布、围嘴、睡衣、袜子、床上用品）
    pub const CLASS_A: &str = "GB18401_A";
    /// B 类：直接接触皮肤产品（内衣、T 恤、衬衫、裤子、裙子、床单、毛巾、泳衣）
    pub const CLASS_B: &str = "GB18401_B";
    /// C 类：非直接接触皮肤产品（大衣、风衣、羽绒服外套、西装、窗帘、床罩）
    pub const CLASS_C: &str = "GB18401_C";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[CLASS_A, CLASS_B, CLASS_C];

    /// 白名单判定：逐字符匹配，词表外（含空串、裸 "A"、"A类"）一律 false
    pub fn is_valid(category: &str) -> bool {
        ALL.contains(&category)
    }

    /// 稳定码到国标原文展示形态的换算（报告"检验依据"拼装与界面回显用，永不入库）。
    /// 词表外返回 `None`，由调用方报错，不静默兜底成某个类别。
    pub fn display_label(category: &str) -> Option<&'static str> {
        match category {
            CLASS_A => Some("A类"),
            CLASS_B => Some("B类"),
            CLASS_C => Some("C类"),
            _ => None,
        }
    }

    /// 严格度秩：A 最严取 1、B 取 2、C 取 3，词表外返回 `None`。
    /// 消费方：送检单建单校验"申报类别不得松于人群要求类别"（秩越小越严，
    /// 申报类别秩必须不大于要求类别秩）。
    pub fn strictness_rank(category: &str) -> Option<u8> {
        match category {
            CLASS_A => Some(1),
            CLASS_B => Some(2),
            CLASS_C => Some(3),
            _ => None,
        }
    }
}

/// 终端人群（送检单按人群定类别的输入维度，GB 31701 的加考触发键）
pub mod textile_end_user_group {
    /// 婴幼儿：36 个月及以下（GB 31701 强制加考，成品类别必须 A 类）
    pub const INFANT: &str = "infant";
    /// 儿童：3 岁至 14 岁（GB 31701 按接触方式加考，绳带条款按身高 130cm 以下分档）
    pub const CHILD: &str = "child";
    /// 成人：仅按 GB 18401 考核，无 GB 31701 加考项
    pub const ADULT: &str = "adult";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[INFANT, CHILD, ADULT];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(group: &str) -> bool {
        ALL.contains(&group)
    }

    /// 人群 → 最低要求安全类别（GB 31701 第 4 章）：婴幼儿须 A 类；儿童须 B 类及以上；
    /// 成人按 C 类兜底（不得低于 C 类）。返回值为 `textile_safety_category` 常量，词表外人群
    /// 返回 `None` 由调用方报错，不静默兜底成某个类别。
    pub fn required_safety_category(group: &str) -> Option<&'static str> {
        use super::textile_safety_category as cat;
        if group == INFANT {
            Some(cat::CLASS_A)
        } else if group == CHILD {
            Some(cat::CLASS_B)
        } else if group == ADULT {
            Some(cat::CLASS_C)
        } else {
            None
        }
    }
}

/// 接触方式（决定 B/C 归类与是否触发童装加考的第二输入维度）
pub mod textile_contact_type {
    /// 直接接触皮肤：穿着或使用时与人体皮肤直接接触的产品
    pub const DIRECT: &str = "direct";
    /// 非直接接触皮肤：穿着使用时不与皮肤直接接触的产品（有外衣层隔开的）
    pub const INDIRECT: &str = "indirect";
    /// 婴幼儿用品专用档：36 个月及以下婴幼儿使用的产品，无论接触方式一律按 A 类考核
    pub const INFANT_USE: &str = "infant_use";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[DIRECT, INDIRECT, INFANT_USE];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(contact: &str) -> bool {
        ALL.contains(&contact)
    }
}

/// 产品形态（GB 18401 第 6 章判定细则的分派键：中间产品与最终产品的限值不同）
pub mod textile_product_form {
    /// 最终产品：面向消费者交付的成品，一律按标准正文限值判定，不得放宽
    pub const FINAL: &str = "final";
    /// 中间产品：后续还要湿处理加工的在制品，仅 pH 项可按标准放宽到 4.0~10.5，
    ///   最终产品不得沿用本档放宽
    pub const INTERMEDIATE: &str = "intermediate";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[FINAL, INTERMEDIATE];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(form: &str) -> bool {
        ALL.contains(&form)
    }
}

/// 不考核豁免标签（GB 18401 第 6.3 章色牢度豁免；一张送检单可挂多个标签，
/// 以英文逗号分隔存列且不含空格，判定器按标签集查豁免规则行）
///
/// 存列形态：`textile_safety_submission.exemption_tags`。空串与 NULL 都表示无豁免。
pub mod textile_exemption_tag {
    /// 后续湿处理加工的中间产品：pH 放宽到 4.0~10.5（只对 pH 项生效，且只对中间产品生效）
    pub const WET_PROCESSING: &str = "wet_processing";
    /// 需洗涤褪色的非最终产品：色牢度项不考核
    pub const LAUNDERING_FADE_NON_FINAL: &str = "laundering_fade_non_final";
    /// 本色及漂白产品：色牢度项不考核（无着色可言）
    pub const PLAIN_OR_BLEACHED: &str = "plain_or_bleached";
    /// 扎染与蜡染等传统手工着色产品：色牢度项不考核
    pub const TRADITIONAL_HAND_DYED: &str = "traditional_hand_dyed";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] =
        &[WET_PROCESSING, LAUNDERING_FADE_NON_FINAL, PLAIN_OR_BLEACHED, TRADITIONAL_HAND_DYED];

    /// 白名单判定：单个标签逐字符匹配，词表外一律 false
    pub fn is_valid(tag: &str) -> bool {
        ALL.contains(&tag)
    }

    /// 色牢度类项目（耐水、耐酸汗渍、耐碱汗渍、耐干摩擦、耐湿摩擦、耐唾液等）的豁免标签集合：
    /// 命中任一即该项目判"不考核"。pH 的放宽不在此集内（放宽不等于不考核，见判定器）。
    pub const FASTNESS_EXEMPTIONS: &[&str] =
        &[LAUNDERING_FADE_NON_FINAL, PLAIN_OR_BLEACHED, TRADITIONAL_HAND_DYED];
}

/// 逐项目判定三态（本域结论词表：合格、不合格、不考核、待判定）
///
/// "不考核"是 GB 18401 的法定第三态（耐唾液仅 A 类考核、豁免标签命中时色牢度不考核），
/// 必须有独立 token，不得折算成合格或不合格，否则汇总整单结论时会出现无依据的合格。
/// 存于逐项目结果行 `verdict` 列与送检单 `overall_verdict` 列；DB CHECK 与 `ALL` 逐元素相等。
pub mod textile_verdict {
    /// 合格：实测值满足限值行要求
    pub const PASS: &str = "pass";
    /// 不合格：实测值超出限值行要求
    pub const FAIL: &str = "fail";
    /// 不考核：按安全类别/产品形态/豁免标签，该项目在本单不适用
    pub const NOT_APPLICABLE: &str = "na";
    /// 待判定：结果行已建但实测值未录入，限值判定尚未执行
    pub const PENDING: &str = "pending";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[PASS, FAIL, NOT_APPLICABLE, PENDING];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(verdict: &str) -> bool {
        ALL.contains(&verdict)
    }

    /// 判定完成三态（不含 pending）：整单结论汇总与报告要素完备性校验的分桶集合
    pub const JUDGED: &[&str] = &[PASS, FAIL, NOT_APPLICABLE];

    /// 整单结论取值域（送检单 `overall_verdict` 列，可空）：pending 不是合法的整单结论，
    /// 未判定用 NULL 表达，判定完成只能是三态之一。
    pub const OVERALL: &[&str] = &[PASS, FAIL, NOT_APPLICABLE];
}

/// 检测项目的测量类型（决定限值行的比较器形状与录入控件校验）
pub mod textile_item_kind {
    /// 数值型：甲醛 mg/kg、pH、铅镉 mg/kg、邻苯百分比等，限值行用上下界
    pub const NUMERIC: &str = "numeric";
    /// 等级型：色牢度等级（1/1-2/2/2-3/3/3-4/4/4-5/5），限值行用等级下限，
    ///   取值域与半级编码见 `textile_fastness_grade`
    pub const GRADE: &str = "grade";
    /// 感官判定型：异味（无/检出某类异味），限值行固定"必须为无"
    pub const SENSORY: &str = "sensory";
    /// 布尔证据型：锐利物残留、金属针、绳带有无等硬性条款，可存值即判，
    ///   存不了值的以结论位人工判定（见 `textile_check_kind`）
    pub const BOOLEAN: &str = "boolean";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[NUMERIC, GRADE, SENSORY, BOOLEAN];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(kind: &str) -> bool {
        ALL.contains(&kind)
    }
}

/// 限值行比较器（限值判定表 `textile_limit_rule.comparator` 列，决定判定器如何比实测值）
pub mod textile_comparator {
    /// 不大于：实测值 `<= limit_high` 为合格（甲醛、铅、镉、邻苯、可分解芳香胺的 20mg/kg 上限、
    ///   腰部绳带伸出 360mm 上限、外层面料燃烧性能 1 级上限）
    pub const LTE: &str = "lte";
    /// 不低于：实测值 `>= limit_low` 为合格（色牢度等级下限，等级型用半级编码数值比较）
    pub const GTE: &str = "gte";
    /// 区间：实测值落在 `[limit_low, limit_high]` 闭区间为合格（pH 4.0~7.5 等）
    pub const BETWEEN: &str = "between";
    /// 禁用：命中检出即不合格（可分解致癌芳香胺染料的"禁用"半边，与 20mg/kg 上限同项目并存两行，
    ///   判定取合取）
    pub const FORBIDDEN: &str = "forbidden";
    /// 必须为无：感官与布尔证据型项目（异味、金属针残留、头颈部绳带）要求实测记录为"无"
    pub const MUST_NONE: &str = "must_none";
    /// 不考核：该项目在本类别/本形态/本豁免下不适用，判定器直接产出 `na`
    pub const NOT_APPLICABLE: &str = "na";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[LTE, GTE, BETWEEN, FORBIDDEN, MUST_NONE, NOT_APPLICABLE];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(comparator: &str) -> bool {
        ALL.contains(&comparator)
    }
}

/// 限值行的维度通配与无条件哨兵（限值判定表专用，不出现在送检单等业务列上）
///
/// 限值矩阵允许以通配行表达"对全部类别/全部形态适用"的同一限值，避免逐类别重复播种；
/// 判定器按特异度分层取值（精确行压过通配行，带条件行压过无条件行）。
/// 送检单等业务列的 CHECK 不含本模块取值——通配只对判定表有意义。
pub mod textile_limit_scope {
    /// 类别通配：该行对 A/B/C 三个安全类别同等生效
    pub const CATEGORY_ANY: &str = "any";
    /// 形态通配：该行对最终产品与中间产品同等生效
    pub const FORM_ANY: &str = "any";
    /// 条件通配（无条件）：该行不依赖任何豁免标签即生效
    pub const TAG_NONE: &str = "none";

    /// 限值行 safety_category 列全部合法取值（三枚稳定码加通配码）：DB CHECK 的唯一取值来源，
    /// 与 m0110 迁移字面量由纸面锁逐元素钉相等。
    pub const CATEGORIES: &[&str] = &[
        super::textile_safety_category::CLASS_A,
        super::textile_safety_category::CLASS_B,
        super::textile_safety_category::CLASS_C,
        CATEGORY_ANY,
    ];

    /// 限值行 product_form 列全部合法取值（两形态加通配）
    pub const FORMS: &[&str] = &[
        super::textile_product_form::FINAL,
        super::textile_product_form::INTERMEDIATE,
        FORM_ANY,
    ];

    /// 限值行 condition_tag 列全部合法取值（无条件哨兵加四枚豁免标签）
    pub const CONDITION_TAGS: &[&str] = &[
        TAG_NONE,
        super::textile_exemption_tag::WET_PROCESSING,
        super::textile_exemption_tag::LAUNDERING_FADE_NON_FINAL,
        super::textile_exemption_tag::PLAIN_OR_BLEACHED,
        super::textile_exemption_tag::TRADITIONAL_HAND_DYED,
    ];
}

/// 色牢度等级取值域（1 到 5 共九档，含四个半级；录入 token 与数值编码一一对应）
///
/// 存值口径：等级型项目实测值以半级编码数值存 `measured_value`（DECIMAL），
/// 限值行 `gte` 比较数值下限；展示形态用本表 token。三端同源，界面不得另写第二张等级表。
pub mod textile_fastness_grade {
    /// token 与编码数值的十进制字面量对照（"3-4" 记 3.5，依国标半级介于两级之间的语义）
    pub const TOKENS: &[(&str, &str)] = &[
        ("1", "1.0"),
        ("1-2", "1.5"),
        ("2", "2.0"),
        ("2-3", "2.5"),
        ("3", "3.0"),
        ("3-4", "3.5"),
        ("4", "4.0"),
        ("4-5", "4.5"),
        ("5", "5.0"),
    ];

    /// 全部合法等级 token
    pub const ALL: &[&str] = &["1", "1-2", "2", "2-3", "3", "3-4", "4", "4-5", "5"];

    /// 白名单判定
    pub fn is_valid(token: &str) -> bool {
        ALL.contains(&token)
    }

    /// token 到编码数值字面量；词表外返回 `None` 由调用方报错。
    pub fn encode(token: &str) -> Option<&'static str> {
        TOKENS.iter().find(|(t, _)| *t == token).map(|(_, v)| *v)
    }
}

/// 感官与布尔证据型项目的取值 flag（异味"无/检出"、金属针"无/检出"、绳带"无/检出"）
///
/// 存列口径：flag 存 `measured_text` 列（限值行 must_none 与 forbidden 消费本词表）。
/// yes/no 供人工核查的通过与否表述复用同一字典，不再另造中文 token 入库。
pub mod textile_flag_value {
    /// 无（未检出、无异味、无残留、无绳带）
    pub const NONE: &str = "none";
    /// 检出（检出异味、检出金属针、检出绳带等硬性禁用项命中）
    pub const DETECTED: &str = "detected";
    /// 满足条款要求（人工核查结论位用）
    pub const YES: &str = "yes";
    /// 不满足条款要求（人工核查结论位用）
    pub const NO: &str = "no";

    /// 全部合法取值：flag 型录入的白名单唯一来源
    pub const ALL: &[&str] = &[NONE, DETECTED, YES, NO];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(flag: &str) -> bool {
        ALL.contains(&flag)
    }
}

/// 送检单状态机（`textile_safety_submission.status`）
///
/// 真实送检流程：建单 → 按人群/接触方式/市场推项目清单 → 取样（同品种同色号同工艺一批）→
/// 送有资质机构 → 收报告 → 核对要素与有效期 → 判定归档 → 关闭；撤回为旁支终态。
pub mod submission_status {
    /// 草稿：已建单，项目清单尚未生成或可重生成
    pub const DRAFT: &str = "draft";
    /// 待取样：项目清单已生成，等取样批次登记
    pub const SAMPLING: &str = "sampling";
    /// 已送检：样品已送出至第三方机构，等报告
    pub const SENT: &str = "sent";
    /// 已收报告：报告已登记，待要素核对与判定
    pub const REPORTED: &str = "reported";
    /// 已判定：逐项目判定完成且报告要素完整，可出整单结论
    pub const JUDGED: &str = "judged";
    /// 已关闭：结论归档，不再接受改动
    pub const CLOSED: &str = "closed";
    /// 已撤回：送检作废（撤单、复检改走新单），终态
    pub const WITHDRAWN: &str = "withdrawn";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[
        DRAFT,
        SAMPLING,
        SENT,
        REPORTED,
        JUDGED,
        CLOSED,
        WITHDRAWN,
    ];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(status: &str) -> bool {
        ALL.contains(&status)
    }

    /// 可编辑态集合（草稿/待取样/已送检在收报告前仍可改项目清单与取样批次；
    /// reported 之后只允许走报告与判定通道，不允许回改送检要素）
    pub const EDITABLE: &[&str] = &[DRAFT, SAMPLING, SENT];
}

/// 第三方报告受理态（`textile_third_party_report.acceptance_status`，报告完整性门的结论位）
///
/// 完整性门在受理侧执行：缺 CMA 或 CNAS 资质标志、六项要素缺任一、有效期已过，
/// 任一不满足都落 rejected 并逐条点名缺项，不允许"先收后进"式静默通过。
pub mod report_acceptance_status {
    /// 待核对：报告已录入，尚未跑完整性门
    pub const PENDING: &str = "pending";
    /// 已受理：资质标志齐、六项要素齐、有效期未过
    pub const ACCEPTED: &str = "accepted";
    /// 已拒绝：完整性门任一不通过，缺项逐条记入 rejection_detail
    pub const REJECTED: &str = "rejected";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[PENDING, ACCEPTED, REJECTED];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(status: &str) -> bool {
        ALL.contains(&status)
    }
}

/// 报告完整性门的缺项代号（受理拒绝时逐条点名的取值域，前后端共用一张词表）
pub mod report_element_code {
    /// CMA 资质标志缺失（检验检测机构资质认定，缺则客户一般不认）
    pub const CMA: &str = "cma";
    /// CNAS 资质标志缺失（中国合格评定国家认可委员会认可）
    pub const CNAS: &str = "cnas";
    /// 样品名称缺失
    pub const SAMPLE_NAME: &str = "sample_name";
    /// 材质成分缺失
    pub const MATERIAL_COMPOSITION: &str = "material_composition";
    /// 色号缺失
    pub const COLOR_NO: &str = "color_no";
    /// 检验依据缺失或不完整（须含完整标准号与安全类别，如 "GB 18401-2010 B类"）
    pub const TEST_BASIS: &str = "test_basis";
    /// 检测项目与结果缺失（逐项目结果行为空）
    pub const ITEM_RESULTS: &str = "item_results";
    /// 判定结论缺失
    pub const CONCLUSION: &str = "conclusion";
    /// 报告已过期（签发有效期已过，或有效期列缺失无法核验）
    pub const EXPIRED: &str = "expired";
    /// 未按批次提供（报告色号或工艺口径与送检单批次三元组不一致）
    pub const BATCH_MISMATCH: &str = "batch_mismatch";

    /// 六项报告要素代号（不含资质标志与有效期，那三项另门）：完整性门的要素集取值来源
    pub const SIX_ELEMENTS: &[&str] = &[
        SAMPLE_NAME,
        MATERIAL_COMPOSITION,
        COLOR_NO,
        TEST_BASIS,
        ITEM_RESULTS,
        CONCLUSION,
    ];

    /// 全部合法取值：入参校验与拒绝明细列拆词校验的唯一取值来源
    pub const ALL: &[&str] = &[
        CMA,
        CNAS,
        SAMPLE_NAME,
        MATERIAL_COMPOSITION,
        COLOR_NO,
        TEST_BASIS,
        ITEM_RESULTS,
        CONCLUSION,
        EXPIRED,
        BATCH_MISMATCH,
    ];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(code: &str) -> bool {
        ALL.contains(&code)
    }
}

/// 取样批次色号角色（`textile_safety_submission.color_role`，取样批次口径的送检代表性标记）
///
/// 行业口径：同一品种、同一色号、同一工艺才算同一批送检；深色常是一批中风险最高的一档，
/// 浅色系列多以最深色号代表送检；客户另有要求以客户为准（本域只表达口径，不替客户裁定）。
pub mod sampling_color_role {
    /// 本批代表色号：按"浅色系列以深色代表"口径被选中送检的色号
    pub const REPRESENTATIVE: &str = "representative";
    /// 客户指定送检色号：客户要求单独送某色号，以客户为准，覆盖代表色口径
    pub const CUSTOMER_SPECIFIED: &str = "customer_specified";
    /// 普通全检色号：本色号单独送检，不代表同批其他色号
    pub const FULL: &str = "full";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[REPRESENTATIVE, CUSTOMER_SPECIFIED, FULL];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(role: &str) -> bool {
        ALL.contains(&role)
    }
}

/// 出口目标市场（送检单 `target_market` 列与出口要求登记表 `market` 列共用）
///
/// 写入侧：送检单上的本列由质量岗录入；产品侧同名字段由产品车道写入，本域只读。
pub mod export_market {
    /// 内销：仅按国标 GB 18401（童装加 GB 31701），不挂出口要求
    pub const DOM: &str = "dom";
    /// 欧盟：OEKO-TEX STANDARD 100 分级 + REACH 高度关注物质清单
    pub const EU: &str = "eu";
    /// 美国：CPSIA 儿童产品含铅与邻苯限制 + 儿童产品证书 CPC
    pub const US: &str = "us";
    /// 日本：各类自主管控标准，甲醛限量要求较严
    pub const JP: &str = "jp";
    /// 其他市场：按客户要求登记，不自动推要求集
    pub const OTHER: &str = "other";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[DOM, EU, US, JP, OTHER];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(market: &str) -> bool {
        ALL.contains(&market)
    }
}

/// OEKO-TEX STANDARD 100 产品分级（报告与出口要求登记的分级位，罗马数字按原文书写）
pub mod oeko_tex_class {
    /// Ⅰ 级：婴幼儿产品（3 岁及以下），要求最严
    pub const CLASS_I: &str = "I";
    /// Ⅱ 级：直接接触皮肤产品
    pub const CLASS_II: &str = "II";
    /// Ⅲ 级：非直接接触皮肤产品
    pub const CLASS_III: &str = "III";
    /// Ⅳ 级：装饰材料（窗帘、床罩、桌布等）
    pub const CLASS_IV: &str = "IV";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[CLASS_I, CLASS_II, CLASS_III, CLASS_IV];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(class: &str) -> bool {
        ALL.contains(&class)
    }
}

/// 出口合规要求类型（`textile_export_market_requirement.requirement_type`，市场加挂要求的分类键）
pub mod export_requirement_type {
    /// OEKO-TEX STANDARD 100 认证分级要求
    pub const OEKO_TEX: &str = "oeko_tex";
    /// REACH 高度关注物质（SVHC）清单管控要求
    pub const REACH_SVHC: &str = "reach_svhc";
    /// CPSIA 含铅限量（儿童产品）
    pub const CPSIA_LEAD: &str = "cpsia_lead";
    /// CPSIA 邻苯二甲酸酯限量（儿童产品）
    pub const CPSIA_PHTHALATE: &str = "cpsia_phthalate";
    /// CPSIA 儿童产品证书 CPC
    pub const CPSIA_CERT: &str = "cpsia_cert";
    /// 日本自主管控标准的甲醛限量要求
    pub const JP_FORMALDEHYDE: &str = "jp_formaldehyde";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[
        OEKO_TEX,
        REACH_SVHC,
        CPSIA_LEAD,
        CPSIA_PHTHALATE,
        CPSIA_CERT,
        JP_FORMALDEHYDE,
    ];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(requirement: &str) -> bool {
        ALL.contains(&requirement)
    }
}

/// 硬性条款的核查方式（`textile_child_safety_check.check_kind` 与检测项目主数据同词表，
/// GB 31701 的物理与条款类项目）
///
/// 设计红线：`measurable` 的项目必须能被存值并自动判定；`manual` 的项目只登记条目与人工
/// 结论位，判定器绝不产出自动结论，界面也不得显示成自动判定。
pub mod textile_check_kind {
    /// 可测值判定：有单位与限值行，实测值入列后判定器自动出三态
    pub const MEASURABLE: &str = "measurable";
    /// 人工结论位：条款型核查（绳带布局、标签缝制位置、附件抗拉、锐利尖端等），
    ///   存核查描述与人工结论位，判定器只搬运人工结论，不做任何推算
    pub const MANUAL: &str = "manual";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[MEASURABLE, MANUAL];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(kind: &str) -> bool {
        ALL.contains(&kind)
    }
}

/// 检测方法与依据标准号来源标识（`textile_test_item.basis_standard`，判定与报告拼装的分派键）
///
/// 只作分类键，不做限值来源；限值一律查 `textile_limit_rule` 行。
pub mod textile_basis_standard {
    /// GB 18401-2010 国家纺织产品基本安全技术规范（强制性，最低门槛）
    pub const GB18401: &str = "GB18401";
    /// GB 31701-2015 婴幼儿及儿童纺织产品安全技术规范（强制性，童装加考）
    pub const GB31701: &str = "GB31701";
    /// 出口市场要求（OEKO-TEX / REACH / CPSIA / 日本标准）
    pub const EXPORT: &str = "EXPORT";
    /// 客户或订单自定义加验项
    pub const CUSTOMER: &str = "CUSTOMER";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[GB18401, GB31701, EXPORT, CUSTOMER];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(basis: &str) -> bool {
        ALL.contains(&basis)
    }
}

/// 送检项目清单的项目来源（`textile_safety_submission_item.source`，清单生成时的归因列）
///
/// 与 `textile_basis_standard` 取值同源但语义不同：basis 钉在主数据项目上说明项目出身，
/// source 钉在清单行上说明该行是"按国标推的/按加考推的/按市场挂的/客户点名加的"，
/// 重生成清单时按 source 区分自动项与人工点名项（人工点名项不被重生成冲掉）。
pub mod textile_item_source {
    /// GB 18401 基本安全项目（按类别自动推入）
    pub const GB18401: &str = "GB18401";
    /// GB 31701 童装加考项目（人群为婴幼儿或儿童时自动推入）
    pub const GB31701: &str = "GB31701";
    /// 出口市场挂验项目（按目标市场登记要求推入）
    pub const EXPORT: &str = "EXPORT";
    /// 客户或质检员点名加验
    pub const CUSTOMER: &str = "CUSTOMER";

    /// 全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[GB18401, GB31701, EXPORT, CUSTOMER];

    /// 白名单判定，口径同 `textile_safety_category::is_valid`
    pub fn is_valid(source: &str) -> bool {
        ALL.contains(&source)
    }
}
