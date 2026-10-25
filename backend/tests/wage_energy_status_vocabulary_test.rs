//! 工资与能耗十三列「Rust 词表常量 ↔ 迁移 CHECK 取值集」纸面双向锁（不连库，CI 常规可跑）。
//!
//! 钉什么：
//! 1. 两支同域迁移（点名 m0102 与收口 m0103）携带的八份 ALLOWED 常量逐字相等，
//!    防止点名口径与收口口径分叉成两套词表。
//! 2. m0103 的 COLUMN_CHECKS 把每个约束名绑到哪一个 ALLOWED 常量，与权威词表模块常量
//!    双向集合相等，多一个少一个都判红，词表侧与 CHECK 侧任一侧单独增删值都红。
//! 3. m0103 施加 CHECK 走 `{allowed}` 占位单源注入，不在 ADD CONSTRAINT 里内联第二套清单。
//! 4. 全部取值都是小写纯 ASCII，中文或大写漂移进词表或 CHECK 即红。
//! 5. 静态链位：本域在 lib.rs 注册下标晚于建表域，防止 CHECK 排到建表之前。
//!
//! 为什么双向相等而不是包含：CHECK 比词表宽，旁路脚本能写出业务永不产生的脏 token；
//! CHECK 比词表窄，某个合法值写入撞 23514 冒裸 500。两侧必须逐元素相等才是同一字典。

use bingxi_backend::models::status::{
    energy_allocation_basis, energy_meter_status, energy_record_status, energy_recording_method,
    energy_rule_status, energy_type, wage_rate_status, wage_record_status,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

const M0102_REL: &str =
    "migration/src/domain/wage_energy_vocab_check/m0102_report_wage_energy_vocab_dirty_values.rs";
const M0103_REL: &str =
    "migration/src/domain/wage_energy_vocab_check/m0103_apply_wage_energy_vocab_checks.rs";

/// 八份 ALLOWED 常量名（m0102 与 m0103 各携带一份，两支必须逐字相等）。
const ALLOWED_CONST_NAMES: &[&str] = &[
    "WAGE_RATE_STATUS_ALLOWED",
    "WAGE_RECORD_STATUS_ALLOWED",
    "ENERGY_METER_STATUS_ALLOWED",
    "ENERGY_RECORD_STATUS_ALLOWED",
    "ENERGY_RECORDING_METHOD_ALLOWED",
    "ENERGY_TYPE_ALLOWED",
    "ENERGY_RULE_STATUS_ALLOWED",
    "ENERGY_ALLOCATION_BASIS_ALLOWED",
];

/// 从迁移原文抽 `const {name}: &str = "'a','b'";` 的取值集合（锚点缺失即 panic，
/// 形态漂移同样判红，禁止静默产出空集让相等退化成两边都空）。
fn parse_allowed_const(src: &str, name: &str, file: &str) -> BTreeSet<String> {
    let anchor = format!("const {name}: &str =");
    let start = src
        .find(&anchor)
        .unwrap_or_else(|| panic!("{file} 缺少常量锚点 {anchor:?}（取值种子形态已改动）"));
    // 等号后可换行缩进（长字面量会被行宽折到下一行），跳空白后必须紧跟起始引号。
    let rest = src[start + anchor.len()..].trim_start();
    let rest = rest
        .strip_prefix('"')
        .unwrap_or_else(|| panic!("{file} 的 {name} 在锚点后未见字面量起始引号，形态已改动"));
    let end = rest
        .find("\";")
        .unwrap_or_else(|| panic!("{file} 的 {name} 字面量未闭合"));
    tokens_in(&rest[..end])
}

/// 提取文本里全部单引号 token，引号不闭合直接 panic。
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

/// 约束名到它绑定的 ALLOWED 常量名与列名的映射，来自 m0103 的 COLUMN_CHECKS 原文。
fn parse_column_checks(src: &str) -> Vec<(String, String, String)> {
    let re = regex::Regex::new(r"(?s)ColumnCheck \{.*?\}").unwrap();
    let tbl = regex::Regex::new(r#"column: "([^"]+)""#).unwrap();
    let con = regex::Regex::new(r#"constraint: "([^"]+)""#).unwrap();
    let allowed = regex::Regex::new(r"allowed: ([A-Z_]+),").unwrap();
    let up_pos = src
        .find("async fn up")
        .unwrap_or_else(|| panic!("m0103 未找到 up 段，无法定位 COLUMN_CHECKS 声明区"));
    let decl_region = &src[..up_pos];
    let mut out = Vec::new();
    for m in re.captures_iter(decl_region) {
        let blk = m.get(0).unwrap().as_str();
        // 表名不参与返回，约束名全局唯一已可归因。
        let c = con
            .captures(blk)
            .unwrap_or_else(|| panic!("COLUMN_CHECKS 行缺 constraint 字段"))
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        let col = tbl
            .captures(blk)
            .unwrap_or_else(|| panic!("COLUMN_CHECKS 行缺 column 字段"))
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        let al = allowed
            .captures(blk)
            .unwrap_or_else(|| panic!("COLUMN_CHECKS 行缺 allowed 绑定"))
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        out.push((c, col, al));
    }
    out
}

/// 每个约束名到权威词表模块常量集合的期望映射（Rust 侧真源，逐个引常量不写字面量）。
fn expected_vocab_sets() -> Vec<(String, BTreeSet<String>)> {
    let from = |vals: &[&str]| -> BTreeSet<String> { vals.iter().map(|s| s.to_string()).collect() };
    vec![
        (
            "chk_process_wage_rate_status".into(),
            from(&[
                wage_rate_status::DRAFT,
                wage_rate_status::ACTIVE,
                wage_rate_status::DISABLED,
            ]),
        ),
        (
            "chk_wage_record_status".into(),
            from(&[
                wage_record_status::DRAFT,
                wage_record_status::CONFIRMED,
                wage_record_status::PAID,
                wage_record_status::CANCELLED,
            ]),
        ),
        (
            "chk_energy_meter_status".into(),
            from(&[
                energy_meter_status::ACTIVE,
                energy_meter_status::INACTIVE,
                energy_meter_status::MAINTENANCE,
            ]),
        ),
        (
            "chk_energy_consumption_record_status".into(),
            from(&[
                energy_record_status::DRAFT,
                energy_record_status::CONFIRMED,
                energy_record_status::CANCELLED,
            ]),
        ),
        (
            "chk_energy_consumption_record_recording_method".into(),
            from(&[
                energy_recording_method::MANUAL,
                energy_recording_method::IOT,
                energy_recording_method::AUTO_CALC,
            ]),
        ),
        (
            "chk_energy_consumption_record_meter_type".into(),
            from(&[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ]),
        ),
        (
            "chk_energy_allocation_rule_status".into(),
            from(&[
                energy_rule_status::DRAFT,
                energy_rule_status::ACTIVE,
                energy_rule_status::DISABLED,
            ]),
        ),
        (
            "chk_energy_allocation_rule_allocation_basis".into(),
            from(&[
                energy_allocation_basis::BY_DURATION,
                energy_allocation_basis::BY_OUTPUT,
                energy_allocation_basis::BY_EQUIPMENT,
                energy_allocation_basis::BY_WORKSHOP,
            ]),
        ),
        (
            "chk_energy_allocation_rule_meter_type".into(),
            from(&[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ]),
        ),
        (
            "chk_energy_allocation_record_status".into(),
            from(&[
                energy_record_status::DRAFT,
                energy_record_status::CONFIRMED,
                energy_record_status::CANCELLED,
            ]),
        ),
        (
            "chk_energy_allocation_record_allocation_basis".into(),
            from(&[
                energy_allocation_basis::BY_DURATION,
                energy_allocation_basis::BY_OUTPUT,
                energy_allocation_basis::BY_EQUIPMENT,
                energy_allocation_basis::BY_WORKSHOP,
            ]),
        ),
        (
            "chk_energy_allocation_record_meter_type".into(),
            from(&[
                energy_type::WATER,
                energy_type::ELECTRICITY,
                energy_type::STEAM,
                energy_type::GAS,
                energy_type::COMPRESSED_AIR,
            ]),
        ),
    ]
}

/// 1. 两支迁移携带的八份 ALLOWED 常量逐字相等（点名口径不得与收口口径分叉）。
#[test]
fn two_branch_migrations_share_identical_allowed_consts() {
    let m0102_src = read_rel(M0102_REL);
    let m0103_src = read_rel(M0103_REL);
    for name in ALLOWED_CONST_NAMES {
        let from_0102 = parse_allowed_const(&m0102_src, name, M0102_REL);
        let from_0103 = parse_allowed_const(&m0103_src, name, M0103_REL);
        assert_eq!(
            from_0102, from_0103,
            "点名 m0102 与收口 m0103 的常量 {name} 取值集分叉，两支必须共用同一份清单：\n\
             m0102={from_0102:?}\nm0103={from_0103:?}"
        );
    }
}

/// 2. 每列 CHECK 绑定的取值集与 Rust 词表常量集合双向相等（多一个少一个都红）。
#[test]
fn each_column_check_set_equals_rust_vocab_bidirectionally() {
    let m0103_src = read_rel(M0103_REL);
    let rows = parse_column_checks(&m0103_src);
    assert_eq!(
        rows.len(),
        13,
        "m0103 的 COLUMN_CHECKS 必须是十三行，实得 {} 行",
        rows.len()
    );

    // 约束名到它绑定的 ALLOWED 常量解析出的取值集
    let mut check_by_constraint: Vec<(String, BTreeSet<String>)> = Vec::new();
    for (constraint, _col, allowed_const) in &rows {
        let set = parse_allowed_const(&m0103_src, allowed_const, M0103_REL);
        assert!(
            !set.is_empty(),
            "约束 {constraint} 绑定的常量 {allowed_const} 解析出空集，判据失守"
        );
        check_by_constraint.push((constraint.clone(), set));
    }

    for (constraint, expected) in expected_vocab_sets() {
        let got = check_by_constraint
            .iter()
            .find(|(key, _)| key == &constraint)
            .map(|(_, set)| set)
            .unwrap_or_else(|| panic!("m0103 的 COLUMN_CHECKS 未覆盖约束 {constraint}"));
        assert_eq!(
            got, &expected,
            "约束 {constraint} 的 CHECK 取值集与 Rust 词表常量集合不等（双向集合相等）：\n\
             CHECK侧={got:?}\n词表侧={expected:?}"
        );
    }
}

/// 3. m0103 施加 CHECK 走 {allowed} 占位单源注入，不在 ADD CONSTRAINT 内联第二套清单。
#[test]
fn m0103_check_uses_allowed_placeholder_single_source() {
    let m0103_src = read_rel(M0103_REL);
    assert!(
        m0103_src.contains("ADD CONSTRAINT \"{constraint}\" CHECK (\"{col}\" IN ({allowed}))"),
        "m0103 的 ADD CONSTRAINT 必须是 `\"{{col}}\" IN ({{{{allowed}}}})` 占位形态，\
         取值域唯一由 ALLOWED 常量注入；改成内联清单即第二套词表漂移"
    );
    // 守卫同用一份 allowed，NULL 或 NOT IN 判据不得夹带额外值集合
    assert!(
        m0103_src.contains("IS NULL OR \"{col}\" NOT IN ({allowed})"),
        "m0103 的存量守卫谓词必须是 `IS NULL OR \"{{col}}\" NOT IN ({{{{allowed}}}})`，\
         与 CHECK 共用同一份 ALLOWED，否则守卫放行的值 CHECK 会拒"
    );
}

/// 4. 全部取值都是小写纯 ASCII，禁止中文或大写漂移。
#[test]
fn all_vocab_values_are_lowercase_ascii() {
    for (_constraint, set) in expected_vocab_sets() {
        for v in set {
            assert!(
                v.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "取值 {v} 含非小写 ASCII 字符（疑似中文或大写漂移）"
            );
        }
    }
    let m0103_src = read_rel(M0103_REL);
    for name in ALLOWED_CONST_NAMES {
        for v in parse_allowed_const(&m0103_src, name, M0103_REL) {
            assert!(
                v.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "m0103 常量 {name} 取值 {v} 含非小写 ASCII 字符"
            );
        }
    }
}

/// 5. 静态链位锁：本域注册下标晚于建表域，防 CHECK 排到建表之前。
#[test]
fn domain_registered_after_table_creation_domain_in_lib() {
    let lib = read_rel("migration/src/lib.rs");
    let up_pos = |needle: &str| -> usize {
        lib.find(needle)
            .unwrap_or_else(|| panic!("lib.rs 未登记 {needle}，本域或建表域注册被挪走"))
    };
    let v15_pos = up_pos("domain::v15::Migration");
    let self_pos = up_pos("domain::wage_energy_vocab_check::Migration");
    assert!(
        self_pos > v15_pos,
        "wage_energy_vocab_check 必须注册在 v15 建表域之后（CHECK 晚于建表），\n\
         实际 v15 偏移 {v15_pos}，本域偏移 {self_pos}"
    );
}
