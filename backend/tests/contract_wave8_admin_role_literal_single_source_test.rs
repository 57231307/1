//! D-4 收口源码扫描锁（PR #942 波次）：admin 判定唯一权威源单点化。
//!
//! 背景：本仓 admin 判定的唯一权威是 `crate::utils::admin_checker::is_admin_role`
//! （按 `roles.code='admin'` 查，查询失败 fail-closed 返回 false）。角色表按名称播种、
//! 主键由序列生成，播种顺序一旦变化，历史遗留的角色主键字面量判定（`role_id != 1` 等）
//! 会造成两类**静默**缺陷：
//! - 真 admin 拿到 id≠1 → 被当普通用户剔字段（功能坏）；
//! - 其他角色恰好拿到 id=1 → 静默扩权看到本该隐藏的金额/成本列（越权）。
//!
//! 双向断言（本仓既有教训：`include_str!` 文本锁对 rustfmt 换行敏感，
//! 所有比对都在"去全部空白归一后"的内容上进行，不依赖具体换行/缩进位置）：
//! - 负向半边：以下 6 个 handler 源码中不得再出现任何角色主键字面量判定写法
//!   （`role_id != 1` / `role_id == 1` / `rid == 1` / `rid != 1`，并加严覆盖
//!   `role_id != Some(1)` / `role_id == Some(1)` 变体）；
//! - 正向半边：这些文件必须确实出现 `admin_checker::is_admin_role` 引用——防止
//!   "把判定整段删掉也算绿"。其中 `crm_pool_handler.rs` 的正向半边是口径注释引用
//!   （该入口的字段脱敏委托 `crm_handler::apply_lead_field_permission`，本文件不
//!   自持判定代码），其余 5 个文件是真实调用点。

/// 去全部空白归一：抗 rustfmt 换行/缩进漂移，只锁定 token 序列本身。
fn normalize(source: &str) -> String {
    source.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 不得回潮的角色主键字面量判定写法（同样按去空白归一后比对）
const FORBIDDEN_ROLE_LITERAL_PATTERNS: &[&str] = &[
    "role_id!=1",
    "role_id==1",
    "rid!=1",
    "rid==1",
    "role_id!=Some(1)",
    "role_id==Some(1)",
];

/// 唯一权威源引用必须存在（正向半边）
const REQUIRED_SINGLE_SOURCE: &str = "admin_checker::is_admin_role";

const SCANNED_HANDLERS: &[(&str, &str)] = &[
    (
        "src/handlers/ap_payment_request_handler.rs",
        include_str!("../src/handlers/ap_payment_request_handler.rs"),
    ),
    (
        "src/handlers/inventory_stock_handler.rs",
        include_str!("../src/handlers/inventory_stock_handler.rs"),
    ),
    (
        "src/handlers/purchase_order_handler.rs",
        include_str!("../src/handlers/purchase_order_handler.rs"),
    ),
    (
        "src/handlers/purchase_receipt_handler.rs",
        include_str!("../src/handlers/purchase_receipt_handler.rs"),
    ),
    (
        "src/handlers/sales_order_handler.rs",
        include_str!("../src/handlers/sales_order_handler.rs"),
    ),
    (
        "src/handlers/crm_pool_handler.rs",
        include_str!("../src/handlers/crm_pool_handler.rs"),
    ),
];

/// 负向半边：6 个 handler 中不得再出现角色主键字面量判定（含 Some(1) 变体）。
#[test]
fn source_scan_no_role_primary_key_literal_checks() {
    for (file, src) in SCANNED_HANDLERS {
        let normalized = normalize(src);
        for pattern in FORBIDDEN_ROLE_LITERAL_PATTERNS {
            assert!(
                !normalized.contains(pattern),
                "{file} 不得出现角色主键字面量判定 `{pattern}`（去空白归一后仍命中）。\
                 admin 判定唯一权威源是 admin_checker::is_admin_role（roles.code='admin'），\
                 播种顺序变化时字面量判定会静默剔权或静默扩权，两类缺陷现有测试都抓不到。"
            );
        }
    }
}

/// 正向半边：6 个 handler 必须引用唯一权威源——删掉判定不等于收口，防"整段删除也算绿"。
#[test]
fn source_scan_single_authority_referenced() {
    let normalized_required = normalize(REQUIRED_SINGLE_SOURCE);
    for (file, src) in SCANNED_HANDLERS {
        let normalized = normalize(src);
        assert!(
            normalized.contains(&normalized_required),
            "{file} 必须出现 `{REQUIRED_SINGLE_SOURCE}` 引用（判定调用或权威源口径注释）。\
             若计划移除 admin 判定，必须以接入唯一权威源的方式收口，而不是删除判定。"
        );
    }
}

/// 逐文件钉住判定调用表达式本体（去空白归一）：5 个真实调用点必须
/// 以 `&state.db` + 请求上下文里的 `role_id` 调 is_admin_role，防止
/// "引用了符号但判定链路旁路"（如转手给另一套本地判定）。
#[test]
fn source_scan_admin_check_call_shape() {
    const CALL: &str = "admin_checker::is_admin_role(&state.db,role_id).await";
    const CALLERS: &[&str] = &[
        "src/handlers/ap_payment_request_handler.rs",
        "src/handlers/inventory_stock_handler.rs",
        "src/handlers/purchase_order_handler.rs",
        "src/handlers/purchase_receipt_handler.rs",
        "src/handlers/sales_order_handler.rs",
    ];
    for (file, src) in SCANNED_HANDLERS {
        if !CALLERS.contains(file) {
            continue;
        }
        let normalized = normalize(src);
        assert!(
            normalized.contains(CALL),
            "{file} 必须存在判定调用 `{CALL}`（去空白归一后），且置于循环外的分支条件处，\
             保证一次请求至多一次判定查询（admin_checker 内部另有 5 分钟缓存）。"
        );
    }
}
