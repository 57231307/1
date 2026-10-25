//! 应付族授权面的三条通道一致性契约锁（不连库，纯 #[test]）。
//!
//! 锁定契约：应付族（ap-*）九枚资源的授权行在两个写入面必须逐字符同集合。
//! - 通道① 新装库播种面：`services/init_service_ops/permission.rs` 里按角色码分组的
//!   (resource_type, action) 列表；
//! - 通道② 存量库补授面：`migration/src/domain/ap_family_grant/m0001_grant_ap_family_keys.rs`
//!   的 GRANTS 三元组 (roles.code, resource_type, action)。
//! 两面的键集合一旦漂移，存量库就会出现「新装库能开、老库 403」的隐口，或者反过来把
//! 未定岗的键授进存量库，而两侧各自都能编译通过，只有本锁能在提交前点名差集。
//!
//! 另锁两件事：通道② 出现的角色码必须正好等于通道① 里持有 ap-* 键的角色码集合（新增
//! 第三个岗位拿应付族键却忘了存量库通道即红）；以及两面涉及的资源名都必须已在
//! `services/init_service.rs` 的 PERMISSION_RESOURCES 注册（未登记资源会被权限中间件
//! 按未知路径全员拒绝，授权行再全也命中不了）。
//!
//! 检测力自证：从通道② 文本里删掉一枚键、把一枚键的动作换成同资源在册的另一枚动作、
//! 以及把通道② 的角色码换成不持应付族键的岗位码，三种注入都必须被差集判据点名。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn read_repo(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {} 失败，锁前提失效: {e}", path.display()))
}

/// 取一行里按出现顺序排列的全部双引号字符串字面量。
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

const MATRIX: &str = "src/services/init_service_ops/permission.rs";
const GRANT_MIGRATION: &str = "migration/src/domain/ap_family_grant/m0001_grant_ap_family_keys.rs";
const REGISTRY: &str = "src/services/init_service.rs";

/// 取一行元组字面量里的各字段，行首不是 (" 的返回 None。
/// 字段一律对整行配对取双引号字面量：先把 (" 前缀剥掉会让资源名的右引号被当成开引号，
/// 其后配对整体错位，元数永远对不上，两面取字面量必须走这一条路。
fn tuple_fields(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with("(\"") {
        return None;
    }
    Some(quoted(t))
}

/// 通道① 解析结果：角色码 → 该岗持有的 ap-* (资源, 动作) 集合。
fn matrix_ap_keys(matrix_src: &str) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let lines: Vec<&str> = matrix_src.lines().collect();
    let mut out: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut i = 0usize;
    while i < lines.len() {
        let t = lines[i].trim();
        let is_role_head = t.starts_with('"')
            && t.ends_with(',')
            && i + 1 < lines.len()
            && lines[i + 1].trim().starts_with("&[");
        if !is_role_head {
            i += 1;
            continue;
        }
        let role = quoted(t)
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("角色码行解析不出引号内容: {t}"));
        let mut keys = BTreeSet::new();
        let mut j = i + 2;
        while j < lines.len() {
            let g = lines[j].trim();
            if g.starts_with(']') {
                break;
            }
            if let Some(fields) = tuple_fields(lines[j]) {
                assert_eq!(
                    fields.len(),
                    2,
                    "通道① {role} 分组的授权行必须是资源与动作两枚字面量: {g}"
                );
                if fields[0].starts_with("ap-") {
                    keys.insert((fields[0].clone(), fields[1].clone()));
                }
            }
            j += 1;
        }
        if !keys.is_empty() {
            out.insert(role, keys);
        }
        i = j;
    }
    out
}

/// 通道② 解析结果：roles.code → (资源, 动作) 集合。
fn migration_ap_keys(grant_src: &str) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let mut out: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let start = grant_src
        .find("const GRANTS")
        .expect("GRANTS 常量缺失，通道② 判据失效");
    let body = &grant_src[start..];
    let end = body.find("];").expect("GRANTS 常量未闭合，通道② 判据失效");
    for line in body[..end].lines() {
        let Some(fields) = tuple_fields(line) else {
            continue;
        };
        let t = line.trim();
        let (code, res, act) = match fields.as_slice() {
            [a, b, c] => (a.clone(), b.clone(), c.clone()),
            _ => panic!("GRANTS 行必须是三枚引号字面量: {t}"),
        };
        assert!(res.starts_with("ap-"), "GRANTS 含非应付族资源 {res}");
        out.entry(code).or_default().insert((res, act));
    }
    out
}

fn diff_report(
    matrix: &BTreeMap<String, BTreeSet<(String, String)>>,
    migration: &BTreeMap<String, BTreeSet<(String, String)>>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut roles: BTreeSet<&String> = BTreeSet::new();
    roles.extend(matrix.keys());
    roles.extend(migration.keys());
    for role in roles {
        let empty = BTreeSet::new();
        let m = matrix.get(role).unwrap_or(&empty);
        let g = migration.get(role).unwrap_or(&empty);
        for k in m.difference(g) {
            problems.push(format!(
                "通道① {role} 持有 {}:{}，通道② 未补授（存量库 403）",
                k.0, k.1
            ));
        }
        for k in g.difference(m) {
            problems.push(format!(
                "通道② 补授 {role} {}:{}，通道① 无此键（未定岗扩权）",
                k.0, k.1
            ));
        }
    }
    problems
}

#[test]
fn ap_family_grant_channels_agree_key_by_key() {
    let matrix = matrix_ap_keys(&read_repo(MATRIX));
    let migration = migration_ap_keys(&read_repo(GRANT_MIGRATION));
    // 正向非空控制：任一侧解析不出键即为判据空转
    assert!(!matrix.is_empty(), "通道① 未解析出任何 ap-* 授权");
    assert!(!migration.is_empty(), "通道② 未解析出任何 ap-* 授权");
    let problems = diff_report(&matrix, &migration);
    assert!(problems.is_empty(), "应付族两通道漂移: {problems:?}");
}

#[test]
fn ap_family_grant_role_sets_are_the_same_two_posts() {
    let matrix = matrix_ap_keys(&read_repo(MATRIX));
    let migration = migration_ap_keys(&read_repo(GRANT_MIGRATION));
    let expect: BTreeSet<String> = ["finance_manager", "accountant"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let matrix_roles: BTreeSet<String> = matrix.keys().cloned().collect();
    let migration_roles: BTreeSet<String> = migration.keys().cloned().collect();
    assert_eq!(matrix_roles, expect, "通道① 持应付族键的岗位集与口径不符");
    assert_eq!(
        migration_roles, expect,
        "通道② 补授岗位集与通道① 不符（新增岗位漏补存量库通道，或补授了不持应付族键的岗位）"
    );
}

#[test]
fn ap_family_resources_are_all_registered_in_permission_resources() {
    let registry = read_repo(REGISTRY);
    let mut resources = BTreeSet::new();
    for keys in matrix_ap_keys(&read_repo(MATRIX)).values() {
        for (res, _) in keys {
            resources.insert(res.clone());
        }
    }
    assert!(!resources.is_empty(), "未解析出应付族资源名");
    for res in &resources {
        let needle = format!("\"{res}\"");
        assert!(
            registry.contains(&needle),
            "应付族资源 {res} 未在 PERMISSION_RESOURCES 登记（中间件按未知路径全员拒绝，授权行命中不了）"
        );
    }
}

/// 检测力自证：三种注入都必须被差集判据点名，否则本锁是假锁。
#[test]
fn checker_detects_injected_drifts() {
    let grant_src = read_repo(GRANT_MIGRATION);
    let matrix = matrix_ap_keys(&read_repo(MATRIX));

    // 注入一，通道② 删掉一枚键（存量库缺该入口）
    let victim = "(\"accountant\", \"ap-reconciliations\", \"create\"),";
    assert!(grant_src.contains(victim), "夹具目标键不存在，注入无效");
    let m1 = migration_ap_keys(&grant_src.replace(victim, ""));
    assert!(
        diff_report(&matrix, &m1)
            .iter()
            .any(|p| p.contains("ap-reconciliations:create") && p.contains("通道② 未补授")),
        "注入（删通道② 一枚键）未被检出"
    );

    // 注入二，通道② 把审核键的动作换成越界面（未定岗扩权）
    let g2 = migration_ap_keys(&grant_src.replace(
        "(\"finance_manager\", \"ap-invoices\", \"approve\"),",
        "(\"finance_manager\", \"ap-invoices\", \"update\"),",
    ));
    assert!(
        diff_report(&matrix, &g2)
            .iter()
            .any(|p| p.contains("ap-invoices:update") && p.contains("通道① 无此键")),
        "注入（通道② 动作换成越界面）未被检出"
    );

    // 注入三，通道② 把出纳拉进取予面（口径是出纳不持任何应付族键）
    let anchor = "(\"accountant\", \"ap-payments\", \"read\"),";
    let injected = format!("{anchor}\n    (\"cashier\", \"ap-payments\", \"read\"),");
    let g3 = migration_ap_keys(&grant_src.replace(anchor, &injected));
    let problems3 = diff_report(&matrix, &g3);
    assert!(
        problems3
            .iter()
            .any(|p| p.contains("cashier") && p.contains("通道① 无此键")),
        "注入（新增不持键岗位）未被检出: {problems3:?}"
    );
}
