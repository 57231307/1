//! 染色配方状态词表守卫测试（CI 常规可跑，不依赖活库）
//!
//! 背景：dye_recipe.status 曾因词表常量取中文值（草稿/待审核/已审核/已停用）导致中英分裂——
//! 后端写库为中文、前端 RecipeTab.vue 按钮/标签用英文，真实数据行永不渲染审批按钮。
//! 本项目口径（用户拍板）：DB/service 用小写英文闭合词表，中文只出现在前端 i18n 展示层。
//!
//! 本测试锁死三条不变量，防止再次漂移：
//! 1. 词表常量值逐项 == 迁移 v15 域尾 CHECK `chk_dye_recipe_status` 的取值集（顺序一致）。
//! 2. 词表常量均为小写纯 ASCII（不含任何中文状态值）。
//! 3. dye_recipe 的 Rust 写入/比较点无中文状态字面量、无裸状态字面量——一律引常量。

use bingxi_backend::models::status::dye_recipe as recipe_status;
use std::fs;
use std::path::PathBuf;

/// 后端 crate 根目录（backend/），测试运行时以 CARGO_MANIFEST_DIR 定位被扫描文件。
fn backend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let path = backend_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {}", path.display(), e))
}

/// 剔除以 `//`（含 `///` 文档注释）起始的注释行，只保留代码内容。
/// 供负向裸字面量守卫使用：守卫本意是禁止「代码」里的裸状态字面量，注释里提及
/// 状态词（如说明历史缺陷的文档注释）是正当用途，不应被纯文本 contains 误报。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 从迁移 v15 域尾源码中解析 CHECK chk_dye_recipe_status 的 IN(...) 取值列表。
fn parse_migration_check_values() -> Vec<String> {
    let sql = read("migration/src/domain/v15/mod.rs");
    // 锚定 ADD CONSTRAINT 定义处（唯一），而非注释里对该约束名的后续提及。
    let anchor = "ADD CONSTRAINT \"chk_dye_recipe_status\"";
    let start = sql
        .find(anchor)
        .unwrap_or_else(|| panic!("迁移 v15 域尾未找到 {anchor}"));
    let tail = &sql[start..];
    // CHECK ("status" IN ( ... ))：从 IN 之后首个左括号取到其匹配右括号前的取值串。
    let in_pos = tail.find("IN (").or_else(|| tail.find("IN("));
    let in_pos = in_pos.unwrap_or_else(|| panic!("{anchor} 缺少 IN (...) 取值"));
    let after_in = &tail[in_pos..];
    let open = after_in
        .find('(')
        .unwrap_or_else(|| panic!("{anchor} 缺少左括号"));
    let close = after_in[open..]
        .find(')')
        .unwrap_or_else(|| panic!("{anchor} 缺少右括号"));
    let inner = &after_in[open + 1..open + close];
    let mut values = Vec::new();
    let mut rest = inner;
    while let Some(q1) = rest.find('\'') {
        let after = &rest[q1 + 1..];
        let q2 = after
            .find('\'')
            .unwrap_or_else(|| panic!("{anchor} 取值引号未闭合"));
        values.push(after[..q2].to_string());
        rest = &after[q2 + 1..];
    }
    values
}

/// 1. 词表常量值逐项 == 迁移 CHECK 取值集（一一对应，不丢状态、不自创状态）。
#[test]
fn dye_recipe_word_list_equals_migration_check_set() {
    let constants = vec![
        recipe_status::DRAFT.to_string(),
        recipe_status::PENDING_APPROVAL.to_string(),
        recipe_status::APPROVED.to_string(),
        recipe_status::DISABLED.to_string(),
    ];
    let check = parse_migration_check_values();
    assert_eq!(
        constants, check,
        "词表常量值集合必须与迁移 CHECK chk_dye_recipe_status 取值集逐项相等"
    );
    // 语义闭合：4 个状态一一对应收口前的中文值（草稿/待审核/已审核/已停用）。
    assert_eq!(recipe_status::DRAFT, "draft");
    assert_eq!(recipe_status::PENDING_APPROVAL, "pending_approval");
    assert_eq!(recipe_status::APPROVED, "approved");
    assert_eq!(recipe_status::DISABLED, "disabled");
    // ALL 常量与逐项一致，作为迁移/校验唯一来源。
    let all: Vec<&str> = recipe_status::ALL.to_vec();
    let want: Vec<&str> = constants.iter().map(String::as_str).collect();
    assert_eq!(all, want);
}

/// 2. 词表常量均为小写纯 ASCII（闭合词表，中文不得出现在取值层）。
#[test]
fn dye_recipe_word_list_is_lowercase_ascii() {
    for v in recipe_status::ALL {
        assert!(
            v.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "状态取值 {v} 含非小写 ASCII 字符（疑似中文/大写）"
        );
    }
}

/// 3. Rust 写入/比较点无中文状态字面量、无裸状态字面量（必须引 recipe_status:: 常量）。
#[test]
fn dye_recipe_rust_sites_have_no_bare_status_literals() {
    // 收口前的中文状态值，作为独立带引号字面量（"草稿" 等）不得再出现在源码里。
    // 仅匹配「双引号 + 该词 + 双引号」的精确取值字面量；业务提示语里作为子串的
    // "已审核的配方不允许删除" 等中文散文不会被误伤（其后不紧跟闭合双引号）。
    let cjk_literals = ["\"草稿\"", "\"待审核\"", "\"已审核\"", "\"已停用\""];

    // 词表常量文件：dye_recipe 模块取值由第 1/2 条测试通过 import 常量精确锁定。
    // 此文件同时定义其它表常量（如 production_recipe::DRAFT = "draft"），故此处不
    // 断言文件级不含 "draft"/"approved" 裸字面量（会自败），只查中文状态字面量。
    let status_file = code_only(&read("src/models/status/quality_dyeing.rs"));
    for lit in &cjk_literals {
        assert!(
            !status_file.contains(*lit),
            "quality_dyeing.rs 出现中文状态字面量 {lit}：词表常量取值不得为中文"
        );
    }

    // dye_recipe 专属的 Rust 写入/比较点：既不得有中文状态字面量，也不得有英文状态词
    // 裸字面量（一律引 recipe_status:: 常量）。用带引号精确匹配，避免把字段名
    // approved_by / created_by 等误判（"approved" 非 "approved_by" 的子串）。
    let ascii_literals = [
        "\"draft\"",
        "\"pending_approval\"",
        "\"approved\"",
        "\"disabled\"",
    ];
    for rel in [
        "src/services/dye_recipe_service.rs",
        "src/handlers/dye_recipe_handler.rs",
        "src/models/dye_recipe.rs",
    ] {
        let src = code_only(&read(rel));
        for lit in cjk_literals.iter().chain(ascii_literals.iter()) {
            assert!(
                !src.contains(*lit),
                "{rel} 出现裸状态字面量 {lit}：dye_recipe 状态必须引 models::status::dye_recipe 常量"
            );
        }
    }

    // 正向：service 确实通过 recipe_status:: 常量读写状态（而非仅依赖上一条的否定式）。
    let svc = read("src/services/dye_recipe_service.rs");
    for cst in ["DRAFT", "PENDING_APPROVAL", "APPROVED", "DISABLED"] {
        let needle = format!("recipe_status::{cst}");
        assert!(
            svc.contains(needle.as_str()),
            "service 未引用 recipe_status::{cst}：状态读写点应引常量"
        );
    }
}
