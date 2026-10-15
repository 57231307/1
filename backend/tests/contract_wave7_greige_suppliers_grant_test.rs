//! 坯布建档与供应商下拉三岗授权三通道同源锁
//!
//! 缺陷事实：`/api/v1/erp/production/greige-fabrics` 的运行时权限键为
//! `greige-fabrics:read` / `greige-fabrics:create`；`/api/v1/erp/purchase/suppliers`
//! 的运行时权限键为 `suppliers:read`（`extract_resource_info` 按路径段推导）。
//! init 矩阵已补授，但存量库若不跑迁移则三岗恒 403，e2e 若 SEED 不含同口径键则
//! 非 admin 登录测不到该功能——属"三通道缺任一即假绿"家族。
//!
//! 三条授予通道必须同口径，本测逐 token 双向钉死，防止"一边改一边忘"：
//! - 通道 1 矩阵 `backend/src/services/init_service_ops/permission.rs`
//!   （warehouse_keeper/warehouse_manager 分组 + production_manager 分组）
//! - 通道 2 迁移 `backend/migration/src/domain/business/m0097_grant_greige_create_and_suppliers_read.rs`
//! - 通道 3 e2e `frontend/e2e/global-setup.ts` SEED_ROLE_EXTRA_PERMISSIONS
//!   的 production_manager / warehouse_keeper / warehouse_manager
//!
//! 最小授权口径：只授 (greige-fabrics read/create) + (suppliers read)，
//! 禁止通配、update、delete。

/// 受授角色码（矩阵权威码，无部署/e2e 别名）
const GRANT_ROLES: &[&str] = &[
    "production_manager",
    "warehouse_keeper",
    "warehouse_manager",
];

/// 应授予的 (resource, action) 配对（与迁移 GRANTS 常量逐字符一致）
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

fn registry_src() -> String {
    include_str!("../src/services/init_service.rs").replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

fn business_chain_src() -> String {
    include_str!("../migration/src/domain/business/mod.rs").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：整行注释逐行剔除。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（仅用于正向存在性判定）：剔全部空白并消掉尾逗号。
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

/// 从矩阵中定位某角色分组体（从角色码行到下一个 `(` 角色码行之前的数组体）
fn find_role_block(code_src: &str, role: &str) -> String {
    let needle = format!("\"{role}\",");
    let start = code_src
        .find(&needle)
        .unwrap_or_else(|| panic!("矩阵必须有角色码 {role} 的分组"));
    let rest = &code_src[start..];
    let block_start = rest.find("&[").expect("角色分组体必须以 &[ 开头");
    let after = &rest[block_start..];
    let next_role = after[2..]
        .find(&needle)
        .or_else(|| after[2..].find("\n            (\"").map(|i| i + 2))
        .or_else(|| after[2..].find("\n            (\"").map(|i| i + 2))
        .unwrap_or(after.len() - 2);
    after[..next_role].to_string()
}

/// m0097 的 up 执行体
fn up_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn up(")
        .expect("m0097 必须有 async fn up");
    let end_rel = code_src[start..]
        .find("async fn down(")
        .expect("m0097 必须有 async fn down（up 的右边界）");
    code_src[start..start + end_rel].to_string()
}

/// m0097 的 down 执行体
fn down_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn down(")
        .expect("m0097 必须有 async fn down（回滚执行体）");
    code_src[start..].to_string()
}

/// 通道 1：矩阵里三岗各分组必须包含三条授予元组
#[test]
fn matrix_three_roles_have_all_grant_tuples() {
    let src = code_only(&matrix_src());
    for role in GRANT_ROLES {
        let block = find_role_block(&src, role);
        for (res, act) in GRANTS {
            let needle = format!("(\"{res}\", \"{act}\")");
            assert!(
                block.contains(&needle),
                "矩阵 {role} 分组必须含 {needle}（通道 1 缺授则新装库 403）"
            );
        }
    }
}

/// 通道 1 最小授权：三岗分组内不得出现通配或写动作
#[test]
fn matrix_forbids_wildcard_and_unauthorized_actions() {
    let src = code_only(&matrix_src());
    let banned_res_actions: &[(&str, &str)] = &[
        ("greige-fabrics", "*"),
        ("greige-fabrics", "update"),
        ("greige-fabrics", "delete"),
        ("suppliers", "*"),
        ("suppliers", "create"),
        ("suppliers", "update"),
        ("suppliers", "delete"),
    ];
    for role in GRANT_ROLES {
        let block = find_role_block(&src, role);
        for (res, act) in banned_res_actions {
            let needle = format!("(\"{res}\", \"{act}\")");
            assert!(
                !block.contains(&needle),
                "最小授权禁项：{role} 分组不得出现 {needle}"
            );
        }
    }
}

/// 通道 2：迁移的 ROLE_CODES 必须恰为三岗矩阵码（无多余别名码）
#[test]
fn migration_role_codes_exact_match_three_roles() {
    let src = code_only(&migration_src());
    let start = src
        .find("const ROLE_CODES")
        .expect("m0097 必须有 ROLE_CODES 常量");
    let end = src[start..].find("];").expect("ROLE_CODES 数组必须闭合");
    let list = &src[start..start + end];
    for role in GRANT_ROLES {
        assert!(
            list.contains(&format!("\"{role}\"")),
            "m0097 ROLE_CODES 缺 {role}（通道 2 不覆盖=存量库该岗仍 403）"
        );
    }
    let extra_tokens: Vec<&str> = list
        .split('"')
        .filter(|t| !t.is_empty() && !GRANT_ROLES.contains(t))
        .collect();
    assert!(
        extra_tokens.is_empty(),
        "m0097 ROLE_CODES 含非口径角色码 {extra_tokens:?}（本迁移受授面无别名码）"
    );
}

/// 通道 2：迁移的 GRANTS 常量必须恰为三条配对
#[test]
fn migration_grants_constants_cover_three_pairs() {
    let src = code_only(&migration_src());
    let start = src.find("const GRANTS").expect("m0097 必须有 GRANTS 常量");
    let end = src[start..].find("];").expect("GRANTS 数组必须闭合");
    let block = &src[start..start + end];
    let flat = canon(block);
    for (res, act) in GRANTS {
        let needle = canon(&format!("(\"{res}\", \"{act}\")"));
        assert!(
            flat.contains(&needle),
            "m0097 GRANTS 必须含 (\"{res}\", \"{act}\")"
        );
    }
}

/// 通道 2：up 执行体必须是三条 NOT EXISTS 幂等 INSERT
#[test]
fn migration_up_is_idempotent_three_inserts() {
    let up = up_execution(&code_only(&migration_src()));
    let flat = canon(&up);
    assert!(
        flat.contains("INSERTINTO\"role_permissions\""),
        "m0097 up 必须写入 role_permissions"
    );
    for (res, act) in GRANTS {
        assert!(
            flat.contains(&canon(&format!("'{res}'")))
                && flat.contains(&canon(&format!("'{act}'"))),
            "m0097 up 必须授 {res}×{act}"
        );
    }
    assert!(
        up.contains("NOT EXISTS"),
        "m0097 必须用 NOT EXISTS 保证重跑等价"
    );
    assert_eq!(
        up.matches("INSERT INTO \"role_permissions\"").count(),
        3,
        "m0097 应恰有 3 条授予 INSERT（read×2 + create×1）"
    );
    for banned in ["ON CONFLICT", "CREATE UNIQUE INDEX", "ALTER TABLE"] {
        assert!(
            !up.contains(banned),
            "m0097 不得动全局约束（禁止 {banned}）"
        );
    }
    assert!(
        up.contains("RAISE NOTICE"),
        "0 命中必须 RAISE NOTICE 可见化，禁止静默通过"
    );
}

/// 通道 2：down 必须精确回收三条键行
#[test]
fn migration_down_revokes_only_three_grants() {
    let down = down_execution(&code_only(&migration_src()));
    let flat = canon(&down);
    assert!(
        flat.contains("DELETEFROM\"role_permissions\""),
        "m0097 down 必须真实回收"
    );
    for (res, act) in GRANTS {
        assert!(
            flat.contains(&canon(&format!("'{res}'")))
                && flat.contains(&canon(&format!("'{act}'"))),
            "m0097 down 必须按 {res}×{act} 回收"
        );
    }
    assert!(
        down.contains("ANY(") && down.contains("roles"),
        "down 必须按角色码圈定，不得整表删"
    );
}

/// 通道 2：注册位置必须在 business 域 up 链尾 / down 链首
#[test]
fn migration_registered_at_business_chain_tail() {
    let src = code_only(&business_chain_src());
    assert!(
        src.contains("mod m0097_grant_greige_create_and_suppliers_read;"),
        "m0097 必须在 business mod.rs 声明"
    );
    let flat = canon(&src);
    assert!(
        flat.contains(
            "m0097_grant_greige_create_and_suppliers_read::Migration.up(manager).await?;"
        ),
        "m0097 必须接入 business up 链"
    );
    assert!(
        flat.contains(
            "m0097_grant_greige_create_and_suppliers_read::Migration.down(manager).await?;"
        ),
        "m0097 必须接入 business down 链"
    );
    let m0096_up = flat
        .find("m0096_grant_sku_mapping_import_purchase_roles::Migration.up(manager).await?;")
        .expect("business up 链有 m0096");
    let m0097_up = flat
        .find("m0097_grant_greige_create_and_suppliers_read::Migration.up(manager).await?;")
        .expect("business up 链有 m0097");
    assert!(m0097_up > m0096_up, "m0097 必须排在 m0096 之后（up 链尾）");
    let m0097_down = flat
        .find("m0097_grant_greige_create_and_suppliers_read::Migration.down(manager).await?;")
        .expect("business down 链有 m0097");
    let m0096_down = flat
        .find("m0096_grant_sku_mapping_import_purchase_roles::Migration.down(manager).await?;")
        .expect("business down 链有 m0096");
    assert!(
        m0097_down < m0096_down,
        "down 逆序：最后应用者（m0097）最先回滚"
    );
}

/// 通道 0：资源码必须在 PERMISSION_RESOURCES 注册
#[test]
fn registry_declares_greige_fabrics_and_suppliers() {
    let src = code_only(&registry_src());
    assert!(
        src.contains("\"greige-fabrics\","),
        "PERMISSION_RESOURCES 必须登记 greige-fabrics"
    );
    assert!(
        src.contains("\"suppliers\","),
        "PERMISSION_RESOURCES 必须登记 suppliers"
    );
}

/// 通道 3：e2e SEED 三岗条目必须含同口径权限码
#[test]
fn e2e_seed_three_roles_carry_same_keys() {
    let src = code_only(&e2e_seed_src());
    for role in GRANT_ROLES {
        let anchor = format!("  {role}: [");
        let start = src
            .find(&anchor)
            .unwrap_or_else(|| panic!("SEED_ROLE_EXTRA_PERMISSIONS 必须有 {role} 条目"));
        let rest = &src[start..];
        let close = rest.find(']').expect("SEED 条目数组必须闭合");
        let body = &rest[..close];
        for (res, act) in GRANTS {
            let key = format!("'{res}:{act}'");
            assert!(
                body.contains(&key),
                "SEED {role} 必须含 {key}（通道 3 缺码=CI 非 admin 测不到该功能）"
            );
        }
    }
}

/// 通道 3：e2e SEED 侧最小授权禁项
#[test]
fn e2e_seed_forbids_wildcard_for_greige_and_suppliers() {
    let src = code_only(&e2e_seed_src());
    for role in GRANT_ROLES {
        let anchor = format!("  {role}: [");
        let start = src.find(&anchor).unwrap();
        let rest = &src[start..];
        let close = rest.find(']').unwrap();
        let body = &rest[..close];
        for banned in [
            "'greige-fabrics:*'",
            "'greige-fabrics:update'",
            "'greige-fabrics:delete'",
            "'suppliers:*'",
            "'suppliers:create'",
            "'suppliers:update'",
            "'suppliers:delete'",
        ] {
            assert!(
                !body.contains(banned),
                "SEED {role} 禁止出现 {banned}（最小授权）"
            );
        }
    }
}
