//! 签约条款渲染与固化腿的纯函数与接线静态锁（CI 常规可跑，不依赖活库）
//!
//! 功能 四组钉。一 占位符解析与填充纯函数的正向与反向 未闭合 未配对 空键 嵌套
//!   缺值 空串值 值引入残留形态 逐一判红并出缺键清单。二 条件出条款判定矩阵
//!   与词表同源，词表外 token 判红不静默。三 固化写库单点 全仓 src 面只有
//!   `services/contract_clause_service.rs` 允许写 `contract_clause` 实体，
//!   合同创建腿必须经该服务在事务内调用（sales 与 purchase 两个 create 各有一个调用点，
//!   且调用行在 txn.commit 之前）。四 两侧合同创建请求结构体的渲染四字段同形
//!   （模板选择 值表 内外销 含税）。
//! 调用方 CI 的 Rust 测试 job。入参 无，读服务与合同两腿源文本。
//! 传给谁 无下游，纯断言。存什么 不写库、不写文件。存哪里 不落地。
//!
//! 判据是逐函数行为与出现位置，不是包含
//! - 渲染腿被绕过（有人直接在合同服务里拼快照行） 单点计数判红；
//! - 固化调用点跑到 commit 之后 回滚保证失守，位置判据判红。

use bingxi_backend::services::contract_clause_service::{
    extract_placeholder_keys, fill_placeholders, variant_matches,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}（路径或改名需同步本锁）"))
}

fn strip_line_comments(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn vals(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// 一 占位符解析与填充
// ---------------------------------------------------------------------------

#[test]
fn placeholder_parser_accepts_valid_and_names_broken_forms() {
    // 正向 提取按出现顺序去重
    let keys =
        extract_placeholder_keys("甲方 {{party_a}} 与 {{party_b}} 及 {{party_a}} 再引用").unwrap();
    assert_eq!(
        keys,
        vec!["party_a".to_string(), "party_b".to_string()],
        "提取应按出现顺序去重"
    );
    // 正向 无占位符
    assert!(
        extract_placeholder_keys("没有占位符的正文")
            .unwrap()
            .is_empty()
    );

    // 反向 未闭合 未配对 空键 嵌套 逐一判红且消息点名形态
    let err = extract_placeholder_keys("未闭合 {{party_a").unwrap_err();
    assert!(err.contains("未闭合"), "未闭合形态应点名，实得 {err}");
    let err = extract_placeholder_keys("先收尾 }} 再开 {{party_a}}").unwrap_err();
    assert!(err.contains("未配对"), "未配对结束标记应点名，实得 {err}");
    let err = extract_placeholder_keys("空键 {{}}").unwrap_err();
    assert!(err.contains("不合法"), "空键应点名，实得 {err}");
    let err = extract_placeholder_keys("嵌套 {{a{{b}}}}").unwrap_err();
    assert!(err.contains("不合法"), "键内嵌套应点名，实得 {err}");
}

#[test]
fn filler_requires_every_key_and_rejects_empty_or_polluting_values() {
    let body = "供货 {{product_name}}，结算 {{payment_terms}}。";
    // 正向 全填
    let out = fill_placeholders(
        body,
        &vals(&[
            ("product_name", "全棉针织布"),
            ("payment_terms", "月结60天"),
        ]),
    )
    .unwrap();
    assert_eq!(
        out, "供货 全棉针织布，结算 月结60天。",
        "填充结果必须是成品原文"
    );

    // 反向 缺键 返回缺键清单
    let missing = fill_placeholders(body, &vals(&[("product_name", "全棉针织布")])).unwrap_err();
    assert_eq!(
        missing,
        vec!["payment_terms".to_string()],
        "缺键清单应点名未给的键"
    );

    // 反向 空串与纯空白值按未填处理
    let missing = fill_placeholders(
        body,
        &vals(&[("product_name", "  "), ("payment_terms", "月结60天")]),
    )
    .unwrap_err();
    assert_eq!(
        missing,
        vec!["product_name".to_string()],
        "纯空白值必须按未填处理，不许用空白冒充已填"
    );

    // 反向 值本身携带占位符形态 成品残留判红
    let polluted = fill_placeholders(
        body,
        &vals(&[
            ("product_name", "{{nested}}"),
            ("payment_terms", "月结60天"),
        ]),
    )
    .unwrap_err();
    assert!(
        polluted.iter().any(|k| k.contains("残留占位符形态")),
        "值引入残留形态必须点名，实得 {polluted:?}"
    );
}

// ---------------------------------------------------------------------------
// 二 条件出条款判定矩阵
// ---------------------------------------------------------------------------

#[test]
fn variant_match_matrix_is_vocab_sourced_and_fail_visible() {
    // 通用条款恒出
    assert!(
        variant_matches(None, None, None).unwrap(),
        "无适用条件的通用条款必须出"
    );
    // 内外销与含税四枚 判定来源逐一对应
    assert!(variant_matches(Some("export"), Some("export"), None).unwrap());
    assert!(!variant_matches(Some("export"), Some("domestic"), None).unwrap());
    assert!(variant_matches(Some("domestic"), Some("domestic"), None).unwrap());
    assert!(variant_matches(Some("tax_inclusive"), None, Some(true)).unwrap());
    assert!(!variant_matches(Some("tax_inclusive"), None, Some(false)).unwrap());
    assert!(variant_matches(Some("tax_exclusive"), None, Some(false)).unwrap());
    assert!(
        !variant_matches(Some("tax_exclusive"), None, None).unwrap(),
        "含税未采集时不得凭猜测出含税侧条款"
    );
    // 订做与框架 合同侧无采集来源 不出变体（必填与否由渲染腿分流拒绝）
    assert!(!variant_matches(Some("custom_made"), Some("domestic"), Some(true)).unwrap());
    assert!(!variant_matches(Some("framework"), Some("domestic"), Some(true)).unwrap());
    // 词表外 token 判红不静默
    let err = variant_matches(Some("when_hot"), None, None).unwrap_err();
    assert!(
        err.contains("不在词表"),
        "词表外条件必须点名拒绝，实得 {err}"
    );
}

// ---------------------------------------------------------------------------
// 三 固化写库单点与调用位置
// ---------------------------------------------------------------------------

/// 全仓 src 面 允许写 `contract_clause::ActiveModel` 的只有固化腿服务一个文件。
/// 出现第二个文件即在模板快照之外开辟了写快照的旁路（伪造正文与版本的第一落点）。
#[test]
fn snapshot_writes_are_single_sourced() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut writers: Vec<String> = Vec::new();
    let mut stack = vec![root];
    let mut scanned = 0usize;
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("读取 {:?} 失败: {e}", dir)) {
            let path = entry.expect("目录项").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            scanned += 1;
            let text = fs::read_to_string(&path).expect("读取源码失败");
            if strip_line_comments(&text).contains("contract_clause::ActiveModel") {
                writers.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
    }
    assert!(
        scanned > 200,
        "只扫到 {scanned} 个源文件，扫描没走到 src 全树，本锁失去检测力"
    );
    assert_eq!(
        writers,
        vec!["contract_clause_service.rs".to_string()],
        "写合同条款快照的通道必须只有固化腿服务一处，实得 {writers:?}"
    );
}

/// 两侧合同创建腿各自消费固化腿入口，且调用发生在事务提交之前
/// （提交后固化即脱离回滚保护，渲染失败会留下无条款的合同行）。
#[test]
fn contract_create_legs_call_freeze_before_commit() {
    for rel in [
        "src/services/sales_contract_service.rs",
        "src/services/purchase_contract_service.rs",
    ] {
        let src = strip_line_comments(&read(rel));
        let body = fn_body(&src, "pub async fn create(");
        let call_at = body
            .find("render_and_pin_for_new_contract")
            .unwrap_or_else(|| {
                panic!("{rel} 的 create 未调用渲染固化腿 render_and_pin_for_new_contract")
            });
        let commit_at = body
            .find("txn.commit()")
            .unwrap_or_else(|| panic!("{rel} 的 create 找不到 txn.commit，无法判定调用位置"));
        assert!(
            call_at < commit_at,
            "{rel} 的固化调用必须发生在 txn.commit 之前（同事务回滚保护），实得调用 {call_at} 提交 {commit_at}"
        );
    }
}

/// 取 `pub async fn create(...)` 的函数体（大括号配平截断，取不到直接 panic 点名）。
fn fn_body(src: &str, sig: &str) -> String {
    let at = src
        .find(sig)
        .unwrap_or_else(|| panic!("源文件里找不到 {sig}（函数形态或改名使本锁失去检测对象）"));
    let bytes = src[at..].as_bytes();
    let mut depth = 0usize;
    let mut end = 0usize;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'{' {
            depth += 1;
        } else if *b == b'}' {
            depth -= 1;
            if depth == 0 {
                end = i + 1;
                break;
            }
        }
    }
    assert!(end > 0, "{sig} 的函数体大括号未配平，无法判定次序");
    src[at..at + end].to_string()
}

// ---------------------------------------------------------------------------
// 四 两侧创建请求结构的渲染字段同形
// ---------------------------------------------------------------------------

#[test]
fn both_create_requests_carry_render_fields() {
    let sales = read("src/services/sales_contract_service.rs");
    let purchase = read("src/services/purchase_contract_service.rs");
    for (rel, src) in [
        ("sales_contract_service.rs", &sales),
        ("purchase_contract_service.rs", &purchase),
    ] {
        for field in [
            "pub template_id: Option<i32>",
            "pub clause_values: Option<std::collections::HashMap<String, String>>",
            "pub contract_market: Option<String>",
            "pub tax_inclusive: Option<bool>",
        ] {
            assert!(
                src.contains(field),
                "{rel} 的创建请求缺渲染字段 {field}（两侧合同腿的固化入口必须同形）"
            );
        }
    }
    // handler DTO 两侧同步消费（出参 DTO 无这四列就是接口没接线）
    let sales_handler = read("src/handlers/sales_contract_handler.rs");
    let purchase_handler = read("src/handlers/purchase_contract_handler.rs");
    for (rel, src) in [
        ("sales_contract_handler.rs", &sales_handler),
        ("purchase_contract_handler.rs", &purchase_handler),
    ] {
        for field in [
            "template_id",
            "clause_values",
            "contract_market",
            "tax_inclusive",
        ] {
            assert!(
                src.contains(&format!("pub {field}:")),
                "{rel} 的创建 DTO 缺字段 {field}"
            );
        }
    }
}
