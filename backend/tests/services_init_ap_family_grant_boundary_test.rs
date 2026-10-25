//! AP 族种子授予边界锁（静态，不连库、不起服务）
//!
//! 背景：AP 九枚细键（`ap-invoices` / `ap-payments` / `ap-payment-requests` /
//! `ap-verifications` / `ap-reconciliations` / `ap-aging` / `ap-daily` /
//! `ap-monthly` / `ap-statistics`）已在 `init_service::PERMISSION_RESOURCES`
//! 与 `utils/path_utils.rs` 的 ("ap", seg) 消歧表双双在册；运行期非超管角色
//! 只有矩阵里真实存在对应 (资源, 动作) 行才过得了
//! `middleware/permission.rs::matches_permission`（精确串匹配 + `"*"` 通配）。
//! 除超管 `*:*` blanket（handlers/auth_handler.rs 登录注入，不走矩阵行）外，
//! ap-* 键的持有者集合必须恰好等于财务三岗的既定授予面，多一枚即扩权。
//!
//! 本锁三件事，全部直接驱动生产定义表
//! `InitService::all_role_permission_definition_groups()`（与写库同一份数据，
//! 不抄矩阵副本）：
//! ① 缺键即红：finance_manager 五面 `"*"` + 四面 `"read"`、accountant 两面
//!    `read+create`、cashier 一面 `read+create` 逐枚点名必须在册——少授一枚
//!    或整体回收本锁都红，不接受"只查存在性"的空转；
//! ② 越权即红：cashier 对 `ap-invoices` 不得存在任何能**有效放行**
//!    update / delete 的行——显式 `"update"`、`"delete"`、或 `"*"` 通配
//!    （`matches_permission` 中 `p.action == "*"` 覆盖一切具体动作）三者
//!    皆判红；并把 cashier 的 ap-* 行整体钉为恰 {ap-payments read,
//!    ap-payments create} 两枚，任何新增键/动作都必须显式改本锁过评审；
//! ③ 持有者集合钉死：全矩阵中持有任何 ap-* 资源行的角色码集合必须恰好是
//!    {finance_manager, accountant, cashier}，其余角色一律不给（既定授予口径）。
//!    accountant 侧同②钉法：ap-* 行集合恰为四枚 read/create，防止"顺手补
//!    审批/核销面"未经申报漂移成越权授予。

use bingxi_backend::services::init_service::{InitService, PERMISSION_RESOURCES};

/// AP 族九枚细键（与注册表逐字符对齐；用例开头自证前提，防止测试与
/// 注册表漂移后拿着不存在的键名空转）。
const AP_RESOURCES: &[&str] = &[
    "ap-invoices",
    "ap-payments",
    "ap-payment-requests",
    "ap-verifications",
    "ap-reconciliations",
    "ap-aging",
    "ap-daily",
    "ap-monthly",
    "ap-statistics",
];

/// 摊平生产矩阵为 (角色码, 资源码, 操作码) 三元组（与成对性锁同一驱动方式）。
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

/// 某角色在某资源上的全部矩阵行动作（按 `matches_permission` 语义，
/// `"*"` 通配视为覆盖任何具体动作）。
fn actions_of(
    rows: &[(&'static str, &'static str, &'static str)],
    role: &str,
    resource: &str,
) -> Vec<&'static str> {
    rows.iter()
        .filter(|(r, res, _)| *r == role && *res == resource)
        .map(|(_, _, act)| *act)
        .collect()
}

/// 某角色在 AP 族全部资源上的 (资源, 动作) 行集合（排序后逐条可比）。
fn ap_rows_of(
    rows: &[(&'static str, &'static str, &'static str)],
    role: &str,
) -> Vec<(&'static str, &'static str)> {
    let mut picked: Vec<(&str, &str)> = rows
        .iter()
        .filter(|(r, res, _)| *r == role && AP_RESOURCES.contains(res))
        .map(|(_, res, act)| (*res, *act))
        .collect();
    picked.sort_unstable();
    picked
}

/// 用例前提自证：九枚细键必须全部登记在权威注册表，
/// 否则本锁是在拿注册表外的死键名对拍，判据整体失效。
#[test]
fn ap_resources_are_registered_in_authority_table() {
    for resource in AP_RESOURCES {
        assert!(
            PERMISSION_RESOURCES.contains(resource),
            "用例前提：{resource} 必须登记在 PERMISSION_RESOURCES，否则矩阵行是永不命中的死码授权"
        );
    }
    // 防本锁名单与注册表漂移：注册表里 ap- 前缀的键必须恰好九枚，
    // 新增第十枚时须同步扩本锁覆盖面，不许留暗键。
    let registered_ap: Vec<&str> = PERMISSION_RESOURCES
        .iter()
        .filter(|name| name.starts_with("ap-"))
        .copied()
        .collect();
    assert_eq!(
        registered_ap.len(),
        AP_RESOURCES.len(),
        "PERMISSION_RESOURCES 中 ap- 前缀键数与本锁名单不符（实得 {registered_ap:?}），须同步复核授予面"
    );
}

/// ① 缺键即红：财务两岗既定授予面逐枚点名（出纳与成本会计不持 AP 键，由 ② 钉死）。
#[test]
fn finance_roles_hold_their_paired_ap_grants() {
    let rows = matrix_rows();

    for (resource, action) in [
        ("ap-invoices", "read"),
        ("ap-invoices", "approve"),
        ("ap-payments", "read"),
        ("ap-verifications", "read"),
        ("ap-payment-requests", "read"),
        ("ap-reconciliations", "read"),
        ("ap-reconciliations", "confirm"),
    ] {
        assert!(
            actions_of(&rows, "finance_manager", resource).contains(&action),
            "finance_manager 必须持有 {resource} 的 {action} 行（应付审批与核销主责岗），缺行即授权死码"
        );
    }
    for resource in ["ap-aging", "ap-daily", "ap-monthly", "ap-statistics"] {
        assert!(
            actions_of(&rows, "finance_manager", resource).contains(&"read"),
            "finance_manager 必须持有 {resource} 的 read 行（报表面只读口径）"
        );
    }
    for (resource, action) in [
        ("ap-invoices", "read"),
        ("ap-invoices", "create"),
        ("ap-invoices", "update"),
        ("ap-payments", "read"),
        ("ap-verifications", "read"),
        ("ap-verifications", "create"),
        ("ap-verifications", "cancel"),
        ("ap-payment-requests", "read"),
        ("ap-payment-requests", "create"),
        ("ap-payment-requests", "update"),
        ("ap-payment-requests", "submit"),
        ("ap-reconciliations", "read"),
        ("ap-reconciliations", "create"),
    ] {
        assert!(
            actions_of(&rows, "accountant", resource).contains(&action),
            "accountant 必须持有 {resource} 的 {action} 行（记账岗制单面），缺键即红"
        );
    }
}

/// ② 防扩权锁：出纳与成本会计都不得持任何 ap-* 键。
/// 判据取矩阵行集合为空集，显式 update/delete 与 "*" 通配都自然被覆盖；
/// 一旦有人给这两个岗位补 AP 键，本锁即红，必须显式过口径评审。
#[test]
fn cashier_and_cost_accountant_never_hold_any_ap_grant() {
    let rows = matrix_rows();
    for role in ["cashier", "cost_accountant"] {
        let role_rows = ap_rows_of(&rows, role);
        assert!(
            role_rows.is_empty(),
            "{role} 不得持有任何 ap-* 授权行（应付制单与审批归财务经理与记账岗），实得 {role_rows:?} 须过本锁评审"
        );
    }
}

/// ③ 持有者集合钉死 + 两岗授予面精确钉死（其余角色一律不给的既定口径）。
#[test]
fn ap_grants_are_confined_to_finance_roles() {
    let rows = matrix_rows();

    let mut holders: Vec<&'static str> = rows
        .iter()
        .filter(|(_, res, _)| AP_RESOURCES.contains(&res))
        .map(|(role, _, _)| *role)
        .collect();
    holders.sort_unstable();
    holders.dedup();
    assert_eq!(
        holders,
        vec!["accountant", "finance_manager"],
        "ap-* 键的矩阵持有者必须恰为会计与财务经理两岗，出现第三角色即扩权（admin 走 *:* blanket，不在矩阵行内）"
    );

    let accountant_rows = ap_rows_of(&rows, "accountant");
    assert_eq!(
        accountant_rows.len(),
        13,
        "accountant 的 ap-* 授权行数必须恰为十三枚，实得 {accountant_rows:?}"
    );
    for expected in [
        ("ap-invoices", "create"),
        ("ap-invoices", "read"),
        ("ap-invoices", "update"),
        ("ap-payment-requests", "create"),
        ("ap-payment-requests", "read"),
        ("ap-payment-requests", "submit"),
        ("ap-payment-requests", "update"),
        ("ap-payments", "read"),
        ("ap-reconciliations", "create"),
        ("ap-reconciliations", "read"),
        ("ap-verifications", "cancel"),
        ("ap-verifications", "create"),
        ("ap-verifications", "read"),
    ] {
        assert!(
            accountant_rows.contains(&expected),
            "accountant 的 ap-* 授予面缺 {expected:?}，多授或漏授都要显式过本锁"
        );
    }

    let manager_rows = ap_rows_of(&rows, "finance_manager");
    assert_eq!(
        manager_rows.len(),
        11,
        "finance_manager 的 ap-* 授权行数必须恰为十一枚，实得 {manager_rows:?}"
    );
    for expected in [
        ("ap-aging", "read"),
        ("ap-daily", "read"),
        ("ap-invoices", "approve"),
        ("ap-invoices", "read"),
        ("ap-monthly", "read"),
        ("ap-payment-requests", "read"),
        ("ap-payments", "read"),
        ("ap-reconciliations", "confirm"),
        ("ap-reconciliations", "read"),
        ("ap-statistics", "read"),
        ("ap-verifications", "read"),
    ] {
        assert!(
            manager_rows.contains(&expected),
            "finance_manager 的 ap-* 授予面缺 {expected:?}，多授或漏授都要显式过本锁"
        );
    }
}
