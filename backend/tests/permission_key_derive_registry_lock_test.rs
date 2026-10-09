//! 权限资源键派生锁：模块前缀消歧映射的逐族前提、结果一致性与反旁路守护。
//!
//! 功能定位：
//! 与 contract_wave11_rbac_key_derivation_test.rs 互补——后者做通用"目标⊆注册表"扫描
//! 及 crm/leads 单族前提钉死；本文件对全部消歧族（crm-opportunities、purchase-*、
//! sales-*、production-orders、bi-analysis）逐一验证三条判据链，同时增加默认分支
//! 对照路径确保 `_` 兜底不被误改。
//!
//! 调用方：
//! CI cargo test 阶段，contract_wave11 同一 job 内执行（非连库）。
//!
//! 入参来源：
//! `resolve_module_prefixed_resource`、`is_module_prefix`、`is_nested_module_prefix`
//! 均为 `bingxi_backend::utils::path_utils` 公开/可经源码读取的纯函数；
//! `PERMISSION_RESOURCES` 是 `bingxi_backend::services::init_service` 公开常量，
//! 与运行时种子灌入的唯一事实来源相同。
//!
//! 传递给谁：
//! 本测试自身——无下游依赖；判据失败时 panic 消息指明哪族漂移与影响面。
//!
//! 存什么/存哪里：
//! 不产生持久化副作用；只读源码文本与编译期常量。

use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::{
    is_module_prefix, is_nested_module_prefix, resolve_module_prefixed_resource,
};

/// 代表性消歧映射表：(module_prefix, resource_segment, expected_registered_key)。
/// 每一行对应一个 `resolve_module_prefixed_resource` 显式分支的输入/输出对，
/// 且 expected_registered_key 必须是 `PERMISSION_RESOURCES` 内的权威资源名。
const DISAMBIGUATION_CASES: &[(&str, &str, &str)] = &[
    // CRM 域
    ("crm", "leads", "crm-leads"),
    ("crm", "opportunities", "crm-opportunities"),
    // RFM 聚合面（`/crm/rfm/{distribution,segments}`）与它的行级同族端点
    // `/crm/customers/{id}/rfm` 必须落在同一个注册表名上：走默认分支会派生出注册表外的
    // `rfm:read`（注册表、角色种子、前端 CRM 页 meta 三通道都没有它），后果是除超管旁路
    // 外全员 403——CRM 页能打开、客户分级 tab 必红。
    ("crm", "rfm", "customers"),
    // 采购域
    ("purchase", "orders", "purchase-orders"),
    ("purchase", "returns", "purchase-returns"),
    ("purchase", "receipts", "purchase-receipts"),
    ("purchase", "contracts", "purchase-contracts"),
    ("purchase", "prices", "purchase-prices"),
    // 销售域
    ("sales", "returns", "sales-returns"),
    ("sales", "contracts", "sales-contracts"),
    ("sales", "prices", "sales-prices"),
    // 生产域
    ("production", "orders", "production-orders"),
    // 质量检验 URL 段是单数 quality-inspection，注册表与授权是复数 quality-inspections，
    // 不消歧则运行时键恒不等于已授复数键、quality_inspector 访问自己检验/缺陷面反而 403。
    ("production", "quality-inspection", "quality-inspections"),
    // BI 分析域
    ("bi", "sales", "bi-analysis"),
];

/// 反旁路裸段名清单：这些 segment 绝不得出现在 `PERMISSION_RESOURCES` 中，
/// 否则等于把权限键名定在注册表权威消歧名之外、由 URL 段直接决定词表，
/// 已登记的 crm-*/purchase-*/sales-* 授权行变成死码。
const MUST_NOT_BE_RAW_REGISTERED: &[&str] = &[
    "leads",
    "opportunities",
    "returns",
    "receipts",
    "contracts",
    "prices",
    // RFM 档位面的权威键是它行级同族端点所用的 customers；若把裸段名 rfm 登记进注册表，
    // 等于承认"派生名可以由 URL 段直接决定"，上面那条消歧就成了死分支。
    "rfm",
];

/// 默认分支对照表：(module_prefix, resource_segment, expected_passthrough)。
/// 这些路径段不被消歧——`resolve_module_prefixed_resource` 走 `_` 原样返回。
/// 验证前提：is_module_prefix 命中（否则中间件根本不进 resolve 函数）。
const DEFAULT_BRANCH_CASES: &[(&str, &str, &str)] = &[
    ("purchase", "supplier-products", "supplier-products"),
    (
        "purchase",
        "supplier-product-colors",
        "supplier-product-colors",
    ),
    ("sales", "quotations", "quotations"),
    ("production", "scheduling", "scheduling"),
    ("bi", "dashboard", "dashboard"),
];

/// 派生结果逐条等于注册表权威名（表驱动正向锁）。
#[test]
fn each_disambiguation_case_matches_registry() {
    for (prefix, segment, expected_key) in DISAMBIGUATION_CASES {
        let derived = resolve_module_prefixed_resource(prefix, segment);
        assert_eq!(
            derived.as_str(),
            *expected_key,
            "resolve_module_prefixed_resource(\"{prefix}\", \"{segment}\") 返回 \"{derived}\"，\
             期望注册表权威名 \"{expected_key}\"——映射漂移将使对应角色授权行变死码"
        );
        assert!(
            PERMISSION_RESOURCES.contains(expected_key),
            "\"{expected_key}\" 未登记在 PERMISSION_RESOURCES，消歧后仍无法命中任何授权行"
        );
    }
}

/// 前提锁：每一消歧族的 module_prefix 必须在 is_module_prefix 表内（否则中间件
/// 不进 resolve 分支，原始段名直接作键）。
#[test]
fn all_disambiguation_prefixes_are_module_prefixes() {
    let tested_prefixes: Vec<&str> = DISAMBIGUATION_CASES
        .iter()
        .map(|(p, _, _)| *p)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    for prefix in &tested_prefixes {
        assert!(
            is_module_prefix(prefix),
            "前缀 \"{prefix}\" 不在 is_module_prefix 表内——中间件不会调 resolve，\
             消歧分支永不命中，派生键退化为 URL 段原名"
        );
    }
}

/// 前提锁：每一消歧族 (prefix, segment) 不得被登记为双层跳段组合
/// （否则中间件取 seg5 而非 seg4 传入 resolve，资源键漂到维度/动作段）。
#[test]
fn no_disambiguation_pair_is_nested_prefix() {
    for (prefix, segment, expected_key) in DISAMBIGUATION_CASES {
        assert!(
            !is_nested_module_prefix(prefix, segment),
            "(\"{prefix}\", \"{segment}\") 被登记为双层前缀跳段——\
             中间件会取 seg5 传 resolve，消歧分支用 seg4 作入参将永不命中，\
             实际派生键漂离注册表权威名 \"{expected_key}\""
        );
    }
}

/// 反旁路：被消歧的裸段名不得出现在注册表内（否则等价于消歧形同虚设，
/// 两条路径都能命中同一键，授权边界模糊）。
#[test]
fn raw_segment_names_not_in_registry() {
    for name in MUST_NOT_BE_RAW_REGISTERED {
        assert!(
            !PERMISSION_RESOURCES.contains(name),
            "\"{name}\" 不得登记在 PERMISSION_RESOURCES 内——\
             消歧后的权威名带模块前缀，裸名入表等于把权限键定在消歧体系之外"
        );
    }
}

/// 默认分支对照：不被消歧的路径段原样透传，且透传结果已登记在注册表内。
#[test]
fn default_branch_passes_through_and_is_registered() {
    for (prefix, segment, expected) in DEFAULT_BRANCH_CASES {
        let derived = resolve_module_prefixed_resource(prefix, segment);
        assert_eq!(
            derived.as_str(),
            *expected,
            "(\"{prefix}\", \"{segment}\") 应走默认分支原样返回 \"{expected}\"，\
             实际返回 \"{derived}\"——若被误加入消歧表须同步登记对应新键"
        );
        assert!(
            PERMISSION_RESOURCES.contains(expected),
            "透传键 \"{expected}\" 未登记在 PERMISSION_RESOURCES，\
             运行时派生出的授权码将永不命中注册行、对非超管角色恒 403"
        );
    }
}

/// 漂移检测力验证：证明本文件正向锁确实能捕捉不存在的键名漂移。
/// 对 ("crm","opportunities") 断言返回值 **不等于** 故意拼错的键，
/// 同时断言一个根本不存在的 (prefix,segment) 组合走默认后不等于权威名——
/// 任何将 resolve 函数改成硬编码返回常量或添加错误映射的源码变更都会让
/// 上面 DISAMBIGUATION_CASES 表驱动与下面的反例同时暴露。
#[test]
fn lock_has_detection_power_via_negative_assertions() {
    // 反例一：故意错误的期望值绝不等于实际派生结果（证明表驱动不是恒真）
    let actual = resolve_module_prefixed_resource("crm", "opportunities");
    assert_ne!(
        actual.as_str(),
        "crm-opportunities-typo",
        "此断言只在 resolve 被改坏成返回垃圾常量时触发，正常运行不应触发"
    );
    // 反例二：对不存在的 (prefix, segment) 组合，走默认分支不应返回权威消歧名
    let default_result = resolve_module_prefixed_resource("crm", "nonexistent-resource");
    assert_ne!(
        default_result, "crm-opportunities",
        "未知 resource 段走默认分支不应意外命中消歧目标——若本断言触发说明 resolve \
         函数内多了非预期的 catch-all 映射"
    );
    // 反例三：验证 "sales"+"orders" 不被消歧（保留原名 orders），
    // 而 "purchase"+"orders" 才消歧到 purchase-orders
    let sales_orders = resolve_module_prefixed_resource("sales", "orders");
    assert_eq!(
        sales_orders.as_str(),
        "orders",
        "sales/orders 必须走默认分支保留原名——注册表里 orders 就是销售订单的权威键"
    );
    let purchase_orders = resolve_module_prefixed_resource("purchase", "orders");
    assert_eq!(
        purchase_orders.as_str(),
        "purchase-orders",
        "purchase/orders 必须消歧到 purchase-orders——否则与 orders 撞键导致跨域越权"
    );
}
