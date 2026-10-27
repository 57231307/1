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
use std::collections::{BTreeMap, BTreeSet};
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
        let code_only = strip_line_comments(&text);
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

// ---------------------------------------------------------------------------
// 结构面与次序面的文本锁（不依赖活库，CI 常规可跑）
// ---------------------------------------------------------------------------

/// 去掉整行注释后的源码，SQL 字面量里的行首 // 不存在，注释行一律不参与判定。
fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取 `async fn <sig>` 的函数体（按大括号配平截断，取不到直接 panic 点名）。
fn fn_body(src: &str, sig: &str) -> String {
    let at = src
        .find(sig)
        .unwrap_or_else(|| panic!("源文件里找不到 {sig}（函数形态或改名使本锁失去检测对象）"));
    let rest = &src[at..];
    let bytes = rest.as_bytes();
    let mut depth = 0usize;
    let mut started = false;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'{' {
            depth += 1;
            started = true;
        } else if *b == b'}' && started {
            depth -= 1;
            if depth == 0 {
                return rest[..i + 1].to_string();
            }
        }
    }
    panic!("{sig} 的函数体大括号未配平，无法判定次序")
}

/// 子迁移文件名与其建的四张表（m0118 是补列，不建表，故不在表名单内）。
const CHILD_TABLES: &[(&str, &str)] = &[
    ("m0114_create_contract_templates", "contract_templates"),
    ("m0115_create_clause_library", "clause_library"),
    (
        "m0116_create_contract_template_clauses",
        "contract_template_clauses",
    ),
    ("m0117_create_contract_clauses", "contract_clauses"),
];

/// 列规格，name 与 not_null 一律按迁移文本与实体逐列核定。
struct ColSpec {
    name: &'static str,
    not_null: bool,
}

const fn col(name: &'static str, not_null: bool) -> ColSpec {
    ColSpec { name, not_null }
}

/// contract_templates 的全部列与可空性。
const CONTRACT_TEMPLATES_COLS: &[ColSpec] = &[
    col("id", true),
    col("template_code", true),
    col("contract_type", true),
    col("party_side", true),
    col("version", true),
    col("title", true),
    col("status", true),
    col("effective_from", false),
    col("created_by", true),
    col("approved_by", false),
    col("approved_at", false),
    col("remark", false),
    col("created_at", true),
    col("updated_at", true),
];

/// clause_library 的全部列与可空性。
const CLAUSE_LIBRARY_COLS: &[ColSpec] = &[
    col("id", true),
    col("clause_code", true),
    col("title", true),
    col("body", true),
    col("applies_contract_type", false),
    col("status", true),
    col("remark", false),
    col("created_by", true),
    col("created_at", true),
    col("updated_at", true),
];

/// contract_template_clauses 的全部列与可空性。body 与 library_clause_id 列侧可空，
/// 二选一由 CHECK 钉住，故实体两侧都是 Option。
const CONTRACT_TEMPLATE_CLAUSES_COLS: &[ColSpec] = &[
    col("id", true),
    col("template_id", true),
    col("clause_code", true),
    col("title", true),
    col("body", false),
    col("library_clause_id", false),
    col("required", true),
    col("sort", true),
    col("applies_when", false),
    col("legal_review_status", true),
    col("legal_reviewed_by", false),
    col("legal_reviewed_at", false),
    col("legal_review_remark", false),
    col("created_at", true),
    col("updated_at", true),
];

/// contract_clauses 快照的全部列与可空性。快照行要么整条存在要么不存在，
/// 所以正文与模板版本一律 NOT NULL，不给空串与零值冒充已固化。
const CONTRACT_CLAUSES_COLS: &[ColSpec] = &[
    col("id", true),
    col("contract_kind", true),
    col("contract_id", true),
    col("clause_code", true),
    col("title", true),
    col("body_rendered", true),
    col("template_id", true),
    col("template_version", true),
    col("sort", true),
    col("created_at", true),
    col("updated_at", true),
];

/// m0118 补列的两侧合同主表名。
const CONTRACT_MAIN_TABLES: &[&str] = &["sales_contracts", "purchase_contracts"];

/// m0118 补到两侧合同主表的列，全部可空且无默认（历史合同保持 NULL 的判据）。
const CONTRACT_MAIN_ADDED_COLS: &[&str] = &[
    "quality_terms",
    "breach_liability",
    "dispute_resolution",
    "performance_period",
    "signed_at",
    "signed_by_user_id",
    "signature_hash",
    "signature_image_url",
    "signature_certificate",
    "stamp_tax_amount",
    "contract_market",
    "tax_inclusive",
];

/// 建表迁移必须把列名与可空性写成契约声明的形态。判据从 CREATE TABLE 括号体内逐行解析，
/// 表名或列名或 NOT NULL 任一改动都会与规格表对不上，缺席多出席名集合。
#[test]
fn create_table_blocks_declare_exactly_the_specified_columns() {
    let specs: [(&str, &str, &[ColSpec]); 4] = [
        (
            "m0114_create_contract_templates",
            "contract_templates",
            CONTRACT_TEMPLATES_COLS,
        ),
        (
            "m0115_create_clause_library",
            "clause_library",
            CLAUSE_LIBRARY_COLS,
        ),
        (
            "m0116_create_contract_template_clauses",
            "contract_template_clauses",
            CONTRACT_TEMPLATE_CLAUSES_COLS,
        ),
        (
            "m0117_create_contract_clauses",
            "contract_clauses",
            CONTRACT_CLAUSES_COLS,
        ),
    ];
    for (child, table, spec) in specs {
        let rel = format!("migration/src/domain/contract_template_schema/{child}.rs");
        let src = strip_line_comments(&read(&rel));
        let open_anchor = format!("CREATE TABLE IF NOT EXISTS {table} (");
        let open = src.find(&open_anchor).unwrap_or_else(|| {
            panic!("{rel} 找不到 {open_anchor}——建表形态或表名被改动，本锁失去检测对象")
        });
        let body_start = open + open_anchor.len();
        let close = find_create_table_close(&src[body_start..], &rel, table);
        let body = &src[body_start..body_start + close];

        let mut declared: BTreeSet<String> = BTreeSet::new();
        let mut nullability: BTreeMap<String, bool> = BTreeMap::new();
        for raw in body.lines() {
            let line = raw.trim();
            if line.is_empty()
                || line.starts_with("CONSTRAINT")
                || line.starts_with("PRIMARY KEY")
                || line.starts_with("UNIQUE (")
                || line.starts_with("CHECK (")
            {
                continue;
            }
            let name = line.split_whitespace().next().unwrap_or("");
            if name.is_empty() || !name.chars().next().unwrap().is_ascii_lowercase() {
                continue;
            }
            declared.insert(name.to_string());
            nullability.insert(
                name.to_string(),
                line.contains("NOT NULL") || line.contains("PRIMARY KEY"),
            );
        }

        let expected: BTreeSet<String> = spec.iter().map(|c| c.name.to_string()).collect();
        assert_eq!(
            declared, expected,
            "{rel} 的 {table} 列集合与规格表不一致。多出的列未登记进本锁，缺席的列会让\
             读写腿按不存在的列解码。规格={expected:?} 实得={declared:?}"
        );
        for c in spec {
            let Some(is_null) = nullability.get(c.name) else {
                panic!("{table} 的建表体内解析不到列 {cname}", cname = c.name);
            };
            assert_eq!(
                *is_null,
                c.not_null,
                "{table}.{cname} 的可空性与规格不符，钉的是 not_null={want}，\
                 迁移文本实得 not_null={got}。可空性漂移会让历史行的空值语义\
                 与服务层解码口径分叉",
                cname = c.name,
                want = c.not_null,
                got = *is_null,
            );
        }
    }
}

/// 从 CREATE TABLE 体内定位与其配平的右括号偏移。体内不含嵌套括号到语句级，
/// 只需要按括号深度配平即可。找不到即 panic 点名，绝不放宽成整文件扫描。
fn find_create_table_close(tail: &str, rel: &str, table: &str) -> usize {
    let bytes = tail.as_bytes();
    let mut depth = 1usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'(' {
            depth += 1;
        } else if *b == b')' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }
    panic!("{rel} 的 {table} 建表体括号未配平，无法界定列清单")
}

/// 实体与 DDL 必须同形，列名集合与可空性两处同源。表列一旦与 Entity 漂移，
/// 解码整行失败或新列在读写腿里静默失踪，都是本仓反复出现过的缺陷形态。
#[test]
fn entities_declare_exactly_the_ddl_columns_with_matching_nullability() {
    let pairs: [(&str, &str, &[ColSpec]); 4] = [
        (
            "m0114_create_contract_templates",
            "src/models/contract_template.rs",
            CONTRACT_TEMPLATES_COLS,
        ),
        (
            "m0115_create_clause_library",
            "src/models/clause_library.rs",
            CLAUSE_LIBRARY_COLS,
        ),
        (
            "m0116_create_contract_template_clauses",
            "src/models/contract_template_clause.rs",
            CONTRACT_TEMPLATE_CLAUSES_COLS,
        ),
        (
            "m0117_create_contract_clauses",
            "src/models/contract_clause.rs",
            CONTRACT_CLAUSES_COLS,
        ),
    ];
    for (child, model_rel, spec) in pairs {
        let ddl = strip_line_comments(&read(&format!(
            "migration/src/domain/contract_template_schema/{child}.rs"
        )));
        let model = strip_line_comments(&read(model_rel));
        let mut fields: BTreeMap<String, bool> = BTreeMap::new();
        for raw in model.lines() {
            let line = raw.trim();
            let Some(rest) = line.strip_prefix("pub ") else {
                continue;
            };
            let Some((name, ty)) = rest.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }
            fields.insert(name.to_string(), ty.trim_start().starts_with("Option<"));
        }
        let expected: BTreeSet<String> = spec.iter().map(|c| c.name.to_string()).collect();
        let declared: BTreeSet<String> = fields.keys().cloned().collect();
        assert_eq!(
            declared, expected,
            "{model_rel} 的字段集合与本锁规格不一致（建表迁移 {child}）。缺席字段让新列\
             在读写腿失踪，多出的字段解码时报列不存在。规格={expected:?} 实得={declared:?}"
        );
        for c in spec {
            let cname = c.name;
            let Some(entity_optional) = fields.get(cname) else {
                panic!("{model_rel} 没有字段 {cname}，实体与建表迁移已脱节");
            };
            assert_eq!(
                *entity_optional, !c.not_null,
                "{model_rel} 的 {cname} 与 {child} 建表体的可空性不同形。\
                 DDL NOT NULL 的列在实体里必须是裸类型，可空列必须是 Option，\
                 否则一侧放行一侧拒写"
            );
            assert!(
                ddl.contains(&format!("{cname} ")) || ddl.contains(&format!("{cname}(")),
                "{child} 的建表体里找不到列 {cname}，规格表与迁移文本已脱节"
            );
        }
    }
}

/// m0118 补列必须两侧同形且一律可空。判据按表名加列名成对出现，缺任一一侧即点名。
#[test]
fn main_table_added_columns_cover_both_sides_and_stay_nullable() {
    let src = strip_line_comments(&read(
        "migration/src/domain/contract_template_schema/m0118_add_contract_main_clause_columns.rs",
    ));
    let down_at = src
        .find("async fn down")
        .expect("m0118 缺少 down，无法分段判 up 与 down 的列集合");
    let (up_src, down_src) = src.split_at(down_at);
    for col_name in CONTRACT_MAIN_ADDED_COLS.iter().copied() {
        for table in CONTRACT_MAIN_TABLES.iter().copied() {
            let add = format!(r#"ALTER TABLE "{table}" ADD COLUMN IF NOT EXISTS "{col_name}""#);
            let drop = format!(r#"ALTER TABLE "{table}" DROP COLUMN IF EXISTS "{col_name}""#);
            let added_in_up = up_src.contains(&add);
            let dropped_in_down = down_src.contains(&drop);
            // 销售侧的四条款列与签署列与印花税列是既有列，本迁移只给采购侧补齐，
            // 再给两侧补内外销与含税两枚，故销售侧只允许这两枚有 ADD 与 DROP 配对。
            let is_sales_new = matches!(col_name, "contract_market" | "tax_inclusive");
            let expected_add = table == "purchase_contracts" || is_sales_new;
            assert_eq!(
                added_in_up, expected_add,
                "m0118 的 {table}.{col_name} 加列形态与规格不符。期望 add={expected_add}，\
                 实得 {added_in_up}。销售侧既有列不该重复加列，采购侧该补的一枚不能少"
            );
            assert_eq!(
                dropped_in_down, added_in_up,
                "m0118 的 {table}.{col_name} up 与 down 不互逆。加过的列必须能撤，\
                 没加的列不该出现在 down，实得 add={added_in_up} drop={dropped_in_down}"
            );
            if added_in_up {
                let stmt = up_src
                    .split(&add)
                    .nth(1)
                    .and_then(|t| t.split(';').next())
                    .unwrap_or_default();
                assert!(
                    !stmt.contains("NOT NULL") && !stmt.contains("DEFAULT"),
                    "m0118 给 {table}.{col_name} 带了 NOT NULL 或 DEFAULT，形={stmt}。\
                     新增列必须可空无默认，否则历史行在新读法下冒充成已采集过的值"
                );
            }
        }
    }
}

/// 域内子迁移的调用次序必须 up 正序、down 严格逆序；建表体内的表与索引也必须
/// up 正序创建、down 逆序撤销。外键方向与依赖方向都在这里钉死，顺序错了 down 必断。
#[test]
fn up_and_down_run_in_strictly_opposite_order() {
    let mod_src = read(MIGRATION_REL);
    let up = fn_body(&mod_src, "async fn up");
    let down = fn_body(&mod_src, "async fn down");

    let mut up_positions: Vec<(usize, &str)> = Vec::new();
    let mut down_positions: Vec<(usize, &str)> = Vec::new();
    for child in CHILD_MIGRATIONS.iter().copied() {
        let up_hits = up.matches(&format!("{child}::Migration")).count();
        let down_hits = down.matches(&format!("{child}::Migration")).count();
        assert_eq!(
            up_hits, 1,
            "域内 up 必须恰好调用 {child} 一次，实得 {up_hits} 次（漏调半张表，多调重复建表）"
        );
        assert_eq!(
            down_hits, 1,
            "域内 down 必须恰好调用 {child} 一次，实得 {down_hits} 次（漏撤留半张表在库里）"
        );
        up_positions.push((up.find(child).expect("up 找不到子迁移名"), child));
        down_positions.push((down.find(child).expect("down 找不到子迁移名"), child));
    }
    up_positions.sort_by_key(|(pos, _)| *pos);
    down_positions.sort_by_key(|(pos, _)| *pos);

    let up_order: Vec<&str> = up_positions.iter().map(|(_, n)| *n).collect();
    let down_order: Vec<&str> = down_positions.iter().map(|(_, n)| *n).collect();
    assert_eq!(
        up_order,
        CHILD_MIGRATIONS.to_vec(),
        "域内 up 的调用次序必须与被引用表在前的建表次序一致，实得 {up_order:?}"
    );
    let mut expected_down = CHILD_MIGRATIONS.to_vec();
    expected_down.reverse();
    assert_eq!(
        down_order, expected_down,
        "域内 down 必须是 up 的严格逆序（先撤引用方再撤被引用表，否则外键挡住 DROP），实得 {down_order:?}"
    );

    // 每支建表迁移内部的表与索引也要 up 正序、down 逆序。
    for &(child, _table) in CHILD_TABLES {
        let rel = format!("migration/src/domain/contract_template_schema/{child}.rs");
        let src = strip_line_comments(&read(&rel));
        let down_at = src
            .find("async fn down")
            .unwrap_or_else(|| panic!("{rel} 缺少 down，无法分段判创建与撤销次序"));
        let (up_part, down_body) = src.split_at(down_at);
        let created = collect_created_objects(up_part);
        let dropped = collect_dropped_objects(down_body);
        assert!(
            !created.is_empty(),
            "{rel} 的 up 段没解析到 CREATE TABLE 或 CREATE INDEX，本锁失去检测对象"
        );
        let mut reversed = created.clone();
        reversed.reverse();
        assert_eq!(
            dropped, reversed,
            "{rel} 的 down 撤销次序不是 up 创建次序的严格逆序。创建次序={created:?}\
             \n撤销次序={dropped:?}\n期望={reversed:?}"
        );
    }
}

/// 收集 up 体内创建的表与索引名（表带前缀 t 索引带前缀 i，保证两类对象不会互相冒充）。
fn collect_created_objects(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("CREATE TABLE IF NOT EXISTS ") {
            out.push(format!("t.{}", object_name(rest)));
        } else if let Some(rest) = line.strip_prefix("CREATE UNIQUE INDEX IF NOT EXISTS ") {
            out.push(format!("i.{}", object_name(rest)));
        } else if let Some(rest) = line.strip_prefix("CREATE INDEX IF NOT EXISTS ") {
            out.push(format!("i.{}", object_name(rest)));
        }
    }
    out
}

/// 收集 down 体内撤销的表与索引名。
fn collect_dropped_objects(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("DROP TABLE IF EXISTS ") {
            out.push(format!("t.{}", object_name(rest)));
        } else if let Some(rest) = line.strip_prefix("DROP INDEX IF EXISTS ") {
            out.push(format!("i.{}", object_name(rest)));
        }
    }
    out
}

/// 取对象名的首段标识符。带引号的写法先剥掉起始引号，再从首字符起取到第一个
/// 非标识符字符为止。花括号占位原样保留，交给调用方判占位形态。
fn object_name(rest: &str) -> String {
    rest.trim()
        .trim_start_matches('"')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '{' || *c == '}')
        .collect()
}

/// up 里 ALTER TABLE ADD CONSTRAINT 的约束，若落在本文件自己新建的表上就随 DROP TABLE 一起消失，
/// 落在既有表上（m0118 的内外销 CHECK）必须在 down 里显式撤销。不互逆的回滚会把约束留在库里。
#[test]
fn added_constraints_are_either_dropped_or_rided_out_with_their_table() {
    let mut own_tables: BTreeSet<String> = BTreeSet::new();
    for (_child, table) in CHILD_TABLES {
        own_tables.insert((*table).to_string());
    }
    for child in CHILD_MIGRATIONS.iter().copied() {
        let rel = format!("migration/src/domain/contract_template_schema/{child}.rs");
        let src = strip_line_comments(&read(&rel));
        let Some(down_at) = src.find("async fn down") else {
            panic!("{rel} 缺少 down，无法判定约束配对");
        };
        let (up_part, down_body) = src.split_at(down_at);
        for raw in up_part.lines() {
            let line = raw.trim();
            let Some(rest) = line.strip_prefix("ALTER TABLE ") else {
                continue;
            };
            let Some(at_add) = rest.find("ADD CONSTRAINT ") else {
                continue;
            };
            let table = object_name(&rest[..at_add]);
            let constraint = object_name(&rest[at_add + "ADD CONSTRAINT ".len()..]);
            // m0118 的 CHECK 走 for 循环加 format 注入，源文本里是 {table} 占位形态，
            // 按两侧合同主表逐一展开后再判配对，占位形态本身不参与own_tables比对。
            let candidates: Vec<(String, String)> = if table.contains('{') {
                CONTRACT_MAIN_TABLES
                    .iter()
                    .map(|t| (t.to_string(), constraint.replace("{table}", *t)))
                    .collect()
            } else {
                vec![(table, constraint)]
            };
            for (table, constraint) in candidates {
                if own_tables.contains(&table) {
                    continue;
                }
                assert!(
                    down_body.contains(&format!(r#"DROP CONSTRAINT IF EXISTS "{constraint}""#)),
                    "{rel} 的 up 在既有表 {table} 上加了约束 {constraint}，down 却没有显式撤销它。\
                     回滚后约束会留在库里，比 up 之前更紧，属不可逆"
                );
            }
        }
    }
}
