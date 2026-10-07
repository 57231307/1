//! 产品分类树权限键三通道同源锁（角色矩阵 purchaser 真缺口 · 裁定 R-6）
//!
//! 缺陷事实：`/api/v1/erp/product-categories*` 由 `routes/mod.rs:368-391` 别名段注册，
//! 运行时权限键是 `product-categories:read`（`middleware/permission.rs::extract_resource_info`
//! 取路径段作资源段），而 `matches_permission`（`permission.rs:601-610`）按资源段**精确**
//! 匹配 ⇒ 已注册的 `categories` 资源码不覆盖它。该键此前既不在 `PERMISSION_RESOURCES`、
//! 也没有任何角色被授予 ⇒ 除 admin（`*:*`）外，采购岗读产品分类树恒 403，
//! 「我方 SKU↔供应商 SKU 对照表」维护页对目标岗位实际不可用，而 e2e 主体用 admin 登录测不到。
//!
//! 三条授予通道必须同口径，本测逐 token 双向钉死，防止"一边改一边忘"：
//! ① 新装库角色矩阵 `backend/src/services/init_service_ops/permission.rs`（purchase 域分组）
//! ② 存量库迁移 `backend/migration/src/domain/business/m0072_grant_product_categories_read.rs`
//! ③ e2e 角色补建 `frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS（purchaser）
//! 并在 ①的注册表 `backend/src/services/init_service.rs` 里钉 `product-categories` 已登记。
//!
//! 质量红线口径：本锁**不许**被"放宽集合"用来变绿。受授岗位集合（三个采购岗码 + e2e 别名码）
//! 与动作集合（只有 read）任何增减都必须同时改三处并在此改判；
//! `("product-categories", "*")` 与 create/update/delete 一律禁止（最小授权：采购岗只需读树），
//! 非采购岗（销售/仓管/生产/财务/CRM）一律不得出现在授予集合里。

/// 通道 ① 矩阵里应授 `product-categories:read` 的角色码（后端权威码）
const MATRIX_PURCHASE_ROLES: &[&str] =
    &["purchase_manager", "purchase_clerk", "sourcing_specialist"];

/// 通道 ② 迁移的目标角色码 = 矩阵码 + e2e/部署侧别名码（库中不存在的码由迁移自然跳过）
const MIGRATION_ALIAS_ROLES: &[&str] = &["purchaser"];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn migration_src() -> String {
    include_str!("../migration/src/domain/business/m0072_grant_product_categories_read.rs")
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

/// 只保留"代码 + 字符串字面量"：整行注释（`//`/`///`/`//!`，TS 侧同形）逐行剔除。
/// 本锁的禁项判据（不得出现 `("product-categories", "*")` / `ON CONFLICT` /
/// `CREATE UNIQUE INDEX`）与计数判据（read 恰 3 处）都必须只针对执行体：
/// 说明性注释（"禁止写 product-categories:*"、"为何不用 ON CONFLICT"）不是授予，计入即假判。
/// 按行处理而不做字符级扫描：迁移里跨行 raw string（SQL）引号成对但行数多，
/// 单行配平会把 SQL 文本当注释误判；宁少剥（行尾尾注释、块注释不动）不可错吃代码。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（仅用于正向存在性判定）：剔全部空白并消掉闭合定界符前的尾逗号，
/// 使判据不受 rustfmt 折行/尾逗号排版影响。
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

/// m0072 的 up 执行体（符号定位，不含文件头/函数头文档注释）
fn up_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn up(")
        .expect("m0072 必须有 async fn up（执行体判据的起点符号）");
    let end_rel = code_src[start..]
        .find("async fn down(")
        .expect("m0072 必须有 async fn down（up 执行体的右边界符号）");
    code_src[start..start + end_rel].to_string()
}

/// m0072 的 down 执行体（down 是最后一个方法，切到文件尾）
fn down_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn down(")
        .expect("m0072 必须有 async fn down（回滚执行体）");
    code_src[start..].to_string()
}

/// 矩阵里 purchase 域分组的函数体（从 `fn purchase_role_resources` 到下一个 `fn` 边界）
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

/// 通道 ①：矩阵内 product-categories 的授予集合必须恰为"三个采购岗 × read"
#[test]
fn role_matrix_grants_product_categories_read_exactly_once_per_purchase_role() {
    let src = code_only(&matrix_src());
    let block = purchase_block(&src);
    let needle = "(\"product-categories\", \"read\")";
    let total = src.matches(needle).count();
    let in_block = block.matches(needle).count();
    assert_eq!(
        in_block, 3,
        "矩阵 purchase 分组里 product-categories:read 应恰 3 处（三个采购岗各一条），实得 {in_block} 处"
    );
    assert_eq!(
        total, in_block,
        "矩阵里 product-categories:read 只能出现在 purchase 分组（越组授予=扩权），全文件计数 {total} vs 分组内 {in_block}"
    );
    for role in MATRIX_PURCHASE_ROLES {
        assert!(
            block.contains(&format!("\"{role}\"")),
            "矩阵 purchase 分组应含角色码 {role}"
        );
    }
}

/// 通道 ① 最小授权禁项：不得出现通配或写动作授予
#[test]
fn role_matrix_forbids_wildcard_and_write_actions_for_product_categories() {
    let src = code_only(&matrix_src());
    for banned in [
        "(\"product-categories\", \"*\")",
        "(\"product-categories\", \"create\")",
        "(\"product-categories\", \"update\")",
        "(\"product-categories\", \"delete\")",
    ] {
        assert!(
            !src.contains(banned),
            "产品分类树对采购岗只读是裁定 R-6 的最小授权口径，矩阵禁止出现 {banned}"
        );
    }
}

/// 通道 ②：迁移的受授码必须覆盖矩阵码（含别名码只多不少），否则存量库授不上=缺口复发
#[test]
fn migration_target_codes_cover_matrix_roles_plus_aliases() {
    let src = code_only(&migration_src());
    let start = src
        .find("const READ_ROLE_CODES")
        .expect("m0072 必须有 READ_ROLE_CODES 常量（受授集合的唯一来源）");
    let end = src[start..]
        .find("];")
        .expect("READ_ROLE_CODES 数组必须有闭合符");
    let list = &src[start..start + end];
    for role in MATRIX_PURCHASE_ROLES.iter().chain(MIGRATION_ALIAS_ROLES) {
        assert!(
            list.contains(&format!("\"{role}\"")),
            "m0072 的受授角色码缺 {role}（通道 ①② 口径不一致，存量库会重演 purchaser 403 缺口）"
        );
    }
}

/// 通道 ②：up 执行体必须是"只授 read 的幂等 INSERT"，且不得借机动全局约束
#[test]
fn migration_up_is_idempotent_read_only_grant() {
    let up = up_execution(&code_only(&migration_src()));
    let flat = canon(&up);
    assert!(
        flat.contains("INSERTINTO\"role_permissions\""),
        "m0072 的 up 必须真实写入 role_permissions"
    );
    assert!(
        flat.contains("'product-categories'") && flat.contains("'read'"),
        "m0072 的 up 必须只授 product-categories × read"
    );
    assert!(
        up.contains("NOT EXISTS"),
        "m0072 必须用 NOT EXISTS 保证重跑等价（同 m0069 幂等口径）"
    );
    assert_eq!(
        up.matches("INSERT INTO \"role_permissions\"").count(),
        1,
        "m0072 只应有一条授予 INSERT（出现第二条=未登记的扩权）"
    );
    for banned in ["ON CONFLICT", "CREATE UNIQUE INDEX", "ALTER TABLE"] {
        assert!(
            !up.contains(banned),
            "m0072 不得为一条授予去动全局约束/表结构（禁止 {banned}）"
        );
    }
    assert!(
        up.contains("RAISE NOTICE"),
        "0 命中/缺失角色码必须 RAISE NOTICE 可见化，禁止静默通过"
    );
}

/// 通道 ② 回滚必须真实可逆（教训：rls_dept 的空 down）
#[test]
fn migration_down_revokes_only_this_grant() {
    let down = down_execution(&code_only(&migration_src()));
    let flat = canon(&down);
    assert!(
        flat.contains("DELETEFROM\"role_permissions\""),
        "m0072 的 down 必须真实回收，禁止空实现"
    );
    assert!(
        flat.contains("'product-categories'") && flat.contains("'read'"),
        "m0072 的 down 必须按 product-categories × read 圈定回收面"
    );
    assert!(
        down.contains("ANY(") && down.contains("roles"),
        "m0072 的 down 必须按本迁移的角色码精确圈定，不得整表删该资源键（人工另授不动）"
    );
}

/// 通道 ②：注册位置必须在 business 域 up 链尾 / down 链首（晚于 system 域的角色/权限表）
#[test]
fn migration_is_registered_at_business_chain_tail_and_down_head() {
    let src = code_only(&business_chain_src());
    let flat = canon(&src);
    assert!(
        src.contains("mod m0072_grant_product_categories_read;"),
        "m0072 必须在本域 mod.rs 声明，否则不参与迁移链（写了文件≠跑了迁移）"
    );
    let m0071_up = flat
        .find("m0071_normalize_array_columns::Migration.up(manager).await?;")
        .expect("business up 链必须有 m0071 调用符号");
    let m0072_up = flat
        .find("m0072_grant_product_categories_read::Migration.up(manager).await?;")
        .expect("business up 链必须调用 m0072（注册在链尾才晚于建表）");
    assert!(
        m0072_up > m0071_up,
        "m0072 必须排在 business 域 up 链尾（m0071 之后）"
    );
    let m0072_down = flat
        .find("m0072_grant_product_categories_read::Migration.down(manager).await?;")
        .expect("business down 链必须调用 m0072");
    let m0071_down = flat
        .find("m0071_normalize_array_columns::Migration.down(manager).await?;")
        .expect("business down 链必须有 m0071 调用符号");
    assert!(
        m0072_down < m0071_down,
        "down 必须逆序：最后应用者（m0072）最先回滚"
    );
}

/// 注册表：资源码必须登记，否则权限面不可见、授了也可能被上游资源白名单拒
#[test]
fn registry_declares_product_categories_resource() {
    let src = code_only(&registry_src());
    assert!(
        src.contains("\"product-categories\","),
        "PERMISSION_RESOURCES 必须登记 product-categories（它与 categories 是两条不同资源码，后者不覆盖前者）"
    );
}

/// 通道 ③：e2e 侧 purchaser 必须带同一权限码，否则 CI 库永远测不到该功能
#[test]
fn e2e_seed_channel_carries_the_same_code() {
    let src = code_only(&e2e_seed_src());
    assert!(
        src.contains("'product-categories:read'"),
        "global-setup.ts 的 SEED_ROLE_EXTRA_PERMISSIONS 必须含 product-categories:read（三通道同口径）"
    );
}
