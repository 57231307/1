//! 采购收货三枚显式权限键（让步接收 / 复检改判 / 收货确认）的派生正确 + 三通道一致静态锁。
//!
//! 功能：以纯静态方式（不连库、不启服务）钉死 concession、rejudge 与 confirm 三枚权限键，从
//!       「URL 末段动作词派生 → 资源段消歧 → 注册表权威名 → 新装矩阵授予 → 存量迁移
//!       授予 → 前端常量与按钮权限门」这条唯一链路上各口径互相同源，任一处漂移即判红。
//!       confirm 还反向锁住"矩阵/迁移不得再出现无端点对应的 approve 行"。
//! 调用方：本文件是集成测试 crate 的可执行用例，由测试运行器（cargo test 的 tests 目标）
//!       在 CI 调用；不参与生产运行期，也不被任何生产模块调用。
//! 入参：向生产函数传入真实请求路径（`/api/v1/erp/purchase/receipts/{收货单ID}/{动作}`）
//!       与真实路径段二元组（`("purchase","receipts")`）；向文本解析器传入以 include_str!
//!       读入的三条通道源文件全文（新装角色矩阵、存量授权迁移、前端权限常量）。
//! 传给谁：把派生结果（末段动作 + 资源段）与解析出的授予集合传给断言，做集合相等与
//!       在场/缺席判定；别名映射（质量经理的在册别名码 ↔ 矩阵码）显式写死并给出依据，
//!       不做含糊合并。
//! 存什么：本文件不写库、不产出持久化数据；它只把"三通道应授予哪些角色 × 哪个动作"
//!       这一契约固化成可回归断言。
//! 存哪里：仅落地为本测试源文件一份，随源码进版本库；断言目标全部指向各自通道的真源
//!       （运行期派生用 `middleware::permission::extract_action_from_path` 与
//!       `utils::path_utils::resolve_module_prefixed_resource` 的真实函数；资源注册表用
//!       `init_service::PERMISSION_RESOURCES` 这一编译期常量；矩阵/迁移/前端以文件文本为
//!       被扫描的外部数据），绝不把事实复刻成测试里的第二套常量。
//!
//! 为什么锁这些：
//! - 权限中间件按 URL 段推导权限键（`extract_resource_info` 取 seg4 交消歧，末段动作由
//!   `extract_action_from_path` 依 PATH_ACTION_KEYWORDS 判定）。授权行按资源段精确匹配，
//!   故派生口径与授予口径必须逐字符同源；派生腿用真实函数调用而非复刻逻辑，是因为一旦
//!   把规则抄进测试就形成第二套事实，抄错也测不出。
//! - concession（让步接收，对不合格品的放行决定）与 rejudge（复检改判，推翻既有质检结论）
//!   是独立于"建单 / 审批"的处置权，属职责分离敏感面：只授采购经理 + 质量岗，绝不授
//!   采购员 / 仓管 / 仓库经理，否则出现"自采自收 / 自判自改"。反向缺席断言即锁这条红线，
//!   哪天有人顺手给这三岗加键会红。
//! - 三通道（新装库角色矩阵 / 存量库迁移 / 前端按钮权限常量）必须同口径，只钉"在场"对
//!   "一边缺一边多"是盲的，故按角色集合双向相等钉死；别名码（质量经理的在册别名）须
//!   显式映射并在迁移侧在场、矩阵侧不双写，防止豁免名单吞掉真实漂移。

use std::collections::BTreeSet;

use bingxi_backend::middleware::permission::extract_action_from_path;
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::resolve_module_prefixed_resource;

/// 本波锁定的资源名（由 ("purchase","receipts") 经 path_utils 消歧得到，注册表内权威名）。
const RESOURCE: &str = "purchase-receipts";
/// 两枚显式权限键的完整字面量（资源段:动作段，与运行时键及前端常量同形）。
const CONCESSION_KEY: &str = "purchase-receipts:concession";
const REJUDGE_KEY: &str = "purchase-receipts:rejudge";

/// 通道① 新装库角色矩阵（真源，以文本读入而非复刻）。
fn matrix_src() -> String {
    include_str!("../src/services/init_service_ops/permission.rs").replace('\r', "")
}

/// 通道② 存量库授权迁移（真源）。
fn migration_src() -> String {
    include_str!(
        "../migration/src/domain/business/m0090_grant_purchase_receipt_concession_rejudge.rs"
    )
    .replace('\r', "")
}

/// 通道③ 前端权限常量（真源）。
fn frontend_src() -> String {
    include_str!("../../frontend/src/constants/permissions.ts").replace('\r', "")
}

/// 只保留"代码 + 字符串字面量"：逐行剔除整行注释（`//`/`///`/`//!`，TS 侧同样 `//`）。
/// 依据与禁令都写在注释里（"不授采购员 / 仓管 / 仓库经理""矩阵用规范码 qc_manager""别名
/// 在册说明"），它们不是授予，计入即假判；故所有集合判据只针对执行体。
/// 按行处理而不做字符级扫描：迁移里跨行 raw SQL 的引号成对但行数多，单行配平会把 SQL
/// 文本误判；宁少剥（行尾尾注释、块注释不动）不可错吃代码。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 区间内全部成对双引号 token（矩阵角色码行 / 迁移常量项 / 前端键值均无未配对引号）。
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

/// 通道① 矩阵各角色分组：分组起始签名 =「"`role_code`", 行 + 紧随的 `&[` 行」；分组体 =
/// 本 `&[` 行之后、到下一分组起始的 `&[` 之前。权限元组只出现在所属分组体内，按体内是否
/// 含目标元组即得"哪个角色被授了什么动作"。此扫描与角色资源在文件里的排布（每角色一组
/// `(role, &[tuples...])`）绑定，若角色资源函数改形态（如引入嵌套 `&[`）本函数须同步改。
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
            let code = lines[i - 1]
                .trim()
                .trim_end_matches(',')
                .trim_matches('"')
                .to_string();
            let mut end = lines.len();
            let mut j = i + 1;
            while j < lines.len() {
                if lines[j].trim() == "&[" && is_role_code_line(lines[j - 1]) {
                    end = j - 1;
                    break;
                }
                j += 1;
            }
            groups.push((code, lines[i..end].join("\n")));
            i = j;
        } else {
            i += 1;
        }
    }
    groups
}

/// 通道①：持有该资源该动作的角色码集合（排序去重，可直接做相等比较）。
fn matrix_roles_with_action(matrix_code: &str, action: &str) -> Vec<String> {
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

/// 通道②：迁移里 `const NAME: &[&str] = &[...]` 的引号 token 清单（剔注释后的执行体解析）。
/// 定位走 `const {name}` 锚点，再取**赋值号后的 `&[`**（跳过类型标注 `&[&str]`），截到首个
/// `]`，避免把类型文本算进清单。取常量清单本身而非测试里手抄口径集，是为了让"只改一条
/// 通道"的漂移无法靠同步改本测试常量掩盖——①②谁真在场以源文件解析为准。
fn migration_role_codes(mig_code: &str, const_name: &str) -> Vec<String> {
    let anchor = mig_code
        .find(&format!("const {const_name}"))
        .unwrap_or_else(|| panic!("迁移里必须存在 const {const_name}（通道集合判据的定位符）"));
    let rest = &mig_code[anchor..];
    let assign = rest
        .find("= &[")
        .unwrap_or_else(|| panic!("const {const_name} 应为 `: &[&str] = &[...]` 赋值形态"));
    let body = &rest[assign + "= &[".len()..];
    let close = body
        .find(']')
        .unwrap_or_else(|| panic!("const {const_name} 的角色清单未闭合"));
    let mut codes = quoted_tokens(&body[..close]);
    codes.sort();
    codes.dedup();
    codes
}

/// 集合工具：把字符串切片排序去重成 BTreeSet 以便稳定相等比较。
fn to_set(v: &[String]) -> BTreeSet<String> {
    v.iter().cloned().collect()
}

/// 依据通道② 的在册别名声明（迁移源里 `// 矩阵用规范码 qc_manager，另授本在册别名码` 紧邻
/// `const CONCESSION_ROLE_CODES` 之上）：质量经理在矩阵用规范码 `qc_manager`，存量迁移
/// 同时列入其部署/e2e 在册别名码 `quality_manager`。做集合比较时须把该别名折回规范码，
/// 否则"在册别名豁免名单"会吞掉真实漂移。别名映射显式写死、不合并含糊。
const MANAGER_ALIAS: &str = "quality_manager";
const MANAGER_CANONICAL: &str = "qc_manager";

/// 派生腿（真调用生产函数，不复刻规则）：两条真实路径末段应派生出对应动作词。
#[test]
fn concession_and_rejudge_derive_from_real_action_extractor() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts/123/concession").as_deref(),
        Some("concession"),
        "让步接收路径末段必须派生为动作 concession（键派生前提：动作词已登记进 PATH_ACTION_KEYWORDS）"
    );
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts/123/rejudge").as_deref(),
        Some("rejudge"),
        "复检改判路径末段必须派生为动作 rejudge"
    );
}

/// 派生腿反例：收货单 ID 段与列表段都不得被误当成动作词。
#[test]
fn numeric_receipt_id_and_collection_segment_are_not_actions() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts/123").as_deref(),
        None,
        "收货单 ID 段 123 不是动作词，必须派生为 None（否则记录 ID 会被当成动作形成伪键）"
    );
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts").as_deref(),
        None,
        "集合段 receipts 不是动作词，必须派生为 None"
    );
}

/// 资源名腿（真调用生产函数）：("purchase","receipts") 经 path_utils 消歧到注册表权威名，
/// 且该权威名确实登记在编译期资源注册表 PERMISSION_RESOURCES 里。用 `pub const` 常量做
/// 成员判定而非再抄一份清单，是把注册表当唯一事实来源、不建第二套事实。
#[test]
fn receipts_resource_disambiguates_to_registered_canonical_name() {
    assert_eq!(
        resolve_module_prefixed_resource("purchase", "receipts"),
        RESOURCE,
        "purchase/receipts 必须消歧到 purchase-receipts，否则矩阵授予的资源名与运行时派生键不同源"
    );
    assert!(
        PERMISSION_RESOURCES.contains(&RESOURCE),
        "purchase-receipts 必须是资源注册表里的权威名，否则消歧结果仍是永不生效的死码授权"
    );
    // 反向自证：裸 receipts 不是经消歧得到的权限资源名（它只在直接资源白名单里作 seg3，
    // 在权限资源注册表内缺席）。若哪天有人把裸 receipts 塞进注册表来绕过消歧，本断言红。
    assert!(
        !PERMISSION_RESOURCES.contains(&"receipts"),
        "禁止把 URL 段原名 receipts 登记进权限资源注册表来绕过消歧（会形成第二套资源名）"
    );
}

/// 通道①↔②：让步接收的角色授予集合，在把在册别名码折回规范码后，必须与矩阵授予集合
/// **双向相等**。另钉别名码只在迁移侧在场、矩阵侧不双写，防豁免名单吞漂移或矩阵双写。
#[test]
fn concession_role_set_matches_matrix_modulo_manager_alias() {
    let matrix = matrix_roles_with_action(&code_only(&matrix_src()), "concession");
    let mig_raw = migration_role_codes(&code_only(&migration_src()), "CONCESSION_ROLE_CODES");

    // 别名折回：把迁移里的在册别名码替换为矩阵规范码（若同时出现两码则 dedup 归一）。
    let mig: BTreeSet<String> = mig_raw
        .iter()
        .map(|c| {
            if c == MANAGER_ALIAS {
                MANAGER_CANONICAL.to_string()
            } else {
                c.clone()
            }
        })
        .collect();

    assert_eq!(
        to_set(&matrix),
        mig,
        "通道① 矩阵与通道② 迁移对 concession 的授予集合须在别名折回后双向相等（改一处必须同步另一处）"
    );
    // 折回前的原集里，别名码确实在场（否则等于凭空豁免）。
    assert!(
        mig_raw.iter().any(|c| c == MANAGER_ALIAS),
        "在册别名码 {MANAGER_ALIAS} 必须在迁移的 concession 清单里在场（存量库的真实受授面）"
    );
    // 矩阵侧只认规范码，不得双写别名码。
    assert!(
        !matrix.iter().any(|c| c == MANAGER_ALIAS),
        "矩阵不得双写别名码 {MANAGER_ALIAS}（质量经理在矩阵用规范码 {MANAGER_CANONICAL}）"
    );
    // 与裁定口径三方一致（防①②同漂躲过两两相等）。
    let adjudicated: BTreeSet<String> = ["purchase_manager", "qc_manager"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        to_set(&matrix),
        adjudicated,
        "concession 矩阵集合须恰为采购经理 + 质量经理两岗"
    );
}

/// 通道①↔②：复检改判的角色授予集合，别名折回后双向相等；同样钉别名在场与不双写。
#[test]
fn rejudge_role_set_matches_matrix_modulo_manager_alias() {
    let matrix = matrix_roles_with_action(&code_only(&matrix_src()), "rejudge");
    let mig_raw = migration_role_codes(&code_only(&migration_src()), "REJUDGE_ROLE_CODES");

    let mig: BTreeSet<String> = mig_raw
        .iter()
        .map(|c| {
            if c == MANAGER_ALIAS {
                MANAGER_CANONICAL.to_string()
            } else {
                c.clone()
            }
        })
        .collect();

    assert_eq!(
        to_set(&matrix),
        mig,
        "通道① 矩阵与通道② 迁移对 rejudge 的授予集合须在别名折回后双向相等"
    );
    assert!(
        mig_raw.iter().any(|c| c == MANAGER_ALIAS),
        "在册别名码 {MANAGER_ALIAS} 必须在迁移的 rejudge 清单里在场"
    );
    assert!(
        !matrix.iter().any(|c| c == MANAGER_ALIAS),
        "矩阵不得双写别名码 {MANAGER_ALIAS}"
    );
    let adjudicated: BTreeSet<String> = ["quality_inspector", "qc_manager"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        to_set(&matrix),
        adjudicated,
        "rejudge 矩阵集合须恰为质检员 + 质量经理两岗"
    );
}

/// 通道③：两枚键的完整字面量与常量名都必须在前端权限常量源里在场（前端按钮 v-permission
/// 引用的即这两个键，键名漂移会让按钮对持权岗位恒不可达 = 假绿盲区）。
#[test]
fn frontend_constants_declare_both_purchase_receipt_keys() {
    let front = code_only(&frontend_src());
    assert!(
        front.contains(CONCESSION_KEY),
        "前端 permissions.ts 必须声明 {CONCESSION_KEY}（让步接收按钮的权限键）"
    );
    assert!(
        front.contains(REJUDGE_KEY),
        "前端 permissions.ts 必须声明 {REJUDGE_KEY}（复检改判按钮的权限键）"
    );
    assert!(
        front.contains("PURCHASE_RECEIPT_CONCESSION") && front.contains("PURCHASE_RECEIPT_REJUDGE"),
        "前端必须以命名常量登记两枚键，禁止在视图里散落裸字面量形成第二套事实"
    );
}

/// 职责分离反向锁：采购员 / 仓管 / 仓库经理两枚键都拿不到。这是"自采自收 / 自判自改"红线
/// 的实质——哪天有人顺手给这三岗加 concession 或 rejudge，本锁红。
#[test]
fn separation_of_duties_roles_hold_neither_key() {
    let matrix_code = code_only(&matrix_src());
    for role in ["purchase_clerk", "warehouse_keeper", "warehouse_manager"] {
        for action in ["concession", "rejudge"] {
            let holders = matrix_roles_with_action(&matrix_code, action);
            assert!(
                !holders.iter().any(|c| c == role),
                "职责分离红线：{role} 不得被授予 {RESOURCE}:{action}"
            );
        }
    }
    // 迁移两清单里同样不得出现这三岗。
    let mig_code = code_only(&migration_src());
    for role in ["purchase_clerk", "warehouse_keeper", "warehouse_manager"] {
        for const_name in ["CONCESSION_ROLE_CODES", "REJUDGE_ROLE_CODES"] {
            let codes = migration_role_codes(&mig_code, const_name);
            assert!(
                !codes.iter().any(|c| c == role),
                "职责分离红线：迁移 {const_name} 不得含 {role}"
            );
        }
    }
}

/// 检测力自证（证明本锁能红，非恒真）：
/// 一、动作词派生器可区分——把末段换成未登记的伪动作词必须得 None；若把 concession/rejudge
///     误登记成 approve 之类，第一条相等断言即红。
/// 二、矩阵解析器可区分——给定"缺 concession 键"的合成输入必须解析出空集，从而让通道①②
///     相等断言在真实授予被删时落到"空集 ≠ 迁移集"而判红。
/// 依据：若把上面任一负例改成恒真（例如断言伪动作也返回 Some，或对缺键输入返回非空），
///     本用例必红，因为它测的就是解析/派生器对"不存在"应当返回空的正确行为。
#[test]
fn lock_will_go_red_when_grant_or_derivation_drifts() {
    // 一、派生器区分：未登记动作词返回 None（若有人把任意末段都当动作，本断言红）。
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts/123/reopen").as_deref(),
        None,
        "reopen 未登记进 PATH_ACTION_KEYWORDS，末段不得被当成动作词"
    );
    // 二、资源名消歧区分：未登记二元组保留原名（若把 ("purchase","receipts") 之外的组合
    //     也错消歧成 purchase-receipts，本断言红）。
    assert_ne!(
        resolve_module_prefixed_resource("purchase", "receipts-ghost"),
        RESOURCE,
        "未登记的资源段不得被消歧成 purchase-receipts（否则会制造注册表外的伪资源名）"
    );
    // 三、矩阵解析器对"缺键"输入返回空集（合成输入不含该元组）。这条一旦变红说明解析器
    //     失去了对授予缺失的判别力，进而通道①② 相等锁形同虚设。
    let synthetic = format!("\"some_role\",\n&[\n(\"{RESOURCE}\", \"read\"),\n],");
    assert!(
        matrix_roles_with_action(&synthetic, "concession").is_empty(),
        "解析器须对不含 concession 的输入返回空集——这是相等锁能检出漂移的前提"
    );
    assert!(
        matrix_roles_with_action(&code_only(&matrix_src()), "concession").len() == 2,
        "真实矩阵里 concession 恰被两岗（采购经理 + 质量经理）持有；解析器对真源应给出非空且计数为二"
    );
}

// ---------------------------------------------------------------------------
// 收货确认键 confirm：真实端点派生 + 三通道同源 + 悬空 approve 缺席
// ---------------------------------------------------------------------------

/// confirm 键完整字面量（与运行期派生键、前端常量同形）。
const CONFIRM_KEY: &str = "purchase-receipts:confirm";

/// 采购经理在部署/e2e 侧的在册别名码 → 矩阵规范码。
const PURCHASE_MANAGER_ALIAS: &str = "purchasing_manager";
const PURCHASE_MANAGER_CANONICAL: &str = "purchase_manager";
/// 采购员在部署/e2e 侧的在册别名码 → 矩阵规范码。
const PURCHASER_ALIAS: &str = "purchaser";
const PURCHASER_CANONICAL: &str = "purchase_clerk";

/// 通道② confirm 的存量库授权迁移（真源）。
fn confirm_migration_src() -> String {
    include_str!("../migration/src/domain/business/m0091_realign_purchase_receipt_confirm.rs")
        .replace('\r', "")
}

/// 把迁移清单里的在册别名码折回矩阵规范码（两码同现时由 dedup 归一）。
fn fold_purchase_aliases(raw: &[String]) -> BTreeSet<String> {
    raw.iter()
        .map(|c| match c.as_str() {
            PURCHASE_MANAGER_ALIAS => PURCHASE_MANAGER_CANONICAL.to_string(),
            PURCHASER_ALIAS => PURCHASER_CANONICAL.to_string(),
            _ => c.clone(),
        })
        .collect()
}

/// 派生腿（真调用生产函数）：确认收货路径末段须派生成 confirm，而不是 approve。
#[test]
fn confirm_derives_from_real_action_extractor() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/receipts/123/confirm").as_deref(),
        Some("confirm"),
        "/receipts/{{id}}/confirm 必须派生成 confirm 动作，授权才落在运行期真正查的那枚键上"
    );
}

/// 通道①↔②↔③：confirm 的受授集合在别名折回后双向相等，且恰为采购经理 + 采购员两岗。
#[test]
fn confirm_role_set_matches_matrix_modulo_purchaser_aliases() {
    let matrix_code = code_only(&matrix_src());
    let matrix = matrix_roles_with_action(&matrix_code, "confirm");
    let mig_raw = migration_role_codes(&code_only(&confirm_migration_src()), "CONFIRM_ROLE_CODES");
    let mig = fold_purchase_aliases(&mig_raw);

    assert_eq!(
        to_set(&matrix),
        mig,
        "通道① 矩阵与通道② 迁移对 confirm 的授予集合须在别名折回后双向相等（改一处必须同步另一处）"
    );
    // 折回前的原集里两枚在册别名确实在场（否则等于凭空豁免真实存量面）。
    for alias in [PURCHASE_MANAGER_ALIAS, PURCHASER_ALIAS] {
        assert!(
            mig_raw.iter().any(|c| c == alias),
            "在册别名码 {alias} 必须在迁移的 confirm 清单里在场（存量库的真实受授面）"
        );
    }
    // 矩阵侧只认规范码，不得双写别名码。
    for alias in [PURCHASE_MANAGER_ALIAS, PURCHASER_ALIAS] {
        assert!(
            !matrix.iter().any(|c| c == alias),
            "矩阵不得双写别名码 {alias}（采购两岗在矩阵用规范码）"
        );
    }
    let adjudicated: BTreeSet<String> = [PURCHASE_MANAGER_CANONICAL, PURCHASER_CANONICAL]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(
        to_set(&matrix),
        adjudicated,
        "confirm 矩阵集合须恰为采购经理 + 采购员两岗"
    );

    let front = code_only(&frontend_src());
    assert!(
        front.contains(CONFIRM_KEY) && front.contains("PURCHASE_RECEIPT_CONFIRM"),
        "前端 permissions.ts 必须以命名常量声明 {CONFIRM_KEY}，否则确认按钮对持权岗位恒不可达"
    );
    // 视图侧确有用这枚键门控入口（只在常量文件里声明而不接按钮＝假就绪）。
    let table_view = include_str!(
        "../../frontend/src/views/purchase-receipt/components/PurchaseReceiptTable.vue"
    )
    .replace('\r', "");
    assert!(
        table_view.contains(CONFIRM_KEY),
        "收货列表的确认入口必须按 {CONFIRM_KEY} 做权限门控，不得让无键岗位看见必吃 403 的按钮"
    );
}

/// 悬空授权反向锁：purchase-receipts 没有 approve 端点，矩阵与迁移都不得再出现 approve 行。
/// 端点缺席这一事实由 approve/reject 成对锁的缺席断言守着，本锁守的是授权侧不再回潮。
#[test]
fn ghost_approve_grant_is_retired_on_both_channels() {
    let matrix = matrix_roles_with_action(&code_only(&matrix_src()), "approve");
    assert!(
        matrix.is_empty(),
        "purchase-receipts 无 approve 端点，矩阵不得保留任何 approve 行（悬空授权回潮会掩盖真实键 confirm 的缺失）"
    );
    // 迁移里 approve 只允许出现在 down 的"恢复原态"语句里，不得出现在任何授予常量中。
    let mig_code = code_only(&confirm_migration_src());
    let granted = migration_role_codes(&mig_code, "CONFIRM_ROLE_CODES");
    assert!(
        !granted.iter().any(|c| c.contains("approve")),
        "confirm 授予清单里不得混入 approve 项"
    );
    assert!(
        mig_code.contains("const LEGACY_APPROVE_ROLE_CODES"),
        "回滚需要按原在册角色码恢复被删的 approve 行，该常量缺失则 down 不可逆"
    );
    // 检测力自证：合成输入含 approve 行时解析器必须给出非空——证明上面的空集断言有判别力。
    let synthetic = format!("\"some_role\",\n&[\n(\"{RESOURCE}\", \"approve\"),\n],");
    assert!(
        !matrix_roles_with_action(&synthetic, "approve").is_empty(),
        "解析器须能检出 approve 行的存在；否则上面的空集断言是恒真假绿"
    );
}
