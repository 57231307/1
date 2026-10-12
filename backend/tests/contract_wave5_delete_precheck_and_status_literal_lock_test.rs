//! 契约锁（wave5）：删除预检不得"检查没做就当通过" + 状态写入方常量不得回潮成字面量
//!
//! 三条根因→锁：
//! 1. `services/sku_mapping_service.rs::delete`：关联 `supplier_products` 行缺失时，
//!    原实现是 `if let Some(code) = ... { 预检 }` —— 快照取不到就整体跳过预检并放行硬删，
//!    属"检查没做就当通过"。现要求：显式 WARN 留痕 + 按 mapping 自身列
//!    (product_id, supplier_id) 联表继续查引用，只有查无引用依据才允许删。
//!    同时禁止回潮成两种一刀切（直接放行 / 无条件拒绝）。
//! 2. `services/quality_standard_service.rs::delete_standard`：原实现 `let referenced_count = 0;`
//!    + 死分支 + 注释声称"检查是否有引用"（注释在撒谎，实际永远不拦）。现要求真实计数查询，
//!    且引用列必须与迁移/写入方一致（`custom_orders.quality_standard_id` 无 FK；
//!    `quality_standards.previous_version_id` 版本链自引用）。
//! 3. `handlers/dye_batch_handler.rs`：缸号状态写入/比较点不得再写字面量，必须引
//!    `models/status/quality_dyeing.rs::dye_batch_lifecycle_status` 权威常量（写入方即词表唯一来源）。
//!
//! 手法与 `tests/contract_wave5_reference_precheck_and_family_test.rs` 的源码扫描锁一致：
//! 这些不变量是"代码形态"级别的（结构回潮不会被行为用例覆盖到），用 include_str! 直接固化。

// ---------------------------------------------------------------------------
// 1) SKU 映射删除预检
// ---------------------------------------------------------------------------

/// 取函数体：从签名处开始，到下一个同级条目（`\n    pub async fn ` / `\n    pub fn ` /
/// `\n    async fn ` / `\n    fn `）最早出现的位置之前为止
fn fn_body(src: &str, signature: &str) -> String {
    let anchor = src
        .find(signature)
        .unwrap_or_else(|| panic!("待锁函数不存在: {signature}"));
    let tail = &src[anchor..];
    let next = [
        "\n    pub async fn ",
        "\n    pub fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

/// SKU 映射删除：快照取不到时必须显式处置（WARN + 退化查引用），不得整体跳过预检
#[test]
fn sku_mapping_delete_must_handle_missing_snapshot_explicitly() {
    let src = include_str!("../src/services/sku_mapping_service.rs").replace('\r', "");
    let body = fn_body(&src, "pub async fn delete(");

    // ① 不允许"有值才查、无值静默放行"的 if-let 单分支形态（这就是被复审判定的旁路）
    assert!(
        !body.contains("if let Some(code)"),
        "delete 预检不得再使用 `if let Some(code) = ...` 单分支形态（快照缺失即整体跳过预检）；\
         实际函数体:\n{body}"
    );
    // ② 必须对"取不到快照"这一分支有显式处置
    assert!(
        body.contains("None =>") || body.contains("None => {"),
        "delete 必须显式处理快照缺失分支（None 分支），实际函数体:\n{body}"
    );
    assert!(
        body.contains("tracing::warn!") || body.contains("warn!("),
        "快照缺失分支必须 WARN 留痕，不得静默"
    );
    // ③ 缺失分支仍要真查引用（联表按 mapping 自身列），并且有拒绝出口
    assert!(
        body.contains("inner_join(purchase_order::Entity)"),
        "快照缺失分支必须按 mapping 自身列联表查 purchase_order_item 引用，实际函数体:\n{body}"
    );
    assert!(
        body.contains("Column::SupplierId.eq(mapping.supplier_id)"),
        "退化预检必须带上 mapping.supplier_id 维度（与映射三元组语义一致）"
    );
    assert!(
        body.contains("Column::ProductId.eq(mapping.product_id)"),
        "退化预检必须带上 mapping.product_id 维度"
    );
    // ④ 两条分支各自都要有"被引用即拒"的出口（不是无条件放行、也不是无条件拒绝）
    assert_eq!(
        body.matches("AppError::business_displayable(").count(),
        2,
        "SKU 映射删除的两条预检分支各应有一个 business_displayable 拒绝出口，实际函数体:\n{body}"
    );
    assert!(
        body.matches(".count(&*self.db)").count() >= 2
            || body.matches("count(&*self.db)").count() >= 2,
        "两条分支都必须真的发起计数查询，实际函数体:\n{body}"
    );
}

// ---------------------------------------------------------------------------
// 2) 质量标准删除预检（红线：注释与实现必须一致）
// ---------------------------------------------------------------------------

/// 质量标准删除必须做真实引用计数，且引用列与迁移/写入方同源
#[test]
fn quality_standard_delete_must_run_real_reference_count() {
    let src = include_str!("../src/services/quality_standard_service.rs").replace('\r', "");
    let body = fn_body(&src, "pub async fn delete_standard(");

    // ① 假预检的三种可识别形态一律禁绝
    assert!(
        !body.contains("let referenced_count = 0"),
        "delete_standard 不得再出现常量 0 的假预检，实际函数体:\n{body}"
    );
    assert!(
        !body.contains("if referenced_count > 0"),
        "delete_standard 不得保留永不成立的死分支"
    );
    assert!(
        body.contains(".count(&*self.db)"),
        "delete_standard 必须发起真实计数查询，实际函数体:\n{body}"
    );

    // ② 引用列必须覆盖 grep 出来的全部引用面（无 FK 的列尤其只能靠应用层预检）
    assert!(
        body.contains("custom_order::Column::QualityStandardId.eq(id)"),
        "必须查 custom_orders.quality_standard_id（migration/src/domain/sales_crm/mod.rs:498 ADD COLUMN 且无 FK）"
    );
    assert!(
        body.contains("quality_standard::Column::PreviousVersionId.eq(id)"),
        "必须查版本链自引用 previous_version_id（写入方 create_version_history）"
    );

    // ③ 拒绝族：被引用属业务规则，走 business_displayable（公开规则文案，不含表名/列名/约束名）
    assert!(
        body.contains("AppError::business_displayable("),
        "被引用时的拒绝必须走 business_displayable 族"
    );
    for forbidden in ["custom_orders", "quality_standards", "foreign key", "23503"] {
        assert!(
            !body.contains(&format!("\"{forbidden}")) && !body.contains(&format!("{forbidden} 表")),
            "出参文案不得含表名/约束名/SQL 细节，命中: {forbidden}"
        );
    }
}

/// 注释不得再声称"检查是否有引用"而实现不查：注释与实现必须一致
#[test]
fn quality_standard_delete_doc_comment_matches_implementation() {
    let src = include_str!("../src/services/quality_standard_service.rs").replace('\r', "");
    // 撒谎注释原文（"跳过 ParentId 检查"）不得回潮
    assert!(
        !src.contains("跳过 ParentId 检查"),
        "delete_standard 的注释不得再出现与实现不符的\"跳过检查\"表述"
    );
    // 注释必须点明真实引用面（与实现里实际查询的两个列同名）
    let anchor = src
        .find("pub async fn delete_standard(")
        .expect("delete_standard 不得消失");
    let head: String = src[..anchor]
        .chars()
        .rev()
        .take(1200)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    for documented in ["custom_orders.quality_standard_id", "previous_version_id"] {
        assert!(
            head.contains(documented),
            "删除预检的文档注释必须写明引用面 {documented}（与实现一致），实际注释:\n{head}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3) 染色缸号 handler 不得再写状态字面量
// ---------------------------------------------------------------------------

/// 缸号 16 态 lifecycle_status 的字面量在 handler 内必须清零（词表唯一来源＝写入方常量模块）
#[test]
fn dye_batch_handler_must_use_lifecycle_status_constants() {
    let src = include_str!("../src/handlers/dye_batch_handler.rs").replace('\r', "");
    let literals = [
        "\"pending_schedule\"",
        "\"scheduled\"",
        "\"preparing\"",
        "\"dyeing\"",
        "\"washing\"",
        "\"fixing\"",
        "\"dehydrating\"",
        "\"drying\"",
        "\"inspecting\"",
        "\"stored\"",
        "\"shipped\"",
        "\"cancelled\"",
        "\"terminated\"",
        "\"rework\"",
        "\"on_hold\"",
        "\"failed\"",
    ];
    for lit in literals {
        assert!(
            !src.contains(lit),
            "dye_batch_handler.rs 不得再出现缸号状态字面量 {lit}，\
             必须引 models/status/quality_dyeing.rs::dye_batch_lifecycle_status 常量"
        );
    }
    // 常量确实在被引用（防止有人改成别的私有常量或字符串拼接绕过上面的扫描）
    assert!(
        src.contains("dye_batch_lifecycle_status as batch_status")
            || src.contains("dye_batch_lifecycle_status::"),
        "dye_batch_handler.rs 必须直接引 dye_batch_lifecycle_status 权威常量"
    );
    for used in ["batch_status::PENDING_SCHEDULE", "batch_status::STORED"] {
        assert!(src.contains(used), "缸号写入/比较点应使用权威常量 {used}");
    }
}
