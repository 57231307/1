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
//! - 负向半边：以下扫描范围内所有源码文件不得再出现任何角色主键字面量判定写法
//!   （`role_id != 1` / `role_id == 1` / `rid == 1` / `rid != 1`，并加严覆盖
//!   `role_id != Some(1)` / `role_id == Some(1)` 变体；`auth.role_id != Some(1)`
//!   与 `auth.role_id == Some(1)` 也显式列入禁列表——此前列表只写了裸 `role_id`
//!   前缀形态，对带 `auth.` 接收者前缀且实写带尾随 `{` 的写法属于永不命中的
//!   空断言，本轮修正为可命中形态）；
//! - 正向半边：这些文件必须确实出现 `admin_checker::is_admin_role` 引用——防止
//!   "把判定整段删掉也算绿"。其中 `crm_pool_handler.rs` 的正向半边是口径注释引用
//!   （该入口的字段脱敏委托 `crm_handler::apply_lead_field_permission`，本文件不
//!   自持判定代码），5 个 handler 是真实调用点，`field_mask.rs` / `cust.rs` /
//!   `lead.rs` 是权威源口径文档注释 + `is_admin: bool` 判定入参形态（见
//!   `ADMIN_PARAM_RECEIVERS`，判定本体由 async 调用方算好后显式传入，同步函数
//!   内不得出现任何角色字面量判据）。

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
    "auth.role_id!=Some(1)",
    "auth.role_id==Some(1)",
];

/// 唯一权威源引用必须存在（正向半边）
const REQUIRED_SINGLE_SOURCE: &str = "admin_checker::is_admin_role";

const SCANNED_FILES: &[(&str, &str)] = &[
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
    (
        "src/utils/field_mask.rs",
        include_str!("../src/utils/field_mask.rs"),
    ),
    (
        "src/services/crm/cust.rs",
        include_str!("../src/services/crm/cust.rs"),
    ),
    (
        "src/services/crm/lead.rs",
        include_str!("../src/services/crm/lead.rs"),
    ),
];

/// 同步掩码函数（拿不到 db 句柄）必须以 `is_admin: bool` 判定入参承接权威源结论，
/// 且入参必须真实参与分支判定（防"签名留参、判定删掉也算绿"）。
/// token 均按去空白归一后的源码比对。
const ADMIN_PARAM_RECEIVERS: &[(&str, &[&str])] = &[
    (
        "src/utils/field_mask.rs",
        &["is_admin:bool", "ifis_admin{", "if!is_admin{"],
    ),
    (
        "src/services/crm/cust.rs",
        &["is_admin:bool", "if!is_admin{"],
    ),
    (
        "src/services/crm/lead.rs",
        &["is_admin:bool", "if!is_admin{"],
    ),
];

/// 负向半边：扫描范围内文件不得再出现角色主键字面量判定（含 Some(1) 变体）。
#[test]
fn source_scan_no_role_primary_key_literal_checks() {
    for (file, src) in SCANNED_FILES {
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

/// 正向半边：扫描范围内文件必须引用唯一权威源——删掉判定不等于收口，防"整段删除也算绿"。
#[test]
fn source_scan_single_authority_referenced() {
    let normalized_required = normalize(REQUIRED_SINGLE_SOURCE);
    for (file, src) in SCANNED_FILES {
        let normalized = normalize(src);
        assert!(
            normalized.contains(&normalized_required),
            "{file} 必须出现 `{REQUIRED_SINGLE_SOURCE}` 引用（判定调用或权威源口径注释）。\
             若计划移除 admin 判定，必须以接入唯一权威源的方式收口，而不是删除判定。"
        );
    }
}

/// 正向半边（同步掩码函数接收端）：`field_mask.rs` / `cust.rs` / `lead.rs` 必须以
/// `is_admin: bool` 显式判定入参承接权威源结论，且判定分支真实存在——
/// 这些同步函数拿不到 db 句柄，判定的正确位置是 async 调用方经
/// `admin_checker::is_admin_role` 每请求循环外算一次后传入（fail-closed：
/// 角色缺失/查询失败必须传 false）。
#[test]
fn source_scan_is_admin_param_in_sync_mask_receivers() {
    for (file, required_tokens) in ADMIN_PARAM_RECEIVERS {
        let src = SCANNED_FILES
            .iter()
            .find(|(name, _)| name == file)
            .unwrap_or_else(|| panic!("{file} 必须同时登记在 SCANNED_FILES 中"))
            .1;
        let normalized = normalize(src);
        for token in *required_tokens {
            assert!(
                normalized.contains(token),
                "{file} 必须出现 `{token}`（去空白归一后）：同步掩码函数的 admin 判定\
                 必须是 `is_admin: bool` 入参并真实参与分支；判据来源唯一 = async 调用方\
                 经 admin_checker::is_admin_role 算出传入，不得改回角色主键字面量判定，\
                 也不得留下未参与判定的死参数。"
            );
        }
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
    for (file, src) in SCANNED_FILES {
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
