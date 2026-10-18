//! 角色互斥表（role_conflicts）引用码 ⇄ 角色种子码 一致性锁（纯静态，不连库）
//!
//! 缺陷类别：`role_conflicts.role_a_code / role_b_code` 是指向 `roles.code` 的**软引用**
//! （建表 DDL 没有外键）。互斥校验 `user_service::check_role_conflict_for_user` 把两侧
//! code 与用户当前/新角色的 code 做字符串相等匹配，所以一条引用了不存在角色码的规则
//! 既不报错、也永远不会命中任何用户——职责分离约束静默失效，比启动失败更难发现。
//!
//! 被锁的源（全部取生产真实定义，测试侧不抄第二份清单）：
//! - 角色码在册集：`src/services/init_service_ops/role.rs` 的业务角色定义表
//!   （name/code/description/data_scope 四元组）+ 同文件 `code: Set("…")` 写法的
//!   manager/operator + `src/utils/admin_checker.rs` 的 `ADMIN_ROLE_CODE`
//!   + 建表迁移 `migration/src/domain/system/m0001_initial_schema.rs` 的 roles 种子行。
//! - 互斥对通道①：`src/services/init_service_ops/permission.rs` 里
//!   `create_default_role_conflicts` 的 `conflicts` 定义表（init 写入侧）。
//! - 互斥对通道②：`migration/src/domain/finance/mod.rs` 建表同域的
//!   `INSERT INTO role_conflicts … VALUES` 种子行（存量库的实际在册内容）。
//!
//! 判据四条，逐条都能因漂移而红：
//! 1. 引用完整性：两条通道里出现的每个角色码都必须在角色在册集里，缺失者逐个点名。
//! 2. 对序合法性：每对必须满足 `role_a_code < role_b_code`。建表 CHECK 违例不是丢一行，
//!    而是整条 INSERT 失败 ⇒ 该通道全部规则都不生效，与悬空码同一种静默失效形态。
//! 3. 通道内唯一：同一通道内不得出现重复对（表上有 uniq_role_pair，重复即写入失败）。
//! 4. 检测力地板：解析出的角色码数与两通道互斥对数低于地板即判红——锚点失配、定义表
//!    改名或换形导致的"空集/半集"都必须红，不允许"没解析到所以没漂移"。
//!
//! 另有两条注入探针：把在册码换成幽灵码、把合法对调序都必须被点名；注入未生效（needle
//!     与源码失配）时直接判红，防探针自身空跑。
//!
//! 计数改判口径：互斥对数量属于定义面，任一通道增删对都必须同步改本文件的计数常量
//! （与 `contract_wave7_piece_permission_grant_test.rs` 同口径），不许放宽集合来变绿。
//!
//! 解析范式出处（照抄仓内同类锁的实际写法，不另造机制）：`include_str!` 读被测源码 +
//! 逐行剔除注释 + 按锚点取定义表区域 + 对称差逐个点名，见
//! `contract_wave11_role_code_parser_shape_lock_test.rs`。本文件在此之上加「四元组第
//! 3/4 项的取值域」对齐哨兵：解析一旦错位，取值必然越界，直接 panic 而不是静默少读。

use bingxi_backend::services::init_service::InitService;
use std::collections::BTreeSet;

/// 角色种子定义表所在源文件（业务角色四元组 + manager/operator 的 ActiveModel 写法）
const ROLE_SEED_SRC: &str = include_str!("../src/services/init_service_ops/role.rs");
/// ADMIN_ROLE_CODE 常量所在源文件（admin 角色码的唯一在册出处，role.rs 引它而非字面量）
const ADMIN_CONST_SRC: &str = include_str!("../src/utils/admin_checker.rs");
/// 建表迁移里 roles 的种子行（CI/部署库最先存在、其它用例按 id 依赖的角色）
const SCHEMA_ROLES_SRC: &str =
    include_str!("../migration/src/domain/system/m0001_initial_schema.rs");
/// 互斥对通道①：init 定义表所在源文件
const CONFLICT_INIT_SRC: &str = include_str!("../src/services/init_service_ops/permission.rs");
/// 互斥对通道②：建表迁移同域种子所在源文件
const CONFLICT_MIGRATION_SRC: &str = include_str!("../migration/src/domain/finance/mod.rs");

/// 角色定义表起始锚点（前/后台两处同形声明，逐处取到数组闭合）
const ROLE_TUPLE_OPEN: &str = "let tuples: &[(&str, &str, &str, &str)] = &[";
/// ActiveModel 直接写字面量角色码的形态（manager / operator）
const ROLE_CODE_SET_OPEN: &str = "code: Set(\"";
/// 互斥对定义表起始锚点（init 通道）
const CONFLICT_TUPLE_OPEN: &str = "let conflicts: &[(&str, &str, &str, &str)] = &[";
/// Rust 数组区域结束锚点
const REGION_END: &str = "];";
/// SQL 种子语句的结束锚点
const SQL_CONFLICT_END: &str = "ON CONFLICT";
const SQL_ROLES_END: &str = ";\n";
const SQL_ROLES_OPEN: &str = "INSERT INTO \"roles\" (";
const SQL_CONFLICTS_OPEN: &str = "INSERT INTO role_conflicts (role_a_code";
/// SQL 种子行的公共关键字（缺失即语句形态已改变）
const SQL_VALUES: &str = "VALUES";

/// 角色在册码数地板（当前实测 44）：低于它说明解析只取到半集或空集，本锁失去意义。
const MIN_ROLE_CODES: usize = 30;
/// init 通道互斥对数（定义面计数，增删对须同步本常量）
const INIT_CONFLICT_PAIRS: usize = 11;
/// 迁移通道互斥对数（建表同域的财务三权分立种子）
const MIGRATION_CONFLICT_PAIRS: usize = 8;

/// 只保留"代码 + 字符串字面量"：Rust 行注释（`//`/`///`/`//!`）与 SQL 行注释（`--`）
/// 整行剔除——说明性文字不是定义条目，计入即假判（同类锁同款口径）。`lines()` 同时
/// 归一行尾，使锚点与注入 needle 不受本机 CRLF 检出形态影响。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| {
            let t = line.trim_start();
            !(t.starts_with("//") || t.starts_with("--"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取 `open_needle` 之后到下一个 `end_needle` 之前的区域文本。锚点失配直接 panic 点名：
/// 静默返回空区域是本锁最危险的失效形态（"没解析到"会被当成"没有漂移"）。
fn region(src: &str, open_needle: &str, end_needle: &str) -> String {
    let at = src.find(open_needle).unwrap_or_else(|| {
        panic!("起始锚点 `{open_needle}` 未命中（定义表已改名/换形，解析器与源码分叉，本锁必须红而不是空跑）")
    });
    let body = &src[at + open_needle.len()..];
    let end = body
        .find(end_needle)
        .unwrap_or_else(|| panic!("结束锚点 `{end_needle}` 未命中（定义表未闭合或结构被改写）"));
    body[..end].to_string()
}

/// 按出现顺序取文本里所有 `quote` 包裹的字面量。含反斜杠转义即点名——两处定义表的码值
/// 一律是无反斜杠的字面量，出现转义说明解析口径与源码形状已分叉。
fn quoted_tokens(text: &str, quote: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(quote) {
        let tail = &rest[start + quote.len_utf8()..];
        let close = tail
            .find(quote)
            .unwrap_or_else(|| panic!("引号 {quote} 未闭合（被截断的源不得解析成半集）"));
        let tok = &tail[..close];
        if tok.contains('\\') {
            panic!("字面量 `{tok}` 含转义——定义表码值不应有转义，解析口径失效");
        }
        out.push(tok.to_string());
        rest = &tail[close + quote.len_utf8()..];
    }
    out
}

/// 逐处取角色定义表区域（同一文件里有前/后台两处同形声明）。
fn role_tuple_regions(code: &str) -> Vec<String> {
    let mut regions = Vec::new();
    let mut rest = code;
    while let Some(rel) = rest.find(ROLE_TUPLE_OPEN) {
        let body = &rest[rel + ROLE_TUPLE_OPEN.len()..];
        let end = body.find(REGION_END).unwrap_or_else(|| {
            panic!("角色定义表数组未以 `{REGION_END}` 闭合（结构被改写，不得静默少读）")
        });
        regions.push(body[..end].to_string());
        rest = &body[end..];
    }
    regions
}

/// 建表迁移 roles 种子行的角色码（单引号 SQL 字面量，每行第 2 项是 code）。
fn migration_seed_role_codes(schema_src: &str) -> Vec<String> {
    let code = code_only(schema_src);
    let stmt = region(&code, SQL_ROLES_OPEN, SQL_ROLES_END);
    sql_rows_after_values(&stmt, "roles")
        .lines()
        .map(|line| line.trim())
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let tokens = quoted_tokens(line, '\'');
            if tokens.len() < 2 {
                panic!("roles 种子行字面量不足 2 项（name/code）：{line}");
            }
            tokens[1].clone()
        })
        .collect()
}

/// 互斥对通道①：init 定义表，四元组按 (a, b, conflict_type, description) 顺序。
fn init_conflict_pairs(conflict_src: &str) -> Vec<(String, String)> {
    let code = code_only(conflict_src);
    let seg = region(&code, CONFLICT_TUPLE_OPEN, REGION_END);
    let tokens = quoted_tokens(&seg, '"');
    if tokens.is_empty() || !tokens.len().is_multiple_of(4) {
        panic!(
            "init 互斥定义表解析出的字面量数 {} 不是 4 的倍数——分组错位",
            tokens.len()
        );
    }
    tokens
        .chunks(4)
        .map(|group| {
            if group[2] != "sod" {
                panic!(
                    "init 互斥定义表第 3 项解析为 `{}`，不在 conflict_type 取值域 sod 内——对齐哨兵失守",
                    group[2]
                );
            }
            (group[0].clone(), group[1].clone())
        })
        .collect()
}

/// 互斥对通道②：建表迁移同域的 SQL 种子行（每行 `('a', 'b', 'sod', '文案')`）。
fn migration_conflict_pairs(conflict_src: &str) -> Vec<(String, String)> {
    let code = code_only(conflict_src);
    let stmt = region(&code, SQL_CONFLICTS_OPEN, SQL_CONFLICT_END);
    sql_rows_after_values(&stmt, "role_conflicts")
        .lines()
        .map(|line| line.trim())
        .filter(|line| line.starts_with('('))
        .map(|line| {
            let tokens = quoted_tokens(line, '\'');
            if tokens.len() < 3 {
                panic!("role_conflicts 种子行字面量不足 3 项（a/b/conflict_type）：{line}");
            }
            if tokens[2] != "sod" {
                panic!(
                    "role_conflicts 种子行 conflict_type 解析为 `{}`——对齐哨兵失守",
                    tokens[2]
                );
            }
            (tokens[0].clone(), tokens[1].clone())
        })
        .collect()
}

/// 取 `INSERT … VALUES` 之后的行文本；VALUES 缺失即点名（语句被改写形）。
fn sql_rows_after_values<'a>(stmt: &'a str, table: &str) -> &'a str {
    let at = stmt
        .find(SQL_VALUES)
        .unwrap_or_else(|| panic!("{table} 种子 INSERT 未带 {SQL_VALUES} 关键字——语句形态已改变"));
    &stmt[at + SQL_VALUES.len()..]
}

/// 取 `ADMIN_ROLE_CODE` 常量的字面量值（admin 角色码的唯一在册出处）。
fn admin_role_code(admin_src: &str) -> String {
    let normalized = code_only(admin_src);
    let line = normalized
        .lines()
        .find(|l| l.contains("pub const ADMIN_ROLE_CODE"))
        .unwrap_or_else(|| panic!("ADMIN_ROLE_CODE 常量声明未命中——改名须同步本锁"));
    quoted_tokens(line, '"')
        .pop()
        .unwrap_or_else(|| panic!("ADMIN_ROLE_CODE 声明后未取到字面量角色码"))
}

/// 角色在册码集合：init 业务角色定义表 + ActiveModel 字面量写法 + admin 常量。
/// 四元组按 (name, code, description, data_scope) 顺序解析，第 4 项取值域做对齐哨兵。
fn seeded_role_codes(role_src: &str, admin_src: &str) -> BTreeSet<String> {
    let code = code_only(role_src);
    let mut set: BTreeSet<String> = BTreeSet::new();

    let regions = role_tuple_regions(&code);
    assert!(
        !regions.is_empty(),
        "角色种子定义表锚点一处都没命中——本锁失去被测对象，判红"
    );
    for seg in &regions {
        let tokens = quoted_tokens(seg, '"');
        if tokens.is_empty() || !tokens.len().is_multiple_of(4) {
            panic!(
                "角色定义表解析出的字面量数 {} 不是 4 的倍数——分组错位",
                tokens.len()
            );
        }
        for group in tokens.chunks(4) {
            if !matches!(group[3].as_str(), "all" | "dept" | "self") {
                panic!(
                    "角色定义表第 4 项解析为 `{}`，不在 data_scope 取值域内——对齐哨兵失守",
                    group[3]
                );
            }
            set.insert(group[1].clone());
        }
    }

    // manager / operator 走 ActiveModel 字面量写法；admin 在该文件里引用常量，此处不命中
    let mut rest = code.as_str();
    while let Some(rel) = rest.find(ROLE_CODE_SET_OPEN) {
        let tail = &rest[rel + ROLE_CODE_SET_OPEN.len()..];
        let close = tail
            .find('"')
            .unwrap_or_else(|| panic!("code: Set( 之后的字面量未闭合"));
        set.insert(tail[..close].to_string());
        rest = &tail[close..];
    }

    set.insert(admin_role_code(admin_src));
    set
}

/// 判据 1：互斥对引用的角色码是否在角色在册集内，未命中者逐个点名（带通道名）。
fn dangling_report(
    channel: &str,
    pairs: &[(String, String)],
    seeds: &BTreeSet<String>,
) -> Vec<String> {
    let mut report = Vec::new();
    for (a, b) in pairs {
        for code in [a.as_str(), b.as_str()] {
            if !seeds.contains(code) {
                report.push(format!(
                    "通道 {channel} 的互斥对 {a}↔{b} 引用了角色种子不存在的码 `{code}`（该规则永不命中）"
                ));
            }
        }
    }
    report
}

/// 判据 2：每对必须满足建表 CHECK (role_a_code < role_b_code)。
fn pair_order_report(channel: &str, pairs: &[(String, String)]) -> Vec<String> {
    pairs
        .iter()
        .filter(|(a, b)| a >= b)
        .map(|(a, b)| {
            format!("通道 {channel} 的互斥对 {a}↔{b} 违反 role_a_code < role_b_code（整条 INSERT 会被 CHECK 拒绝）")
        })
        .collect()
}

/// 判据 3：同一通道内不得出现重复对（表上有 uniq_role_pair）。
fn duplicate_report(channel: &str, pairs: &[(String, String)]) -> Vec<String> {
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    let mut dup: BTreeSet<(String, String)> = BTreeSet::new();
    for pair in pairs {
        if !seen.insert(pair.clone()) {
            dup.insert(pair.clone());
        }
    }
    dup.into_iter()
        .map(|(a, b)| {
            format!("通道 {channel} 出现重复互斥对 {a}↔{b}（uniq_role_pair 会让整条写入失败）")
        })
        .collect()
}

/// 判据 1 + 4：互斥表引用的每个角色码都能在角色种子里找到；解析规模不得低于地板。
#[test]
fn conflict_referenced_role_codes_all_exist_in_role_seed() {
    let mut seeds = seeded_role_codes(ROLE_SEED_SRC, ADMIN_CONST_SRC);
    seeds.extend(migration_seed_role_codes(SCHEMA_ROLES_SRC));

    assert!(
        seeds.len() >= MIN_ROLE_CODES,
        "解析出的角色在册码只有 {} 个，低于地板 {MIN_ROLE_CODES}——解析器只取到半集（或空集）时本锁失去意义",
        seeds.len()
    );
    for must in [
        "admin",
        "manager",
        "operator",
        "accountant",
        "finance_manager",
    ] {
        assert!(
            seeds.contains(must),
            "角色在册集缺少必然存在的 `{must}`——解析锚点与源码已分叉"
        );
    }

    let init_pairs = init_conflict_pairs(CONFLICT_INIT_SRC);
    let migration_pairs = migration_conflict_pairs(CONFLICT_MIGRATION_SRC);
    assert_eq!(
        init_pairs.len(),
        INIT_CONFLICT_PAIRS,
        "init 通道互斥对数与定义面不符（增删互斥规则须同步本锁计数）"
    );
    assert_eq!(
        migration_pairs.len(),
        MIGRATION_CONFLICT_PAIRS,
        "迁移通道互斥对数与定义面不符（增删互斥规则须同步本锁计数）"
    );

    let mut report = dangling_report("init", &init_pairs, &seeds);
    report.extend(dangling_report("migration", &migration_pairs, &seeds));
    assert!(
        report.is_empty(),
        "role_conflicts 引用的角色码必须全部在角色种子在册，悬空即该条 SoD 规则永不命中：{report:?}"
    );
}

/// 判据 2 + 3：对序合法且通道内唯一。
#[test]
fn conflict_pairs_satisfy_dict_order_and_are_channel_unique() {
    let init_pairs = init_conflict_pairs(CONFLICT_INIT_SRC);
    let migration_pairs = migration_conflict_pairs(CONFLICT_MIGRATION_SRC);

    let mut report = pair_order_report("init", &init_pairs);
    report.extend(pair_order_report("migration", &migration_pairs));
    report.extend(duplicate_report("init", &init_pairs));
    report.extend(duplicate_report("migration", &migration_pairs));
    assert!(report.is_empty(), "互斥对形状不合法：{report:?}");
}

/// 探针一：把在册码换成幽灵码，引用完整性判据必须点名；注入未生效时直接判红。
#[test]
fn injected_ghost_role_code_is_reported() {
    let base = code_only(CONFLICT_INIT_SRC);
    let mutated = base.replace("\"accountant\",", "\"ghost_role_for_probe\",");
    assert_ne!(
        mutated, base,
        "注入未生效（needle 与源码失配）——探针自身失效必须红，否则本用例空跑"
    );

    let seeds = seeded_role_codes(ROLE_SEED_SRC, ADMIN_CONST_SRC);
    let ghost_pairs = init_conflict_pairs(&mutated);
    assert!(
        ghost_pairs.iter().any(|(a, _)| a == "ghost_role_for_probe"),
        "注入没有落到互斥定义表区域（needle 只命中了文件里的其它同名串）——探针空跑，判红"
    );
    let report = dangling_report("init", &ghost_pairs, &seeds);
    assert!(
        report.iter().any(|r| r.contains("ghost_role_for_probe")),
        "注入的幽灵角色码没有被点名，本锁无检测力（属假绿），判红：{report:?}"
    );

    // 未注入的迁移通道不应报悬空码——这里若红，说明种子与定义表已再度漂移
    let clean_migration = migration_conflict_pairs(CONFLICT_MIGRATION_SRC);
    assert!(
        dangling_report("migration", &clean_migration, &seeds).is_empty(),
        "未注入的迁移通道不应报悬空码：{clean_migration:?}"
    );
}

/// 探针二：调序形态必须被对序判据点名，且不误报合法对。
#[test]
fn injected_pair_order_violation_is_reported() {
    let pairs = vec![
        (
            "finance_manager".to_string(),
            "purchase_manager".to_string(),
        ),
        ("warehouse_keeper".to_string(), "purchase_clerk".to_string()),
    ];
    let report = pair_order_report("init", &pairs);
    assert!(
        report
            .iter()
            .any(|r| r.contains("warehouse_keeper↔purchase_clerk")),
        "违反字典序的互斥对必须被点名：{report:?}"
    );
    assert_eq!(
        report.len(),
        1,
        "只应报出调序的那一对，其余对不得被误报（否则判据本身不可信）：{report:?}"
    );
}

/// 旁证：互斥对引用的岗若在权限矩阵在册，就必须同时在角色种子在册——授权面与角色面
/// 不同源时该岗有授权无角色行，互斥校验取不到角色，属同一类断链。
#[test]
fn conflict_codes_agree_with_permission_registry_and_role_seed() {
    let mut registry_roles: BTreeSet<String> = BTreeSet::new();
    for slice in InitService::all_role_permission_definition_groups() {
        for (role_code, _) in slice {
            registry_roles.insert(role_code.to_string());
        }
    }
    assert!(
        !registry_roles.is_empty(),
        "权限矩阵聚合入口返回空集——上游定义表已不可用，本锁锚点须重新核对"
    );
    let seeds = seeded_role_codes(ROLE_SEED_SRC, ADMIN_CONST_SRC);
    let mut report = Vec::new();
    for (a, b) in init_conflict_pairs(CONFLICT_INIT_SRC) {
        for code in [a.as_str(), b.as_str()] {
            if registry_roles.contains(code) && !seeds.contains(code) {
                report.push(format!(
                    "互斥对 {a}↔{b} 引用的 `{code}` 在权限矩阵在册、却不在角色种子在册"
                ));
            }
        }
    }
    assert!(
        report.is_empty(),
        "角色定义与授权定义两侧不同源：{report:?}"
    );
}
