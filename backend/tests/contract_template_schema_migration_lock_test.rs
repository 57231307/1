//! 合同模板体系词表 ↔ 迁移 CHECK 取值集静态锁（CI 常规可跑，不依赖活库）
//!
//! 功能 钉住合同模板体系八组取值域在两侧同为一套，写入方权威词表是
//!   `models/status/contract_template.rs` 的各 `ALL`，数据库侧兜底是
//!   `migration/src/domain/contract_template_schema/mod.rs` 的八枚词表常量。
//!   本锁逐组比对两侧取值集**集合相等**，并钉住「迁移的值域只能由常量单点注入 CHECK」
//!   这一接线形态，防止有人另抄一套清单进 SQL 字符串。
//! 调用方 CI 的 Rust 测试 job 与本仓本地门。入参 无，读两份源文件原文与词表常量。
//! 传给谁 无下游，纯断言。存什么 不写库、不写文件。存哪里 不落地。
//!
//! 判据是集合逐元素相等，不是包含
//! - 词表加值而 CHECK 未加 合法写入撞 23514，冒无据可依的裸错误；
//! - CHECK 比词表宽 旁路脚本能写出业务永不产生的脏 token，读取侧按词表分支时静默漏判。
//! 形态照 `tests/fulfillment_source_vocab_check_parity_test.rs` 的同族锁，从源文件按锚点
//! 解析，解析失守（锚点或引号或占位形态变化）直接 panic 点名，绝不解析出空集就当通过。

use bingxi_backend::models::status::contract_template::contract_applies_when;
use bingxi_backend::models::status::contract_template::contract_clause_code;
use bingxi_backend::models::status::contract_template::contract_legal_review_status;
use bingxi_backend::models::status::contract_template::contract_market;
use bingxi_backend::models::status::contract_template::contract_party_side;
use bingxi_backend::models::status::contract_template::contract_side_kind;
use bingxi_backend::models::status::contract_template::contract_template_status;
use bingxi_backend::models::status::contract_template::contract_type;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// 承载八组词表常量的迁移域原文（域名与台账名同源，改名即红）。
const MIGRATION_REL: &str = "migration/src/domain/contract_template_schema/mod.rs";
/// 应用侧权威词表原文。
const VOCAB_REL: &str = "src/models/status/contract_template.rs";

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}（路径或改名需同步本锁）"))
}

/// 从文本中提取 `const <name>: &str = "..."` 字面量里的全部单引号 token。
/// 锚点缺失、引号未闭合、解析出空集都点名 panic。
fn parse_const_tokens(src: &str, name: &str) -> BTreeSet<String> {
    let anchor = format!("const {name}: &str =");
    let start = src
        .find(&anchor)
        .unwrap_or_else(|| panic!("源文件缺少常量 {name}（取值清单的种子形态被改动）"));
    let rest = &src[start..];
    let open = rest.find('"').expect("常量字面量起始引号缺失");
    let body = &rest[open + 1..];
    let close = body
        .find("\";")
        .expect("常量字面量未闭合（引号分号收尾形态被改动）");
    let literal = &body[..close];
    let mut set = BTreeSet::new();
    let mut cursor = literal;
    while let Some(o) = cursor.find('\'') {
        cursor = &cursor[o + 1..];
        let c = cursor.find('\'').expect("常量内单引号未闭合");
        set.insert(cursor[..c].to_string());
        cursor = &cursor[c + 1..];
    }
    assert!(
        !set.is_empty(),
        "常量 {name} 解析出空集——解析失守，拒绝继续比对"
    );
    set
}

/// 从应用侧词表文件提取某个 `pub mod` 内 `ALL: &[&str] = &[...]` 引用的常量名，
/// 再逐个取同文件里的 `pub const 名: &str = "值"` 字面值，组成取值集。
/// 这样比对的是真实 token，不是标识符拼写。
fn parse_vocab_all(src: &str, module: &str) -> BTreeSet<String> {
    let mod_anchor = format!("pub mod {module} {{");
    let start = src
        .find(&mod_anchor)
        .unwrap_or_else(|| panic!("词表缺少 pub mod {module}（分组形态被改动）"));
    let rest = &src[start + mod_anchor.len()..];
    let all_at = rest
        .find("pub const ALL: &[&str] = &[")
        .unwrap_or_else(|| panic!("pub mod {module} 内缺少 ALL 清单（唯一取值来源被改动）"));
    let after_all = &rest[all_at + "pub const ALL: &[&str] = &[".len()..];
    let close = after_all.find("];").expect("ALL 清单未闭合");
    let list = &after_all[..close];
    let mut set = BTreeSet::new();
    for ident in list.split(',') {
        let ident = ident.trim();
        if ident.is_empty() {
            continue;
        }
        let value_anchor = format!("pub const {ident}: &str =");
        let v_start = src.find(&value_anchor).unwrap_or_else(|| {
            panic!("ALL 引用了常量 {ident}，但词表内没有它的字面值（清单与定义已脱节）")
        });
        let v_rest = &src[v_start + value_anchor.len()..];
        let o = v_rest.find('"').expect("常量字面量起始引号缺失");
        let v_body = &v_rest[o + 1..];
        let c = v_body.find('"').expect("常量字面量结束引号缺失");
        set.insert(v_body[..c].to_string());
    }
    assert!(
        !set.is_empty(),
        "词表 pub mod {module} 的 ALL 解析出空集——解析失守，拒绝继续比对"
    );
    set
}

fn to_set(tokens: &[&str]) -> BTreeSet<String> {
    tokens.iter().map(|t| (*t).to_string()).collect()
}

/// 逐组锁定 迁移常量 ↔ 应用词表 ALL 集合相等，并钉住常量在域内只声明一次的单点形态。
fn lock_group(mig_src: &str, vocab_src: &str, mig_const: &str, vocab_module: &str) {
    let check_set = parse_const_tokens(mig_src, mig_const);
    let vocab_set = parse_vocab_all(vocab_src, vocab_module);
    assert_eq!(
        vocab_set, check_set,
        "词表 {vocab_module}::ALL 与迁移常量 {mig_const} 必须集合相等：\n词表={vocab_set:?}\n迁移={check_set:?}"
    );
    let declarations = mig_src
        .matches(&format!("const {mig_const}: &str ="))
        .count();
    assert_eq!(
        declarations, 1,
        "迁移常量 {mig_const} 必须在域内恰好声明一次（多处声明等于两套清单，必然漂移），实得 {declarations} 处"
    );
}

#[test]
fn migration_constants_equal_authority_vocabularies_exactly() {
    let mig = read(MIGRATION_REL);
    let vocab = read(VOCAB_REL);

    lock_group(&mig, &vocab, "CONTRACT_TYPE_ALLOWED_SQL", "contract_type");
    lock_group(
        &mig,
        &vocab,
        "PARTY_SIDE_ALLOWED_SQL",
        "contract_party_side",
    );
    lock_group(
        &mig,
        &vocab,
        "TEMPLATE_STATUS_ALLOWED_SQL",
        "contract_template_status",
    );
    lock_group(
        &mig,
        &vocab,
        "CLAUSE_CODE_ALLOWED_SQL",
        "contract_clause_code",
    );
    lock_group(
        &mig,
        &vocab,
        "LEGAL_REVIEW_STATUS_ALLOWED_SQL",
        "contract_legal_review_status",
    );
    lock_group(
        &mig,
        &vocab,
        "APPLIES_WHEN_ALLOWED_SQL",
        "contract_applies_when",
    );
    lock_group(
        &mig,
        &vocab,
        "CONTRACT_KIND_ALLOWED_SQL",
        "contract_side_kind",
    );
    lock_group(
        &mig,
        &vocab,
        "CONTRACT_MARKET_ALLOWED_SQL",
        "contract_market",
    );

    // 八组闭合值数各自钉死，防止「加一个没人写的 token」被当成两侧一致通过。
    assert_eq!(contract_type::ALL.len(), 10, "合同类别是模板清单的闭合十类");
    assert_eq!(
        contract_party_side::ALL.len(),
        3,
        "我方立场是卖方与买方与对等的闭合三值"
    );
    assert_eq!(
        contract_template_status::ALL.len(),
        3,
        "模板生命周期是 draft 与 active 与 retired 的闭合三值"
    );
    assert_eq!(
        contract_clause_code::ALL.len(),
        19,
        "条款码是十六枚必备条款加三枚专章条款的闭合十九值"
    );
    assert_eq!(
        contract_legal_review_status::ALL.len(),
        3,
        "法务复核状态是 pending 与 approved 与 rejected 的闭合三值"
    );
    assert_eq!(
        contract_applies_when::ALL.len(),
        6,
        "条件出条款标记是闭合六值，空值另由 Option 层放行"
    );
    assert_eq!(
        contract_side_kind::ALL.len(),
        2,
        "合同侧别只有销售与采购两值"
    );
    assert_eq!(contract_market::ALL.len(), 2, "内外销只有内销与出口两值");
}

#[test]
fn mandatory_clause_skeleton_is_sixteen_codes_inside_the_closed_set() {
    let mandatory = to_set(contract_clause_code::MANDATORY);
    assert_eq!(
        mandatory.len(),
        16,
        "每份合同的条款骨架是十六项，缺任一项算模板不完整，实得 {mandatory:?}"
    );
    let all = to_set(contract_clause_code::ALL);
    assert!(
        mandatory.is_subset(&all),
        "十六枚必备条款码必须都在闭合集内，差集={:?}",
        mandatory.difference(&all).collect::<Vec<_>>()
    );
    // 三枚专章是闭合集减必备集的差集，逐枚点名，防止差集里混进未申报的第四枚。
    let extras: BTreeSet<String> = all.difference(&mandatory).cloned().collect();
    assert_eq!(
        extras,
        to_set(&[
            contract_clause_code::EXPORT_SPECIAL_TERMS,
            contract_clause_code::MANDATORY_SAFETY_STANDARD,
            contract_clause_code::EXPORT_CONTROL_SCREENING,
        ]),
        "必备集之外只允许三枚专章条款码，实得 {extras:?}"
    );
}

/// 取值为全小写下划线纯 ASCII：CHECK 逐字符敏感，大写或中文 token 会造成
/// 「服务层放行、DB 拒收」或反向的分裂（本仓历史缺陷同型）。空串不得成为合法取值。
#[test]
fn vocab_tokens_are_lowercase_ascii_and_non_empty() {
    let groups: &[&[&str]] = &[
        contract_type::ALL,
        contract_party_side::ALL,
        contract_template_status::ALL,
        contract_clause_code::ALL,
        contract_legal_review_status::ALL,
        contract_applies_when::ALL,
        contract_side_kind::ALL,
        contract_market::ALL,
    ];
    for group in groups {
        for token in *group {
            assert!(
                !token.is_empty() && token.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "取值 {token} 含非小写 ASCII 字符或为空（词表禁止中文与大写与空串）"
            );
        }
    }
    // is_valid 逐字符匹配，不做大小写归一，不吞尾随空白。
    assert!(!contract_type::is_valid("DOMESTIC_SALES"));
    assert!(!contract_type::is_valid("domestic_sales "));
    assert!(!contract_clause_code::is_valid(""));
    assert!(contract_clause_code::is_valid(
        contract_clause_code::SUBJECT_AND_SPEC
    ));
    assert!(!contract_side_kind::is_valid(BANNED_KIND));
    assert!(contract_side_kind::is_valid(contract_side_kind::SALES));
}

/// 越界 token 的字面值，与本文件其余断言共用一份，防止比对对象自己漂。
const BANNED_KIND: &str = "labor";

/// 本域五支子迁移的相对文件名（与域内 mod.rs 的声明同名，改名即红）。
const CHILD_MIGRATIONS: &[&str] = &[
    "m0114_create_contract_templates",
    "m0115_create_clause_library",
    "m0116_create_contract_template_clauses",
    "m0117_create_contract_clauses",
    "m0118_add_contract_main_clause_columns",
];

/// 词表清单不得被内联进子迁移。判据取每组词表的前两枚 token 拼成的签名串——
/// 手工把 `IN (...)` 清单抄进 SQL 字面量时，相邻两枚 token 必然一起被抄进来。
/// 钉这一条是因为两处各维护一套取值集就是漂移起点，词表唯一来源是域内八枚常量。
#[test]
fn vocabulary_lists_are_never_inlined_into_child_migrations() {
    let vocab_src = read(VOCAB_REL);
    let signatures: Vec<String> = [
        "contract_type",
        "contract_party_side",
        "contract_template_status",
        "contract_clause_code",
        "contract_legal_review_status",
        "contract_applies_when",
        "contract_side_kind",
        "contract_market",
    ]
    .iter()
    .map(|module| {
        let set = parse_vocab_all(&vocab_src, module);
        let first_two: Vec<&String> = set.iter().take(2).collect();
        assert_eq!(
            first_two.len(),
            2,
            "词表 {module} 不足两枚取值，签名串失去检测力"
        );
        format!("'{}','{}'", first_two[0], first_two[1])
    })
    .collect();

    for child in CHILD_MIGRATIONS {
        let rel = format!("migration/src/domain/contract_template_schema/{child}.rs");
        let text = read(&rel);
        let code_only = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for signature in &signatures {
            assert!(
                !code_only.contains(signature),
                "{rel} 内联了词表清单片段 {signature}——取值集必须由域内常量单点注入，\
                 两处各维护一套必然漂移"
            );
        }
    }
}

/// 迁移域必须注册在链尾，且台账名是域名而不是单文件名（并域会被台账整体跳过，四表全缺席）。
#[test]
fn contract_template_schema_domain_is_registered_at_chain_tail() {
    let lib = read("migration/src/lib.rs");
    let domain_mod = read("migration/src/domain/mod.rs");

    let tail = "Box::new(domain::contract_template_schema::Migration),";
    let pos = lib
        .find(tail)
        .unwrap_or_else(|| panic!("迁移链里找不到 {tail}（域未注册进 lib.rs，活库上四表必缺席）"));
    let after = &lib[pos + tail.len()..];
    assert!(
        !after.contains("Box::new(domain::"),
        "contract_template_schema 必须是链尾最后一枚注册项，其后不得再有其它域注册"
    );

    assert!(
        domain_mod.contains("pub mod contract_template_schema;"),
        "域必须在 domain/mod.rs 声明，否则 lib.rs 的注册引用不到"
    );
    let mod_src = read(MIGRATION_REL);
    assert!(
        mod_src.contains("\"m_contract_template_schema\""),
        "域必须手写 MigrationName 返回域名 m_contract_template_schema（derive 会把 mod.rs 折叠成同名 mod）"
    );
    // 五支子迁移都要在域内串起来，缺支即半张表。
    for child in [
        "m0114_create_contract_templates",
        "m0115_create_clause_library",
        "m0116_create_contract_template_clauses",
        "m0117_create_contract_clauses",
        "m0118_add_contract_main_clause_columns",
    ] {
        assert!(
            mod_src.contains(&format!("mod {child};")),
            "域内缺少子迁移 {child} 的声明"
        );
        assert!(
            mod_src.contains(&format!("{child}::Migration")),
            "域内 up 或 down 未调用子迁移 {child}"
        );
    }
}

/// 本域五支迁移不得越界写业务数据，也不得留下空 down。
/// 判据是域内 SQL 面不出现 UPDATE 与 DELETE 与 TRUNCATE 的语句形态，
/// down 侧只允许 DROP 与显式 no-op。
#[test]
fn contract_template_schema_never_touches_business_data_rows() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("migration/src/domain/contract_template_schema");
    let entries = fs::read_dir(&dir).unwrap_or_else(|e| panic!("读取迁移域目录 {dir:?} 失败: {e}"));
    let mut checked = 0usize;
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("读取目录项失败: {e}"))
            .path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {e}", path));
        let code_only = text
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for banned in ["UPDATE \"", "DELETE FROM", "TRUNCATE "] {
            assert!(
                !code_only.contains(banned),
                "{:?} 的代码面出现语句形态 {banned}——合同模板体系域绝不写业务数据行，\
                 存量脏值一律交只读探测点名后人工处置",
                path
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 5,
        "只扫到 {checked} 个源文件，说明扫描没走到迁移域目录，本锁已失去检测力"
    );
}
