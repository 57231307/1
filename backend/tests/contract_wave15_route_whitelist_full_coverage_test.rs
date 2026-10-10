//! 路由资源段白名单的全量覆盖锁
//!
//! 功能：权限中间件在超管旁路**之前**先用 `validate_route_whitelist` 校验路径的
//! 资源段是否登记在 `path_utils` 的白名单里；未登记即对所有角色返回
//! 403 FORBIDDEN「未知的资源路径」，连管理员也过不去。本锁把"已注册路由的资源段
//! ⊆ 白名单"这条不变量逐条核对，缺一个就点名缺哪个。
//! 调用方：CI 的 Rust 集成测试任务（静态锁，不连库、不起服务）。
//! 入参：`frontend/scripts/route-snapshot.txt`（由 `route-snapshot.mjs --write` 从
//! 生产路由树导出，与 `--check` 同一条链）与运行时豁免表 `PUBLIC_PATHS` /
//! `AUTH_ONLY_PATHS` 的当前内容。
//! 传给谁：判定全部走生产谓词 `is_known_resource_segment`（模块前缀表 +
//! 直接资源表），测试不复制任何名单副本，因此不存在副本过期导致的假绿。
//! 存什么：不写任何数据，纯读。
//! 存哪里：不适用（无落库）。

use std::collections::BTreeSet;

use bingxi_backend::middleware::public_routes::{AUTH_ONLY_PATHS, PUBLIC_PATHS};
use bingxi_backend::utils::path_utils::is_known_resource_segment;

const SNAPSHOT: &str = include_str!("../../frontend/scripts/route-snapshot.txt");

/// 取 `/api/v1/erp/<资源段>/...` 里的资源段；非该前缀返回 None（这类路径不经资源门）。
fn erp_resource_segment(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/api/v1/erp/")?;
    let seg = rest.split('/').next().filter(|s| !s.is_empty())?;
    // 路径变量段（如 `{id}`）不是资源名，跳过
    if seg.starts_with('{') || seg == "*" {
        return None;
    }
    Some(seg)
}

/// 豁免表里出现过的资源段：这些路径由匿名/仅认证通道放行，本就不该登记进资源白名单。
fn exempt_segments() -> BTreeSet<String> {
    PUBLIC_PATHS
        .iter()
        .chain(AUTH_ONLY_PATHS.iter())
        .filter_map(|p| erp_resource_segment(p.split(['?', '#']).next().unwrap_or(p)))
        .map(str::to_string)
        .collect()
}

fn snapshot_segments() -> BTreeSet<String> {
    SNAPSHOT
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|line| {
            // 行形态：METHOD \t /path \t handler \t domain
            let path = line.split('\t').nth(1)?;
            erp_resource_segment(path.trim()).map(str::to_string)
        })
        .collect()
}

#[test]
fn every_registered_route_segment_is_whitelisted_or_exempt() {
    let exempt = exempt_segments();
    let segments = snapshot_segments();

    // 检测力自证：解析塌了（空集）会让下面的断言恒真，故先证解析确实工作。
    assert!(
        segments.len() > 100,
        "从路由快照解析出的资源段只有 {} 个，远低于实际规模——解析口径与快照格式漂移，\
         本锁此刻没有检测力（禁止把'空集'当'无缺陷'）",
        segments.len()
    );
    assert!(
        !exempt.is_empty(),
        "豁免表解析为空：PUBLIC_PATHS/AUTH_ONLY_PATHS 的形态变了，本锁的豁免通道失效"
    );
    // 反向探针：白名单确实会拒绝未登记资源，否则"全部通过"可能来自判据恒真。
    assert!(
        !is_known_resource_segment("zzz-not-a-real-resource-segment"),
        "白名单对明显未登记的资源段返回了放行，判据已失效"
    );

    let uncovered: BTreeSet<&String> = segments
        .iter()
        .filter(|s| !exempt.contains(s.as_str()) && !is_known_resource_segment(s))
        .collect();

    assert!(
        uncovered.is_empty(),
        "以下已注册路由的资源段既不在豁免表、也不在 path_utils 白名单，\
         会被 validate_route_whitelist 在任何角色（含超管）之前判 403「未知的资源路径」：{}。\
         修法：确认该资源语义后登记进 is_core_direct_resource / is_misc_direct_resource \
         或对应的模块前缀表，并按三通道（前端权限常量、init 角色矩阵、存量库授权迁移）补授权键。",
        uncovered
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

#[test]
fn segment_must_be_in_both_tables_to_be_reachable() {
    // 锁的是"登记错层"这一具体教训：只把资源名加进角色资源表而不加进路径白名单，
    // 运行期依然 403。路径白名单是可达性的第一道，必须先为真。
    assert!(
        is_known_resource_segment("period-report-snapshots"),
        "期末报表快照的资源段必须在 path_utils 路径白名单内；只在角色资源表登记 \
         不足以让端点可达，validate_route_whitelist 会先把它拦在 403"
    );
}
