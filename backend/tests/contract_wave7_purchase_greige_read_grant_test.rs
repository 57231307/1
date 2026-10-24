//! 采购两岗坯布档案只读的三通道同源锁（purchase_manager / purchase_clerk）
//!
//! 背景：`GET /api/v1/erp/production/greige-fabrics` 的运行时权限键为
//! `greige-fabrics:read`（键提取见 `middleware/permission.rs::extract_resource_info`）。
//! 坯布采购与到货核对要选、要比对坯布档案，而采购两岗此前没有该键，整张候选列表恒 403。
//! 授予为只读扩权：建档与改删仍归生产与仓储岗（写面见 m0097）。
//!
//! 三条授予通道必须同口径，本测逐条双向钉死，防止一边改一边忘：
//! ① 新装库角色矩阵 `backend/src/services/init_service_ops/permission.rs`（purchase 域分组）
//! ② 存量库迁移 `backend/migration/src/domain/purchase_greige_read_grant/m0102_grant_purchase_greige_read.rs`
//!    （顶层域 m_purchase_greige_read_grant，链尾登记在 lib.rs）
//! ③ e2e 角色补建 `frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS
//!
//! 锁强度：①②③ 对两岗的同一条授予做存在性判定（迁移常量以源文件解析为准，防止改测试
//! 常量掩盖单通道漂移），并钉最小授权禁项（采购两岗对坯布档案禁止 create/update/delete
//! 与通配），另钉 m0102 与 m0097 的受授角色集合不相交，避免 down 回收误伤。

/// 受授角色码（采购矩阵码）
const PURCHASE_ROLES: &[&str] = &["purchase_manager", "purchase_clerk"];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn migration_src() -> String {
    include_str!(
        "../migration/src/domain/purchase_greige_read_grant/m0102_grant_purchase_greige_read.rs"
    )
    .replace('\r', "")
}

fn prior_migration_src() -> String {
    include_str!("../migration/src/domain/business/m0097_grant_greige_create_and_suppliers_read.rs")
        .replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

fn business_chain_src() -> String {
    include_str!("../migration/src/domain/business/mod.rs").replace('\r', "")
}

fn top_domain_src() -> String {
    include_str!("../migration/src/domain/purchase_greige_read_grant/mod.rs").replace('\r', "")
}

fn lib_chain_src() -> String {
    include_str!("../migration/src/lib.rs").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：逐行剔除以 `//` / `///` / `//!` 起始的注释行。
/// 禁项判据（不得出现 `("greige-fabrics", "create")`）若含注释会假判，故负向断言前过此函数。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形：剔全部空白并消掉闭合定界符前的尾逗号，使判据不受排版折行影响。
fn canon(src: &str) -> String {
    let mut out: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

/// 矩阵 purchase 域分组的函数体（从 `fn purchase_role_resources` 到下一个 `fn` 边界）。
fn purchase_block(code_src: &str) -> String {
    let start = code_src
        .find("fn purchase_role_resources")
        .expect("矩阵必须有 purchase_role_resources（采购岗授予的归属分组）");
    let rest = &code_src[start..];
    let end_rel = rest[1..]
        .find("\n    fn ")
        .expect("purchase_role_resources 之后必须还有下一个角色分组函数（右边界符号）")
        + 1;
    rest[..end_rel].to_string()
}

fn up_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn up(")
        .expect("m0102 必须有 async fn up");
    let end_rel = code_src[start..]
        .find("async fn down(")
        .expect("m0102 必须有 async fn down");
    code_src[start..start + end_rel].to_string()
}

fn down_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn down(")
        .expect("m0102 必须有 async fn down");
    code_src[start..].to_string()
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

/// 迁移 `const NAME: &[...] = &[...]` 的引号 token 清单。
fn const_tokens(code: &str, const_name: &str) -> Vec<String> {
    let anchor = code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("m0102 必须存在 const {const_name}"));
    let rest = &code[anchor..];
    let open = rest.find("&[").expect("m0102 常量应为 &[...] 形态");
    let close = rest[open..].find("];").expect("m0102 常量清单未闭合");
    quoted_tokens(&rest[open..open + close])
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

/// 通道①：矩阵 purchase 分组里两岗各持 greige-fabrics:read，且只授读。
#[test]
fn matrix_grants_read_to_both_purchase_roles() {
    let src = code_only(&matrix_src());
    let block = purchase_block(&src);
    let needle = "(\"greige-fabrics\", \"read\")";
    assert_eq!(
        block.matches(needle).count(),
        2,
        "矩阵 purchase 分组应为 purchase_manager 与 purchase_clerk 各授一条 greige-fabrics:read"
    );
    for role in PURCHASE_ROLES {
        assert!(
            block.contains(&format!("\"{role}\"")),
            "矩阵 purchase 分组应含角色码 {role}"
        );
    }
}

/// 通道① 最小授权：采购分组不得出现坯布档案的写动作或通配。
#[test]
fn matrix_forbids_write_and_wildcard_for_purchase_roles() {
    let src = code_only(&matrix_src());
    let block = purchase_block(&src);
    for banned in [
        "(\"greige-fabrics\", \"*\")",
        "(\"greige-fabrics\", \"create\")",
        "(\"greige-fabrics\", \"update\")",
        "(\"greige-fabrics\", \"delete\")",
    ] {
        assert!(
            !block.contains(banned),
            "采购两岗对坯布档案只读是最小授权口径，矩阵 purchase 分组禁止出现 {banned}"
        );
    }
}

/// 通道②：迁移的 ROLE_CODES 恰为采购两岗。
#[test]
fn migration_role_codes_are_exactly_purchase_roles() {
    let code = code_only(&migration_src());
    let mut tokens = const_tokens(&code, "ROLE_CODES");
    tokens.sort();
    let mut expected: Vec<String> = PURCHASE_ROLES.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(
        tokens, expected,
        "m0102 ROLE_CODES 应恰为采购两岗，不得增删角色面"
    );
}

/// 通道②：迁移的 GRANTS 恰为一条 (greige-fabrics, read) 配对。
#[test]
fn migration_grants_are_exactly_one_read_pair() {
    let code = code_only(&migration_src());
    let tokens = const_tokens(&code, "GRANTS");
    assert_eq!(
        tokens,
        vec!["greige-fabrics".to_string(), "read".to_string()],
        "m0102 GRANTS 应恰为 greige-fabrics 的 read 一枚，不得夹带写动作"
    );
}

/// 通道②：up 幂等用 NOT EXISTS，只写一条授予，不提前动全局唯一约束。
#[test]
fn migration_up_is_idempotent_and_single_grant() {
    let code = code_only(&migration_src());
    let up = up_execution(&code);
    assert!(
        up.contains("NOT EXISTS"),
        "m0102 up 应用 WHERE NOT EXISTS 保证重跑等价"
    );
    assert!(
        !up.contains("ON CONFLICT"),
        "m0102 不应依赖 domain/v15 才建的唯一索引用 ON CONFLICT"
    );
    assert_eq!(
        up.matches("INSERT INTO \"role_permissions\"").count(),
        1,
        "m0102 只应有一条授予 INSERT"
    );
    assert!(
        up.contains("RAISE NOTICE"),
        "0 命中/缺失角色码必须 RAISE NOTICE 可见化，禁止静默通过"
    );
    for banned in ["ALTER TABLE", "CREATE UNIQUE INDEX"] {
        assert!(
            !up.contains(banned),
            "m0102 不得为一条授予去动表结构或全局约束（禁止 {banned}）"
        );
    }
}

/// 通道②：down 的回收面按 GRANTS 常量的 (resource, action) 对圈定。
/// 回收 SQL 由迭代 GRANTS 经 format! 插值生成，资源与动作字面量存在于常量源、
/// 不在函数体内，故判据同源解析：GRANTS 集合恰为 {(greige-fabrics, read)} 一枚，
/// 且函数体同时带 resource_type 与 action 两个谓词并按角色码 ANY 圈定；
/// 多一枚少一枚、或任一拍缺失即红。
#[test]
fn migration_down_revokes_only_this_grant() {
    let code = code_only(&migration_src());
    let down = down_execution(&code);
    let flat = canon(&down);

    let grants = const_tokens(&code, "GRANTS");
    assert_eq!(
        grants,
        vec!["greige-fabrics".to_string(), "read".to_string()],
        "m0102 GRANTS 回收面必须恰为 greige-fabrics 的 read 一对 (resource, action)，多一枚少一枚都红"
    );

    assert!(
        flat.contains("DELETEFROM\"role_permissions\""),
        "m0102 down 必须有真实回收，禁止空实现"
    );
    assert!(
        down.contains("GRANTS") && down.contains(".iter()") && down.contains(".map("),
        "m0102 down 必须由迭代 GRANTS 生成 DELETE，禁止把回收面写死在函数体或干脆不迭代"
    );
    assert!(
        flat.contains("rp.\"resource_type\"='{res}'") && flat.contains("rp.\"action\"='{act}'"),
        "m0102 down 必须带 resource_type 与 action 双谓词，逐对插值 GRANTS 的解构变量"
    );
    assert!(
        down.contains("ANY(") && down.contains("sql_array(ROLE_CODES)"),
        "m0102 down 必须按本迁移的角色码数组精确圈定，人工另授不动"
    );
    assert!(!down.contains("\"*\""), "m0102 down 不得删除通配授予");
}

/// 通道②：与 m0097 的受授角色集合不相交，down 不会误伤生产与仓储岗的同名键。
#[test]
fn migration_role_codes_are_disjoint_from_prior_greige_grant() {
    let code = code_only(&migration_src());
    let prior = code_only(&prior_migration_src());
    let now = const_tokens(&code, "ROLE_CODES");
    let before = const_tokens(&prior, "ROLE_CODES");
    for role in &now {
        assert!(
            !before.contains(role),
            "m0102 与 m0097 的受授角色 {role} 重叠会致 down 回收越界"
        );
    }
}

/// 通道②：注册形态必须是顶层域 m_purchase_greige_read_grant，位置以依赖锚点判定。
/// 嵌入已应用的 business 域会在复用库上随整域跳过而永不执行（本仓既有家族缺陷），
/// 故钉死四点：域文件内真实调用 m0102、lib.rs 顶层清单恰登记一次、登记位置排在
/// 升域锚 ap_family_grant 与依赖域 v15 之后、business 链不残留双注册。
/// 数组末位不是判据：后续合并合法追加的新域只会排在本域之后，不许因此顶红本锁；
/// 反向同理，本域被重排到依赖之前或被内嵌回聚合域，两形都必红。
#[test]
fn migration_is_registered_as_chain_tail_top_domain() {
    let domain = code_only(&top_domain_src());
    let flat_domain = canon(&domain);
    assert!(
        domain.contains("mod m0102_grant_purchase_greige_read;"),
        "m0102 必须在新顶层域 mod.rs 声明，否则不参与迁移链（写了文件不等于跑了迁移）"
    );
    assert!(
        flat_domain.contains("fnname(&self)->&'staticstr{\"m_purchase_greige_read_grant\"}"),
        "新顶层域的台账名必须是 m_purchase_greige_read_grant，链尾登记与账本核对靠它唯一认定"
    );
    assert!(
        flat_domain.contains("m0102_grant_purchase_greige_read::Migration.up(manager).await?;"),
        "新顶层域 up 必须真实调用 m0102，只声明不调用等于没注册"
    );
    assert!(
        flat_domain.contains("m0102_grant_purchase_greige_read::Migration.down(manager).await?;"),
        "新顶层域 down 必须真实调用 m0102 的回滚，否则该域不可独立回退"
    );

    let lib = code_only(&lib_chain_src());
    let mig_at = lib
        .find("fn migrations() -> Vec<Box<dyn MigrationTrait>>")
        .expect("lib.rs 缺顶层迁移清单函数 migrations");
    let body = &lib[mig_at..];
    let reg_needle = "Box::new(domain::purchase_greige_read_grant::Migration)";
    assert_eq!(
        body.matches(reg_needle).count(),
        1,
        "purchase_greige_read_grant 在顶层清单必须恰登记一次（重复登记会在部分库上重复执行回收）"
    );
    let prior_reg = body
        .find("Box::new(domain::ap_family_grant::Migration)")
        .expect("lib.rs 链必须有前链尾 ap_family_grant 登记");
    let this_reg = body
        .find(reg_needle)
        .expect("lib.rs 链必须登记新顶层域 purchase_greige_read_grant");
    assert!(
        this_reg > prior_reg,
        "m0102 顶层域必须保持在既有链尾 ap_family_grant 之后（升域前它已是最后应用的域）"
    );
    let v15_reg = body
        .find("Box::new(domain::v15::Migration)")
        .expect("lib.rs 必须有 v15 顶层域登记");
    assert!(
        this_reg > v15_reg,
        "本域补授依赖 v15 建的 uq_role_permissions 唯一索引，登记必须始终排在其后（前置即回退时序失真）"
    );

    let business = code_only(&business_chain_src());
    assert!(
        !business.contains("m0102_grant_purchase_greige_read"),
        "business 域不得再残留 m0102 声明或调用，双注册会在部分库上重复执行回收"
    );
    assert!(
        !business.contains("purchase_greige_read_grant::Migration"),
        "business 域不得把本顶层域内嵌回自己的清单（内嵌即随整域跳过而永不执行，防退化反向钉）"
    );
}

/// 通道③：e2e SEED 对 greige-fabrics:read 覆盖采购两岗。
#[test]
fn e2e_seed_covers_both_purchase_roles() {
    let roles = e2e_seed_roles_with("greige-fabrics:read");
    for role in PURCHASE_ROLES {
        assert!(
            roles.iter().any(|r| r == role),
            "通道③ e2e SEED 权限 greige-fabrics:read 应覆盖角色 {role}，实际 {roles:?}"
        );
    }
}
