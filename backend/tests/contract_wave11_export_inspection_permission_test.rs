//! 出口商检权限键三通道同源锁（`export-inspections` 资源段自始未注册）
//!
//! 缺陷事实：`/api/v1/erp/export-inspections*`（建单 POST、登记结果 PUT `/{id}/result`、
//! 列表/详情 GET、打印 `/{id}/print`）的运行时权限键是
//! `export-inspections:{read,create,update,print}`（`middleware/permission.rs::extract_resource_info`
//! 取 seg3=export-inspections 作资源段；action 由方法/末段推导，见 `method_to_action`）。
//! 该资源段虽在直接资源白名单，却从未进 `init_service::PERMISSION_RESOURCES`，也没有任何角色
//! 被授予它；`matches_permission` 按资源段精确匹配 ⇒ 除 admin 外全员访问出口商检端点一律 403，
//! 而 CI e2e 用 admin 登录测不到（假绿家族）。
//!
//! 三条授予通道必须同口径，本测逐动作双向钉死，防止"一边改一边忘"：
//! ⓪ 注册表 `src/services/init_service.rs` 的 `PERMISSION_RESOURCES` 登记 export-inspections；
//! ① 新装库角色矩阵 `src/services/init_service_ops/permission.rs`；
//! ② 存量库迁移 `migration/src/domain/production/m0083_grant_export_inspections_perms.rs`；
//! ③ e2e 角色补建 `frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS。
//!
//! 锁强度（存在的意义就是检出通道间漂移，纯"在场"断言钉不住"一边缺一边多"）：
//! · ①↔②：对每个动作 read/create/update/print **双向集合相等**；read 面在剔除
//!   m0083 登记在册的唯一部署别名码 `salesperson`（CI/部署库建它而不建 sales_rep）后相等，
//!   并单独钉别名码确实在 ② 在场——防别名豁免名单吞掉真实漂移。
//! · ③：只断言 e2e SEED 承载 export-inspections 码的角色集是 ①∩② 的**子集**；CI 的
//!   SEED_ROLES 不建 customs_specialist/logistics_coordinator，故写动作在 e2e 侧无覆盖，
//!   属如实挂账（见 `e2e_seed_export_inspection_roles_are_subset_of_backend_channels`）。
//!
//! 最小授权：库里 export-inspections 只有 GET/POST/PUT/print 四类端点，
//! 禁 `("export-inspections", "*")` 与 `delete`（无删除端点挂载）。

const RESOURCE: &str = "export-inspections";
const ACTIONS: [&str; 4] = ["read", "create", "update", "print"];

/// m0083 read 面登记在册的唯一部署/e2e 别名码（CI/部署库建 salesperson 不建 sales_rep）。
const REGISTERED_READ_ALIAS_CODES: &[&str] = &["salesperson"];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn migration_src() -> String {
    include_str!("../migration/src/domain/production/m0083_grant_export_inspections_perms.rs")
        .replace('\r', "")
}

fn registry_src() -> String {
    include_str!("../src/services/init_service.rs").replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：整行注释逐行剔除。
/// 三通道锁含**禁项**（不得出现 `("export-inspections", "*")`）与**授予**两类判据，
/// 二者都必须只针对执行体；说明性注释（"禁止写 * "）不是授予。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 通道① 矩阵各角色分组：分组起始签名 =「`"role_code",` 行 + 紧随的 `&[` 行」，
/// 分组体 = 本角色码行之后到下一分组起始之前。权限元组只出现在所属分组体内。
fn matrix_role_groups(matrix_code: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = matrix_code.lines().collect();
    let is_role_code_line = |l: &str| -> bool {
        let t = l.trim();
        t.len() > 3
            && t.starts_with('"')
            && t.ends_with("\",")
            && t[1..t.len() - 2]
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '_')
    };
    let mut groups = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "&[" && i > 0 && is_role_code_line(lines[i - 1]) {
            let code = lines[i - 1].trim().trim_end_matches(',').trim_matches('"');
            let mut end = lines.len();
            let mut j = i + 1;
            while j < lines.len() {
                if lines[j].trim() == "&[" && is_role_code_line(lines[j - 1]) {
                    end = j - 1;
                    break;
                }
                j += 1;
            }
            groups.push((code.to_string(), lines[i..end].join("\n")));
            i = j;
        } else {
            i += 1;
        }
    }
    groups
}

/// 通道① 矩阵中持有该 export-inspections 动作的角色码集合（排序去重）。
fn matrix_roles_with(matrix_code: &str, action: &str) -> Vec<String> {
    let needle = format!(r#"("{RESOURCE}", "{action}")"#);
    let mut codes: Vec<String> = matrix_role_groups(matrix_code)
        .into_iter()
        .filter(|(_, body)| body.contains(&needle))
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// 区间内全部成对双引号 token。
fn quoted_tokens(seg: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = seg.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut tok = String::new();
            for n in chars.by_ref() {
                if n == '"' {
                    break;
                }
                tok.push(n);
            }
            out.push(tok);
        }
    }
    out
}

/// m0083 里 `const NAME: &[&str] = &[...]` 的引号 token 清单（剔注释后解析）。
/// 取**常量清单本身**而非手抄口径集，让"只改一条通道"的漂移无法靠同步改本测试常量掩盖。
fn migration_role_codes(mig_code: &str, const_name: &str) -> Vec<String> {
    let anchor = mig_code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("m0083 必须存在 const {const_name}（通道② 集合判据定位符）"));
    let rest = &mig_code[anchor..];
    // 从 `const NAME: &[&str] = &[` 的首个 `&[` 起，到清单收尾的 `];` 止：
    // 该跨度内的成对双引号 token 即角色码清单（`&str]` 类型标注无引号，不误入）。
    let open = rest
        .find("&[")
        .unwrap_or_else(|| panic!("m0083 常量 {const_name} 应为 &[&str] 形态"));
    let close = rest[open..]
        .find("];")
        .unwrap_or_else(|| panic!("m0083 常量 {const_name} 清单未闭合"));
    let mut codes = quoted_tokens(&rest[open..open + close]);
    codes.sort();
    codes.dedup();
    codes
}

/// 动作名 → m0083 对应常量名。
fn const_name_for(action: &str) -> &'static str {
    match action {
        "read" => "READ_ROLE_CODES",
        "create" => "CREATE_ROLE_CODES",
        "update" => "UPDATE_ROLE_CODES",
        "print" => "PRINT_ROLE_CODES",
        other => panic!("未知动作 {other}（新增动作须同步本锁与 m0083）"),
    }
}

/// 通道③：解析 e2e SEED_ROLE_EXTRA_PERMISSIONS 中承载指定权限码的角色码集合。
fn e2e_seed_roles_with(perm: &str) -> Vec<String> {
    let src = code_only(&e2e_seed_src());
    let anchor = src
        .find("const SEED_ROLE_EXTRA_PERMISSIONS")
        .expect("global-setup 必须存在 SEED_ROLE_EXTRA_PERMISSIONS");
    let rest = &src[anchor..];
    let open = rest
        .find('{')
        .expect("SEED_ROLE_EXTRA_PERMISSIONS 对象字面量未开启");
    let close = rest
        .find("\n};")
        .expect("SEED_ROLE_EXTRA_PERMISSIONS 对象字面量未闭合");
    let body = &rest[open..close];
    let mut roles = Vec::new();
    let mut current: Option<(String, String)> = None;
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let colon = t.find(':');
        let starts_entry = match colon {
            Some(c) => {
                c > 0
                    && t[..c].starts_with(|ch: char| ch.is_ascii_lowercase())
                    && t[..c]
                        .chars()
                        .all(|ch| ch.is_ascii_lowercase() || ch == '_')
                    && t[c + 1..].trim_start().starts_with('[')
            }
            None => false,
        };
        if starts_entry {
            if let Some((code, acc)) = current.take() {
                if acc.contains(perm) {
                    roles.push(code);
                }
            }
            let c = colon.expect("starts_entry 已含冒号位");
            current = Some((t[..c].to_string(), t[c + 1..].to_string()));
        } else if let Some((_, acc)) = current.as_mut() {
            acc.push('\n');
            acc.push_str(t);
        }
    }
    if let Some((code, acc)) = current {
        if acc.contains(perm) {
            roles.push(code);
        }
    }
    roles.sort();
    roles
}

fn sorted_refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// 通道⓪：资源段必须登记进权限资源注册表（否则权限目录里根本没有该资源，
/// 矩阵授权行运行期也永不命中——init 闸门①同样会判红）。
#[test]
fn export_inspections_resource_is_registered() {
    let src = code_only(&registry_src());
    assert!(
        src.contains("pub const PERMISSION_RESOURCES"),
        "PERMISSION_RESOURCES 注册表缺失"
    );
    assert!(
        src.contains(r#""export-inspections","#),
        "export-inspections 必须登记进 PERMISSION_RESOURCES（资源段由 URL seg3 推导）"
    );
}

/// 通道①↔②：每个动作的角色集合双向相等（read 面剔除在册别名后）。
/// 这是检出漂移的主锁：任一通道单边增删角色码必红。
#[test]
fn matrix_and_migration_role_sets_are_equal_per_action() {
    let matrix_code = code_only(&matrix_src());
    let mig_code = code_only(&migration_src());
    for action in ACTIONS {
        let matrix_roles = matrix_roles_with(&matrix_code, action);
        let mig_roles = migration_role_codes(&mig_code, const_name_for(action));
        let mig_cmp: Vec<&str> = if action == "read" {
            mig_roles
                .iter()
                .map(String::as_str)
                .filter(|c| !REGISTERED_READ_ALIAS_CODES.contains(c))
                .collect()
        } else {
            mig_roles.iter().map(String::as_str).collect()
        };
        assert_eq!(
            sorted_refs(&matrix_roles),
            mig_cmp,
            "通道① 矩阵与通道② m0083 对 {RESOURCE}:{action} 的授予集合必须双向相等\
             （改一处必须同步改另一处并在此改判；read 面在册别名 {REGISTERED_READ_ALIAS_CODES:?} 除外）"
        );
        // 非 read 动作一律不得携带在册别名码（别名只承载销售只读面）
        if action != "read" {
            for alias in REGISTERED_READ_ALIAS_CODES {
                assert!(
                    !matrix_roles.iter().any(|c| c == alias)
                        && !mig_roles.iter().any(|c| c == alias),
                    "别名码 {alias} 不得出现在 {RESOURCE}:{action}（写动作只授报关专岗，别名仅承载销售只读）"
                );
            }
        }
    }
    // 在册别名码必须在② read 面在场（CI/部署库真实受授面），且① 不双写
    let mig_read = migration_role_codes(&mig_code, const_name_for("read"));
    let matrix_read = matrix_roles_with(&matrix_code, "read");
    for alias in REGISTERED_READ_ALIAS_CODES {
        assert!(
            mig_read.iter().any(|c| c == alias),
            "在册别名码 {alias} 必须在 m0083 read 面在场"
        );
        assert!(
            !matrix_read.iter().any(|c| c == alias),
            "矩阵不得双写别名码 {alias}（销售面矩阵只认规范码 sales_rep）"
        );
    }
    // 通道内自洽：写动作角色必须先能读（create/update/print ⊆ read）
    for code in matrix_roles_with(&matrix_code, "create")
        .into_iter()
        .chain(matrix_roles_with(&matrix_code, "update"))
        .chain(matrix_roles_with(&matrix_code, "print"))
    {
        assert!(
            matrix_read.contains(&code),
            "矩阵 {RESOURCE} 写动作角色 {code} 必须同时持有 {RESOURCE}:read"
        );
    }
}

/// 通道①：最小授权禁项——库里没有 export-inspections:* 与 delete 端点。
#[test]
fn matrix_and_seed_never_grant_wildcard_or_delete() {
    let matrix_code = code_only(&matrix_src());
    for banned in [
        &format!(r#"("{RESOURCE}", "*")"#),
        &format!(r#"("{RESOURCE}", "delete")"#),
    ] {
        assert!(
            !matrix_code.contains(banned),
            "禁止超范围授予 {RESOURCE} 键（越权面放大）: {banned}"
        );
    }
    let seed_src = code_only(&e2e_seed_src());
    assert!(
        !seed_src.contains(&format!("'{RESOURCE}:*'"))
            && !seed_src.contains(&format!("'{RESOURCE}:delete'")),
        "e2e 侧同样禁止超范围授 {RESOURCE} 键"
    );
}

/// 通道③：e2e SEED 的 export-inspections 授予面必须是 ①∩②（read 面并上在册别名）的子集。
/// 如实挂账：customs_specialist/logistics_coordinator 不在 CI 的 SEED_ROLES，写动作在 e2e
/// 无运行期覆盖；因此本测只钉子集，不钉 ③=①∩② 全等（全等当下必红）。反向若 SEED 授予
/// ①② 都不认的角色码则子集断言红；为防空集假绿，read 面额外要求 SEED 至少解析出一个角色。
#[test]
fn e2e_seed_export_inspection_roles_are_subset_of_backend_channels() {
    let matrix_code = code_only(&matrix_src());
    let mig_code = code_only(&migration_src());
    for action in ACTIONS {
        let matrix_roles = matrix_roles_with(&matrix_code, action);
        let mig_roles = migration_role_codes(&mig_code, const_name_for(action));
        let mut allowed: Vec<String> = mig_roles
            .iter()
            .filter(|c| matrix_roles.contains(c))
            .cloned()
            .collect();
        if action == "read" {
            for alias in REGISTERED_READ_ALIAS_CODES {
                if mig_roles.iter().any(|c| c == alias) {
                    allowed.push(alias.to_string());
                }
            }
        }
        let seeded = e2e_seed_roles_with(&format!("'{RESOURCE}:{action}'"));
        if action == "read" {
            assert!(
                !seeded.is_empty(),
                "e2e SEED 未解析出任何 '{RESOURCE}:{action}' 授予角色——要么授予被删（CI 角色回到 403 假绿），\
                 要么 SEED_ROLE_EXTRA_PERMISSIONS 分组签名变了导致解析空转；空集会让子集断言恒真，拒绝放行"
            );
        }
        for role in &seeded {
            assert!(
                allowed.contains(role),
                "通道③ e2e SEED 给 {role} 授 '{RESOURCE}:{action}'，但该码不在通道①② 规范受授面\
                 （∪在册别名）内：①={matrix_roles:?} ②={mig_roles:?}"
            );
        }
    }
}

/// 通道②：m0083 必须注册进 production 域 up 链尾、down 链首（回滚逆序对称）。
#[test]
fn migration_is_registered_in_production_chain() {
    let src =
        code_only(&include_str!("../migration/src/domain/production/mod.rs").replace('\r', ""));
    assert!(
        src.contains("mod m0083_grant_export_inspections_perms;"),
        "m0083 未在 production 域声明"
    );
    let up = &src[..src.find("async fn down").expect("production mod 缺 down")];
    let down = &src[src.find("async fn down").expect("production mod 缺 down")..];
    assert!(
        up.contains("m0083_grant_export_inspections_perms::Migration"),
        "m0083 未接入 production 域 up 链"
    );
    assert!(
        down.contains("m0083_grant_export_inspections_perms::Migration"),
        "m0083 未接入 production 域 down 链（回滚缺口）"
    );
    // 逆序对称：down 里 m0083 必须先于 m0079（后应用者先回滚）
    let i83 = down
        .find("m0083_grant_export_inspections_perms")
        .expect("down 缺 m0083");
    let i79 = down
        .find("m0079_add_approval_reason_columns")
        .expect("down 缺 m0079");
    assert!(
        i83 < i79,
        "down 顺序必须与 up 逆序对称（m0083 最后应用故最先回滚）"
    );
}

/// 通道②：CHECK 迁移注册在 `migration/src/domain/v15/` 之后的专用后置域（目标表建于该域，不能排在它之前）。
#[test]
fn check_migration_registered_after_v15_domain() {
    let lib = code_only(&include_str!("../migration/src/lib.rs").replace('\r', ""));
    let v15 = lib
        .find("domain::v15::Migration")
        .expect("lib.rs 缺 v15 域注册");
    let chk = lib
        .find("domain::export_inspection_vocab_check::Migration")
        .expect("export_inspection_vocab_check 域未注册进迁移链");
    assert!(
        chk > v15,
        "CHECK 迁移域必须排在 v15 之后（export_inspection 表由 v15 建表，加约束须晚于建表）"
    );
    let domain_mod = code_only(&include_str!("../migration/src/domain/mod.rs").replace('\r', ""));
    assert!(
        domain_mod.contains("pub mod export_inspection_vocab_check;"),
        "export_inspection_vocab_check 未在 domain/mod.rs 声明"
    );
}
