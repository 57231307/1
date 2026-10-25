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

/// ① 缺键即红：三岗既定授予面逐枚点名。
#[test]
fn finance_three_roles_hold_their_paired_ap_grants() {
    let rows = matrix_rows();

    for resource in [
        "ap-invoices",
        "ap-payments",
        "ap-payment-requests",
        "ap-verifications",
        "ap-reconciliations",
    ] {
        assert!(
            actions_of(&rows, "finance_manager", resource).contains(&"*"),
            "finance_manager 必须持有 {resource} 的 \"*\" 全动作行（应付链路审批/核销主责岗），缺行即授权死码"
        );
    }
    for resource in ["ap-aging", "ap-daily", "ap-monthly", "ap-statistics"] {
        assert!(
            actions_of(&rows, "finance_manager", resource).contains(&"read"),
            "finance_manager 必须持有 {resource} 的 read 行（报表面只读口径）"
        );
    }
    for resource in ["ap-invoices", "ap-payments"] {
        for action in ["read", "create"] {
            assert!(
                actions_of(&rows, "accountant", resource).contains(&action),
                "accountant 必须持有 {resource} 的 {action} 行（记账岗录入面），缺键即红"
            );
        }
    }
    for action in ["read", "create"] {
        assert!(
            actions_of(&rows, "cashier", "ap-payments").contains(&action),
            "cashier 必须持有 ap-payments 的 {action} 行（出纳付款执行面），缺键即红"
        );
    }
}

/// ② 防扩权锁：cashier 不得持 ap-invoices 的 update 与 delete。
/// 判据按运行时语义取"有效放行"：显式 update/delete 行、或 "*" 通配行
/// （matches_permission: `p.action == "*" ⇒ 任意动作放行`）三者任一在场即红。
/// 并把 cashier 的整张 ap-* 行集合钉死为两枚，堵住换键绕行。
#[test]
fn cashier_never_holds_ap_invoice_update_or_delete() {
    let rows = matrix_rows();

    // 前提：cashier 的 ap-payments 正向行在场（否则本锁退化为恒真的空锁，
    // 由 ① 同样保证，这里再钉一次防止只留负例时整体被删空仍绿）。
    let cashier_payments = actions_of(&rows, "cashier", "ap-payments");
    assert!(
        cashier_payments.contains(&"read") && cashier_payments.contains(&"create"),
        "用例前提：cashier 的 ap-payments read+create 行必须在册，否则本锁失去锚点（空锁）"
    );

    let cashier_invoice = actions_of(&rows, "cashier", "ap-invoices");
    for forbidden in ["update", "delete", "*"] {
        assert!(
            !cashier_invoice.contains(&forbidden),
            "cashier 不得持有 ap-invoices 的 {forbidden} 授权行（\"*\" 在运行期同样放行 update/delete），实得 {cashier_invoice:?}"
        );
    }

    // 整体钉死：cashier 的 ap-* 行集合恰为 {ap-payments create, ap-payments read}，
    // 新增任何 ap 键/动作（含给 ap-invoices 授 read 这类"看起来无害"的扩面）
    // 都必须显式改本锁与授予口径，不许顺手漂移。
    assert_eq!(
        ap_rows_of(&rows, "cashier"),
        vec![("ap-payments", "create"), ("ap-payments", "read")],
        "cashier 的 ap-* 授予面必须恰为 ap-payments 的 read+create 两枚，实得突变须过本锁评审"
    );
}

/// ③ 持有者集合钉死 + accountant 面钉死（其余角色一律不给的既定口径）。
#[test]
fn ap_grants_are_confined_to_finance_three_roles() {
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
        vec!["accountant", "cashier", "finance_manager"],
        "ap-* 键的矩阵持有者必须恰为财务三岗，出现第四角色即扩权（admin 走 *:* blanket，不在矩阵行内）"
    );

    // accountant 面钉死：ap-* 行集合恰为四枚 read/create。审批（approve/reject）、
    // 核销、对账、付款申请与四张报表面都不归记账岗。
    assert_eq!(
        ap_rows_of(&rows, "accountant"),
        vec![
            ("ap-invoices", "create"),
            ("ap-invoices", "read"),
            ("ap-payments", "create"),
            ("ap-payments", "read"),
        ],
        "accountant 的 ap-* 授予面必须恰为 ap-invoices/ap-payments 的 read+create 四枚"
    );

    // finance_manager 面钉死：九枚 ap-* 全覆盖（五枚 "*" + 四枚 "read"），
    // 多授/漏授任何一枚都要显式过本锁。
    assert_eq!(
        ap_rows_of(&rows, "finance_manager"),
        vec![
            ("ap-aging", "read"),
            ("ap-daily", "read"),
            ("ap-invoices", "*"),
            ("ap-monthly", "read"),
            ("ap-payment-requests", "*"),
            ("ap-payments", "*"),
            ("ap-reconciliations", "*"),
            ("ap-statistics", "read"),
            ("ap-verifications", "*"),
        ],
        "finance_manager 的 ap-* 授予面必须恰为五枚全动作 + 四枚报表只读"
    );
}
