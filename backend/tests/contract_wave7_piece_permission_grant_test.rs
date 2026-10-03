//! 匹号领域权限键三通道同源锁（复审 #4669 挂账：`pieces` 资源自始未注册）
//!
//! 缺陷事实：`/api/v1/erp/inventory/pieces`（四维选匹）与
//! `/api/v1/erp/inventory/pieces/{id}/print`（#220 成品布入库标签）的权限键是
//! `pieces:read` / `pieces:print`（`middleware/permission.rs::extract_resource_info`
//! 对模块前缀 inventory 取 segment4，`utils/path_utils.rs:102-117` 默认分支），
//! 而 `matches_permission`（`permission.rs:601-610`）按资源段**精确**匹配
//! （只有 `*` 通配可覆盖）⇒ 该资源既不在 `PERMISSION_RESOURCES`、也没有任何角色被授予时，
//! 除 admin 外所有人访问匹号端点一律 403，本波交付的「发货匹号选择器」「成品布标签」
//! 对仓管/验布员/销售实际不可用；CI e2e 用 admin 登录，测不到（假绿家族）。
//!
//! 三条授予通道必须同口径，本测逐 token 双向钉死，防止"一边改一边忘"：
//! ① 新装库角色矩阵 `backend/src/services/init_service_ops/permission.rs`
//! ② 存量库迁移 `backend/migration/src/domain/production/m0069_grant_piece_read_and_print.rs`
//! ③ e2e 角色补建 `frontend/e2e/global-setup.ts` 的 SEED_ROLE_EXTRA_PERMISSIONS
//! 并在 ①的注册表 `backend/src/services/init_service.rs` 里钉 `pieces` 资源已登记。
//!
//! 质量红线口径：本锁不许被"放宽集合"用来变绿——集合扩/缩都必须同时改三处并在此改判，
//! 且 `("pieces", "*")` / update / delete / create 一律禁止（最小授权：库里只有 read、
//! print 两个 pieces 端点，见 `routes/inventory.rs::piece_routes`）。

/// 应被授 `pieces:read` 的角色码（矩阵码 + 部署/e2e 别名码，见 m0069 文件头口径）
const READ_ROLES: &[&str] = &[
    "inventory_manager",
    "warehouse_keeper",
    "warehouse_manager",
    "sales_rep",
    "salesperson",
    "quality_inspector",
    "fabric_inspector",
    "production_manager",
];

/// 应被授 `pieces:print` 的角色码（本岗产出成品布/入库的人）
const PRINT_ROLES: &[&str] = &[
    "inventory_manager",
    "warehouse_keeper",
    "warehouse_manager",
    "quality_inspector",
    "fabric_inspector",
];

fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

fn migration_src() -> String {
    include_str!("../migration/src/domain/production/m0069_grant_piece_read_and_print.rs")
        .replace('\r', "")
}

fn registry_src() -> String {
    include_str!("../src/services/init_service.rs").replace('\r', "")
}

fn e2e_seed_src() -> String {
    include_str!("../../frontend/e2e/global-setup.ts").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：整行注释（`//`/`///`/`//!`，TS 侧同样是 `//`）逐行剔除。
/// 三通道锁里有**禁项**（不得出现 `("pieces", "*")` / `ON CONFLICT` / `CREATE UNIQUE INDEX`）
/// 与**计数**（read 恰 6 处、print 恰 4 处）两类判据，二者都必须只针对执行体：
/// 说明性注释（"禁止写 pieces:*"、"为何不用 ON CONFLICT"）不是授予，计入即假判。
/// 按行处理而不做字符级扫描：迁移里跨行 raw string（SQL）的引号成对但行数多，
/// 单行配平会把 SQL 文本当代码/注释误判；宁少剥（行尾尾注释、块注释不动）不可错吃代码。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（**仅用于正向存在性判定**）：剔全部空白并消掉闭合定界符前的尾逗号。
/// 迁移链注册用的 needle 原本是"符号 + 换行 + 12 空格缩进"的字面量，rustfmt 一改排版
/// 就假失败——链式调用是否发生与它怎么折行无关，故双侧同一套 canon 后再比。
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

/// m0069 的 up 执行体：符号定位（`async fn up(` → `async fn down(`），
/// 不含文件头/函数头文档注释——原写法 `&src[..find("async fn down")]` 从 0 起切，
/// 把文件头「幂等实现细节」那段解释"为何不用 ON CONFLICT"的说明算进了执行体。
fn up_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn up(")
        .expect("m0069 必须有 async fn up（执行体判据的起点符号）");
    let end_rel = code_src[start..]
        .find("async fn down(")
        .expect("m0069 必须有 async fn down（up 执行体的右边界符号）");
    code_src[start..start + end_rel].to_string()
}

/// m0069 的 down 执行体：同样从符号起切，切到文件尾（down 是最后一个方法）。
fn down_execution(code_src: &str) -> String {
    let start = code_src
        .find("async fn down(")
        .expect("m0069 必须有 async fn down（回滚执行体）");
    code_src[start..].to_string()
}

/// 通道 ①：矩阵里 pieces 的授予集合必须恰为口径集（数量+最小授权禁项）
#[test]
fn role_matrix_grants_exactly_read_and_print_for_pieces() {
    let src = code_only(&matrix_src());
    assert_eq!(
        src.matches(r#"("pieces", "read")"#).count(),
        6,
        "pieces:read 在角色矩阵中应恰为 6 处（库存经理/仓管/销售代表/生产经理/质检员/验布员）"
    );
    assert_eq!(
        src.matches(r#"("pieces", "print")"#).count(),
        4,
        "pieces:print 在角色矩阵中应恰为 4 处（库存经理/仓管/质检员/验布员）"
    );
    // 最小授权：库里 pieces 只有 GET /pieces 与 GET /pieces/{id}/print 两个端点
    for banned in [
        r#"("pieces", "*")"#,
        r#"("pieces", "create")"#,
        r#"("pieces", "update")"#,
        r#"("pieces", "delete")"#,
        r#"("pieces", "export")"#,
    ] {
        assert!(
            !src.contains(banned),
            "禁止超范围授予 pieces 键（越权面放大）: {banned}"
        );
    }
}

/// 通道 ⓪：资源段必须登记进权限资源注册表（否则权限目录里根本没有该资源）
#[test]
fn pieces_resource_is_registered_in_permission_catalog() {
    let src = code_only(&registry_src());
    assert!(
        src.contains(r#"pub const PERMISSION_RESOURCES"#),
        "PERMISSION_RESOURCES 注册表缺失"
    );
    assert!(
        src.contains("\"pieces\","),
        "pieces 必须登记进 PERMISSION_RESOURCES（资源段由 URL segment4 推导，与 inventory 不同源）"
    );
}

/// 通道 ②：迁移按角色码授予，且口径与矩阵一致；幂等实现不得依赖唯一索引
#[test]
fn migration_grants_same_role_set_and_is_idempotent_without_on_conflict() {
    let mig = code_only(&migration_src());
    // 角色码全部在迁移常量清单里逐字在场
    for code in READ_ROLES {
        assert!(
            mig.contains(&format!("\"{code}\"")),
            "m0069 缺少受授角色码: {code}"
        );
    }
    // print 集合必须是 read 集合子集（先能看匹，才谈得上打印）
    for code in PRINT_ROLES {
        assert!(
            READ_ROLES.contains(code),
            "口径自检失败：print 角色 {code} 不在 read 集合内"
        );
    }
    // 两条 INSERT 各自 NOT EXISTS 幂等；禁止 ON CONFLICT（依赖 v15 才建的唯一索引）
    assert_eq!(
        mig.matches(r#"SELECT r."id", 'pieces'"#).count(),
        2,
        "m0069 应恰有 read / print 两条授予 INSERT"
    );
    assert_eq!(
        mig.matches("AND NOT EXISTS (").count(),
        2,
        "两条授予都必须带 NOT EXISTS 幂等守卫"
    );
    // 禁项只查执行体（文件头注释里出现"ON CONFLICT"是解释为何不用它）——
    // 执行体的定位必须是**符号**：早先用 `&src[..find("async fn down")]` 从文件 0 起切，
    // 把 m0069 文件头 §幂等实现细节 那段"用 WHERE NOT EXISTS 而不是 ON CONFLICT (…)
    // 动全局约束、并可能因存量重复行让迁移链中断"的说明文字算进了执行体（#4671 判责
    // B1①/B1③ 的同一形态，且本行注释自陈"禁项只查执行体"——实现与意图矛盾）。
    let up_body = up_execution(&mig);
    for banned in ["ON CONFLICT", "CREATE UNIQUE INDEX"] {
        assert!(
            !up_body.contains(banned),
            "m0069 执行体不得使用 {banned}（见文件头「幂等实现细节」）：\n{up_body}"
        );
    }
    // 授予结果不得静默：0 命中/缺失角色码都要打 NOTICE
    assert!(
        up_body.contains("RAISE NOTICE") && up_body.contains("hit = 0"),
        "m0069 必须把「一条都没授上」的情形可见化（禁止静默通过）"
    );
    // down 真实回滚：按角色码回收 read 与 print 两类行，不留空实现
    let down = down_execution(&mig);
    assert!(
        down.contains("DELETE FROM \"role_permissions\"")
            && down.contains("rp.\"action\" = 'read'")
            && down.contains("rp.\"action\" = 'print'"),
        "m0069 down 必须按角色码精确回收 read/print 两组授予"
    );
}

/// 通道 ②：注册在场且顺序正确（本域 up 链尾应用 / down 链首回滚）
#[test]
fn migration_is_registered_in_production_domain_chain() {
    let src =
        code_only(&include_str!("../migration/src/domain/production/mod.rs").replace('\r', ""));
    assert!(
        src.contains("mod m0069_grant_piece_read_and_print;"),
        "m0069 未在 production 域声明"
    );
    let up = &src[..src.find("async fn down").expect("production mod 缺 down")];
    let down = &src[src.find("async fn down").expect("production mod 缺 down")..];
    // 链式调用被 rustfmt 拆行是常态（原 needle 把"换行 + 12 空格缩进"写进字面量，
    // 排版一变就假失败）⇒ 正向存在性判定走 canon，只锁"确实调了 up/down"，不锁排版。
    assert!(
        canon(up).contains(&canon(
            "m0069_grant_piece_read_and_print::Migration.up(manager)"
        )),
        "m0069 未接入 production 域 up 链"
    );
    assert!(
        canon(down).contains(&canon(
            "m0069_grant_piece_read_and_print::Migration.down(manager)"
        )),
        "m0069 未接入 production 域 down 链（回滚缺口）"
    );
    // 逆序对称：down 里 m0069 必须先于 m0067（后应用者先回滚）
    let i69 = down.find("m0069_grant_piece_read_and_print").unwrap();
    let i67 = down
        .find("m0067_aftersales_status_add_accepted_evaluated")
        .unwrap();
    assert!(i69 < i67, "down 顺序必须与 up 逆序对称");
}

/// 通道 ③：e2e 补建角色的权限码集合与后端同口径（否则 CI 角色账号仍是 403 假绿）
#[test]
fn e2e_role_seed_permissions_carry_pieces_keys() {
    let src = code_only(&e2e_seed_src());
    assert!(
        src.contains("'pieces:read'"),
        "e2e SEED_ROLE_EXTRA_PERMISSIONS 必须授 pieces:read（发货匹号选择器/标签面板）"
    );
    assert!(
        src.contains("'pieces:print'"),
        "e2e SEED_ROLE_EXTRA_PERMISSIONS 必须授 pieces:print（仓管/质检侧标签打印）"
    );
    assert!(
        !src.contains("'pieces:*'") && !src.contains("'pieces:update'"),
        "e2e 侧同样禁止超范围授 pieces 键"
    );
}
