//! 权限资源键派生锁（静态，不连库、不起服务）
//!
//! 功能：
//!   1. 叶子型模块前缀（`custom-orders`、`quotations`、`color-cards`、`color-prices`、
//!      `audit-logs`、`oa-announcements`、`bulk-color-approvals`、`webhooks`、`collection-tasks`、
//!      `quality-8d-reports`、`bad-debts`、`finance-alerts`、`data-permissions`）直接在前面挂记录
//!      ID 时，资源名必须回落到模块前缀本身、ID 从同一段取回。不回落则派生出 `123:read` 这种
//!      `PERMISSION_RESOURCES` 里不存在、任何角色都授予不了的死键，非超管打开详情面必 403。
//!   2. 反向自证：回落规则不得铺成"模块前缀下一律取 seg3"，子资源面仍须取 seg4 并消歧
//!      （`purchase/orders/45` → `purchase-orders`），否则本锁会因判据过宽而假绿。
//!   3. 派生资源名 ⊆ `PERMISSION_RESOURCES` 的全量活体棘轮：逐端点复算派生名，未登记且不在
//!      存量基线 `resource_key_baseline.txt` 内即判红；基线只准降不准升。
//!
//! 已知边界（如实声明，不静默）：本锁断言"违例集合 ⊆ 基线"与"违例数量 ≤ 基线数量"，因此修好一项
//! 后若忘记从基线删除，本锁仍绿——该项只能在评审/`E` 波清理时人工核对，锁不谎称它能抓。
//!
//! 调用方：CI 的 Rust 集成测试任务。
//! 入参：`frontend/scripts/route-snapshot.txt`（与 `route-snapshot.mjs --check` 同一条链导出）
//!       与运行时豁免表 `PUBLIC_PATHS` / `AUTH_ONLY_PATHS` 当前内容。
//! 传给谁：判定全部走生产谓词 `extract_resource_info` 与生产注册表 `PERMISSION_RESOURCES`，
//!         测试不复制任何名单副本，避免副本过期造成假绿。
//! 存什么/存哪里：基线文件由本锁消费；测试本身不写任何数据。

use std::collections::BTreeSet;

use bingxi_backend::middleware::permission::extract_resource_info;
use bingxi_backend::middleware::public_routes::{AUTH_ONLY_PATHS, PUBLIC_PATHS};
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;

const SNAPSHOT: &str = include_str!("../../frontend/scripts/route-snapshot.txt");
const BASELINE: &str = include_str!("resource_key_baseline.txt");

/// 取 `/api/v1/erp/` 之后的路径段；非该前缀或首段是路径参数时返回 None。
fn erp_segments(path: &str) -> Option<Vec<&str>> {
    let rest = path.strip_prefix("/api/v1/erp/")?;
    let segs: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    if segs.is_empty() {
        return None;
    }
    Some(segs)
}

/// 资源段（segment3）；`*`/`{id}` 形态不是资源名。
fn segment3(path: &str) -> Option<&str> {
    let first = *erp_segments(path)?.first()?;
    if first.starts_with('{') || first == "*" {
        return None;
    }
    Some(first)
}

/// 匿名/仅认证通道经过的资源段：这些面不走资源权限门，不参与违例统计。
fn exempt_segments() -> BTreeSet<String> {
    PUBLIC_PATHS
        .iter()
        .chain(AUTH_ONLY_PATHS.iter())
        .filter_map(|p| segment3(p.split(['?', '#']).next().unwrap_or(p)).map(str::to_string))
        .collect()
}

/// 快照把路径参数导出为 `*`；运行时该位是数字 ID，复算派生必须还原成数字段，
/// 否则"ID 被当成资源名"这一族缺陷在静态侧永远看不见。
fn runtime_path(path: &str) -> String {
    path.replace('*', "1")
}

fn snapshot_paths() -> Vec<String> {
    SNAPSHOT
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(1).map(str::to_string))
        .filter(|p| p.starts_with("/api/v1/erp/"))
        .collect()
}

#[test]
fn leaf_prefixed_detail_face_derives_module_prefix_as_resource() {
    let prefixes = [
        "custom-orders",
        "quotations",
        "color-cards",
        "color-prices",
        "audit-logs",
        "oa-announcements",
        "bulk-color-approvals",
        "webhooks",
        "collection-tasks",
        "quality-8d-reports",
        "bad-debts",
        "finance-alerts",
        "data-permissions",
    ];
    for prefix in prefixes {
        let path = format!("/api/v1/erp/{prefix}/123");
        let (resource, id) = extract_resource_info(&path);
        assert_eq!(resource, prefix, "{path} 的资源名应回落到模块前缀本身");
        assert_eq!(
            id,
            Some(123),
            "{path} 的记录 ID 应从同一段取回，否则行级归属门拿到 None"
        );
    }
}

#[test]
fn sub_resource_face_still_derives_fourth_segment() {
    let (resource, id) = extract_resource_info("/api/v1/erp/purchase/orders/45");
    assert_eq!(
        resource, "purchase-orders",
        "子资源面被误回落会抹掉域前缀，派生出注册表外的 orders 键"
    );
    assert_eq!(id, Some(45));

    let (nested, _) = extract_resource_info("/api/v1/erp/advanced/ai/recipe-optimization");
    assert_ne!(
        nested, "advanced",
        "双层前缀面必须继续取到 seg5，不得被回落规则吃掉"
    );

    let (flat, flat_id) = extract_resource_info("/api/v1/erp/vouchers/9");
    assert_eq!(flat, "vouchers");
    assert_eq!(flat_id, Some(9));
}

#[test]
fn derived_resource_names_are_registered_or_baseline_ratcheted() {
    let exempt = exempt_segments();
    let baseline: BTreeSet<String> = BASELINE
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();

    let mut violations: BTreeSet<String> = BTreeSet::new();
    let mut checked = 0_usize;
    for path in snapshot_paths() {
        match segment3(&path) {
            Some(seg3) if exempt.contains(seg3) => continue,
            Some(_) => {}
            None => continue,
        }
        checked += 1;
        let (resource, _) = extract_resource_info(&runtime_path(&path));
        if !PERMISSION_RESOURCES.contains(&resource.as_str()) {
            violations.insert(resource);
        }
    }

    assert!(
        checked > 1_500,
        "扫描端点数异常（{checked}），快照读取或路径过滤失效，本锁已在空转"
    );

    let onsets: Vec<&String> = violations
        .iter()
        .filter(|v| !baseline.contains(*v))
        .collect();
    assert!(
        onsets.is_empty(),
        "派生资源名不在 PERMISSION_RESOURCES 且不在存量基线内（非超管授予不了的死键）：{onsets:?}"
    );
    assert!(
        violations.len() <= baseline.len(),
        "违例数 {} 超过基线条数 {}，棘轮只准降不准升",
        violations.len(),
        baseline.len()
    );
}

#[test]
fn ratchet_detector_can_see_an_unregistered_key() {
    // 判定器自证：造一个注册表外资源段，必须被判为违例，否则上面的绿不可信。
    let (resource, _) = extract_resource_info("/api/v1/erp/wave16-canary-not-a-real-resource/1");
    assert_eq!(resource, "wave16-canary-not-a-real-resource");
    assert!(
        !PERMISSION_RESOURCES.contains(&resource.as_str()),
        "哨兵资源名意外命中注册表，说明判定器失去分辨力"
    );
}
