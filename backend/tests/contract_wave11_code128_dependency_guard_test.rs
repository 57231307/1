//! Code128 条码渲染依赖的供应链与行为治理锁。
//!
//! 本文件钉四件事：
//! 1. `code128` 依赖声明的固定版本号与供应链最小化特性面（`default-features = false`）；
//! 2. 渲染函数产出真实 PNG 字节（非空 + 魔数校验），锁住从依赖到光栅输出全链路可用；
//! 3. 上游消费点在空码值路径上 fail-closed（拒绝打印，不静默产出空白条码）。
//!
//! 依赖形态说明：Cargo 声明层 `"0.2.1"` 等价 `^0.2.1`（caret 范围），
//! 真正的精确锁定由 `Cargo.lock` resolved version 提供。本锁同时断言：
//! - Cargo.toml 声明层为 `"0.2.1"`（排除通配 `*` 或宽范围 `>=x` 的退化）；
//! - Cargo.lock resolved version 为精确 `0.2.1`。

use std::fs;
use std::path::Path;

/// `Cargo.toml` 中 `code128` 行：`version = "0.2.1"` 不含通配符或宽范围。
#[test]
fn code128_dependency_declares_stable_patch_version_not_wildcard() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml_path = Path::new(manifest_dir).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", cargo_toml_path.display()));

    let code128_line = content
        .lines()
        .find(|l| l.trim_start().starts_with("code128"))
        .expect("Cargo.toml 应声明 code128 依赖");

    assert!(
        code128_line.contains("version = \"0.2.1\""),
        "code128 声明行应包含精确 version = \"0.2.1\"（排除通配符/宽范围退化），实际行: {code128_line}"
    );
    assert!(
        !code128_line.contains("\"*\"") && !code128_line.contains("\">=\""),
        "code128 声明禁止使用通配符 `*` 或比较范围，实际行: {code128_line}"
    );
}

/// Cargo.lock 中 code128 resolved version 为精确 `0.2.1`（锁定实际构建版本）。
#[test]
fn code128_lockfile_pins_exact_resolved_version() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let lock_path = Path::new(manifest_dir).join("Cargo.lock");
    let content = fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", lock_path.display()));

    let code128_section = content
        .split("[[package]]")
        .find(|s| s.contains("name = \"code128\""))
        .expect("Cargo.lock 应包含 code128 包条目");

    assert!(
        code128_section.contains("version = \"0.2.1\""),
        "Cargo.lock 中 code128 的 resolved version 应为 0.2.1，实际段:\n{code128_section}"
    );
}

/// 供应链最小化：`code128` 显式声明 `default-features = false`。
/// 防止默认特性（若存在）引入不必要的传递依赖，扩大编译面与供应链攻击面积。
#[test]
fn code128_dependency_disables_default_features() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let cargo_toml_path = Path::new(manifest_dir).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path)
        .unwrap_or_else(|e| panic!("无法读取 {}: {e}", cargo_toml_path.display()));

    let code128_line = content
        .lines()
        .find(|l| l.trim_start().starts_with("code128"))
        .expect("Cargo.toml 应声明 code128 依赖");

    assert!(
        code128_line.contains("default-features = false"),
        "code128 声明必须包含 default-features = false（供应链最小化治理点），实际行: {code128_line}"
    );
}

/// 渲染真值：对合法码值调用 `render_code128_png` 产出非空 PNG 且以标准魔数开头。
#[test]
fn render_code128_png_produces_valid_png_bytes_for_real_barcode() {
    let result = bingxi_backend::utils::barcode::render_code128_png("PC202501001");

    let image = result.expect("render_code128_png 对有效码值应返回 Ok");

    assert!(!image.png.is_empty(), "产出 PNG 字节不应为空");

    let png_magic: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(
        &image.png[..8],
        &png_magic,
        "产出字节前 8 位应为 PNG 魔数 \\x89PNG\\r\\n\\x1a\\n"
    );

    assert!(
        image.width_px > 0 && image.height_px > 0,
        "PNG 像素尺寸应非零，实际 width={}px height={}px",
        image.width_px,
        image.height_px
    );
}

/// 渲染真值（边界）：短码值同样产出有效 PNG，锁住编码器最小输入不崩溃。
#[test]
fn render_code128_png_produces_valid_png_for_single_char_input() {
    let result = bingxi_backend::utils::barcode::render_code128_png("A");

    let image = result.expect("单字符码值应成功渲染");

    let png_magic: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    assert_eq!(
        &image.png[..8],
        &png_magic,
        "单字符输入仍应产出有效 PNG 魔数头"
    );
}

/// Fail-closed 源码锁：消费点在调用渲染函数前对空/空白条码执行拒绝路径。
///
/// 按源码实际行为（`print_service.rs` 成品布标签装配函数的缺值校验段），空值产生
/// `AppError::validation_displayable(...)`（HTTP 400 + VALIDATION_ERROR），
/// 而非静默跳过。此扫描锁钉死该保护不被削弱或删除。
const PRINT_SERVICE_SRC: &str = include_str!("../src/services/print_service.rs");

/// 断言 `get_inventory_piece_label_print_data` 函数体内，`render_code128_png` 调用
/// 之前存在对 `barcode` 字段的 `is_empty` 检查（fail-closed 不可移除）。
#[test]
fn print_service_fail_closes_empty_barcode_before_render_call() {
    let fn_body = PRINT_SERVICE_SRC
        .split("async fn get_inventory_piece_label_print_data")
        .nth(1)
        .expect("应存在 get_inventory_piece_label_print_data 函数定义");

    let render_call_pos = fn_body
        .find("render_code128_png")
        .expect("函数体内应调用 render_code128_png");

    let before_render = &fn_body[..render_call_pos];

    assert!(
        before_render.contains(".is_empty()"),
        "render_code128_png 调用之前应存在 is_empty 检查（fail-closed 守卫）"
    );

    assert!(
        before_render.contains("barcode"),
        "空值守卫应覆盖 barcode 字段"
    );

    assert!(
        before_render.contains("missing.push"),
        "空值命中后应推入 missing 列表（拒绝路径非静默跳过）"
    );

    assert!(
        before_render.contains("validation_displayable") && before_render.contains("return Err"),
        "missing 列表非空时应在进入渲染函数之前返回 Err（fail-closed），不允许静默放行"
    );
}

/// 断言 `render_code128_png` 函数本身不对空输入做兜底/默认值掩盖——
/// 空串若误达此函数，依赖 code128 crate 的行为（可能 panic 或产出最小图形），
/// 而非 `unwrap_or_default()` 式静默兜底。
#[test]
fn render_function_has_no_fallback_or_default_for_empty_input() {
    let barcode_src = include_str!("../src/utils/barcode.rs");

    assert!(
        !barcode_src.contains("unwrap_or_default"),
        "render_code128_png 不应使用 unwrap_or_default 兜底（会掩盖空值到渲染层的真实缺陷）"
    );

    assert!(
        !barcode_src.contains("unwrap_or("),
        "render_code128_png 不应使用 unwrap_or 兜底"
    );
}
