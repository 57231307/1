//! 生产岗 `boms:read` 的三通道一致静态锁（不连库、不启服务）
//!
//! 功能：钉死"物料清单读权限"这一枚键在三条通道上同源——① 新装库角色矩阵（走
//!       `InitService::all_role_permission_definition_groups()` 这份权威定义，不抄副本）；
//!       ② 存量库授权迁移 `business/m0092_grant_boms_read_production_manager.rs` 的角色常量；
//!       ③ 前端路由门 `router/index.ts` 对 /bom 声明的 `boms:read`。任一处漂移即判红。
//! 调用方：集成测试 crate 的可执行用例，CI 里由测试运行器调用。
//! 入参：无（三条通道都以 include_str! 读入真源文本，矩阵以生产函数取真值）。
//! 传给谁：把解析出的集合交给相等断言；资源名另与 `PERMISSION_RESOURCES` 对拍。
//! 存什么：不落库、不产文件，只把"该键授给哪些岗、前端按哪枚键门控"固化为回归断言。
//! 存哪里：仅本测试源文件一份，随源码进版本库。
//!
//! 为什么要这枚键：/bom 页的路由门与 `GET /boms` 的运行期派生键都是 `boms:read`，而生产域
//! 此前没有任何 boms 授权 ⇒ 生产经理连自己工单的用料依据都查不到。只授读不授写：
//! BOM 的建立/改版/审批属产品与工艺侧职责。

use std::collections::BTreeSet;

use bingxi_backend::services::init_service::{InitService, PERMISSION_RESOURCES};

/// 被锁定的资源名与动作名（与运行期派生键 `boms:read` 同形）。
const RESOURCE: &str = "boms";
const ACTION: &str = "read";
const KEY: &str = "boms:read";

/// 摊平生产矩阵为 (角色码, 资源码, 动作码) 三元组（同一份定义表，不抄副本）。
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

/// 通道② 真源（存量库授权迁移）。
fn migration_src() -> String {
    include_str!("../migration/src/domain/business/m0092_grant_boms_read_production_manager.rs")
        .replace('\r', "")
}

/// 只保留执行体：逐行剔除整行注释（注释里出现的角色/键名不是授予，计入即假判）。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
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

/// 通道②：迁移里 `const ROLE_CODES: &[&str] = &[...]` 的角色码清单（缺锚点即 panic，
/// 绝不把"解析不到"当成"没有授予"）。
fn migration_role_codes(code: &str) -> BTreeSet<String> {
    let anchor = code
        .find("const ROLE_CODES")
        .unwrap_or_else(|| panic!("迁移里必须存在 const ROLE_CODES（通道集合判据的定位符）"));
    let rest = &code[anchor..];
    let assign = rest
        .find("= &[")
        .unwrap_or_else(|| panic!("const ROLE_CODES 应为 `: &[&str] = &[...]` 赋值形态"));
    let body = &rest[assign + "= &[".len()..];
    let close = body
        .find(']')
        .unwrap_or_else(|| panic!("const ROLE_CODES 的角色清单未闭合"));
    quoted_tokens(&body[..close]).into_iter().collect()
}

/// 资源名必须是注册表内的权威名（否则运行期查的是注册表外资源，前端与后端都对不上）。
#[test]
fn boms_resource_is_registered_canonical_name() {
    assert!(
        PERMISSION_RESOURCES.contains(&RESOURCE),
        "{RESOURCE} 必须在 init_service::PERMISSION_RESOURCES 里在册，否则 boms:read 是注册表外键"
    );
}

/// 通道①：矩阵侧只有生产经理持 `boms/read`，且不得出现 `boms/"*"`（通配会把写侧能力一并放出）。
#[test]
fn matrix_grants_boms_read_to_production_manager_only() {
    let rows = matrix_rows();
    let holders: BTreeSet<String> = rows
        .iter()
        .filter(|(_, res, act)| *res == RESOURCE && (*act == ACTION || *act == "*"))
        .map(|(role, _, _)| role.to_string())
        .collect();
    assert_eq!(
        holders,
        ["production_manager".to_string()].into_iter().collect(),
        "{KEY} 的持有岗位须恰为 production_manager 一岗（出现别的岗或通配都算越界扩大）"
    );
    assert!(
        !rows
            .iter()
            .any(|(_, res, act)| *res == RESOURCE && *act == "*"),
        "boms 资源不得以 \"*\" 通配授予：本岗只需读，通配会把建/改/删/审批一并放出"
    );
}

/// 通道①↔②：新装矩阵与存量迁移的受授集合相等（改一处必须同步另一处）。
#[test]
fn matrix_and_migration_grantee_sets_agree() {
    let rows = matrix_rows();
    let from_matrix: BTreeSet<String> = rows
        .iter()
        .filter(|(_, res, act)| *res == RESOURCE && *act == ACTION)
        .map(|(role, _, _)| role.to_string())
        .collect();
    let from_migration = migration_role_codes(&code_only(&migration_src()));
    assert_eq!(
        from_matrix, from_migration,
        "矩阵与迁移对 {KEY} 的受授集合必须双向相等，否则新装库能进、存量库升级后仍 403"
    );
}

/// 通道③：前端 /bom 路由门声明的就是这枚键（键名漂移会让持权岗位被守卫拦在门外）。
#[test]
fn frontend_route_gate_declares_the_same_key() {
    let router = include_str!("../../frontend/src/router/index.ts").replace('\r', "");
    assert!(
        router.contains(KEY),
        "router/index.ts 必须按 {KEY} 门控物料清单页，与后端派生键同形"
    );
    // 检测力自证：同一段里键名是被数组包裹的权限声明，不是随手出现的字符串。
    let anchored = router
        .lines()
        .any(|line| line.contains("permission:") && line.contains(KEY));
    assert!(
        anchored,
        "{KEY} 必须出现在路由 meta 的 permission 声明里，仅以裸字符串出现在别处不算接通"
    );
}

/// 反空操作自证：证明上面几条相等/集合判据有判别力，而不是恒真。
#[test]
fn lock_detects_drift_instead_of_passing_silently() {
    // 一、迁移解析器能检出受授面被扩大：合成两岗清单须解析出 2 个码。
    let with_extra = "const ROLE_CODES: &[&str] = &[\"production_manager\", \"dyeing_master\"];";
    assert_eq!(
        migration_role_codes(with_extra).len(),
        2,
        "解析器须能检出受授清单被扩到两岗——扩清单却不被判红即为恒真假绿"
    );
    // 二、真实矩阵扫描非空：若定义表解析塌成空集，前面的相等断言会"两边都空"而假绿。
    let rows = matrix_rows();
    assert!(
        rows.len() > 100,
        "矩阵三元组总数异常偏低（实得 {}），定义表解析面失效，本锁无从比对",
        rows.len()
    );
    assert!(
        rows.iter()
            .any(|(_, res, act)| *res == RESOURCE && *act == ACTION),
        "真实矩阵必须含 ({RESOURCE}, {ACTION}) 行，否则通道① 缺席而通道② 在场＝半截授权"
    );
}
