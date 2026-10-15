//! 坯布建档与供应商下拉三岗授予的三通道同源锁（production_manager / warehouse_keeper /
//! warehouse_manager）
//!
//! 背景：`/api/v1/erp/production/greige-fabrics`（坯布建档）与 `/api/v1/erp/purchase/suppliers`
//! （建档表单的供应商下拉）的派生权限键为 `greige-fabrics:read`、`greige-fabrics:create` 与
//! `suppliers:read`（键提取见 `middleware/permission.rs::extract_resource_info` 对模块前缀
//! 取资源段，缺键的角色访问建档/下拉恒 403，而 e2e 主链用 admin 登录测不到此缺口）。
//!
//! 三条授予通道必须同口径，本测逐 token 双向钉死，防止"一边改一边忘"：
//! ① 新装库角色矩阵 `backend/src/services/init_service_ops/permission.rs`
//! ② 存量库迁移 `backend/migration/src/domain/business/m0097_grant_greige_create_and_suppliers_read.rs`
//! ③ e2e 角色补建 `frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS
//!
//! 锁强度：①②③ 对三岗各三条授予做集合级判定（迁移常量以源文件解析为准，防止改测试常量掩盖
//! 单通道漂移），并钉最小授权禁项（greige-fabrics / suppliers 一律禁止 `*` 与 update/delete）。

/// 受授角色码（三岗矩阵码）
const FABRIC_ROLES: &[&str] = &[
    "production_manager",
    "warehouse_keeper",
    "warehouse_manager",
];

/// 授予的 (resource, action) 配对（矩阵写法为 `("resource", "action")`，含单空格）
const GRANTS: &[(&str, &str)] = &[
    ("greige-fabrics", "read"),
    ("greige-fabrics", "create"),
    ("suppliers", "read"),
];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn migration_src() -> String {
    include_str!("../migration/src/domain/business/m0097_grant_greige_create_and_suppliers_read.rs")
        .replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：逐行剔除以 `//` / `///` / `//!` 起始的注释行。
/// 禁项判据（不得出现 `("greige-fabrics", "*")`）若含注释会假判，故负向断言前一律过此函数。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
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

/// 迁移 `const NAME: &[&str] = &[...]` 或 `const NAME: &[(&str, &str)] = &[...]` 的引号 token 清单。
fn migration_const_tokens(code: &str, const_name: &str) -> Vec<String> {
    let anchor = code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("m0097 必须存在 const {const_name}"));
    let rest = &code[anchor..];
    let open = rest.find("&[").expect("m0097 常量应为 &[...] 形态");
    let close = rest[open..].find("];").expect("m0097 常量清单未闭合");
    quoted_tokens(&rest[open..open + close])
}

/// up 执行体：从 `async fn up(` 到 `async fn down(`。
fn up_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn up(")
        .expect("m0097 必须有 async fn up");
    let end_rel = code_src[start..]
        .find("async fn down(")
        .expect("m0097 必须有 async fn down");
    code_src[start..start + end_rel].to_string()
}

/// down 执行体：从 `async fn down(` 到文件尾。
fn down_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn down(")
        .expect("m0097 必须有 async fn down");
    code_src[start..].to_string()
}

/// 通道① 矩阵各角色分组：分组起始签名 =「`"role_code",` 行 + 紧随的 `&[` 行」。
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

/// 通道① 矩阵中持有 `("resource", action)` 的角色码集合（排序去重）。
fn matrix_roles_with(matrix_code: &str, resource: &str, action: &str) -> Vec<String> {
    let needle = format!(r#"("{resource}", "{action}")"#);
    let mut codes: Vec<String> = matrix_role_groups(matrix_code)
        .into_iter()
        .filter(|(_, body)| body.contains(&needle))
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// 通道③ 解析 e2e SEED_ROLE_EXTRA_PERMISSIONS 中承载指定权限码的角色码集合。
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

/// 通道②：迁移的 ROLE_CODES 常量恰为三岗集合。
#[test]
fn migration_role_codes_are_three_fabric_roles() {
    let code = code_only(&migration_src());
    let mut tokens = migration_const_tokens(&code, "ROLE_CODES");
    tokens.sort();
    let mut expected: Vec<String> = FABRIC_ROLES.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        tokens, expected,
        "m0097 ROLE_CODES 应恰为三岗，不得增删角色面"
    );
}

/// 通道②：迁移的 GRANTS 常量恰为三条 (resource, action) 配对（按对拼成 `resource|action` 比较）。
#[test]
fn migration_grants_are_exactly_three_pairs() {
    let code = code_only(&migration_src());
    let tokens = migration_const_tokens(&code, "GRANTS");
    assert!(
        tokens.len().is_multiple_of(2),
        "GRANTS 引号 token 数应为偶数（每对 2 个）: {tokens:?}"
    );
    let mut pairs: Vec<String> = Vec::new();
    for chunk in tokens.chunks(2) {
        pairs.push(format!("{}|{}", chunk[0], chunk[1]));
    }
    pairs.sort();
    let mut expected: Vec<String> = GRANTS.iter().map(|(r, a)| format!("{r}|{a}")).collect();
    expected.sort();
    assert_eq!(
        pairs, expected,
        "m0097 GRANTS 应恰为坯布 read/create 与供应商 read 三对"
    );
}

/// 通道②：up 幂等用 NOT EXISTS，不提前动全局唯一约束（无 ON CONFLICT）。
#[test]
fn migration_up_is_idempotent_and_avoids_on_conflict() {
    let code = code_only(&migration_src());
    let up = up_execution(&code);
    assert!(
        up.contains("NOT EXISTS"),
        "m0097 up 应用 WHERE NOT EXISTS 保证重跑等价"
    );
    assert!(
        !up.contains("ON CONFLICT"),
        "m0097 不应依赖 domain/v15 才建的唯一索引用 ON CONFLICT"
    );
}

/// 通道②：down 逐条按角色码收窄回收授予、不出现通配、不空实现。
/// （SQL 由 `format!` 拼装，源码文本里是 `'{res}'` 占位符而非解析后的资源名，
/// 故此处只断执行体结构，不按具体资源名做文本比对。）
#[test]
fn migration_down_revokes_only_granted_keys() {
    let code = code_only(&migration_src());
    let down = down_execution(&code);
    assert!(down.contains("DELETE"), "m0097 down 必须有真实回收");
    assert!(
        down.contains("role_permissions"),
        "m0097 down 应作用于 role_permissions"
    );
    assert!(down.contains("resource_type"), "m0097 down 应按资源段过滤");
    assert!(
        down.contains(r#"r."code" = ANY"#),
        "m0097 down 应按角色码数组收窄，只回收本次三岗的授予"
    );
    assert!(!down.contains(r#""*""#), "m0097 down 不得删除通配授予");
}

/// 通道①：三岗在矩阵里各持坯布 read/create 与供应商 read。
#[test]
fn matrix_grants_all_three_perms_for_three_roles() {
    let code = code_only(&matrix_src());
    for role in FABRIC_ROLES {
        for (res, act) in GRANTS {
            let holders = matrix_roles_with(&code, res, act);
            assert!(
                holders.iter().any(|h| h == role),
                "通道① 角色 {role} 应被授予 ({res}, {act})，实际持有者 {holders:?}"
            );
        }
    }
}

/// 通道① 最小授权：三岗分组体内对坯布与供应商一律禁止通配与改删
/// （全矩阵别处如 greige_manager 对坯布、采购岗对供应商本就持 `*`/update 属其本职，
/// 故禁项只能按"本次被扩权的三岗"逐组判定，不能对整表断言，否则假红）。
#[test]
fn matrix_forbids_wildcard_and_unauthorized_actions_for_three_roles() {
    let code = code_only(&matrix_src());
    let groups = matrix_role_groups(&code);
    for role in FABRIC_ROLES {
        let body = groups
            .iter()
            .find(|(c, _)| c == role)
            .map(|(_, b)| b.as_str())
            .unwrap_or_else(|| panic!("矩阵必须存在角色分组 {role}"));
        for res in ["greige-fabrics", "suppliers"] {
            for banned in ["*", "update", "delete", "export"] {
                let needle = format!(r#"("{res}", "{banned}")"#);
                assert!(
                    !body.contains(&needle),
                    "禁止三岗 {role} 对 {res} 超范围授予（越权面放大）: {needle}"
                );
            }
        }
    }
}

/// 通道③：e2e SEED 对三条权限码都覆盖三岗。
#[test]
fn e2e_seed_covers_three_roles_with_same_three_perms() {
    for perm in [
        "greige-fabrics:read",
        "greige-fabrics:create",
        "suppliers:read",
    ] {
        let roles = e2e_seed_roles_with(perm);
        for role in FABRIC_ROLES {
            assert!(
                roles.iter().any(|r| r == role),
                "通道③ e2e SEED 权限 {perm} 应覆盖角色 {role}，实际 {roles:?}"
            );
        }
    }
}

/// 跨通道：迁移受授的三岗 ⊆ 通道① 矩阵持坯布 read 的角色集（防止迁移给了矩阵没给的人）。
#[test]
fn migration_roles_are_within_matrix_greige_read_holders() {
    let mcode = code_only(&migration_src());
    let matrix_code = code_only(&matrix_src());
    let mig_roles = migration_const_tokens(&mcode, "ROLE_CODES");
    let matrix_read = matrix_roles_with(&matrix_code, "greige-fabrics", "read");
    for r in &mig_roles {
        assert!(
            matrix_read.iter().any(|m| m == r),
            "迁移受授角色 {r} 必须在矩阵坯布 read 集内，实际矩阵持有者 {matrix_read:?}"
        );
    }
}
