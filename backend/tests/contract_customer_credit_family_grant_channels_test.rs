//! 客户信用族（customer-credits）授权面的通道一致性契约锁（不连库，纯静态测试）。
//!
//! 锁定契约：客户信用动作键在两个写入面的裁定岗位集内必须逐字符同集合。
//! - 通道一 新装库播种面：services/init_service_ops/permission.rs 里按角色码分组的
//!   (resource_type, action) 列表中 resource_type 恰为 customer-credits 的那些条目；
//! - 通道二 存量库补授面：migration/src/domain/customer_credit_family_grant/
//!   m0001_grant_customer_credit_family_keys.rs 的 GRANTS 三元组 (roles.code, resource_type, action)。
//! 两面一旦漂移，存量库就会出现 新装库能开、老库 403 的隐口，或反过来把未定岗的键授进存量库，
//! 而两侧各自都能编译通过，只有本锁能在提交前点名差集。
//!
//! 比对范围以裁定岗位集为界，通道一 在面外的存量持有（crm_manager 的通配与 sales_manager 的
//! 单枚读）不是通道二 的补授义务，由面外存量台账锁单独钉住在场性，
//! 扩面须同步改裁定岗位集与台账两处，逐键比对不许静默放行面外键。
//!
//! 另锁四件事：
//! 一，通道二 出现的角色码必须正好是本次裁定的两岗（finance_manager 与 accountant），新增第三岗
//!    拿客户信用键却忘了通道一 分组即红；
//! 二，通道二 的每枚动作都必须能由 routes/crm.rs 里真实注册的 customer-credits 端点派生出来
//!    （派生判据复用 middleware/permission.rs 的 PATH_ACTION_KEYWORDS 与方法映射，本测试现读
//!    那份表、不另写第二套词表），勾不出来的键运行时永不命中，属死键；
//! 三，release 必须与 create 同岗：占用、评级、调整、停用、评估这些 POST 端点末段都不在动作
//!    关键字表内、一律派生成 create，只给 release 会造成能释放不能占用的半截授权；
//! 四，资源名 customer-credits 必须已在 init_service.rs 的 PERMISSION_RESOURCES 登记（未登记的
//!    资源中间件按未知路径全员拒绝，授权行再全也命中不了）。
//! 通道三 CI 侧的 SEED_ROLE_EXTRA_PERMISSIONS 对本域故意登记 0 枚：两岗都在 SEED_ROLES 里建号、
//! 权限面来自通道一 的 init 播种，重复登记会撞 role_permissions 的同键唯一索引，该事实由
//! frontend/e2e/global-setup.ts 的现有清单可核，本锁不重复断言前端文件。
//!
//! 另含注册存在性集合锁（domain_customer_credit_three_domains_are_registered）：三域必须已登记进
//! migration/src/lib.rs 的 migrations 列表，只做集合成员判定、不声称自身是链尾末位。该锁可证伪：
//! 把 family_grant 的注册行注释掉后本测试必须红（CI 侧执行）。
//!
//! 检测力自证：从通道二 删掉一枚键、给通道二 多加一枚通道一 没有的键、把一枚键挪给不该拿它的岗位，
//! 几种注入都必须被差集判据点名，否则本锁自身是假绿。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const MATRIX: &str = "src/services/init_service_ops/permission.rs";
const GRANT_MIGRATION: &str =
    "migration/src/domain/customer_credit_family_grant/m0001_grant_customer_credit_family_keys.rs";
const REGISTRY: &str = "src/services/init_service.rs";
const CRM_ROUTES: &str = "src/routes/crm.rs";
const PERMISSION_MIDDLEWARE: &str = "src/middleware/permission.rs";
const MIGRATOR: &str = "migration/src/lib.rs";
const NAME_BACKFILL_MIGRATION: &str = "migration/src/domain/customer_credit_name_backfill/mod.rs";
const REMARK_COLUMN_MIGRATION: &str = "migration/src/domain/customer_credit_remark_column/mod.rs";
const FAMILY_GRANT_DOMAIN_MOD: &str = "migration/src/domain/customer_credit_family_grant/mod.rs";

/// 本域唯一涉及的资源名（URL 第四段原名，未经消歧映射改写）
const CREDIT_RESOURCE: &str = "customer-credits";

/// 本域补授的裁定角色集（岗位码来自裁定口径，通道二 角色集锁与通道一 比对收窄共用同源）
const ARBITRATION_ROLES: [&str; 2] = ["accountant", "finance_manager"];

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

/// 整行注释剔除（口径与仓内同类锁一致：说明性注释不是矩阵条目，计入即假判）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 通道一 解析结果：角色码 到 该岗持有的 customer-credits (资源, 动作) 集合。
/// 判据形态：角色码行形如 "xxx", 且紧邻下一行以 &[ 开头，随后到 ] 之前是权限元组。
fn matrix_credit_keys(matrix_src: &str) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let stripped = code_only(matrix_src);
    let lines: Vec<&str> = stripped.lines().collect();
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
            if g.starts_with("(\"") {
                let fields = quoted(g);
                if fields.len() == 2 && fields[0] == CREDIT_RESOURCE {
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

/// 通道二 解析结果：roles.code 到 (资源, 动作) 集合（GRANTS 三元组常量）。
fn migration_credit_keys(grant_src: &str) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let code = code_only(grant_src);
    let start = code
        .find("const GRANTS")
        .expect("GRANTS 常量缺失，通道二 判据失效");
    let body = &code[start..];
    let end = body.find("];").expect("GRANTS 常量未闭合，通道二 判据失效");
    let mut out: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    let mut rows = 0usize;
    for line in body[..end].lines() {
        let t = line.trim();
        if !t.starts_with("(\"") {
            continue;
        }
        let fields = quoted(t);
        let (role, res, act) = match fields.as_slice() {
            [a, b, c] => (a.clone(), b.clone(), c.clone()),
            other => panic!("GRANTS 行必须是三枚引号字面量，实测 {other:?}: {t}"),
        };
        assert_eq!(
            res, CREDIT_RESOURCE,
            "GRANTS 含非客户信用资源 {res}（本域只补客户信用族键，掺别的资源即越界）"
        );
        out.entry(role).or_default().insert((res, act));
        rows += 1;
    }
    assert!(
        rows > 0,
        "GRANTS 解析出 0 枚键——空集即判红，本锁不许静默放行"
    );
    out
}

/// 双向漂移点名器：只对本域涉及的岗位逐键比对，缺哪一枚、多哪一枚都打出来。
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
                "通道一 {role} 持有 {}:{}，通道二 未补授（存量库该入口恒 403）",
                k.0, k.1
            ));
        }
        for k in g.difference(m) {
            problems.push(format!(
                "通道二 补授 {role} {}:{}，通道一 无此键（未定岗扩权）",
                k.0, k.1
            ));
        }
    }
    problems
}

/// 把通道一 解析结果收窄到裁定角色集，面外持有交存量台账锁单独钉住。
fn restrict_to_arbitration(
    matrix: &BTreeMap<String, BTreeSet<(String, String)>>,
) -> BTreeMap<String, BTreeSet<(String, String)>> {
    matrix
        .iter()
        .filter(|(role, _)| ARBITRATION_ROLES.contains(&role.as_str()))
        .map(|(role, keys)| (role.clone(), keys.clone()))
        .collect()
}

/// 从 middleware/permission.rs 现读 PATH_ACTION_KEYWORDS 的字面量清单（同源，不抄第二套）。
/// 数组体定位必须先跳过声明等号，否则会命中类型标注里的切片方括号，把类型片段当成员解析。
fn path_action_keywords(mw_src: &str) -> BTreeSet<String> {
    let code = code_only(mw_src);
    let marker = "const PATH_ACTION_KEYWORDS";
    let at = code
        .find(marker)
        .expect("PATH_ACTION_KEYWORDS 常量缺失，动作派生判据失效");
    let rest = &code[at..];
    let eq = rest
        .find('=')
        .expect("PATH_ACTION_KEYWORDS 声明缺等号，解析锁前提失效");
    let after_eq = &rest[eq..];
    let open = after_eq
        .find("&[")
        .expect("PATH_ACTION_KEYWORDS 数组值未以切片字面量初始化");
    let close = open
        + after_eq[open..]
            .find(']')
            .expect("PATH_ACTION_KEYWORDS 数组未闭合");
    let inner = &after_eq[open + 2..close];
    let mut out = BTreeSet::new();
    for token in inner.split(',') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        let lits = quoted(t);
        if lits.len() != 1 {
            panic!("PATH_ACTION_KEYWORDS 成员应是一枚字符串字面量，实测: {t}");
        }
        out.insert(lits[0].clone());
    }
    assert!(
        !out.is_empty(),
        "PATH_ACTION_KEYWORDS 解析出 0 项（空集即判红）"
    );
    out
}

/// 方法到派生动作（判据来源：middleware/permission.rs 的 method_to_action）。
fn method_action(method: &str) -> Option<&'static str> {
    match method {
        "get" => Some("read"),
        "post" => Some("create"),
        "put" | "patch" => Some("update"),
        "delete" => Some("delete"),
        _ => None,
    }
}

/// 从 routes/crm.rs 的 customer_credits 函数体取出全部端点，按中间件同一判据派生动作集合。
/// 派生规则（与 middleware/permission.rs 的 extract_action_from_path 与 method_to_action 同语义）：
/// 路径末段命中 PATH_ACTION_KEYWORDS 时取该末段，否则按 HTTP 方法映射。
fn route_derived_actions(routes_src: &str, keywords: &BTreeSet<String>) -> BTreeSet<String> {
    let code = code_only(routes_src);
    let sig_at = code
        .find("pub fn customer_credits()")
        .expect("routes/crm.rs 未找到 customer_credits 函数，解析锁前提失效");
    let body_open = sig_at
        + code[sig_at..]
            .find('{')
            .expect("customer_credits 函数体缺少起始花括号");
    // 花括号配对取函数体（路由注册体内无嵌套花括号，配对即够）
    let bytes = code.as_bytes();
    let mut depth = 0usize;
    let mut body_close = None;
    for (idx, b) in bytes.iter().enumerate().skip(body_open) {
        if *b == b'{' {
            depth += 1;
        } else if *b == b'}' {
            depth -= 1;
            if depth == 0 {
                body_close = Some(idx);
                break;
            }
        }
    }
    let body_close = body_close.expect("customer_credits 函数体花括号未闭合");
    let body = &code[body_open..=body_close];

    let re = regex::Regex::new(r#"\.route\(\s*"([^"]+)",\s*([a-z]+)\("#).unwrap();
    let mut out = BTreeSet::new();
    let mut matched = 0usize;
    for cap in re.captures_iter(body) {
        let path = &cap[1];
        let method = &cap[2];
        let last = path
            .split('/')
            .rfind(|seg| !seg.is_empty())
            .unwrap_or_default();
        let action = if keywords.contains(last) {
            last.to_string()
        } else {
            method_action(method)
                .unwrap_or_else(|| panic!("端点 {path} 的方法 {method} 派生不出动作"))
                .to_string()
        };
        out.insert(action);
        matched += 1;
    }
    assert!(
        matched > 0,
        "customer_credits 里解析出 0 条端点——解析器与路由形状已分叉，空跑不得算通过"
    );
    out
}

/// 通道二 每枚动作都必须有真实端点可派生（勾不出来的键是死键）。
fn phantom_action_report(
    migration: &BTreeMap<String, BTreeSet<(String, String)>>,
    derived: &BTreeSet<String>,
) -> Vec<String> {
    let mut problems = Vec::new();
    for (role, keys) in migration {
        for (_, act) in keys {
            if !derived.contains(act) {
                problems.push(format!(
                    "通道二 给 {role} 授的动作 {act} 派生不出（customer_credits 路由里没有会产出 {act} 的端点，该授权行是永不命中的死键）"
                ));
            }
        }
    }
    problems
}

/// release 与 create 必须同岗（只给 release 就是能释放不能占用的半截授权）。
fn half_grant_report(migration: &BTreeMap<String, BTreeSet<(String, String)>>) -> Vec<String> {
    let mut problems = Vec::new();
    for (role, keys) in migration {
        let acts: BTreeSet<&str> = keys.iter().map(|(_, a)| a.as_str()).collect();
        if acts.contains("release") && !acts.contains("create") {
            problems.push(format!(
                "{role} 拿到 release 却没有 create：占用、评级、调整、停用、评估这些 POST 端点全部派生成 create，缺 create 即能释放不能占用"
            ));
        }
    }
    problems
}

#[test]
fn credit_family_grant_channels_agree_key_by_key() {
    let matrix = matrix_credit_keys(&read_repo(MATRIX));
    let migration = migration_credit_keys(&read_repo(GRANT_MIGRATION));
    let in_scope = restrict_to_arbitration(&matrix);
    assert!(
        !in_scope.is_empty(),
        "通道一 未解析出裁定角色集内的任何 customer-credits 授权"
    );
    let problems = diff_report(&in_scope, &migration);
    assert!(
        problems.is_empty(),
        "客户信用族两通道漂移（按裁定岗位逐键双向点名）: {problems:?}"
    );
}

/// 面外存量台账锁：通道一 里裁定角色集之外的岗位持有的 customer-credits 键必须恰为
/// 存量两枚，一枚不多一枚不少。crm_manager 的通配授权与 sales_manager 的单枚读都是
/// init 播种时代的存量事实，两枚都不在通道二 的补授义务内（补授面按裁定只覆盖财务两岗）。
/// 新岗位在通道一 拿到本域键、或面外动作形态变化时本锁即红，逼人同步改裁定面与本台账，
/// 而不是让逐键比对把面外键静默排除后失去在场性。
#[test]
fn credit_family_out_of_arbitration_holders_match_legacy_ledger() {
    let matrix = matrix_credit_keys(&read_repo(MATRIX));
    let mut legacy: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    legacy.insert(
        "crm_manager".to_string(),
        [(CREDIT_RESOURCE.to_string(), "*".to_string())]
            .into_iter()
            .collect(),
    );
    legacy.insert(
        "sales_manager".to_string(),
        [(CREDIT_RESOURCE.to_string(), "read".to_string())]
            .into_iter()
            .collect(),
    );
    let observed: BTreeMap<String, BTreeSet<(String, String)>> = matrix
        .into_iter()
        .filter(|(role, _)| !ARBITRATION_ROLES.contains(&role.as_str()))
        .collect();
    assert_eq!(
        observed, legacy,
        "通道一 非裁定角色持有的 customer-credits 键必须与存量台账一致（要扩授予面须同步改裁定岗位集与本台账，两通道比对不许静默放行面外键）"
    );
}

#[test]
fn credit_family_grant_role_set_is_exactly_the_two_finance_posts() {
    let migration = migration_credit_keys(&read_repo(GRANT_MIGRATION));
    let expect: BTreeSet<String> = ARBITRATION_ROLES.iter().map(|s| s.to_string()).collect();
    let roles: BTreeSet<String> = migration.keys().cloned().collect();
    assert_eq!(
        roles, expect,
        "通道二 补授岗位集必须恰为财务两岗（裁定口径），实测 {roles:?}"
    );
    // 通配符动作一律不许出现在本域（通配会经前端 actionEquivalent 与域别名映射扩出别页可达）
    for (role, keys) in &migration {
        for (_, act) in keys {
            assert_ne!(
                act, "*",
                "通道二 给 {role} 授了通配动作，本域只准逐枚补动作键"
            );
        }
    }
}

#[test]
fn credit_family_granted_actions_are_all_route_derivable() {
    let keywords = path_action_keywords(&read_repo(PERMISSION_MIDDLEWARE));
    let derived = route_derived_actions(&read_repo(CRM_ROUTES), &keywords);
    let migration = migration_credit_keys(&read_repo(GRANT_MIGRATION));
    let problems = phantom_action_report(&migration, &derived);
    assert!(
        problems.is_empty(),
        "通道二 出现派生不出的动作键: {problems:?}\n（本域端点实际派生出的动作集合={:?}）",
        derived
    );
    // 反向在场性：本域授出去的 read 与 create 与 update 与 delete 与 release 五枚
    // 都必须真由端点派生出来，少一枚说明有键是凭空造的
    for act in ["read", "create", "update", "delete", "release"] {
        assert!(
            derived.contains(act),
            "customer_credits 路由派生不出 {act}，而本域键面含 {act} 的前提已失效"
        );
    }
}

#[test]
fn credit_family_release_is_paired_with_create() {
    let migration = migration_credit_keys(&read_repo(GRANT_MIGRATION));
    let problems = half_grant_report(&migration);
    assert!(problems.is_empty(), "本域出现半截授权: {problems:?}");
}

#[test]
fn credit_family_resource_is_registered_in_permission_resources() {
    let registry = read_repo(REGISTRY);
    assert!(
        registry.contains(&format!("\"{CREDIT_RESOURCE}\"")),
        "资源名 customer-credits 未在 PERMISSION_RESOURCES 登记（中间件按未知路径全员拒绝，授权行命中不了）"
    );
}

/// 注册存在性集合锁：三域必须已登记进 migrations 列表，并各自实现 MigrationName 的 name()。
/// 只做集合成员判定，不声称本域是链尾末位（合并次序以先合者为尾，后合者在其之上重挂）。
/// 证伪口径：把 family_grant 的 Box::new 注册行注释掉后，本测试必须红。
#[test]
fn domain_customer_credit_three_domains_are_registered() {
    let migrator = read_repo(MIGRATOR);
    for (domain, box_line, name_file, name_lit) in [
        (
            "customer_credit_name_backfill",
            "Box::new(domain::customer_credit_name_backfill::Migration)",
            NAME_BACKFILL_MIGRATION,
            "\"m_customer_credit_name_backfill\"",
        ),
        (
            "customer_credit_remark_column",
            "Box::new(domain::customer_credit_remark_column::Migration)",
            REMARK_COLUMN_MIGRATION,
            "\"m_customer_credit_remark_column\"",
        ),
        (
            "customer_credit_family_grant",
            "Box::new(domain::customer_credit_family_grant::Migration)",
            FAMILY_GRANT_DOMAIN_MOD,
            "\"m_customer_credit_family_grant\"",
        ),
    ] {
        assert!(
            migrator.contains(box_line),
            "域 {domain} 未登记进 migration/src/lib.rs 的 migrations 列表（注册缺失即该域永不执行）"
        );
        // 各域台账名与登记文件在场，构成可重放的稳定标识
        assert!(
            read_repo(name_file).contains(name_lit),
            "域 {domain} 缺 MigrationName 聚合台账名 {name_lit}"
        );
    }
    assert!(
        read_repo(GRANT_MIGRATION).contains("m0001_grant_customer_credit_family_keys"),
        "customer_credit_family_grant 子迁移台账名缺失"
    );
}

/// 通道二 的 up 与 down 形态锁：幂等冲突目标、拒绝行只点名不覆盖、回退按三元组配对且
/// 只删 allowed=true 行（少了这条会把人工显式拒绝的行一并抹掉）。
#[test]
fn credit_family_grant_migration_up_down_shape_is_fail_visible() {
    let src = read_repo(GRANT_MIGRATION);
    let up_at = src.find("async fn up").expect("通道二 缺 up 实现");
    let down_at = src.find("async fn down").expect("通道二 缺 down 实现");
    let up = &src[up_at..down_at];
    let down = &src[down_at..];

    assert!(
        up.contains(r#"ON CONFLICT ("role_id", "resource_type", "action") DO NOTHING"#),
        "up 的冲突目标必须是 role_permissions 真实唯一索引列集合，否则同键重放会撞索引报错"
    );
    assert!(
        up.contains(r#"x."allowed" = false"#) && up.contains("RAISE WARNING"),
        "up 必须分辨并点名同键显式拒绝行（allowed=false），只准 RAISE WARNING 不许覆盖"
    );
    assert!(
        up.contains("RAISE NOTICE"),
        "up 必须打印命中角色数与逐键实写行数，不许静默"
    );
    assert!(
        down.contains(r#"rp."allowed" = true"#),
        "down 必须带 allowed=true 收窄条件：不带就会把人工显式拒绝行一并抹掉"
    );
    assert!(
        down.contains(r#"(r."code", rp."resource_type", rp."action") IN"#),
        "down 必须按三元组配对匹配，否则会把另一岗的同动作键一起删掉"
    );
    assert!(
        down.contains("GET DIAGNOSTICS") && down.contains("RAISE NOTICE"),
        "down 必须打印实删行数，回退结果不许静默"
    );
    // 字面量判据只作用于可执行 SQL：说明性注释（以 -- 起始的行）允许点名 crm_manager
    // 解释"不动它的通配行"，可执行体里出现该字面量才说明回退面被越界写死。
    let down_exec: String = down
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !down_exec.contains("crm_manager"),
        "down 的可执行 SQL 不许按字面量圈 crm_manager 名下的行（回退角色面只能来自 GRANTS 派生）"
    );
}

/// 检测力自证：几种注入都必须被差集判据点名，否则本锁是假锁。
#[test]
fn checker_detects_injected_drifts() {
    let grant_src = read_repo(GRANT_MIGRATION);
    let matrix = matrix_credit_keys(&read_repo(MATRIX));

    // 注入一，通道二 删掉一枚键（存量库缺该入口）
    let victim = r#"("accountant", "customer-credits", "update"),"#;
    assert!(
        grant_src.contains(victim),
        "夹具目标键不存在，注入无效（本测试的 needle 已与源码失配）"
    );
    let m1 = migration_credit_keys(&grant_src.replace(victim, ""));
    assert!(
        diff_report(&matrix, &m1)
            .iter()
            .any(|p| p.contains("customer-credits:update") && p.contains("通道二 未补授")),
        "注入（删通道二 一枚键）未被检出"
    );

    // 注入二，通道二 凭空多给一枚前端零调用方的 print 键（通道一 无此键）
    let anchor = r#"("accountant", "customer-credits", "read"),"#;
    let injected = format!("{anchor}\n    (\"accountant\", \"customer-credits\", \"print\"),");
    let m2 = migration_credit_keys(&grant_src.replace(anchor, &injected));
    assert!(
        diff_report(&matrix, &m2)
            .iter()
            .any(|p| p.contains("customer-credits:print") && p.contains("通道一 无此键")),
        "注入（通道二 多加一枚 print 键）未被检出"
    );

    // 注入三，通道二 把处置面 delete 挪给 accountant（破坏制单与审核互斥，同时留 create 缺席）
    let m3 = migration_credit_keys(&grant_src.replace(
        r#"("finance_manager", "customer-credits", "delete"),"#,
        r#"("accountant", "customer-credits", "delete"),"#,
    ));
    let problems3 = diff_report(&matrix, &m3);
    assert!(
        problems3.iter().any(|p| p.contains("finance_manager")
            && p.contains("customer-credits:delete")
            && p.contains("通道二 未补授"))
            && problems3
                .iter()
                .any(|p| p.contains("accountant") && p.contains("通道一 无此键")),
        "注入（处置面挪岗）未被双向点名: {problems3:?}"
    );

    // 注入四，把 create 从 finance_manager 摘掉而留 release（半截授权必须被成对判据点名）
    let m4 = migration_credit_keys(
        &grant_src.replace(r#"("finance_manager", "customer-credits", "create"),"#, ""),
    );
    assert!(
        half_grant_report(&m4)
            .iter()
            .any(|p| p.contains("finance_manager") && p.contains("release")),
        "注入（只留 release 摘掉 create）未被半截授权判据检出"
    );

    // 注入五，通道二 换成不持客户信用键的岗位码（越界扩权）
    let m5 = migration_credit_keys(&grant_src.replace(
        r#"("accountant", "customer-credits", "read"),"#,
        r#"("cashier", "customer-credits", "read"),"#,
    ));
    assert!(
        diff_report(&matrix, &m5)
            .iter()
            .any(|p| p.contains("cashier") && p.contains("通道一 无此键")),
        "注入（新增不持键岗位）未被检出"
    );

    // 注入六，注册存在性集合锁：把 family_grant 的注册行删掉后，集合锁判据必须失配
    let migrator = read_repo(MIGRATOR);
    let grant_box = "Box::new(domain::customer_credit_family_grant::Migration)";
    assert!(
        migrator.contains(grant_box),
        "夹具目标注册行不存在，注入无效（needle 已与 lib.rs 失配）"
    );
    let removed = migrator.replace(grant_box, "");
    assert!(
        !removed.contains(grant_box),
        "注入（删 family_grant 注册行）未使集合判据失配——本锁是空转锁"
    );
}
