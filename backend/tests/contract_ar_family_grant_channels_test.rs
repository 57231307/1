//! 应收族（AR）八枚细键的三通道一致性与运行期命中契约锁（不连库，纯 #[test]）。
//!
//! ## 背景
//! 应收族的运行时资源键由 URL 段派生（`middleware/permission.rs::extract_resource_info`），
//! 动作段按中间件 79 到 81 行的优先级取（查询参数 action 或 URL 末段动作关键字或方法派生），
//! 命中判定是 `matches_permission`（709 行）的资源段等值加通配符的精确匹配。粗域展开只
//! 存在于前端路由守卫，后端从不认 finance 这类粗键，所以「前端入口可见」与「后端放行」是
//! 两件事，必须分别钉。
//! 本批补授前通道① 与通道② 都只有 ar-reconciliations 一族（grep 实测两枚命中，八枚细键
//! 零命中），除超管 *:* blanket 外任何角色调应收族端点都必 403。
//!
//! ## 锁的六件事
//! 1. 派生链自证：逐枚真实应收端点走 `extract_resource_info` 与动作派生算出实际键集，
//!    与硬编期望逐字比对；八枚每枚都至少被一条端点派生到（缺一枚即名单里有暗键）。
//! 2. 正向命中：把通道① 生产矩阵摊平成 role_permission::Model 行集，对每一枚
//!    「应有角色 × 该角色应得的键」真实调用 matches_permission，必须命中。
//!    断的是 matcher 的返回值，不是矩阵字段存在性；按 ID 的端点还要以 Some(id) 再判一次，
//!    钉住「角色级授权覆盖该资源全部实例」这条语义真的成立。
//! 3. 反向 403：对每一枚「不该有的角色 × 应收族键」遍历该角色全部 allowed=true 行真调
//!    matcher，都必须判假（判假即中间件 696 行 any 为假、走 403 分支）。
//!    含制单与审核两岗互不越界、报表四枚不外泄给制单岗、本轮刻意不授的三枚两岗都判假、
//!    以及出纳与成本会计与销售两岗与仓管对整族九种动作全判假。
//! 4. 通道① 与通道② 逐字符同集合：迁移 GRANTS 三元组与矩阵 ar-* 行差集两侧皆空，
//!    并与用例内期望面三方对齐（防两侧一起漂移而无人发现）。
//! 5. 通道③ 零枚：e2e SEED_ROLE_EXTRA_PERMISSIONS 不得登记任何 ar-* 键
//!    （登记即与通道① 重复落行并撞唯一索引 uq_role_permissions_role_resource_action）。
//! 6. 检测力自证：从通道② 文本里摘一枚键、换一枚键的动作、换一枚键的角色码，三种注入
//!    都必须被差集点名；并给反向断言配一枚「补一行即判真」的对照夹具，证判假有分辨力。
//!
//! ## 入参与传给谁
//! 入参：只读本仓源码文本与生产定义表 `InitService::all_role_permission_definition_groups`
//! （与写库同一份数据，不抄矩阵副本）。传给谁：纯断言，不发 HTTP、不连库、不落盘。

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use axum::http::Method;
use bingxi_backend::middleware::permission::{
    extract_action_from_path, extract_resource_info, matches_permission, method_to_action,
};
use bingxi_backend::models::role_permission;
use bingxi_backend::services::init_service::{InitService, PERMISSION_RESOURCES};
use chrono::Utc;

const GRANT_MIGRATION: &str = "migration/src/domain/ar_family_grant/m0125_grant_ar_family_keys.rs";
const REGISTRY: &str = "src/services/init_service.rs";
const E2E_SETUP: &str = "../frontend/e2e/global-setup.ts";

/// 本域八枚细键（应收族在 ar 前缀下消歧出的权威资源名）
const AR_RESOURCES: &[&str] = &[
    "ar-invoices",
    "ar-payments",
    "ar-verifications",
    "ar-collections",
    "ar-aging",
    "ar-daily",
    "ar-monthly",
    "ar-statistics",
];

/// 应收族在册端点与其方法（取自 routes/finance.rs 的 ar_invoice_routes 等四组，逐条在册）。
/// 期望键不在本表硬编：由派生链算出后与授予面闭环比对，本表只负责「有哪些端点」。
/// 带 {id} 的端点用字面量 12 代表记录级请求，用于验角色级授权覆盖实例这条语义。
const AR_ENDPOINTS: &[(&str, &str)] = &[
    ("GET", "/api/v1/erp/ar/invoices"),
    ("POST", "/api/v1/erp/ar/invoices"),
    ("GET", "/api/v1/erp/ar/invoices/export"),
    ("GET", "/api/v1/erp/ar/invoices/12"),
    ("PUT", "/api/v1/erp/ar/invoices/12"),
    ("DELETE", "/api/v1/erp/ar/invoices/12"),
    ("POST", "/api/v1/erp/ar/invoices/12/approve"),
    ("POST", "/api/v1/erp/ar/invoices/12/cancel"),
    ("POST", "/api/v1/erp/ar/invoices/12/mark-as-paid"),
    ("GET", "/api/v1/erp/ar/payments"),
    ("POST", "/api/v1/erp/ar/payments"),
    ("GET", "/api/v1/erp/ar/payments/12"),
    ("PUT", "/api/v1/erp/ar/payments/12"),
    ("POST", "/api/v1/erp/ar/payments/12/confirm"),
    ("POST", "/api/v1/erp/ar/payments/12/cancel"),
    ("GET", "/api/v1/erp/ar/collections/12/print"),
    ("GET", "/api/v1/erp/ar/verifications"),
    ("GET", "/api/v1/erp/ar/verifications/12"),
    ("POST", "/api/v1/erp/ar/verifications/auto"),
    ("POST", "/api/v1/erp/ar/verifications/manual"),
    ("POST", "/api/v1/erp/ar/verifications/12/cancel"),
    ("GET", "/api/v1/erp/ar/verifications/unverified/invoices"),
    ("GET", "/api/v1/erp/ar/verifications/unverified/payments"),
    ("GET", "/api/v1/erp/ar/reports/statistics"),
    ("GET", "/api/v1/erp/ar/reports/daily"),
    ("GET", "/api/v1/erp/ar/reports/monthly"),
    ("GET", "/api/v1/erp/ar/reports/aging"),
    ("GET", "/api/v1/erp/ar/reports/aging/by-salesperson"),
];

/// 本域实际可派生出的 (资源, 动作) 全集（用例 1 由端点算出后逐字比对，两侧都不许多不许少）。
const DERIVABLE_KEYS: &[(&str, &str)] = &[
    ("ar-invoices", "read"),
    ("ar-invoices", "create"),
    ("ar-invoices", "update"),
    ("ar-invoices", "delete"),
    ("ar-invoices", "approve"),
    ("ar-invoices", "cancel"),
    ("ar-invoices", "export"),
    ("ar-payments", "read"),
    ("ar-payments", "create"),
    ("ar-payments", "update"),
    ("ar-payments", "confirm"),
    ("ar-payments", "cancel"),
    ("ar-collections", "print"),
    ("ar-verifications", "read"),
    ("ar-verifications", "create"),
    ("ar-verifications", "cancel"),
    ("ar-aging", "read"),
    ("ar-daily", "read"),
    ("ar-monthly", "read"),
    ("ar-statistics", "read"),
];

/// 既定授予面：角色码 → 该岗应得的 (资源, 动作) 集合。
/// 通道① 矩阵分组、通道② 迁移 GRANTS 与本期望面三方对照，任一侧漂移即被差集点名。
fn expected_grants() -> BTreeMap<&'static str, BTreeSet<(&'static str, &'static str)>> {
    let mut map: BTreeMap<&str, BTreeSet<(&str, &str)>> = BTreeMap::new();
    map.insert(
        "finance_manager",
        [
            ("ar-invoices", "read"),
            ("ar-invoices", "approve"),
            ("ar-invoices", "cancel"),
            ("ar-payments", "read"),
            ("ar-payments", "confirm"),
            ("ar-verifications", "read"),
            ("ar-collections", "print"),
            ("ar-aging", "read"),
            ("ar-daily", "read"),
            ("ar-monthly", "read"),
            ("ar-statistics", "read"),
        ]
        .into_iter()
        .collect(),
    );
    map.insert(
        "accountant",
        [
            ("ar-invoices", "read"),
            ("ar-invoices", "create"),
            ("ar-invoices", "update"),
            ("ar-payments", "read"),
            ("ar-payments", "create"),
            ("ar-payments", "update"),
            ("ar-verifications", "read"),
            ("ar-verifications", "create"),
            ("ar-verifications", "cancel"),
            ("ar-collections", "print"),
        ]
        .into_iter()
        .collect(),
    );
    map
}

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

/// 取一行元组字面量里的各字段，行首不是 (" 的返回 None。
/// 整行配对取字面量，先剥前缀会让资源名的右引号被当成开引号、其后配对整体错位。
fn tuple_fields(line: &str) -> Option<Vec<String>> {
    let t = line.trim();
    if !t.starts_with("(\"") {
        return None;
    }
    let fields = quoted(t);
    if fields.is_empty() {
        None
    } else {
        Some(fields)
    }
}

/// 摊平生产矩阵为 (角色码, 资源码, 动作码) 三元组（与写库同一份定义表，不抄副本）。
fn matrix_rows() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut rows = Vec::new();
    for definitions in InitService::all_role_permission_definition_groups() {
        for &(role_code, resources) in definitions {
            for &(resource, action) in resources {
                rows.push((role_code, resource, action));
            }
        }
    }
    rows
}

/// 某角色在矩阵里的全部行，构造成 matcher 的入参形态
/// （中间件 680 行按 allowed=true 过滤，本函数同样只出 allowed=true 行）。
fn permission_models(role: &str) -> Vec<role_permission::Model> {
    let now = Utc::now();
    matrix_rows()
        .into_iter()
        .filter(|(r, _, _)| *r == role)
        .map(|(_, res, act)| role_permission::Model {
            id: 1,
            role_id: 1,
            resource_type: res.to_string(),
            resource_id: None,
            action: act.to_string(),
            allowed: true,
            permission_code: None,
            created_at: now,
            updated_at: now,
        })
        .collect()
}

/// 按中间件 696 行的同一条路判可达：逐行真调 matches_permission，any 为假即 403。
/// resource_id 传入的是「请求侧的记录 id」，None 为整集合请求，Some 为按 ID 请求。
fn role_allowed(role: &str, resource: &str, action: &str, resource_id: Option<i32>) -> bool {
    permission_models(role)
        .iter()
        .any(|p| matches_permission(p, resource, resource_id, action))
}

/// 从真实端点算出该请求的键三元组（资源名、动作、记录 id），与中间件同源三函数。
fn derive_key(method: &str, path: &str) -> (String, String, Option<i32>) {
    let m = Method::from_bytes(method.as_bytes()).expect("端点方法字面量必须合法");
    let (resource, resource_id) = extract_resource_info(path);
    let action = extract_action_from_path(path).unwrap_or_else(|| method_to_action(&m));
    (resource, action, resource_id)
}

/// 通道② 迁移文本里的 GRANTS 三元组集合（只取常量数组区段，避免把注释与文档计入）。
fn migration_grant_triples(migration_src: &str) -> BTreeSet<(String, String, String)> {
    let start = migration_src
        .find("const GRANTS")
        .expect("通道② 迁移必须定义 GRANTS 常量，缺失即授予面不可核对");
    let body = &migration_src[start..];
    let end = body
        .find("];")
        .expect("GRANTS 常量必须以右中括号加分号收尾");
    body[..end]
        .lines()
        .filter_map(tuple_fields)
        .map(|f| {
            assert_eq!(
                f.len(),
                3,
                "通道② GRANTS 每行必须是 (角色码, 资源名, 动作) 三元组，实得 {f:?}"
            );
            (f[0].clone(), f[1].clone(), f[2].clone())
        })
        .collect()
}

/// 通道① 矩阵里的 ar-* 三元组集合（只取本域八枚，对账族两枚另行核对）。
fn matrix_ar_triples() -> BTreeSet<(String, String, String)> {
    matrix_rows()
        .into_iter()
        .filter(|(_, res, _)| AR_RESOURCES.contains(res))
        .map(|(role, res, act)| (role.to_string(), res.to_string(), act.to_string()))
        .collect()
}

/// 期望面三元组（与通道①、通道② 双向比对的第三方锚）。
fn expected_triples() -> BTreeSet<(String, String, String)> {
    let mut expect: BTreeSet<(String, String, String)> = BTreeSet::new();
    for (role, keys) in expected_grants() {
        for (resource, action) in keys {
            expect.insert((role.to_string(), resource.to_string(), action.to_string()));
        }
    }
    expect
}

/// 用例前提自证：八枚细键必须在权威注册表里，否则本锁是在拿注册表外的死键名对拍。
#[test]
fn ar_resources_are_registered_in_authority_table() {
    let registry_src = read_repo(REGISTRY);
    for resource in AR_RESOURCES {
        assert!(
            PERMISSION_RESOURCES.contains(resource),
            "用例前提：{resource} 必须登记在 PERMISSION_RESOURCES，否则授权行是永不命中的死码"
        );
        assert!(
            registry_src.contains(&format!("\"{resource}\"")),
            "用例前提：{resource} 必须逐字符出现在 init_service.rs 的注册表文本里"
        );
    }
    // 防本锁名单与注册表漂移：注册表里 ar- 前缀的键必须恰好是本域八枚加对账族两枚
    //（ar-reconciliations 与 ar-reconciliations-enhanced 挂根级第四段、不经 ar 前缀消歧）。
    let registered_ar: BTreeSet<&str> = PERMISSION_RESOURCES
        .iter()
        .filter(|name| name.starts_with("ar-"))
        .copied()
        .collect();
    let mut expect: BTreeSet<&str> = AR_RESOURCES.iter().copied().collect();
    expect.insert("ar-reconciliations");
    expect.insert("ar-reconciliations-enhanced");
    assert_eq!(
        registered_ar, expect,
        "PERMISSION_RESOURCES 里 ar- 前缀键集合与本锁名单加对账族不符，新增暗键须同步复核授予面"
    );
}

/// 1 派生链自证：逐枚真实端点算出键，必须全落在本域八枚内，且算出的 (资源, 动作) 集
/// 与 DERIVABLE_KEYS 逐字相等；八枚每枚都至少被一条端点派生到。
#[test]
fn every_ar_endpoint_derives_a_key_inside_the_family() {
    let mut derived: BTreeSet<(String, String)> = BTreeSet::new();
    for (method, path) in AR_ENDPOINTS {
        let (resource, action, _) = derive_key(method, path);
        assert!(
            AR_RESOURCES.contains(&resource.as_str()),
            "端点 {method} {path} 派生出资源名 {resource}，不在应收族八枚之内（派生链或本锁名单漂移）"
        );
        derived.insert((resource, action));
    }
    let expect: BTreeSet<(String, String)> = DERIVABLE_KEYS
        .iter()
        .map(|(res, act)| (res.to_string(), act.to_string()))
        .collect();
    assert_eq!(
        derived, expect,
        "应收族实际可派生的 (资源, 动作) 集与本锁钉住的形态不一致；多出项说明有新端点未登记，\
         缺少项说明派生链被改动，两侧都要人工复核后才能动授予面"
    );
    // 授予面不得含派生不出来的键（授了也永不命中，属悬空授权）
    let derivable: BTreeSet<(&str, &str)> = DERIVABLE_KEYS.iter().copied().collect();
    for (role, keys) in &expected_grants() {
        for key in keys {
            assert!(
                derivable.contains(key),
                "角色 {role} 的授予面含派生不出的键 {key:?}，是运行时永不命中的悬空授权"
            );
        }
    }
}

/// 2 正向命中：应有角色对每枚应得键必须被 matcher 真放行；断的是返回值不是字段存在性。
/// 键与端点闭环：对每一枚在册端点，两岗的放行结论必须与既定授予面逐项相等，
/// 按 ID 的端点还要以 Some(12) 再判一次，钉住角色级授权覆盖该资源全部实例。
#[test]
fn expected_roles_are_matched_for_every_ar_key() {
    let grants = expected_grants();
    for (role, keys) in &grants {
        assert!(
            !keys.is_empty(),
            "用例前提失效：角色 {role} 的期望授予面为空，正向断言成了空转"
        );
        for (resource, action) in keys {
            assert!(
                role_allowed(role, resource, action, None),
                "角色 {role} 对键 {resource} 的动作 {action} 必须被 matches_permission 放行，\
                 判假即该岗打开该面必 403（通道① 缺行或通道② 缺补授）"
            );
        }
    }

    // 键与端点闭环：对每一枚在册端点，两岗的放行结论必须与既定授予面逐项相等。
    // 按 ID 的端点派生出 Some(12)，还要以该形态再判一次，钉住「角色级授权覆盖该资源
    // 全部实例」这条语义真的成立（否则非超管改任何一条应收单据都会被误判越权）。
    for (method, path) in AR_ENDPOINTS {
        let (resource, action, resource_id) = derive_key(method, path);
        for role in ["finance_manager", "accountant"] {
            let should_hold = grants[role].contains(&(resource.as_str(), action.as_str()));
            let matched = role_allowed(role, &resource, &action, resource_id);
            assert_eq!(
                matched, should_hold,
                "端点 {method} {path} 派生键 ({resource}, {action}) 的记录形态 {resource_id:?}，\
                 角色 {role} 的 matcher 实判为 {matched}，与既定授予面应判 {should_hold} 不符：\
                 判假即该岗点这条端线必 403，判真即授出了未申报的键"
            );
        }
    }
}

/// 3 反向 403：不持该键的角色必须被 matcher 判假。
#[test]
fn roles_without_a_key_are_denied_by_the_matcher() {
    // 3a 制单与审核互斥：两岗各自的半程不得互相放行（含按 ID 请求形态）
    for resource in ["ar-invoices", "ar-payments", "ar-verifications"] {
        for action in ["approve", "confirm"] {
            assert!(
                !role_allowed("accountant", resource, action, None)
                    && !role_allowed("accountant", resource, action, Some(12)),
                "accountant 不得被放行 {resource} 的 {action}（审核与确认腿归 \
                 finance_manager，制单审核互斥），matcher 判真即扩权"
            );
        }
        for action in ["create", "update"] {
            assert!(
                !role_allowed("finance_manager", resource, action, None)
                    && !role_allowed("finance_manager", resource, action, Some(12)),
                "finance_manager 不得被放行 {resource} 的 {action}（制单与改单腿归 accountant）"
            );
        }
    }
    // 3b 报表四枚只授审核岗，制单岗判假
    for resource in ["ar-aging", "ar-daily", "ar-monthly", "ar-statistics"] {
        assert!(
            !role_allowed("accountant", resource, "read", None),
            "accountant 不得被放行 {resource} 的 read（报表面归 finance_manager，与应付族同口径）"
        );
        assert!(
            role_allowed("finance_manager", resource, "read", None),
            "对照失效：finance_manager 对报表键 {resource} 的 read 本应放行却判假，\
             说明 3b 的判假没有分辨力"
        );
    }
    // 3c 本轮刻意不授的三枚（删单属管理面；导出按钮有 v-permission 守卫；收款作废前端零调用方）
    for (resource, action) in [
        ("ar-invoices", "delete"),
        ("ar-invoices", "export"),
        ("ar-payments", "cancel"),
    ] {
        for role in ["finance_manager", "accountant"] {
            assert!(
                !role_allowed(role, resource, action, None),
                "角色 {role} 不得被放行 {resource} 的 {action}（本轮刻意不授，见通道① 分组注释）"
            );
        }
    }
    // 3d 非财务岗整族判假：出纳、成本会计、销售两岗与仓管对九种动作全判假
    let rows = matrix_rows();
    for role in [
        "cashier",
        "cost_accountant",
        "sales_manager",
        "sales_rep",
        "warehouse_keeper",
    ] {
        assert!(
            rows.iter().any(|(r, _, _)| *r == role),
            "用例前提失效：角色码 {role} 不在生产矩阵里，反向断言成了空转，须换在册角色码"
        );
        for resource in AR_RESOURCES {
            for action in [
                "read", "create", "update", "delete", "approve", "confirm", "cancel", "print",
                "export",
            ] {
                assert!(
                    !role_allowed(role, resource, action, None)
                        && !role_allowed(role, resource, action, Some(12)),
                    "非财务岗 {role} 不得被放行 {resource} 的 {action}，判真即应收族授权面外泄到该岗"
                );
            }
        }
    }
    // 3e 应收对账族是另一枚资源名，本域八枚的判假不许把它一起判掉
    assert!(
        role_allowed("finance_manager", "ar-reconciliations", "update", None),
        "对照失效：finance_manager 既有 ar-reconciliations 通配行必须放行 update，\
         判假说明 permission_models 构造或 matcher 调用形态本身坏了"
    );
}

/// 4 通道① 与通道② 逐字符同集合，并与期望面三方对齐。
#[test]
fn seed_matrix_and_migration_grants_are_the_same_set() {
    let migration_src = read_repo(GRANT_MIGRATION);
    let from_migration = migration_grant_triples(&migration_src);
    let from_matrix = matrix_ar_triples();
    let expect = expected_triples();

    let only_in_seed: Vec<_> = from_matrix.difference(&from_migration).collect();
    let only_in_migration: Vec<_> = from_migration.difference(&from_matrix).collect();
    assert!(
        only_in_seed.is_empty(),
        "通道① 有而通道② 没有（存量库永缺这些键，老库开该面必 403）：{only_in_seed:?}"
    );
    assert!(
        only_in_migration.is_empty(),
        "通道② 有而通道① 没有（新装库缺这些键，且存量库授出了未定岗的键）：{only_in_migration:?}"
    );
    assert_eq!(
        from_migration, expect,
        "通道② 迁移 GRANTS 与本锁既定授予面不等，任一侧改动都要三处同批改"
    );
    assert_eq!(
        from_matrix, expect,
        "通道① 矩阵的 ar-* 行与本锁既定授予面不等，任一侧改动都要三处同批改"
    );
    // 通道② 的角色码集合必须恰是两岗（新增第三岗位拿应收族键却忘了通道① 即红）
    let mut migration_roles: BTreeSet<&str> = BTreeSet::new();
    for (role, _, _) in &from_migration {
        migration_roles.insert(role.as_str());
    }
    assert_eq!(
        migration_roles,
        ["accountant", "finance_manager"].into_iter().collect(),
        "通道② 的角色码集合必须恰为财务两岗，出现第三角色即扩权（admin 走 *:* blanket 不在矩阵行内）"
    );
}

/// 5 通道③ 零枚：e2e SEED_ROLE_EXTRA_PERMISSIONS 不得登记任何 ar-* 键。
#[test]
fn e2e_seed_channel_holds_no_ar_keys() {
    let setup_src = read_repo(E2E_SETUP);
    let offenders: Vec<&str> = AR_RESOURCES
        .iter()
        .filter(|resource| setup_src.contains(&format!("'{resource}:")))
        .copied()
        .collect();
    assert!(
        offenders.is_empty(),
        "通道③ e2e SEED 里登记了应收族键 {offenders:?}：两岗权限面来自通道① 的 init 播种，\
         重复落行会撞唯一索引 uq_role_permissions_role_resource_action，本通道必须 0 枚"
    );
}

/// 6 检测力自证：四种注入都必须被判据点名，否则上面的绿不可信。
#[test]
fn detector_can_see_injected_drift() {
    let migration_src = read_repo(GRANT_MIGRATION);
    let baseline = migration_grant_triples(&migration_src);
    assert_eq!(
        baseline,
        expected_triples(),
        "用例前提失效：通道② 基线与期望面本应相等，后续注入比对才有意义"
    );

    // 注入一 摘掉一枚键
    let removed = migration_src.replace("(\"finance_manager\", \"ar-invoices\", \"approve\"),", "");
    let after_removal = migration_grant_triples(&removed);
    assert!(
        after_removal.len() + 1 == baseline.len()
            && !after_removal.contains(&(
                "finance_manager".to_string(),
                "ar-invoices".to_string(),
                "approve".to_string()
            )),
        "注入「摘掉 finance_manager 的 ar-invoices approve」未被检测到，通道② 取字面量是空转"
    );

    // 注入二 把一枚键的动作换成同资源在册的另一枚
    let swapped = migration_src.replace(
        "(\"accountant\", \"ar-verifications\", \"cancel\"),",
        "(\"accountant\", \"ar-verifications\", \"update\"),",
    );
    let after_swap = migration_grant_triples(&swapped);
    assert!(
        after_swap.len() == baseline.len()
            && !after_swap.contains(&(
                "accountant".to_string(),
                "ar-verifications".to_string(),
                "cancel".to_string()
            ))
            && after_swap.contains(&(
                "accountant".to_string(),
                "ar-verifications".to_string(),
                "update".to_string()
            )),
        "注入「ar-verifications 的 cancel 换成 update」未被检测到，逐枚动作维度比对是空转"
    );

    // 注入三 角色码换成不持应收族键的岗位码
    let misroled = migration_src.replace(
        "(\"accountant\", \"ar-payments\", \"create\"),",
        "(\"cashier\", \"ar-payments\", \"create\"),",
    );
    let after_misrole = migration_grant_triples(&misroled);
    assert!(
        !after_misrole.contains(&(
            "accountant".to_string(),
            "ar-payments".to_string(),
            "create".to_string()
        )) && after_misrole.contains(&(
            "cashier".to_string(),
            "ar-payments".to_string(),
            "create".to_string()
        )),
        "注入「accountant 换成 cashier」未被检测到，角色码维度比对是空转"
    );

    // 注入四 反向断言的对照夹具：给 cashier 补一行 ar-invoices read 后 matcher 必须判真，
    // 证 3d 的「判假」有分辨力而不是恒真（恒真断言即本锁自证失败）。
    let now = Utc::now();
    let mut models = permission_models("cashier");
    let before = models
        .iter()
        .any(|p| matches_permission(p, "ar-invoices", None, "read"));
    assert!(
        !before,
        "夹具前提失效：cashier 本就持有 ar-invoices read 行，补行对照失去意义，须重选岗位"
    );
    models.push(role_permission::Model {
        id: 2,
        role_id: 1,
        resource_type: "ar-invoices".to_string(),
        resource_id: None,
        action: "read".to_string(),
        allowed: true,
        permission_code: None,
        created_at: now,
        updated_at: now,
    });
    let after = models
        .iter()
        .any(|p| matches_permission(p, "ar-invoices", None, "read"));
    assert!(
        after,
        "夹具失效：补一行 cashier 的 ar-invoices read 后 matcher 仍判假，说明反向断言恒真"
    );
}
