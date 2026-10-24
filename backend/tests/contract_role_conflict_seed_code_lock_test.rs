//! 角色互斥种子表的在册码与对序静态契约锁（不连库，纯 #[test]）。
//!
//! 锁定契约：`role_conflicts` 的出厂种子在两个维度上必须成立。
//! - 维度① 两侧角色码都必须在册：种子面是
//!   `migration/src/domain/finance/mod.rs` 建表同域的 INSERT INTO role_conflicts，
//!   在册面是 `src/services/init_service_ops/permission.rs` 里按角色码分组的授权矩阵角色头
//!   （与 contract_ap_family_grant_channels_test 取的是同一批角色码）。
//!   `role_conflicts` 没有指向 `roles.code` 的外键，引用一个不存在的码既不报错也永不命中，
//!   `user_service::check_role_conflict_for_user`（PUT /users/{id} 换角色的写口校验）
//!   与 GET /roles/conflicts 都按两侧 code 匹配，悬空码的表现是"职责分离约束恒不生效"，
//!   属静默失效而非报错。
//! - 维度② 每对必须满足 role_a_code < role_b_code 字典序：建表 CHECK (role_a_code < role_b_code)
//!   与 uniq_role_pair 一起定义行形状，违例不是丢一行而是整条 INSERT 被拒，
//!   于是本域全部互斥对一条也不会落库。互斥语义与左右无关（消费方双向匹配），
//!   所以书写顺序是唯一需要守的约束。
//!
//! 两维都是纯文本判据，CI 无活库时也能跑；活库行为面由
//! contract_role_conflict_builtin_defaults_test 负责，本锁不重复它的断言。

use std::collections::BTreeSet;
use std::path::PathBuf;

const FINANCE_DOMAIN: &str = "migration/src/domain/finance/mod.rs";
const MATRIX: &str = "src/services/init_service_ops/permission.rs";

fn read_repo(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败，锁前提失效: {e}", path.display()))
}

/// 取一行里按出现顺序排列的全部单引号字符串字面量（SQL 种子行用单引号）。
fn sql_literals(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut inside = false;
    for ch in line.chars() {
        if ch == '\'' {
            if inside {
                out.push(std::mem::take(&mut cur));
            }
            inside = !inside;
        } else if inside {
            cur.push(ch);
        }
    }
    out
}

/// 取一行里按出现顺序排列的全部双引号字符串字面量（授权矩阵角色头用双引号）。
fn quoted(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut inside = false;
    for ch in line.chars() {
        if ch == '"' {
            if inside {
                out.push(std::mem::take(&mut cur));
            }
            inside = !inside;
        } else if inside {
            cur.push(ch);
        }
    }
    out
}

/// 种子互斥对：只取 role_a_code 与 role_b_code 两列，按 INSERT 块内的出现顺序返回。
/// 传入的是 finance 域建表脚本全文，判据以 INSERT INTO role_conflicts 起始、
/// 到该语句的 ON CONFLICT 收尾为止，块外的注释与其它表的行一律不算。
fn seed_pairs(src: &str) -> Vec<(String, String)> {
    let start = src
        .find("INSERT INTO role_conflicts (role_a_code, role_b_code, conflict_type, description)")
        .expect("role_conflicts 种子 INSERT 缺失，判据前提失效");
    let tail = &src[start..];
    let end = tail
        .find("ON CONFLICT (role_a_code, role_b_code)")
        .expect("种子 INSERT 未闭合");
    let mut out = Vec::new();
    for line in tail[..end].lines() {
        let t = line.trim();
        if !t.starts_with("('") {
            continue;
        }
        let fields = sql_literals(t);
        assert_eq!(
            fields.len(),
            4,
            "种子行必须是 角色A 角色B 类型 说明 四枚字面量: {t}"
        );
        out.push((fields[0].clone(), fields[1].clone()));
    }
    out
}

/// 授权矩阵的角色码集合（在册面），取法与应付族通道锁一致：
/// 角色码行形如 "xxx", 且紧邻下一行以 &[ 开头。
fn matrix_role_codes(matrix_src: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = matrix_src.lines().collect();
    let mut out = BTreeSet::new();
    let mut i = 0usize;
    while i + 1 < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('"') && t.ends_with(',') && lines[i + 1].trim().starts_with("&[") {
            if let Some(code) = quoted(t).first().cloned() {
                out.insert(code);
            }
            let mut j = i + 2;
            while j < lines.len() && !lines[j].trim().starts_with(']') {
                j += 1;
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

/// 判据：种子对里不存在的角色码，返回 (角色码, 所在对) 列表。
/// 由本文件两个用例与检测力自证共用，返回空集合才算成立。
fn dangling_codes(
    pairs: &[(String, String)],
    registered: &BTreeSet<String>,
) -> Vec<(String, (String, String))> {
    let mut out = Vec::new();
    for pair in pairs {
        for code in [pair.0.as_str(), pair.1.as_str()] {
            if !registered.contains(code) {
                out.push((code.to_string(), pair.clone()));
            }
        }
    }
    out
}

/// 判据：违反 role_a_code < role_b_code 的行，返回这些对。
fn order_violations(pairs: &[(String, String)]) -> Vec<(String, String)> {
    pairs.iter().filter(|(a, b)| a >= b).cloned().collect()
}

#[test]
fn every_seeded_conflict_code_is_a_registered_role() {
    let pairs = seed_pairs(&read_repo(FINANCE_DOMAIN));
    let registered = matrix_role_codes(&read_repo(MATRIX));
    assert!(
        !pairs.is_empty(),
        "解析不到任何种子互斥对，说明判据已与被锁文本脱钩"
    );
    assert!(
        !registered.is_empty(),
        "授权矩阵解析不到任何角色码，说明在册面判据已失效"
    );
    let dangling = dangling_codes(&pairs, &registered);
    assert!(
        dangling.is_empty(),
        "role_conflicts 种子里出现不在册的角色码，互斥校验对这些对恒不命中：{:?}",
        dangling
            .iter()
            .map(|(code, (a, b))| format!("{code} 见于 {a}<->{b}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn seeded_pairs_respect_dictionary_order() {
    let pairs = seed_pairs(&read_repo(FINANCE_DOMAIN));
    let bad = order_violations(&pairs);
    assert!(
        bad.is_empty(),
        "种子对未满足 role_a_code < role_b_code，建表 CHECK 会让整条 INSERT 被拒、一对都落不下去：{:?}",
        bad
    );
}

/// 检测力自证：把种子文本注入两类缺陷后，判据必须点名；不自我证明的锁不算锁。
#[test]
fn judges_react_to_injected_defects() {
    let base = read_repo(FINANCE_DOMAIN);
    let registered = matrix_role_codes(&read_repo(MATRIX));
    let clean = seed_pairs(&base);
    assert!(
        dangling_codes(&clean, &registered).is_empty() && order_violations(&clean).is_empty(),
        "基准文本自身不干净，注入判据无从谈起"
    );

    // 注入一：把首对的一侧换成一个必不在册的码
    let (a0, b0) = clean[0].clone();
    let injected_dangling = base.replace(
        &format!("('{a0}', '{b0}',"),
        &format!("('no_such_role_code', '{b0}',"),
    );
    assert_ne!(
        base, injected_dangling,
        "替换没命中种子行，注入失败（文本形状已变，请同步本锁的取行判据）"
    );
    let pairs_dangling = seed_pairs(&injected_dangling);
    let hits = dangling_codes(&pairs_dangling, &registered);
    assert!(
        hits.iter().any(|(code, _)| code == "no_such_role_code"),
        "悬空码注入没被点名，在册面判据失效"
    );

    // 注入二：把首对左右调换，制造 a > b
    let injected_order = base.replace(&format!("('{a0}', '{b0}',"), &format!("('{b0}', '{a0}',"));
    assert_ne!(base, injected_order, "调换顺序的替换没命中种子行");
    let bad = order_violations(&seed_pairs(&injected_order));
    assert!(
        bad.contains(&(b0, a0.clone())),
        "倒序对注入没被点名，对序判据失效"
    );
}
