//! 打印「单据类型目录」同源性锁（纯文本与常量比对，不依赖活库，CI 常规可跑）。
//!
//! 锁定三条不变量：
//! 1. `SUPPORTED_DOC_TYPES` 与 `PrintService::get_print_data` 的 match 分支取值集逐项相等，
//!    新增渲染分支必须同步在常量登记（否则模板 doc_type 校验会放行一个打不出的类型）；
//! 2. 该集合与内置模板种子的 doc_type 全集相等——模板管理页的单据类型候选由现有模板行的
//!    doc_type 去重派生，两者一旦分叉，界面上就出现"能选但无模板"或"有模板但选不到"的缺口；
//! 3. `get_print_data` 保留词表外 fail-closed 分支，非法 doc_type 绝不静默渲染。

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use bingxi_backend::handlers::print_handler::builtin_print_templates;
use bingxi_backend::services::print_service::SUPPORTED_DOC_TYPES;

fn read_rel(manifest_rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(manifest_rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 抽取 `get_print_data` 函数体内全部 `"token" =>` 分支取值（函数体以四空格缩进的右花括号收尾）。
fn render_arms(src: &str) -> BTreeSet<String> {
    let re = regex::Regex::new(r#"(?s)pub async fn get_print_data.*?\n    \}\n"#).unwrap();
    let m = re
        .find(src)
        .expect("print_service 未找到 get_print_data 函数体，解析锁前提失效");
    regex::Regex::new(r#""([a-z0-9_]+)"\s*=>"#)
        .unwrap()
        .captures_iter(m.as_str())
        .map(|c| c[1].to_string())
        .collect()
}

/// 渲染分支取值集必须与 SUPPORTED_DOC_TYPES 完全相等，且常量内无重复项。
#[test]
fn supported_doc_types_equals_render_arms() {
    let src = read_rel("src/services/print_service.rs");
    let arms = render_arms(&src);
    let declared: BTreeSet<String> = SUPPORTED_DOC_TYPES.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        SUPPORTED_DOC_TYPES.len(),
        declared.len(),
        "SUPPORTED_DOC_TYPES 存在重复取值：集合长度与数组长度不等"
    );
    let missing: Vec<&String> = arms.difference(&declared).collect();
    let stale: Vec<&String> = declared.difference(&arms).collect();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "渲染分支与 SUPPORTED_DOC_TYPES 分叉。仅存在于 match: {:?}；仅存在于常量: {:?}",
        missing,
        stale
    );
}

/// 常量集合必须与内置模板种子的 doc_type 全集相等：模板页单据类型候选取自现有模板行，
/// 分叉即出现"选得到但没模板"或"有模板但选不到"的静默缺口。
#[test]
fn supported_doc_types_cover_builtin_template_seed() {
    let seeded: BTreeSet<String> = builtin_print_templates()
        .into_iter()
        .map(|t| t.doc_type)
        .collect();
    let declared: BTreeSet<String> = SUPPORTED_DOC_TYPES.iter().map(|s| s.to_string()).collect();
    assert!(
        seeded.is_subset(&declared),
        "内置模板种子存在渲染链路不支持的 doc_type（模板存得进却永远打不出）: {:?}",
        seeded.difference(&declared).collect::<Vec<_>>()
    );
    assert_eq!(
        seeded, declared,
        "内置模板种子集合与 SUPPORTED_DOC_TYPES 不等：模板页派生的单据类型目录不完整"
    );
}

/// get_print_data 必须保留词表外 fail-closed 分支（未知单据类型报 404，不静默返回空数据）。
#[test]
fn unknown_doc_type_is_fail_closed() {
    let src = read_rel("src/services/print_service.rs");
    assert!(
        src.contains("Unknown document type"),
        "get_print_data 缺失未知单据类型的 fail-closed 分支"
    );
}
