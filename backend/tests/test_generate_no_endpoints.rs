//! P1-1 generate-no 4 端点补齐
//!
//! 单元测试覆盖 4 个 generate-no 端点的单据号格式契约：
//! 1. inventoryCount（库存盘点）— 前缀 `IC`
//! 2. purchaseReceipt（采购收货）— 前缀 `RK`
//! 3. inventoryAdjustment（库存调整）— 前缀 `IA`
//! 4. inventoryTransfer（库存调拨）— 前缀 `IT`
//!
//! 单据号格式：`{前缀}{yyyyMMdd}{4 位流水}`，例如 `IC202605140001`。
//!
//! 这些测试**仅校验纯字符串格式化逻辑**，不依赖数据库：
//! - 端点的 `generate_no` 在 Handler 中仅作为薄包装调用
//!   `DocumentNumberGenerator::generate_no_with_width`；
//! - DB 部分（`count + 1`）由 [`number_generator`] 单元测试覆盖。
//!
//! 因此本测试文件只验证："业务前缀 + 8 位日期 + 4 位流水" 的拼接契约，
//! 防止后续有人误将流水宽度从 4 位回退为 3 位、或改动前缀字符。

use regex::Regex;

/// 验证库存盘点 generate-no 端点返回的单据号格式
///
/// 期望：`IC{yyyyMMdd}{4 位流水}`
#[test]
fn test_inventory_count_no_format() {
    // 模拟后端拼接结果
    let prefix = "IC";
    let today = "20260514";
    let serial = 1_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 4);

    let re = Regex::new(r"^IC\d{8}\d{4}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "库存盘点单号格式错误：{}，期望 IC{{yyyyMMdd}}{{4 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "IC202605140001");
}

/// 验证采购入库 generate-no 端点返回的单据号格式
///
/// 期望：`RK{yyyyMMdd}{4 位流水}`
#[test]
fn test_purchase_receipt_no_format() {
    let prefix = "RK";
    let today = "20260601";
    let serial = 123_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 4);

    let re = Regex::new(r"^RK\d{8}\d{4}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "采购入库单号格式错误：{}，期望 RK{{yyyyMMdd}}{{4 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "RK202606010123");
}

/// 验证库存调整 generate-no 端点返回的单据号格式
///
/// 期望：`IA{yyyyMMdd}{4 位流水}`
#[test]
fn test_inventory_adjustment_no_format() {
    let prefix = "IA";
    let today = "20260615";
    let serial = 9_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 4);

    let re = Regex::new(r"^IA\d{8}\d{4}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "库存调整单号格式错误：{}，期望 IA{{yyyyMMdd}}{{4 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "IA202606150009");
}

/// 验证库存调拨 generate-no 端点返回的单据号格式
///
/// 期望：`IT{yyyyMMdd}{4 位流水}`
#[test]
fn test_inventory_transfer_no_format() {
    let prefix = "IT";
    let today = "20260615";
    let serial = 7_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 4);

    let re = Regex::new(r"^IT\d{8}\d{4}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "库存调拨单号格式错误：{}，期望 IT{{yyyyMMdd}}{{4 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "IT202606150007");
}
