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
//! 锁的强度（本锁存在的意义就是检出通道间漂移，纯"在场"断言钉不住"一边缺一边多"）：
//! - ①↔②：对 `pieces:print` **双向集合相等**；对 `pieces:read` 在剔除
//!   m0069 文件头登记在册的唯一部署别名码（`salesperson`，CI/部署库建它而不建
//!   sales_rep）后双向集合相等，并单独钉别名码确实在 ② 在场——防别名豁免名单
//!   吞掉真实漂移。
//! - ③：只断言 e2e SEED 的 pieces 角色集是 ①∩② 的**子集**，③ 是 CI 补建面的
//!   子集而非全等通道（fabric_inspector 覆盖缺口见
//!   `e2e_seed_pieces_roles_are_subset_of_backend_channels` 文档注释，如实挂账）。
//!
//! 质量红线口径：本锁不许被"放宽集合"用来变绿——集合扩/缩都必须同时改三处并在此改判，
//! 且 `("pieces", "*")` / update / delete / create 一律禁止（最小授权：库里只有 read、
//! print 两个 pieces 端点，见 `routes/inventory.rs::piece_routes`）。
//!
//! print 首授集合 = 5 岗（计数改判记录）：`inventory_manager / warehouse_keeper /
//! warehouse_manager / quality_inspector / fabric_inspector`。
//! 口径出处：通道② 建档提交 69972b26（"fix(rbac): 注册并授予 pieces:read/pieces:print"）
//! 的 PRINT_ROLE_CODES 即含 warehouse_manager（5 角），同一提交在通道① 矩阵只落了
//! 4 角——建档即不对称，此后矩阵计数钉 4 只是把①的缺漏固化成"预期"。本批由用户裁定
//! 确认 5 岗口径（依据：本岗产出人 + 行业惯例——打卷/验布/仓储打签；salesperson /
//! sales_rep / production_manager 只给 read 不给 print，销售可读列表/选匹发货，实物
//! 标签不由他打，重打由仓管代做、工作流不阻塞），据此补齐通道① 的 warehouse_manager
//! 分组，矩阵计数断言由 read 6 / print 4 改为 read 7 / print 5。

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
/// 与**计数**（read 恰 7 处、print 恰 5 处，口径改判见文件头）两类判据，二者都必须只针对执行体：
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

/// m0069 里 `const NAME: &[&str] = &[...]` 的引号 token 清单（剔注释后的执行体解析）。
/// 取**常量清单本身**而非测试文件里手抄的口径集，是为了让"只改一条通道"的漂移
/// 无法靠同步改本测试常量来掩盖——①② 谁真的在场，以源文件解析结果为准。
fn migration_role_codes(mig_code: &str, const_name: &str) -> Vec<String> {
    let anchor = mig_code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("m0069 必须存在 const {const_name}（通道② 集合判据的定位符）"));
    let rest = &mig_code[anchor..];
    let open = rest.find("&[").expect("m0069 常量应为 &[&str] 形态");
    let close = rest[open..].find("];").expect("m0069 常量清单未闭合");
    let mut codes = quoted_tokens(&rest[open..open + close]);
    codes.sort();
    codes.dedup();
    codes
}

/// 区间内全部成对双引号 token（矩阵角色码行 / 迁移常量项均无未配对引号）。
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

/// 通道① 矩阵各角色分组：分组起始签名 =「`"role_code",` 行 + 紧随的 `&[` 行」，
/// 分组体 = 本角色码行之后到下一分组起始之前。权限元组只出现在所属分组体内，
/// 故按体内是否含目标 `("pieces", action)` 归户即得"哪个角色被授了什么"。
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

/// 通道① 矩阵中持有该 pieces 动作的角色码集合（排序去重，可直接做相等比较）。
fn matrix_roles_with(matrix_code: &str, action: &str) -> Vec<String> {
    let needle = format!(r#"("pieces", "{action}")"#);
    let mut codes: Vec<String> = matrix_role_groups(matrix_code)
        .into_iter()
        .filter(|(_, body)| body.contains(&needle))
        .map(|(code, _)| code)
        .collect();
    codes.sort();
    codes.dedup();
    codes
}

/// m0069 read 面登记在册的**唯一**部署/e2e 别名码。
/// 依据 m0069 文件头「角色码同时覆盖后端矩阵码与 e2e/部署侧别名码（salesperson/
/// warehouse_manager）」：CI/部署库由 SEED_ROLES 建 `salesperson` 而非矩阵码
/// sales_rep，故 ② 的 read 面比 ① 多这一个码属在册口径。文件头同句把
/// warehouse_manager 也称为"别名码"，但它实为 init 播种的真角色（role.rs:179-184，
/// is_system=true），本批已按裁定进矩阵，不再享受别名豁免。
/// 别名码是否继续承载 read、`warehouse`/`qc_manager` 等其余别名是否给重打权，
/// 均属待用户裁定项——本测只钉现状（② 在场且 ① 不双写），不预先放宽也不预先收紧。
const REGISTERED_READ_ALIAS_CODES: &[&str] = &["salesperson"];

/// 通道③：解析 e2e SEED_ROLE_EXTRA_PERMISSIONS 中承载指定权限码的角色码集合。
/// 分组形态 = `裸标识符: [ ... ]`（单行或跨行数组体）。
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

fn adjudicated_sorted<'a>(list: &'a [&'a str]) -> Vec<&'a str> {
    let mut v = list.to_vec();
    v.sort_unstable();
    v
}

/// 通道 ①：矩阵里 pieces 的授予集合必须恰为口径集（数量+最小授权禁项）
#[test]
fn role_matrix_grants_exactly_read_and_print_for_pieces() {
    let src = code_only(&matrix_src());
    // 计数改判（read 6→7、print 4→5）：warehouse_manager 分组系本批按用户裁定
    // 补建（原通道① 整组缺失，新装库仓库经理打签 403；口径自 69972b26 的 m0069
    // 建档即 5 角 print），文件头「print 首授集合 = 5 岗（计数改判记录）」载明依据。
    // 计数只是双保险，通道间**集合相等**判据见
    // matrix_and_migration_print_role_sets_are_equal。
    assert_eq!(
        src.matches(r#"("pieces", "read")"#).count(),
        7,
        "pieces:read 在角色矩阵中应恰为 7 处（库存经理/仓管/仓库经理/销售代表/生产经理/质检员/验布员）"
    );
    assert_eq!(
        src.matches(r#"("pieces", "print")"#).count(),
        5,
        "pieces:print 在角色矩阵中应恰为 5 处（库存经理/仓管/仓库经理/质检员/验布员）"
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

/// 通道 ①↔②：pieces:print 的角色集合**双向相等**——本判据才是检出漂移的那条锁。
/// 上一版只钉"①计数恰 4 + ②逐码在场"，通道① 整组缺 warehouse_manager 依然全绿
/// （三通道互不回流的事故族形态），"在场"钉法对"一边缺一边多"是盲的。
/// 漂移必红示例：删 permission.rs warehouse_manager 分组里的 ("pieces","print")
/// ⇒ ① 少一元素，本断言红（计数 print 4≠5 也红）；只给 m0069 的 PRINT_ROLE_CODES
/// 新增一个角色 ⇒ ② 多一元素，本断言同样红。
#[test]
fn matrix_and_migration_print_role_sets_are_equal() {
    let matrix = matrix_roles_with(&code_only(&matrix_src()), "print");
    let mig = migration_role_codes(&code_only(&migration_src()), "PRINT_ROLE_CODES");
    assert_eq!(
        sorted_refs(&matrix),
        sorted_refs(&mig),
        "通道① 矩阵与通道② m0069 对 pieces:print 的授予集合必须双向相等（同源契约：改一处必须同步改另一处并在此改判）"
    );
    // 与裁定口径集（本文件头）三方一致：防 ①② 同漂时双双躲过两两相等
    assert_eq!(
        sorted_refs(&matrix),
        adjudicated_sorted(PRINT_ROLES),
        "pieces:print 集合须等于裁定口径 5 岗（inventory_manager/warehouse_keeper/warehouse_manager/quality_inspector/fabric_inspector）"
    );
    // 通道内自洽：print ⊆ read（先能看匹，才谈得上打印）
    let matrix_read = matrix_roles_with(&code_only(&matrix_src()), "read");
    for code in &matrix {
        assert!(
            matrix_read.contains(code),
            "矩阵 pieces:print 角色 {code} 必须同时持有 pieces:read"
        );
    }
    let mig_read = migration_role_codes(&code_only(&migration_src()), "READ_ROLE_CODES");
    for code in &mig {
        assert!(
            mig_read.contains(code),
            "m0069 pieces:print 角色 {code} 必须同时持有 pieces:read"
        );
    }
}

/// 通道 ①↔②：pieces:read 在**剔除在册别名码后**双向集合相等。
/// read 面 ② 比 ① 多 `salesperson`（CI/部署库建它不建 sales_rep，见
/// REGISTERED_READ_ALIAS_CODES 注释）——这是在册口径差，不是漂移；除此之外
/// 任何一边多/少一个角色码都必红。别名码单独钉"② 在场且 ① 不双写"，
/// 防豁免名单吞掉真实漂移或矩阵双写两套销售码。
#[test]
fn matrix_and_migration_read_role_sets_are_equal_modulo_registered_aliases() {
    let matrix_read = matrix_roles_with(&code_only(&matrix_src()), "read");
    let mig_read = migration_role_codes(&code_only(&migration_src()), "READ_ROLE_CODES");
    let mig_without_alias: Vec<&str> = mig_read
        .iter()
        .map(String::as_str)
        .filter(|c| !REGISTERED_READ_ALIAS_CODES.contains(c))
        .collect();
    assert_eq!(
        sorted_refs(&matrix_read),
        mig_without_alias,
        "通道① 与通道② 对 pieces:read 的集合须在剔除在册别名码 {:?} 后双向相等",
        REGISTERED_READ_ALIAS_CODES
    );
    // 与裁定口径集三方一致（矩阵面 = READ_ROLES 全集剔别名；销售面矩阵用规范码 sales_rep）
    let adjudicated_read: Vec<&str> = adjudicated_sorted(READ_ROLES)
        .iter()
        .copied()
        .filter(|c| !REGISTERED_READ_ALIAS_CODES.contains(c))
        .collect();
    assert_eq!(
        sorted_refs(&matrix_read),
        adjudicated_read,
        "pieces:read 矩阵集合须等于裁定口径集（salesperson 为 ② 侧在册别名，不在矩阵）"
    );
    for alias in REGISTERED_READ_ALIAS_CODES {
        assert!(
            mig_read.iter().any(|c| c == alias),
            "在册别名码 {alias} 必须在 m0069 read 面在场（CI/部署库的真实受授面）"
        );
        assert!(
            !matrix_read.iter().any(|c| c == alias),
            "矩阵不得双写别名码 {alias}（销售面矩阵只认规范码 sales_rep）"
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

/// 通道 ③：e2e SEED 的 pieces 授予面必须是 ①∩②（read 面再并上在册别名码）的**子集**。
///
/// 现状如实挂账（不许为凑三通道相等写一条现在必然红的断言，也不许为让断言成立删人）：
/// `fabric_inspector` 在通道①② 两条后端通道均持有 pieces:read/pieces:print，但
/// e2e 的 `SEED_ROLES`（global-setup.ts:391-429）**根本不建该角色码**，
/// ensureRoleUsers 无从补建 ⇒ 验布打卷岗在 CI 无任何运行期覆盖，属已知待决项
/// （是否将 fabric_inspector 纳入 SEED_ROLES 由用户裁定——新增角色会改变角色
/// 矩阵分片集合与既有基线，不在本批处理；见任务看板）。因此本测只钉子集关系，
/// 不钉 ③=①∩② 全等（全等当下必红），也不因该缺口放宽 ①↔② 的全等锁。
///
/// 反向保护：③ 若授予 ①② 都不认的角色码（如手滑给 sales_rep 的 CI 别名面写错），
/// 子集断言红；解析签名为空亦红（空集会让子集恒真——假绿通道）。
#[test]
fn e2e_seed_pieces_roles_are_subset_of_backend_channels() {
    let matrix_code = code_only(&matrix_src());
    let mig_code = code_only(&migration_src());
    for action in ["read", "print"] {
        let const_name = if action == "read" {
            "READ_ROLE_CODES"
        } else {
            "PRINT_ROLE_CODES"
        };
        let matrix_roles = matrix_roles_with(&matrix_code, action);
        let mig_roles = migration_role_codes(&mig_code, const_name);
        // ①∩②：两条后端通道都认的规范受授面（①=② 由上方全等锁保证，交集为防御性写法）
        let mut allowed: Vec<String> = mig_roles
            .iter()
            .filter(|c| matrix_roles.contains(c))
            .cloned()
            .collect();
        if action == "read" {
            // read 面 ② 的在册别名码允许出现在 ③（CI 库建 salesperson 不建 sales_rep）
            for alias in REGISTERED_READ_ALIAS_CODES {
                if mig_roles.iter().any(|c| c == alias) {
                    allowed.push(alias.to_string());
                }
            }
        }
        let seeded = e2e_seed_roles_with(&format!("'pieces:{action}'"));
        assert!(
            !seeded.is_empty(),
            "e2e SEED 未解析出任何 'pieces:{action}' 授予角色——要么授予被删（CI 角色将回到 403 假绿），\
             要么 SEED_ROLE_EXTRA_PERMISSIONS 分组签名变了导致解析空转；空集会令子集断言恒真，拒绝放行"
        );
        for role in &seeded {
            assert!(
                allowed.contains(role),
                "通道③ e2e SEED 给 {role} 授 'pieces:{action}'，但该码不在通道①② 的规范受授面\
                 （∪在册别名）内：①={matrix_roles:?} ②={mig_roles:?}"
            );
        }
    }
    // ③ 的最小授权禁项与后端两通道一致
    let seed_src = code_only(&e2e_seed_src());
    assert!(
        !seed_src.contains("'pieces:*'") && !seed_src.contains("'pieces:update'"),
        "e2e 侧同样禁止超范围授 pieces 键"
    );
}
