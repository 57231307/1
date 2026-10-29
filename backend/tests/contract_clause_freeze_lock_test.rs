//! 签约条款渲染与固化腿的纯函数与接线静态锁（CI 常规可跑，不依赖活库）
//!
//! 功能 六组钉。一 占位符解析与填充纯函数的正向与反向 未闭合 未配对 空键 嵌套
//!   缺值 空串值 值引入残留形态 逐一判红并出缺键清单。二 条件出条款判定矩阵
//!   与词表同源，词表外 token 判红不静默。三 固化写库单点 全仓 src 面只有
//!   `services/contract_clause_service.rs` 允许写 `contract_clause` 实体，
//!   合同创建腿必须经该服务在事务内调用（sales 与 purchase 两个 create 各有一个调用点，
//!   且调用行在 txn.commit 之前）。四 两侧合同创建请求结构体的渲染四字段同形
//!   （模板选择 值表 内外销 含税）。五 已固化快照与模板生命周期解耦 停用与删除腿
//!   不得动快照行，详情与打印两个读函数不得按模板状态过滤（模板停用后已固化快照必须原样返回）。
//!   六 新固化行的溯源指针必写非空 PlannedClause 的 template_clause_id 是 i32 而非
//!   Option，pin_plan_txn 里 Set(Some(`row.template_clause_id`)) 必现，缺位即溯源断链。
//! 调用方 CI 的 Rust 测试 job。入参 无，读服务与合同两腿源文本。
//! 传给谁 无下游，纯断言。存什么 不写库、不写文件。存哪里 不落地。
//!
//! 判据是逐函数行为与出现位置，不是包含
//! - 渲染腿被绕过（有人直接在合同服务里拼快照行） 单点计数判红；
//! - 固化调用点跑到 commit 之后 回滚保证失守，位置判据判红；
//! - 停用或删除模板顺手动快照 追溯改写被复活，函数体扫描判红；
//! - 读侧接模板状态过滤 停用即抹掉合同条款可见性，函数体扫描判红；
//! - 新固化行不带溯源指针 必填标记回查断链，Set(Some( 与 i32 双点判红。

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

// ---------------------------------------------------------------------------
// 五 已固化快照与模板生命周期解耦
// ---------------------------------------------------------------------------

/// 模板停用与删除两条腿不得触碰快照表。判据按函数体扫描 retire 与 delete 两支大括号段，
/// 段内不出现 `contract_clause::Entity::update` 与 `contract_clause::Entity::delete` 与
/// `contract_clause::ActiveModel`；只允许出现 `contract_clause::Entity::find` 作引用探测。
/// 停用顺手动快照等于把“模板改版不回溯已签约定内容”这条切片A 的建模前提推翻。
#[test]
fn template_lifecycle_never_mutates_frozen_snapshots() {
    let src = strip_line_comments(&read("src/services/contract_template_service.rs"));
    for sig in ["pub async fn retire(", "pub async fn delete("] {
        let body = fn_body(&src, sig);
        for banned in [
            "contract_clause::Entity::update",
            "contract_clause::Entity::delete",
            "contract_clause::ActiveModel",
        ] {
            assert!(
                !body.contains(banned),
                "模板生命周期腿 {sig} 出现 {banned}：快照行只增不改，模板停用与删除必须原样保留已固化条款"
            );
        }
    }
}

/// 合同详情的条款数组与打印摘要两个读函数不得按模板状态过滤。判据按函数体扫描
/// `list_clauses_for_contract` 与 `clauses_summary_for_print` 与 `resolve_required` 三支，
/// 段内不出现 `contract_template_status::`，也不出现 `contract_template::Entity::find`
/// 后接状态比较的形态（回查只按主键或 (模板, 条款码)，一旦掺状态过滤，模板停用即抹掉合同条款可见性）。
#[test]
fn snapshot_read_paths_are_status_blind() {
    let src = strip_line_comments(&read("src/services/contract_clause_service.rs"));
    for sig in [
        "pub async fn list_clauses_for_contract(",
        "pub async fn clauses_summary_for_print(",
        "async fn resolve_required(",
    ] {
        let body = fn_body(&src, sig);
        assert!(
            !body.contains("contract_template_status::"),
            "读侧 {sig} 出现 contract_template_status:: 常量：模板状态不得参与已固化快照的读出"
        );
        assert!(
            !body.contains("contract_template::Entity"),
            "读侧 {sig} 回查了 contract_template::Entity：详情与打印只读快照表本身与条款项，不回读模板主表"
        );
    }
}

// ---------------------------------------------------------------------------
// 六 新固化行的溯源指针必写非空
// ---------------------------------------------------------------------------

/// PlannedClause 结构体 template_clause_id 必须是 i32 非 Option。
/// 一旦漂成 Option，pin_plan_txn 里 Set(Some(...)) 就会退化成 Set(...)，
/// 未渲染就落库的行可以携带空指针，详情腿的必填标记回查断链。
#[test]
fn planned_clause_carries_non_null_provenance_id() {
    let src = strip_line_comments(&read("src/services/contract_clause_service.rs"));
    let body = fn_body_struct(&src, "pub struct PlannedClause");
    assert!(
        body.contains("pub template_clause_id: i32"),
        "PlannedClause 缺 pub template_clause_id: i32（非 Option），实得函数体：{body}"
    );
    assert!(
        !body.contains("pub template_clause_id: Option<i32>"),
        "PlannedClause.template_clause_id 漂成 Option<i32>：新固化行允许空指针即溯源断链"
    );
}

/// pin_plan_txn 每行必写 `template_clause_id: Set(Some(`。缺位即快照行落库时留空指针，
/// 详情腿的多义回落逻辑会被伪造的必填标记消费。判据取函数体扫描，Set 表达式逐字符敏感。
#[test]
fn pin_writes_provenance_id_as_some() {
    let src = strip_line_comments(&read("src/services/contract_clause_service.rs"));
    let body = fn_body(&src, "pub async fn pin_plan_txn(");
    assert!(
        body.contains("template_clause_id: Set(Some(row.template_clause_id))"),
        "pin_plan_txn 未把 template_clause_id 写为 Set(Some(row.template_clause_id))：新固化行的溯源指针必须非空，实得函数体：{body}"
    );
    assert!(
        !body.contains("template_clause_id: Set(None)"),
        "pin_plan_txn 把 template_clause_id 显式写成 Set(None)：新固化行的溯源指针断链"
    );
}

/// 取 `pub struct X { ... }` 或 `pub async fn f(...) { ... }` 的顶层大括号体。
/// struct 与 fn 共用同一扫描器；找不到 sig 或大括号不配平直接 panic 点名。
fn fn_body_struct(src: &str, sig: &str) -> String {
    let at = src
        .find(sig)
        .unwrap_or_else(|| panic!("源文件里找不到 {sig}（结构体形态或改名使本锁失去检测对象）"));
    let bytes = src[at..].as_bytes();
    let mut depth = 0usize;
    let mut end = 0usize;
    let mut seen_open = false;
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'{' {
            depth += 1;
            seen_open = true;
        } else if *b == b'}' {
            depth = depth.saturating_sub(1);
            if seen_open && depth == 0 {
                end = i + 1;
                break;
            }
        }
    }
    assert!(end > 0, "{sig} 的大括号未配平，无法判定字段集合");
    src[at..at + end].to_string()
}
