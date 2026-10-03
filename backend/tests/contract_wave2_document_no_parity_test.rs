//! 任务 #154 契约锁：取号/展示端 与 落库端「前缀 + 流水位数」逐字一致（防双轨复发）
//!
//! 覆盖 4 个单据类型（展示端 handler generate_no 端点 vs 落库权威 service）：
//! - 库存调整单 `handlers/inventory_adjustment_handler.rs` vs
//!   `services/inventory_adjustment_service.rs`（impl_generate_no! "ADJ"）
//! - 采购入库单 `handlers/purchase_receipt_handler.rs` vs
//!   `services/purchase_receipt_service.rs`（impl_generate_no! "PR"）
//! - 库存调拨单 `handlers/inventory_transfer_handler.rs` vs
//!   `services/inv/inventory_move.rs`（impl_generate_no! "TRF"）
//! - 盘点单 `handlers/inventory_count_handler.rs` vs
//!   `services/inventory_count_service.rs`（generate_no_with_txn "IC"，默认位数）
//!
//! 策略：不用硬编码期望值断言（会随一侧改动即失效），而是从两侧源码真实解析
//! `(prefix, width)` 后逐字比较；默认位数从 `utils/number_generator.rs` 的
//! 实现体解析（generate_no / generate_no_with_txn 各自转调 with_width 的实参），
//! 保证「解析器 → 真实代码」单向依赖，不存在第二套手写常量。
//!
//! 另锁 #154 第 5 项：`handlers/document_no_handler.rs` 查重已委托
//! `utils/number_generator.rs::is_document_no_taken` 注册表，且注册表包含
//! 前端仍在取号查重的 `outsourcing_receipt`（真实表真实列 receipt_no，
//! 委外收回单 receipt_no 仍由前端预生成——OVRC 只是入库凭证号，不替代收回单号）
//! 与 `inventory_count`，不得再出现 handler 内嵌 doc_type match。

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
}

/// 从 `from` 起找第一个双引号字符串字面量，返回 (内容, 闭引号后的下标)
fn first_string_lit(src: &str, from: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    let open = src[from..]
        .find('"')
        .map(|i| from + i)
        .expect("未找到字符串字面量（前缀常量）");
    let mut i = open + 1;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'"' {
            return (src[open + 1..i].to_string(), i + 1);
        }
        i += 1;
    }
    panic!("字符串字面量未闭合")
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// 从 `from` 起找第一个「独立数字」（前后不邻接标识符字符），返回 (值, 结束下标)
fn first_bare_number(src: &str, from: usize) -> (usize, usize) {
    let bytes = src.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let prev_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let next_ok = j >= bytes.len() || !is_ident_byte(bytes[j]);
            if prev_ok && next_ok {
                return (src[i..j].parse().unwrap(), j);
            }
            i = j;
            continue;
        }
        i += 1;
    }
    panic!("未找到独立数字（位数实参）")
}

/// 标识符读法：从 `from` 起读取 [A-Za-z0-9_] 串（用于取 generate_no* 函数名）
fn read_ident(src: &str, from: usize) -> (String, usize) {
    let bytes = src.as_bytes();
    let mut j = from;
    while j < bytes.len() && is_ident_byte(bytes[j]) {
        j += 1;
    }
    (src[from..j].to_string(), j)
}

/// 一次 `DocumentNumberGenerator::<fn_name>(... "<prefix>" ...[, width])` 调用的解析结果
#[derive(Debug, PartialEq, Eq)]
struct NoSpec {
    prefix: String,
    width: usize,
}

/// number_generator.rs 中 `generate_no` / `generate_no_with_txn` 的默认位数：
/// 解析其转调 `Self::generate_no_with_width{,_txn}(...)` 的末位实参，绝不写死 3。
fn default_width(gen_src: &str, transduce_call_marker: &str) -> usize {
    let idx = gen_src
        .find(transduce_call_marker)
        .unwrap_or_else(|| panic!("number_generator.rs 未找到转调标记 {transduce_call_marker}"));
    // 取参数列表右括号前的最后一个独立数字：本文件内该转调形如
    // `Self::generate_no_with_width(db, prefix, _entity, column, 3)`，
    // 参数中除末位 width 外无其它独立数字，取最后一个数字段最稳。
    let close = idx + gen_src[idx..].find(')').expect("转调调用未闭合");
    let mut last = None;
    let mut pos = idx;
    while pos < close && gen_src[pos..close].contains(|c: char| c.is_ascii_digit()) {
        let (n, end) = first_bare_number(&gen_src[..close], pos);
        last = Some(n);
        pos = end;
    }
    last.expect("转调调用无位数实参")
}

/// 解析包含 `fn_marker`（如 `pub async fn generate_no(`、`impl_generate_no!(`、
/// `DocumentNumberGenerator::generate_no_with_txn(`）的那次取号调用的 (prefix, width)。
///
/// - handler 侧：定位函数体后取 `DocumentNumberGenerator::` 调用名与前缀；
///   名称含 `with_width` → 位数取调用内显式实参，否则取生成器默认。
/// - service 宏侧：`impl_generate_no!` 调用无 width 位（宏展开走默认位数），
///   按展开目标（4 参 → generate_no；5 参 → generate_no_with_txn）解析默认。
fn parse_spec(file_src: &str, fn_marker: &str, gen_src: &str) -> NoSpec {
    let start = file_src
        .find(fn_marker)
        .unwrap_or_else(|| panic!("未找到标记 {fn_marker}"));

    if fn_marker.contains("impl_generate_no") {
        // 宏调用：参数为 (fn, "prefix", Entity, Column[, txn])，无 width。
        let (prefix, after_lit) = first_string_lit(file_src, start);
        let macro_end = file_src[after_lit..]
            .find(';')
            .map(|i| after_lit + i)
            .expect("impl_generate_no! 调用未以 ';' 结束");
        let call_text = &file_src[start..macro_end];
        // 参数逗号计数（宏调用内部无嵌套括号与逗号字符串，4 逗号=4 参、5 逗号=txn 变体）
        let commas = call_text.matches(',').count();
        let width = match commas {
            3 => default_width(
                gen_src,
                "Self::generate_no_with_width(db, prefix, _entity, column",
            ),
            4 => default_width(
                gen_src,
                "Self::generate_no_with_width_txn(txn, prefix, _entity, column",
            ),
            n => panic!("impl_generate_no! 参数数量异常（逗号数 {n}）"),
        };
        return NoSpec { prefix, width };
    }

    // 普通生成器调用（handler 端点 / service 直调），全程使用 file_src 绝对字节偏移
    let marker_at = file_src[start..]
        .find("DocumentNumberGenerator::")
        .map(|i| start + i)
        .expect("函数体内未出现 DocumentNumberGenerator 调用");
    let name_start = marker_at + "DocumentNumberGenerator::".len();
    let (name, after_name) = read_ident(file_src, name_start);
    let call_end = after_name + file_src[after_name..].find(')').expect("调用未闭合");
    let (prefix, after_lit) = first_string_lit(file_src, after_name);
    let width = if name.contains("with_width") {
        first_bare_number(&file_src[..call_end], after_lit).0
    } else {
        match name.as_str() {
            "generate_no" => default_width(
                gen_src,
                "Self::generate_no_with_width(db, prefix, _entity, column",
            ),
            "generate_no_with_txn" => default_width(
                gen_src,
                "Self::generate_no_with_width_txn(txn, prefix, _entity, column",
            ),
            other => panic!("未知的生成器函数名 {other}"),
        }
    };
    NoSpec { prefix, width }
}

const GEN: &str = "src/utils/number_generator.rs";

#[test]
fn inventory_adjustment_display_matches_storage_authority() {
    let gen_src = read(GEN);
    let display = parse_spec(
        &read("src/handlers/inventory_adjustment_handler.rs"),
        "pub async fn generate_no(",
        &gen_src,
    );
    let storage = parse_spec(
        &read("src/services/inventory_adjustment_service.rs"),
        "impl_generate_no!(",
        &gen_src,
    );
    assert_eq!(
        display, storage,
        "库存调整单双轨复发：展示端 {display:?} != 落库端 {storage:?}"
    );
}

#[test]
fn purchase_receipt_display_matches_storage_authority() {
    let gen_src = read(GEN);
    let display = parse_spec(
        &read("src/handlers/purchase_receipt_handler.rs"),
        "pub async fn generate_no(",
        &gen_src,
    );
    let storage = parse_spec(
        &read("src/services/purchase_receipt_service.rs"),
        "impl_generate_no!(",
        &gen_src,
    );
    assert_eq!(
        display, storage,
        "采购入库单双轨复发：展示端 {display:?} != 落库端 {storage:?}"
    );
}

#[test]
fn inventory_transfer_display_matches_storage_authority() {
    let gen_src = read(GEN);
    let display = parse_spec(
        &read("src/handlers/inventory_transfer_handler.rs"),
        "pub async fn generate_no(",
        &gen_src,
    );
    let storage = parse_spec(
        &read("src/services/inv/inventory_move.rs"),
        "impl_generate_no!(",
        &gen_src,
    );
    assert_eq!(
        display, storage,
        "库存调拨单双轨复发：展示端 {display:?} != 落库端 {storage:?}"
    );
}

#[test]
fn inventory_count_display_matches_storage_authority() {
    let gen_src = read(GEN);
    let display = parse_spec(
        &read("src/handlers/inventory_count_handler.rs"),
        "pub async fn generate_no(",
        &gen_src,
    );
    let storage = parse_spec(
        &read("src/services/inventory_count_service.rs"),
        "DocumentNumberGenerator::generate_no_with_txn(",
        &gen_src,
    );
    assert_eq!(
        display, storage,
        "盘点单双轨复发（同前缀不同位数也属双轨）：展示端 {display:?} != 落库端 {storage:?}"
    );
}

/// #154 第 5 项：查重 handler 必须委托注册表，不得回退为 handler 内嵌 doc_type match；
/// 注册表必须覆盖前端仍在「取号 + 查重」的全部类型（真实表真实列）。
#[test]
fn doc_no_check_handler_delegates_to_registry() {
    let handler = read("src/handlers/document_no_handler.rs");
    assert!(
        handler.contains("is_document_no_taken("),
        "查重 handler 必须委托 utils::number_generator::is_document_no_taken"
    );
    assert!(
        !handler.contains("match q.doc_type") && !handler.contains("match &q.doc_type"),
        "查重 handler 不得内嵌 doc_type match（旧白名单漂移根源）"
    );
}

/// 委外收回单 receipt_no 仍由前端预生成并查重（outsourcing_ops/receipt.rs 直接使用
/// req.receipt_no；OVRC 只是入库凭证号），注册表映射必须是真实表真实列。
#[test]
fn registry_covers_frontend_checked_doc_types_with_real_columns() {
    let gen_src = read(GEN);
    let expected: &[(&str, &str)] = &[
        (
            "outsourcing_receipt",
            "outsourcing_receipt::Column::ReceiptNo",
        ),
        ("inventory_count", "inventory_count::Column::CountNo"),
        ("outsourcing_order", "outsourcing_order::Column::OrderNo"),
        ("dye_batch", "dye_batch::Column::BatchNo"),
        ("dye_recipe", "dye_recipe::Column::RecipeNo"),
        ("finance_invoice", "finance_invoice::Column::InvoiceNo"),
        ("labor_contract", "labor_contract::Column::ContractNo"),
    ];
    for (doc_type, column) in expected {
        assert!(
            gen_src.contains(&format!("\"{doc_type}\" =>")),
            "注册表缺少前端查重类型 {doc_type}"
        );
        assert!(
            gen_src.contains(&format!("{column}.eq(no)")),
            "注册表 {doc_type} 缺少真实列匹配 {column}"
        );
    }

    // 真实列存在于模型（不是臆造映射）
    let model = read("src/models/outsourcing_receipt.rs");
    assert!(
        model.contains("pub receipt_no"),
        "outsourcing_receipt 模型必须有 receipt_no 列"
    );

    // 收回单创建确实消费前端 receipt_no（若未来改服务端取号，此断言应失败提醒清理映射）
    let receipt_svc = read("src/services/outsourcing_ops/receipt.rs");
    assert!(
        receipt_svc.contains("receipt_no: Set(req.receipt_no.clone())"),
        "收回单若不再使用前端 receipt_no，需同步移除查重映射（勿留死映射）"
    );
}
