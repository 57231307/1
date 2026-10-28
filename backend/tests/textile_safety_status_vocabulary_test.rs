//! 纺织品安全合规域「Rust 词表常量 ↔ 迁移 CHECK 取值集」纸面双向锁（不连库，CI 常规可跑）。
//!
//! 钉什么：
//! 1. m0111 的 ALLOWED 常量与 models::status::textile_safety 下各词表模块常量双向集合相等，
//!    多一个少一个都判红；词表侧与 CHECK 侧任一侧单独增删值都红。
//! 2. m0111 的 COLUMN_CHECKS 把每个约束名绑到正确的 ALLOWED 常量（约束与列与常量三方对齐，
//!    绑错即拿 A 列词表钉 B 列的漂移）。
//! 3. 裁定 4 的稳定码形态：安全类别必须带 `GB18401_` 前缀、目标市场内销码必须是 `dom`，
//!    任何"顺手简化回裸 A/B/C 或 domestic"的改动直接判红。
//! 4. m0113a 与 m0113b 两支携带的 ALLOWED 字面量逐字相等，且与
//!    purchase_inventory::purchase_inspection_result::ALL 集合相等（点名口径与收口口径同一字典）。
//! 5. 限值通配维（textile_limit_scope）与业务词表的并集关系：CATEGORIES 恰为三类别加 any、
//!    FORMS 恰为两形态加 any、CONDITION_TAGS 恰为 none 加四豁免标签。
//! 6. 等级编码表：九档 token 齐、半级编码 3-4 对 3.5、编码值互异（判定器数值比较的前提）。
//! 7. 静态链位：textile_safety 域与 purchase_inspection_result_check 域在 lib.rs 注册晚于
//!    wage_energy_vocab_check，且 textile 三支的域内次序为先建表、再 CHECK、后播种。
//! 8. m0112 种子的项目编码唯一出现（ON CONFLICT 幂等的键存在）。
//!
//! 为什么双向相等而不是包含：CHECK 比词表宽，旁路脚本能写出业务永不产生的脏 token；
//! CHECK 比词表窄，某个合法值写入撞 23514 冒裸 500。两侧必须逐元素相等才是同一字典。

use bingxi_backend::models::status::{
    purchase_inventory::purchase_inspection_result, textile_safety::*,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

const M0111_REL: &str =
    "migration/src/domain/textile_safety/m0111_apply_textile_safety_vocab_checks.rs";
const M0112_REL: &str = "migration/src/domain/textile_safety/m0112_seed_textile_safety_matrix.rs";
const M0113A_REL: &str = "migration/src/domain/purchase_inspection_result_check/m0113a_report_purchase_inspection_result_dirty_values.rs";
const M0113B_REL: &str = "migration/src/domain/purchase_inspection_result_check/m0113b_apply_purchase_inspection_result_check.rs";
const LIB_REL: &str = "migration/src/lib.rs";
const TEXTILE_DOMAIN_MOD_REL: &str = "migration/src/domain/textile_safety/mod.rs";

/// 提取文本里全部单引号 token（与 wage 纸面锁同口径；引号不闭合直接 panic，形态漂移判红）。
fn tokens_in(text: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let mut cursor = text;
    while let Some(open) = cursor.find('\'') {
        cursor = &cursor[open + 1..];
        let close = cursor
            .find('\'')
            .unwrap_or_else(|| panic!("取值单引号未闭合: {text}"));
        set.insert(cursor[..close].to_string());
        cursor = &cursor[close + 1..];
    }
    set
}

/// 从迁移原文抽 `const {name}: &str = "'a','b'";` 的取值集合（锚点缺失即 panic）。
fn parse_allowed_const(src: &str, name: &str, file: &str) -> BTreeSet<String> {
    let anchor = format!("const {name}: &str =");
    let start = src
        .find(&anchor)
        .unwrap_or_else(|| panic!("{file} 缺少常量锚点 {anchor:?}（取值种子形态已改动）"));
    let rest = src[start + anchor.len()..].trim_start();
    let rest = rest
        .strip_prefix('"')
        .unwrap_or_else(|| panic!("{file} 的 {name} 在锚点后未见字面量起始引号，形态已改动"));
    let end = rest
        .find("\";")
        .unwrap_or_else(|| panic!("{file} 的 {name} 字面量未闭合"));
    tokens_in(&rest[..end])
}

fn from_consts(vals: &[&str]) -> BTreeSet<String> {
    vals.iter().map(|s| s.to_string()).collect()
}

/// 约束名、列名、ALLOWED 常量名、期望词表集合的四方对齐表（Rust 侧真源逐项引用常量）。
fn expected_bindings() -> Vec<(&'static str, &'static str, &'static str, BTreeSet<String>)> {
    vec![
        (
            "chk_textile_test_item_item_kind",
            "item_kind",
            "ITEM_KIND_ALLOWED",
            from_consts(textile_item_kind::ALL),
        ),
        (
            "chk_textile_test_item_basis_standard",
            "basis_standard",
            "BASIS_STANDARD_ALLOWED",
            from_consts(textile_basis_standard::ALL),
        ),
        (
            "chk_textile_test_item_check_kind",
            "check_kind",
            "CHECK_KIND_ALLOWED",
            from_consts(textile_check_kind::ALL),
        ),
        (
            "chk_textile_limit_rule_safety_category",
            "safety_category",
            "LIMIT_CATEGORY_ALLOWED",
            from_consts(textile_limit_scope::CATEGORIES),
        ),
        (
            "chk_textile_limit_rule_product_form",
            "product_form",
            "LIMIT_FORM_ALLOWED",
            from_consts(textile_limit_scope::FORMS),
        ),
        (
            "chk_textile_limit_rule_condition_tag",
            "condition_tag",
            "LIMIT_TAG_ALLOWED",
            from_consts(textile_limit_scope::CONDITION_TAGS),
        ),
        (
            "chk_textile_limit_rule_comparator",
            "comparator",
            "COMPARATOR_ALLOWED",
            from_consts(textile_comparator::ALL),
        ),
        (
            "chk_textile_safety_submission_end_user_group",
            "end_user_group",
            "END_USER_GROUP_ALLOWED",
            from_consts(textile_end_user_group::ALL),
        ),
        (
            "chk_textile_safety_submission_contact_type",
            "contact_type",
            "CONTACT_TYPE_ALLOWED",
            from_consts(textile_contact_type::ALL),
        ),
        (
            "chk_textile_safety_submission_safety_category",
            "safety_category",
            "SUB_CATEGORY_ALLOWED",
            from_consts(textile_safety_category::ALL),
        ),
        (
            "chk_textile_safety_submission_product_form",
            "product_form",
            "SUB_FORM_ALLOWED",
            from_consts(textile_product_form::ALL),
        ),
        (
            "chk_textile_safety_submission_target_market",
            "target_market",
            "TARGET_MARKET_ALLOWED",
            from_consts(export_market::ALL),
        ),
        (
            "chk_textile_safety_submission_color_role",
            "color_role",
            "COLOR_ROLE_ALLOWED",
            from_consts(sampling_color_role::ALL),
        ),
        (
            "chk_textile_safety_submission_status",
            "status",
            "SUBMISSION_STATUS_ALLOWED",
            from_consts(submission_status::ALL),
        ),
        (
            "chk_textile_safety_submission_overall_verdict",
            "overall_verdict",
            "OVERALL_VERDICT_ALLOWED",
            from_consts(textile_verdict::OVERALL),
        ),
        (
            "chk_textile_safety_submission_item_source",
            "source",
            "ITEM_SOURCE_ALLOWED",
            from_consts(textile_item_source::ALL),
        ),
        (
            "chk_textile_third_party_report_acceptance_status",
            "acceptance_status",
            "ACCEPTANCE_STATUS_ALLOWED",
            from_consts(report_acceptance_status::ALL),
        ),
        (
            "chk_textile_third_party_report_oeko_tex_class",
            "oeko_tex_class",
            "OEKO_CLASS_ALLOWED",
            from_consts(oeko_tex_class::ALL),
        ),
        (
            "chk_textile_report_item_result_verdict",
            "verdict",
            "RESULT_VERDICT_ALLOWED",
            from_consts(textile_verdict::ALL),
        ),
        (
            "chk_textile_child_safety_check_check_kind",
            "check_kind",
            "CHECK_KIND_ALLOWED",
            from_consts(textile_check_kind::ALL),
        ),
        (
            "chk_textile_child_safety_check_manual_conclusion",
            "manual_conclusion",
            "MANUAL_CONCLUSION_ALLOWED",
            from_consts(textile_verdict::OVERALL),
        ),
        (
            "chk_textile_child_safety_check_verdict",
            "verdict",
            "RESULT_VERDICT_ALLOWED",
            from_consts(textile_verdict::ALL),
        ),
        (
            "chk_textile_export_market_requirement_market",
            "market",
            "REQ_MARKET_ALLOWED",
            from_consts(&[
                export_market::EU,
                export_market::US,
                export_market::JP,
                export_market::OTHER,
            ]),
        ),
        (
            "chk_textile_export_market_requirement_requirement_type",
            "requirement_type",
            "REQ_TYPE_ALLOWED",
            from_consts(export_requirement_type::ALL),
        ),
        (
            "chk_textile_export_market_requirement_oeko_tex_class",
            "oeko_tex_class",
            "OEKO_CLASS_ALLOWED",
            from_consts(oeko_tex_class::ALL),
        ),
    ]
}

/// 1+2. m0111 每份 ALLOWED 常量与词表双向相等，且约束名与列名与常量的绑定在原文对齐。
#[test]
fn m0111_allowed_consts_match_vocab_and_bindings() {
    let src = read_rel(M0111_REL);
    for (constraint, column, const_name, expected) in expected_bindings() {
        let actual = parse_allowed_const(&src, const_name, M0111_REL);
        assert_eq!(
            actual, expected,
            "m0111 常量 {const_name}（约束 {constraint}）与 Rust 词表集合不相等：迁移侧多出或残缺 token"
        );
        // 绑定对齐：ColumnCheck 配方里该约束必须绑到该常量并钉在该列上
        let anchor = format!("constraint: \"{constraint}\",");
        let pos = src
            .find(&anchor)
            .unwrap_or_else(|| panic!("m0111 缺少约束 {constraint} 的 ColumnCheck 配方锚点"));
        let window = &src[pos.saturating_sub(400)..(pos + 400).min(src.len())];
        assert!(
            window.contains(&format!("allowed: {const_name},")),
            "约束 {constraint} 未绑定常量 {const_name}（绑定漂移，同名词表可能被错钉到别的列）"
        );
        assert!(
            window.contains(&format!("column: \"{column}\",")),
            "约束 {constraint} 未绑定列 {column}"
        );
    }
}

/// 3. 裁定 4 的稳定码：类别必须带前缀，内销码必须是 dom（防止改回裸字母或 domestic）。
#[test]
fn ruling4_prefixed_codes_pinned() {
    assert_eq!(textile_safety_category::CLASS_A, "GB18401_A");
    assert_eq!(textile_safety_category::CLASS_B, "GB18401_B");
    assert_eq!(textile_safety_category::CLASS_C, "GB18401_C");
    assert_eq!(export_market::DOM, "dom");
    assert_eq!(export_market::EU, "eu");
    assert_eq!(export_market::US, "us");
    assert_eq!(export_market::JP, "jp");
    assert_eq!(export_market::OTHER, "other");
    // 展示文本只在换算函数里出现，存储 token 不得混入中文形态
    assert_eq!(
        textile_safety_category::display_label(textile_safety_category::CLASS_B),
        Some("B类")
    );
    assert_eq!(textile_safety_category::display_label("B类"), None);
    assert!(!textile_safety_category::is_valid("B"));
    assert!(!export_market::is_valid("domestic"));
}

/// 4. m0113 两支 ALLOWED 逐字相等且与采购质检词表集合相等（点名口径即收口口径）。
#[test]
fn m0113_pair_matches_purchase_vocab() {
    let a = read_rel(M0113A_REL);
    let b = read_rel(M0113B_REL);
    let set_a = parse_allowed_const(&a, "PURCHASE_INSPECTION_RESULT_ALLOWED", M0113A_REL);
    let set_b = parse_allowed_const(&b, "PURCHASE_INSPECTION_RESULT_ALLOWED", M0113B_REL);
    let expected = from_consts(purchase_inspection_result::ALL);
    assert_eq!(
        set_a, set_b,
        "m0113a 与 m0113b 词表字面量分叉（点名口径与收口口径必须同一字典）"
    );
    assert_eq!(
        set_a, expected,
        "m0113 词表与 purchase_inspection_result::ALL 不相等（任一侧单独增删值都红）"
    );
}

/// 5. 限值通配维与业务词表的并集关系逐项钉死。
#[test]
fn limit_scope_is_business_vocab_plus_wildcard() {
    assert_eq!(from_consts(textile_limit_scope::CATEGORIES), {
        let mut s = from_consts(textile_safety_category::ALL);
        s.insert("any".to_string());
        s
    });
    assert_eq!(from_consts(textile_limit_scope::FORMS), {
        let mut s = from_consts(textile_product_form::ALL);
        s.insert("any".to_string());
        s
    });
    assert_eq!(from_consts(textile_limit_scope::CONDITION_TAGS), {
        let mut s = from_consts(textile_exemption_tag::ALL);
        s.insert("none".to_string());
        s
    });
}

/// 6. 等级编码表：九档齐、3-4 对 3.5、编码互异（半级取值域含四个半级）。
#[test]
fn fastness_grade_encoding_table() {
    assert_eq!(textile_fastness_grade::ALL.len(), 9);
    assert_eq!(textile_fastness_grade::encode("3-4"), Some("3.5"));
    assert_eq!(textile_fastness_grade::encode("4-5"), Some("4.5"));
    assert_eq!(textile_fastness_grade::encode("0"), None);
    let mut encodings = BTreeSet::new();
    for (token, value) in textile_fastness_grade::TOKENS {
        assert_eq!(
            textile_fastness_grade::encode(token),
            Some(*value),
            "等级 token {token} 的编码与对照表不一致"
        );
        assert!(
            encodings.insert(*value),
            "等级编码值 {value} 重复，数值比较将歧义"
        );
    }
}

/// 7. 静态链位：两支新域晚于 wage 域注册，textile 域内三支次序固定为先建表后 CHECK 再播种。
#[test]
fn chain_position_registered_after_wage_and_ordered_in_domain() {
    let lib = read_rel(LIB_REL);
    let wage = lib
        .find("domain::wage_energy_vocab_check::Migration")
        .expect("lib.rs 缺 wage 域注册锚点");
    let textile = lib
        .find("domain::textile_safety::Migration")
        .expect("lib.rs 缺 textile_safety 域注册锚点");
    let pi = lib
        .find("domain::purchase_inspection_result_check::Migration")
        .expect("lib.rs 缺 purchase_inspection_result_check 域注册锚点");
    assert!(
        wage < textile,
        "textile_safety 域必须晚于 wage_energy_vocab_check 注册"
    );
    assert!(
        textile < pi,
        "purchase_inspection_result_check 域必须晚于 textile_safety 注册"
    );

    let dm = read_rel(TEXTILE_DOMAIN_MOD_REL);
    let create = dm
        .find("m0110_create_textile_safety_tables::Migration")
        .expect("域 mod 缺建表锚点");
    let check = dm
        .find("m0111_apply_textile_safety_vocab_checks::Migration")
        .expect("域 mod 缺 CHECK 锚点");
    let seed = dm
        .find("m0112_seed_textile_safety_matrix::Migration")
        .expect("域 mod 缺播种锚点");
    assert!(
        create < check && check < seed,
        "域内 up 链次序必须是建表、CHECK、播种（种子行必须受已生效 CHECK 校验）"
    );
}

/// 8. m0112 每枚种子项目编码都以 VALUES 元组形态出现（ON CONFLICT (item_code) 幂等键存在），
///    且 manual 条款项在全文件不挂任何限值行（只有人工结论位，不许被自动判定误消费）。
#[test]
fn seed_item_codes_present_and_manual_items_have_no_rules() {
    let src = read_rel(M0112_REL);
    for code in [
        "HCHO",
        "PH",
        "AZO",
        "ODOR",
        "FAST_WATER",
        "FAST_ACID_SWEAT",
        "FAST_ALKALI_SWEAT",
        "FAST_DRY_RUB",
        "FAST_WET_RUB",
        "FAST_SALIVA",
        "LEAD",
        "CADMIUM",
        "PHTHALATE",
        "FLAM_EXTERIOR",
        "METAL_SHARP",
        "ACCESSOR_TENSILE",
        "SHARP_EDGE",
        "SMALL_PARTS",
        "CORD_HEAD",
        "CORD_WAIST",
        "LABEL_POSITION",
    ] {
        let needle = format!("('{code}', '");
        assert!(
            src.contains(&needle),
            "m0112 缺项目 {code} 的 VALUES 元组（种子唯一键形态漂移）"
        );
    }
    for manual in [
        "ACCESSOR_TENSILE",
        "SHARP_EDGE",
        "SMALL_PARTS",
        "LABEL_POSITION",
    ] {
        let rule_anchor = format!("WHERE item_code = '{manual}'");
        assert!(
            !src.contains(&rule_anchor),
            "manual 条款项 {manual} 不应挂任何限值行（它只有人工结论位）"
        );
    }
}
