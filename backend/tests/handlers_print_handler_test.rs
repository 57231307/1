//! 打印 Handler 单元测试（批次 394 补测）
//!
//! 覆盖目标：
//! - builtin_print_templates 静态模板列表（5 个测试）
use bingxi_backend::handlers::print_handler::*;
use std::collections::HashSet;

/// test_builtin_print_templatesfh6gmb；验证内置打印模板数量为 6（对应 6 种单据类型）
#[test]
fn test_builtin_print_templatesfh6gmb() {
    let templates = builtin_print_templates();
    assert_eq!(templates.len(), 63, "应有 6 个内置打印模板");
}

/// test_builtin_print_templates_idwyqlx；验证所有模板的 id 为 1-N，唯一且连续
#[test]
fn test_builtin_print_templates_idwyqlx() {
    let templates = builtin_print_templates();
    let n = templates.len() as i32;
    let ids: Vec<i32> = templates.iter().map(|t| t.id).collect();
    assert_eq!(ids, (1..=n).collect::<Vec<i32>>(), "id 应为 1-N 连续");

    // 唯一性检查
    let unique_ids: std::collections::HashSet<i32> = ids.iter().copied().collect();
    assert_eq!(unique_ids.len(), templates.len(), "id 应唯一");
}

/// test_builtin_print_templates_doc_typewy；验证所有模板的 doc_type 互不相同
#[test]
fn test_builtin_print_templates_doc_typewy() {
    let templates = builtin_print_templates();
    let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();
    let unique: std::collections::HashSet<&str> = doc_types.iter().copied().collect();
    assert_eq!(unique.len(), templates.len(), "doc_type 应唯一");
}

/// test_builtin_print_templatesqbwmrmb；验证所有内置模板的 is_default 均为 true
#[test]
fn test_builtin_print_templatesqbwmrmb() {
    let templates = builtin_print_templates();
    for t in &templates {
        assert!(t.is_default, "模板 {} 应为默认模板", t.name);
    }
}

/// test_builtin_print_templatesfg6zdjlx；验证模板覆盖全部 6 种业务单据类型： sales_order /
/// sales_contract / purchase_order / purchase_receipt / inventory_transfer / voucher
#[test]
fn test_builtin_print_templatesfg6zdjlx() {
    let templates = builtin_print_templates();
    let doc_types: Vec<&str> = templates.iter().map(|t| t.doc_type.as_str()).collect();

    let expected = [
        "sales_order",
        "sales_contract",
        "purchase_order",
        "purchase_receipt",
        "inventory_transfer",
        "voucher",
    ];
    for t in &expected {
        assert!(doc_types.contains(t), "应包含单据类型 {}", t);
    }

    // 名称不应为空
    for t in &templates {
        assert!(!t.name.is_empty(), "模板 {} 的名称不应为空", t.doc_type);
        assert!(
            !t.template_content.is_empty(),
            "模板 {} 的内容不应为空",
            t.doc_type
        );
    }
}
