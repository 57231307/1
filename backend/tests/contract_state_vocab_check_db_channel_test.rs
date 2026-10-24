//! 状态词表 DB CHECK 补齐迁移的双向契约锁（文本对文本，无库连接）。
//!
//! 锁定的契约面：
//! 1. 迁移侧 13 列配方的 ALLOWED SQL 取值集与 models/status 各权威词表集合**相等**
//!    （逐元素，不是包含）：词表加了值而 CHECK 未加 ⇒ 写入撞 23514 冒裸 500；
//!    CHECK 比词表宽 ⇒ 旁路脚本可写出业务永不产生的脏 token。两种漂移这条都必须判红。
//! 2. 配方清单完整性：13 个「表.列/约束名」三件套必须在配方中出现且约束名只声明一次，
//!    新增列漏进词表比对、约束名漏配的漂移会被点名。
//! 3. 接线形态：取值只能经 allowed 占位注入 CHECK/守卫，迁移内不得出现 UPDATE 清洗语句。
//!
//! 调用方：cargo test（CI backend job）。入参：仅读两侧源码文本（CRLF 归一后比对）。
//! 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。

use bingxi_backend::models::status::{
    bpm_process_definition_status, greige_fabric_status, inventory_count,
    inventory_stock_quality_status, inventory_stock_status, inventory_transfer, logistics_waybill,
    purchase_order, purchase_receipt, quality_inspection_result, quality_inspection_type,
    sales_order,
};
use std::collections::BTreeSet;

fn read_migration_source() -> String {
    let raw = std::fs::read_to_string(format!(
        "{}/migration/src/domain/status_vocab_check/mod.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("读取 status_vocab_check/mod.rs 失败（迁移文件被移动或删除必须显式判红）");
    raw.replace("\r\n", "\n")
}

/// 从迁移原文按常量名提取单引号 token 集合（同出口商检先例的锚点解析，空集即判红）。
fn parse_allowed_tokens(mig_src: &str, const_name: &str) -> BTreeSet<String> {
    let anchor = format!("const {const_name}: &str =");
    let start = mig_src
        .find(&anchor)
        .unwrap_or_else(|| panic!("迁移缺少常量 {const_name}（CHECK 取值种子形态被改动）"));
    let rest = &mig_src[start..];
    let open = rest.find('"').expect("常量字面量起始引号缺失");
    let body = &rest[open + 1..];
    let close = body
        .find("\";")
        .expect("常量字面量未闭合（引号分号收尾形态被改动）");
    let sql_text = &body[..close];
    let mut set = BTreeSet::new();
    let mut cursor = sql_text;
    while let Some(o) = cursor.find('\'') {
        cursor = &cursor[o + 1..];
        let c = cursor.find('\'').expect("单引号未闭合");
        set.insert(cursor[..c].to_string());
        cursor = &cursor[c + 1..];
    }
    assert!(
        !set.is_empty(),
        "常量 {const_name} 解析出空集——解析失守，拒绝继续比对"
    );
    set
}

fn to_set(tokens: &[&str]) -> BTreeSet<String> {
    tokens.iter().map(|t| t.to_string()).collect()
}

fn lock_column(
    mig_src: &str,
    const_name: &str,
    vocab: &BTreeSet<String>,
    table: &str,
    column: &str,
    constraint: &str,
) {
    let check_set = parse_allowed_tokens(mig_src, const_name);
    assert_eq!(
        check_set, *vocab,
        "{table}.{column} 的 CHECK 取值集与应用词表必须集合相等：\nCHECK={check_set:?}\n词表={vocab:?}"
    );
    let spec_anchor = format!(
        "table: \"{table}\",\n        column: \"{column}\",\n        constraint: \"{constraint}\","
    );
    assert!(
        mig_src.contains(&spec_anchor),
        "配方缺少三件套 {table}.{column} → {constraint}"
    );
    assert_eq!(
        mig_src.matches(&format!("\"{constraint}\"")).count(),
        1,
        "约束名 {constraint} 应只在配方里声明一次（up/down 均经配方字段取用，内联复写=第二套清单）"
    );
}

#[test]
fn lock_db_check_equals_authoritative_vocabularies_exactly() {
    let mig = read_migration_source();

    lock_column(
        &mig,
        "SALES_ORDER_STATUS_ALLOWED",
        &to_set(sales_order::ALL),
        "sales_orders",
        "status",
        "chk_sales_orders_status",
    );
    lock_column(
        &mig,
        "PURCHASE_ORDER_STATUS_ALLOWED",
        &to_set(&[
            purchase_order::DRAFT,
            purchase_order::PENDING_APPROVAL,
            purchase_order::SUBMITTED,
            purchase_order::APPROVED,
            purchase_order::REJECTED,
            purchase_order::CLOSED,
            purchase_order::CANCELLED,
            purchase_order::COMPLETED,
            purchase_order::PARTIAL_RECEIVED,
        ]),
        "purchase_orders",
        "order_status",
        "chk_purchase_orders_order_status",
    );
    lock_column(
        &mig,
        "PURCHASE_RECEIPT_STATUS_ALLOWED",
        &to_set(&[
            purchase_receipt::DRAFT,
            purchase_receipt::CONFIRMED,
            purchase_receipt::COMPLETED,
        ]),
        "purchase_receipt",
        "receipt_status",
        "chk_purchase_receipt_receipt_status",
    );
    lock_column(
        &mig,
        "INVENTORY_TRANSFER_STATUS_ALLOWED",
        &to_set(inventory_transfer::ALL),
        "inventory_transfers",
        "status",
        "chk_inventory_transfers_status",
    );
    lock_column(
        &mig,
        "INVENTORY_COUNT_STATUS_ALLOWED",
        &to_set(&[
            inventory_count::PENDING,
            inventory_count::IN_REVIEW,
            inventory_count::COMPLETED,
        ]),
        "inventory_counts",
        "status",
        "chk_inventory_counts_status",
    );
    lock_column(
        &mig,
        "INVENTORY_STOCK_STATUS_ALLOWED",
        &to_set(inventory_stock_status::ALL),
        "inventory_stocks",
        "stock_status",
        "chk_inventory_stocks_stock_status",
    );
    lock_column(
        &mig,
        "INVENTORY_STOCK_QUALITY_ALLOWED",
        &to_set(inventory_stock_quality_status::ALL),
        "inventory_stocks",
        "quality_status",
        "chk_inventory_stocks_quality_status",
    );
    lock_column(
        &mig,
        "GREIGE_FABRIC_STATUS_ALLOWED",
        &to_set(greige_fabric_status::ALL),
        "greige_fabric",
        "status",
        "chk_greige_fabric_status",
    );
    lock_column(
        &mig,
        "LOGISTICS_WAYBILL_STATUS_ALLOWED",
        &to_set(&[
            logistics_waybill::IN_TRANSIT,
            logistics_waybill::DELIVERED,
            logistics_waybill::SIGNED,
        ]),
        "logistics_waybills",
        "status",
        "chk_logistics_waybills_status",
    );
    // 质检两张表的 inspection_type 共用同一权威词表：配方两处都必须引用同一枚
    // QI_TYPE_ALLOWED 常量（第二套清单即判红），比对只做一次、出现两次逐处校验。
    let qi_tokens = to_set(quality_inspection_type::ALL);
    lock_column(
        &mig,
        "QI_TYPE_ALLOWED",
        &qi_tokens,
        "quality_inspection_standards",
        "inspection_type",
        "chk_quality_inspection_standards_inspection_type",
    );
    lock_column(
        &mig,
        "QI_TYPE_ALLOWED",
        &qi_tokens,
        "quality_inspection_records",
        "inspection_type",
        "chk_quality_inspection_records_inspection_type",
    );
    assert_eq!(
        mig.matches("allowed: QI_TYPE_ALLOWED").count(),
        2,
        "质检两张表的类型列必须各自独立引用同一枚 QI_TYPE_ALLOWED（合计两处配方取值）"
    );
    lock_column(
        &mig,
        "QI_RECORD_RESULT_ALLOWED",
        &to_set(quality_inspection_result::ALL),
        "quality_inspection_records",
        "inspection_result",
        "chk_quality_inspection_records_inspection_result",
    );
    lock_column(
        &mig,
        "BPM_DEFINITION_STATUS_ALLOWED",
        &to_set(bpm_process_definition_status::ALL),
        "bpm_process_definition",
        "status",
        "chk_bpm_process_definition_status",
    );
}

/// 钉"两侧同源"的接线形态：CHECK/守卫必须经 allowed 占位注入；存量闸门必须
/// fail-visible（RAISE EXCEPTION）且迁移内不得出现 UPDATE 清洗语句；幂等前置在场。
#[test]
fn lock_wiring_shape_placeholder_and_fail_visible_guard() {
    let mig = read_migration_source();
    assert!(
        mig.contains("NOT IN ({allowed})") && mig.contains("IN ({allowed})"),
        "守卫与 CHECK 的取值必须由 allowed 占位注入，禁止内联第二套清单"
    );
    assert!(mig.contains("RAISE EXCEPTION"), "缺少词表外存量的中止闸门");
    assert!(
        !mig.contains("UPDATE \""),
        "迁移内不得出现 UPDATE 造值清洗（脏值交业务定性后重跑）"
    );
    assert!(
        mig.contains("DROP CONSTRAINT IF EXISTS"),
        "缺少幂等前置（重跑/半态库必炸）"
    );
}
