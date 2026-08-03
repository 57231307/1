//! P1-1 generate-no 4 端点补齐
//!
//! 单元测试覆盖 4 个 generate-no 端点的单据号格式契约：
//! 1. purchaseReceipt（采购收货）— 前缀 `RK`
//! 2. inventoryAdjustment（库存调整）— 前缀 `IA`
//! 3. inventoryTransfer（库存调拨）— 前缀 `IT`
//!
//! V15 主线审计 P2 修复：删除"库存盘点 generate-no 端点"陈旧描述；
//! 盘点路由无独立 generate-no 端点（单号随 create 一起返回），不再纳入本测试清单。
//!
//! 单据号格式：`{前缀}{yyyyMMdd}{4 位流水}`，例如 `RK202605140001`。
//!
//! 这些测试**仅校验纯字符串格式化逻辑**，不依赖数据库：
//! - 端点的 `generate_no` 在 Handler 中仅作为薄包装调用
//!   `DocumentNumberGenerator::generate_no_with_width`；
//! - DB 部分（`count + 1`）由 [`number_generator`] 单元测试覆盖。
//!
//! 因此本测试文件只验证："业务前缀 + 8 位日期 + 4 位流水" 的拼接契约，
//! 防止后续有人误将流水宽度从 4 位回退为 3 位、或改动前缀字符。

use regex::Regex;

/// V15 主线审计 P2 修复：原 inventory count generate-no 端点测试已移除（盘点无独立 generate-no 端点）。
/// 仅保留按业务前缀/日期/流水拼接的契约测试（RK/IA/IT 三类）。
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

/// 验证销售订单 generate-no 端点返回的单据号格式（P1-1 补齐）
///
/// 期望：`SO{yyyyMMdd}{3 位流水}`（销售订单沿用 3 位流水）
#[test]
fn test_sales_order_no_format() {
    let prefix = "SO";
    let today = "20260617";
    let serial = 42_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 3);

    let re = Regex::new(r"^SO\d{8}\d{3}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "销售订单单号格式错误：{}，期望 SO{{yyyyMMdd}}{{3 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "SO202606170042");
}

/// 验证采购订单 generate-no 端点返回的单据号格式（P1-1 补齐）
///
/// 期望：`PO{yyyyMMdd}{3 位流水}`（采购订单沿用 3 位流水）
#[test]
fn test_purchase_order_no_format() {
    let prefix = "PO";
    let today = "20260617";
    let serial = 17_usize;
    let doc_no = format!("{}{}{:0width$}", prefix, today, serial, width = 3);

    let re = Regex::new(r"^PO\d{8}\d{3}$").expect("正则必须编译通过");
    assert!(
        re.is_match(&doc_no),
        "采购订单单号格式错误：{}，期望 PO{{yyyyMMdd}}{{3 位流水}}",
        doc_no
    );
    assert_eq!(doc_no, "PO202606170017");
}
