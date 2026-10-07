//! `MAX_HTTP_BODY_BYTES` 跨模块同源防回潮锁（清理波遗留：常量正源迁移）
//!
//! 背景（源码侧 a7a139eb..ff76d9f5 批次落地）：全局 HTTP 请求体上限常量的正源从
//! `bootstrap/middleware_bootstrap.rs` 迁到 `constants.rs`，bootstrap 侧改为
//! `pub use crate::constants::MAX_HTTP_BODY_BYTES;` 再导出
//! （`middleware_bootstrap.rs:33`）。迁移的原因是本仓存在 **lib/bin 双 crate 树**：
//! `bootstrap` 模块只挂在 bin crate（`main.rs:1 mod bootstrap;`，`lib.rs` 无该模块），
//! lib（`bingxi_backend::`，即集成测试可见的 crate）取不到 bin 侧模块；而
//! `src/constants.rs` 被两侧各自 `mod` 同一份源文件（`lib.rs:11` + `main.rs:5`），
//! 所以"同一数值"的**唯一真源就是这一个文件**。
//!
//! 为什么本锁必须是"运行时数值断言 + 源码扫描"两段式（而不是简单的等值断言）：
//! 集成测试编译进的是 lib crate，`bingxi_backend::bootstrap::middleware_bootstrap`
//! **不存在**，`assert_eq!(constants侧, middleware_bootstrap侧)` 这种写法根本无法编译。
//! 若再导出被改回"在 bootstrap 里另落一份 `pub const MAX_HTTP_BODY_BYTES`"，
//! 两侧仍然各自编译通过、编译器永远发现不了漂移——这正是本次迁移要消除的假同源形态，
//! 只能用源码扫描锁把"再导出未断"钉死（与本仓既有源码扫描锁同范式，
//! 先例：`contract_wave2_po_item_update_fields_test.rs:707` 的 include_str! 扫描）。
//!
//! 这条锁能检出的退化（逐条对应断言）：
//! 1. 有人改动数值（如 12MB→8MB/20MB）而不改口径 ⇒ 值断言红；
//! 2. 有人把"CSV 导入 10MB + 余量"的文档口径改掉/删掉，数值与文档脱钩 ⇒ 文档块扫描红；
//! 3. 有人删掉 `middleware_bootstrap.rs` 的 `pub use` 再导出、改回落地 const（第二正源回潮）
//!    ⇒ 再导出存在性扫描红 + 本地定义禁项扫描红；
//! 4. 有人在全局层或 handler 单点覆写里改引别的常量/写死字面量 ⇒ 引用点扫描红。

/// 正源文件（lib/bin 两侧 mod 的同一份源文件）
const CONSTANTS_SRC: &str = include_str!("../src/constants.rs");
/// 再导出方（bin crate 树的全局中间件层）
const MIDDLEWARE_BOOTSTRAP_SRC: &str = include_str!("../src/bootstrap/middleware_bootstrap.rs");
/// 单点覆写方（供应商资质附件 handler，业务侧 5MB 校验的前置通道）
const SUPPLIER_HANDLER_SRC: &str = include_str!("../src/handlers/supplier_handler.rs");

/// 取 `pub const <NAME>` 定义行紧邻上方的连续 `///` 文档块（脱钩检测的作用域：
/// 只认该常量自己的文档，不接受同文件别处出现口径字样充数）。
fn doc_block_above_const(src: &str, const_name: &str) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let def_idx = lines
        .iter()
        .position(|l| {
            l.trim_start()
                .starts_with(&format!("pub const {const_name}"))
        })
        .unwrap_or_else(|| panic!("constants.rs 应存在 `pub const {const_name}` 定义"));
    let mut block = Vec::new();
    let mut i = def_idx;
    while i > 0 {
        i -= 1;
        let line = lines[i].trim();
        if line.starts_with("///") {
            block.push(line.to_string());
        } else {
            break;
        }
    }
    block.reverse();
    block.join("\n")
}

/// ① 数值钉：12MB 与 lib 侧可见的正源值（bin 侧 `crate::constants::MAX_HTTP_BODY_BYTES`
/// 与本值**同文件同编译单元内容**，见文件头"双 crate 树"说明，不存在第三份数值）。
/// 口径算术同时钉死：CSV 导入 10MB + 编码/头部余量 2MB——余量脱钩（比如把上限调到刚好
/// 10MB 掐死合法 CSV 导入，或无限膨胀）在此即红。
#[test]
fn max_http_body_bytes_value_pinned_12mb_over_csv_10mb_plus_margin() {
    assert_eq!(
        bingxi_backend::constants::MAX_HTTP_BODY_BYTES,
        12 * 1024 * 1024,
        "全局 HTTP 请求体上限既定值 12MB（请求体上限越界防护），改动需连带口径评审"
    );
    assert_eq!(
        bingxi_backend::constants::MAX_HTTP_BODY_BYTES - 10 * 1024 * 1024,
        2 * 1024 * 1024,
        "口径：CSV 导入 10MB + 2MB 编码/头部余量；差值不再是 2MB 即口径漂移"
    );
}

/// ② 文档口径不脱钩：正源常量自己的 `///` 文档块必须同时携带"12 MB"数值口径与
/// "CSV 导入 10MB"来源口径（`constants.rs:40` 现状）。注释被删/改成别的数值而代码没动，
/// 或代码改了注释没跟——都在此红。
#[test]
fn max_http_body_bytes_doc_keeps_csv_10mb_plus_margin_calibration() {
    let doc = doc_block_above_const(CONSTANTS_SRC, "MAX_HTTP_BODY_BYTES");
    assert!(
        doc.contains("12 MB"),
        "常量文档块须声明 12 MB 数值口径，实际文档块:\n{doc}"
    );
    assert!(
        doc.contains("CSV 导入 10MB"),
        "常量文档块须声明'CSV 导入 10MB + 余量'的来源口径，防止数值失去可审计依据，实际文档块:\n{doc}"
    );
}

/// ③ 再导出未断：`middleware_bootstrap.rs` 必须以 `pub use crate::constants::...`
/// 引用正源，且**不得**再落本地 `const MAX_HTTP_BODY_BYTES` 定义（第二正源回潮）。
/// 同时钉全局层的引用点：`DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES)` 仍在挂层处生效
/// （`middleware_bootstrap.rs:154` 现状），防止层被删除或改引它值后本锁失明。
#[test]
fn middleware_bootstrap_reexports_single_source_and_never_redefines_locally() {
    assert!(
        MIDDLEWARE_BOOTSTRAP_SRC.contains("pub use crate::constants::MAX_HTTP_BODY_BYTES;"),
        "bootstrap 侧必须保持对 constants.rs 正源的 pub use 再导出（迁移回潮检测）"
    );
    let local_definition = MIDDLEWARE_BOOTSTRAP_SRC
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .any(|l| {
            l.starts_with("const MAX_HTTP_BODY_BYTES")
                || l.starts_with("pub const MAX_HTTP_BODY_BYTES")
        });
    assert!(
        !local_definition,
        "禁止在 middleware_bootstrap.rs 另落一份 MAX_HTTP_BODY_BYTES 定义：\
         bin crate 的 bootstrap 模块对 lib（tests）不可见，双份定义编译器抓不到，\
         数值漂移只能靠本扫描锁防回潮"
    );
    assert!(
        MIDDLEWARE_BOOTSTRAP_SRC.contains("DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES)"),
        "全局请求体上限层必须继续引用本常量（引用点脱钩即安全边界回潮前兆）"
    );
}

/// ④ handler 单点覆写同源：资质附件上传的 `DefaultBodyLimit::max(...).apply` 必须引用
/// `crate::constants::MAX_HTTP_BODY_BYTES`（`supplier_handler.rs:804` 现状），不许写死
/// 字面量或改引它值——否则业务侧 5MB 显式校验的可达通道与全局层脱钩。
#[test]
fn supplier_handler_body_limit_override_references_same_constant() {
    assert!(
        SUPPLIER_HANDLER_SRC.contains(
            "DefaultBodyLimit::max(crate::constants::MAX_HTTP_BODY_BYTES).apply(&mut request)"
        ),
        "资质附件 handler 的单点覆写必须引用 constants 正源（与全局层同源，不造第二套上限）"
    );
}
