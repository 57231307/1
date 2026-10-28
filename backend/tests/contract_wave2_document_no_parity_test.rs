//! 契约锁：展示端取号端点废除后的单据号三层锁（防假可供性复发）
//!
//! 背景（20261028 裁定）：五枚对外可见的单据号展示端 GET generate-no 端点属假可供性
//! （展示码与落库码不一致，或制造的号不落单），已全废；单号一律从建单 POST 响应回读。
//! 本文件按三层锁逐单钉死废除后的口径：
//! - 展示腿：五枚 handler 不得再现 generate_no / generate_voucher_no 展示端函数，
//!   也不得再现 generate-no 路径文案。
//! - 落库腿：落库权威 service 必须在建单事务内取号并同事务插入
//!   （generate 取号(&txn) + insert(&txn) + 单号字段 Set 消费取号结果）；
//!   宏面四枚字面前缀互异；生成器双入口 generate_no 与 generate_no_with_txn
//!   的默认位数一致。
//! - 回读腿：建单响应必须回带真实单号字段或整模型序列化形态，且前端消费点
//!   从 POST 建单响应读取该单号，不得另有取号来源。
//!
//! 逐单五测覆盖调整单、盘点单、调拨单、入库单与凭证；调拨列表无建单落库入口，
//! 其回读腿只钉在建单响应整模型序列化形态上，无前端回读点。
//! 全局路由面的兜底另见 tests/test_generate_no_endpoints.rs 反向静态锁。
//!
//! 第 5 项旧锁保留：document_no 查重 handler 仍必须委托注册表，且注册表覆盖
//! 前端本地取号查重族（outsourcing_receipt 与 inventory_count 等属废除范围外的
//! 真消费），不得回退为 handler 内嵌 doc_type match。
//!
//! 策略不变：不用硬编码期望值断言（会随一侧改动即失效），前缀与位数一律从
//! 两侧源码真实解析后比较，保证解析器到真实代码单向依赖，不存在第二套手写常量。

use std::path::PathBuf;

fn read(rel: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", p.display()))
}

/// 读前端工作区文件（回读腿用）：rel 相对 frontend 根，如 src/views/...
fn read_frontend(rel: &str) -> String {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../frontend");
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

/// 从 `from` 起找第一个"独立数字"（前后不邻接标识符字符），返回 (值, 结束下标)
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

/// 取 `marker` 之后的第一个字符串字面量（即前缀常量），源码解析不写死前缀
fn prefix_after(src: &str, marker: &str) -> String {
    let start = src
        .find(marker)
        .unwrap_or_else(|| panic!("未找到标记 {marker}"));
    first_string_lit(src, start).0
}

const GEN: &str = "src/utils/number_generator.rs";

// ============================================================
// 三层锁：逐单五测（展示腿废除、落库腿事务内取号、回读腿建单响应）
// ============================================================

/// 调整单：展示腿已废，落库腿事务内取号加同事务插入，回读腿建单响应带 adjustment_no
#[test]
fn inventory_adjustment_no_comes_from_create_txn_and_response() {
    let handler = read("src/handlers/inventory_adjustment_handler.rs");
    assert!(!handler.contains("pub async fn generate_no("),
        "调整单展示端函数不得再现（假可供性已废除，单号只从建单响应回读）");
    assert!(!handler.contains("generate-no"), "调整单 handler 不得再现展示端路径文案");
    let svc = read("src/services/inventory_adjustment_service.rs");
    assert!(svc.contains("self.generate_adjustment_no(&txn)"), "调整单必须在建单事务内取号");
    assert!(svc.contains("adjustment.insert(&txn)"), "调整单必须与取号同事务插入");
    assert!(svc.contains("adjustment_no: Set(adjustment_no)"), "调整单落库必须消费事务内取号结果");
    assert!(handler.contains("pub adjustment_no: String"),
        "调整单建单响应必须回带真实单号字段 adjustment_no");
    let view = read_frontend("src/views/inventory-adjustment/tabs/AdjustmentFormDialogTab.vue");
    assert!(view.contains("res.data?.adjustment_no"),
        "调整单前端必须从建单响应回读单号（不得另有取号来源）");
}

/// 盘点单：展示腿已废，落库腿事务内取号加同事务插入，建单入参不得回填单号
#[test]
fn inventory_count_no_comes_from_create_txn_and_response() {
    let handler = read("src/handlers/inventory_count_handler.rs");
    assert!(!handler.contains("pub async fn generate_no("), "盘点单展示端函数不得再现");
    assert!(!handler.contains("generate-no"), "盘点单 handler 不得再现展示端路径文案");
    let svc = read("src/services/inventory_count_service.rs");
    assert!(svc.contains("DocumentNumberGenerator::generate_no_with_txn("), "盘点单必须在建单事务内取号");
    assert!(svc.contains("count_active.insert(&txn)"), "盘点单必须与取号同事务插入");
    assert!(svc.contains("count_no: Set(count_no)"), "盘点单落库必须消费事务内取号结果");
    assert!(!svc.contains("count_no: req.count_no"), "盘点单建单入参不得回填单号（后端为唯一取号来源）");
    assert!(handler.contains("pub count_no: String"), "盘点单建单响应必须回带真实单号字段 count_no");
    let view = read_frontend("src/views/inventory-count/tabs/CountFormDialogTab.vue");
    assert!(view.contains("res.data?.count_no"), "盘点单前端必须从建单响应回读单号");
}

/// 调拨单：展示腿已废，落库腿事务内取号、查重后同事务插入；调拨列表无建单落库
/// 入口，回读腿只钉在建单响应整模型序列化形态上，无前端回读点
#[test]
fn inventory_transfer_no_comes_from_create_txn_and_response() {
    let handler = read("src/handlers/inventory_transfer_handler.rs");
    assert!(!handler.contains("pub async fn generate_no("), "调拨单展示端函数不得再现");
    assert!(!handler.contains("generate-no"), "调拨单 handler 不得再现展示端路径文案");
    let svc = read("src/services/inv/inventory_move.rs");
    assert!(svc.contains("self.generate_transfer_no(&txn)"), "调拨单必须在建单事务内取号");
    assert!(svc.contains("transfer.insert(&txn)"), "调拨单必须与取号同事务插入");
    assert!(svc.contains("validate_transfer_no_uniqueness(&txn, &transfer_no)"), "调拨单事务内取号后必须查重");
    assert!(handler.contains("serde_json::to_value(transfer)"),
        "调拨单建单响应必须整模型序列化回带（含真实 transfer_no）");
    assert!(read("src/models/inventory_transfer.rs").contains("pub transfer_no: String"),
        "调拨单模型必须有 transfer_no 列供建单响应回读");
}

/// 入库单：展示腿已废，取号宏在 facade、事务腿在 ops/crud，回读腿整模型序列化
#[test]
fn purchase_receipt_no_comes_from_create_txn_and_response() {
    let handler = read("src/handlers/purchase_receipt_handler.rs");
    assert!(!handler.contains("pub async fn generate_no("), "入库单展示端函数不得再现");
    assert!(!handler.contains("generate-no"), "入库单 handler 不得再现展示端路径文案");
    let svc = read("src/services/purchase_receipt_service.rs");
    assert!(svc.contains("receipt_no: Set(receipt_no)"), "入库单落库必须消费事务内取号结果");
    let crud = read("src/services/purchase_receipt_ops/crud.rs");
    assert!(crud.contains("self.generate_receipt_no(&txn)"), "入库单必须在建单事务内取号");
    assert!(crud.contains(".insert(&txn)"), "入库单必须与取号同事务插入");
    assert!(handler.contains("serde_json::to_value(receipt)"),
        "入库单建单响应必须整模型序列化回带（含真实 receipt_no）");
    assert!(read("src/models/purchase_receipt.rs").contains("pub receipt_no: String"),
        "入库单模型必须有 receipt_no 列供建单响应回读");
    let view = read_frontend("src/views/purchase-receipt/composables/usePrcProc.ts");
    assert!(view.contains("res.data?.receipt_no"), "入库单前端必须从建单响应回读单号");
}

/// 凭证：展示腿已废（generate_voucher_no），落库腿事务内取号加同事务插入，
/// 回读腿建单响应回带整模型且前端从 POST 响应读 voucher_no
#[test]
fn voucher_no_comes_from_create_txn_and_response() {
    let handler = read("src/handlers/voucher_handler.rs");
    assert!(!handler.contains("pub async fn generate_voucher_no("), "凭证展示端函数不得再现");
    assert!(!handler.contains("generate-no"), "凭证 handler 不得再现展示端路径文案");
    let crud = read("src/services/voucher_ops/crud.rs");
    assert!(crud.contains("DocumentNumberGenerator::generate_no_with_txn("), "凭证必须在建单事务内取号");
    assert!(crud.contains("voucher_no: sea_orm::Set(voucher_no)"), "凭证落库必须消费事务内取号结果");
    assert!(crud.contains("active_model.insert(&txn)"), "凭证必须与取号同事务插入");
    assert!(handler.contains("Json<ApiResponse<voucher::Model>>"), "凭证建单响应必须回带整模型（含真实 voucher_no）");
    assert!(read("src/models/voucher.rs").contains("pub voucher_no: String"), "凭证模型必须有 voucher_no 列供建单响应回读");
    let view = read_frontend("src/views/voucher/tabs/composables/useVchrLst.ts");
    assert!(view.contains("res.data?.voucher_no"), "凭证前端必须从建单响应回读单号");
}

// ============================================================
// 落库腿全局锁：双入口默认位数一致、宏面字面前缀互异
// ============================================================

/// 生成器双入口默认位数必须一致：转调 with_width 与 with_width_txn 的末位实参
/// 同源解析，同前缀不同位数的双轨（盘点单旧疾）不得复活
#[test]
fn generator_default_width_is_consistent_across_entries() {
    let gen_src = read(GEN);
    let plain = default_width(
        &gen_src,
        "Self::generate_no_with_width(db, prefix, _entity, column",
    );
    let txn = default_width(
        &gen_src,
        "Self::generate_no_with_width_txn(txn, prefix, _entity, column",
    );
    assert_eq!(
        plain, txn,
        "generate_no 默认位数 {plain} 与 generate_no_with_txn 默认位数 {txn} 不一致，同前缀不同位数双轨将复发"
    );
}

/// 落库面四枚宏直调前缀互异（调整 ADJ、入库 PR、调拨 TRF、盘点 IC）：
/// 前缀一律源码解析，不写死期望值；凭证落库面前缀按凭证类型在 crud.rs 内分支
/// 取 SK、FK、ZZ、JZ 四枚字面量，不属于本互异面的单枚常量，其落库腿由凭证逐单锁钉
#[test]
fn storage_leg_prefixes_are_pairwise_distinct() {
    const MACRO: &str = "crate::impl_generate_no!(";
    let mut prefixes: Vec<(String, &str)> = vec![
        (
            prefix_after(&read("src/services/inventory_adjustment_service.rs"), MACRO),
            "src/services/inventory_adjustment_service.rs",
        ),
        (
            prefix_after(&read("src/services/purchase_receipt_service.rs"), MACRO),
            "src/services/purchase_receipt_service.rs",
        ),
        (
            prefix_after(&read("src/services/inv/inventory_move.rs"), MACRO),
            "src/services/inv/inventory_move.rs",
        ),
    ];
    prefixes.push((
        prefix_after(
            &read("src/services/inventory_count_service.rs"),
            "DocumentNumberGenerator::generate_no_with_txn(",
        ),
        "src/services/inventory_count_service.rs",
    ));
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (prefix, file) in prefixes {
        for (earlier, earlier_file) in &seen {
            assert_ne!(
                &prefix, earlier,
                "落库面前缀撞车：{file} 与 {earlier_file} 同为 {prefix}"
            );
        }
        seen.push((prefix, file));
    }
}

// ============================================================
// 第 5 项旧锁保留：document_no 查重委托注册表
// ============================================================

/// 第 5 项：查重 handler 必须委托注册表，不得回退为 handler 内嵌 doc_type match；
/// 注册表必须覆盖前端本地取号查重族仍在查重的全部类型（真实表真实列）。
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
