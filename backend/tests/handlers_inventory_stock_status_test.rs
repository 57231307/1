//! 库存台账状态/质量状态取值域（handlers/inventory_stock_handler.rs）
//!
//! `inventory_stocks.stock_status` 与 `quality_status` 都是无 CHECK 的 VARCHAR，取值域分别是
//! 中文主数据「正常/报废/已删除」与「合格/待检/不合格」。历史上 `POST /inventory/stock` 按
//! 通用主数据域写入 `active`/`qualified`，而可用量、可出库与缺料预警一律按中文值过滤——
//! 这批库存对所有可用性查询永久不可见（账上有货、界面与出库无货）；同时仪表盘按同样的
//! 错误词表过滤，于是"库存统计恒 0"与"这批货看不见"两个错误互相抵消，16 轮 CI 无人发现。
//! 本用例钉住两列词表、建单初始值落在各自域内、以及列表/导出筛选对越界值一律拒绝。

use bingxi_backend::handlers::inventory_stock_handler::{
    initial_stock_statuses, validate_stock_status_param,
};
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::status::purchase_inventory::{
    inventory_stock_quality_status, inventory_stock_status,
};

#[test]
fn test_stock_status_vocabulary_is_the_chinese_domain() {
    assert_eq!(inventory_stock_status::ALL, &["正常", "报废", "已删除"]);
    // 通用主数据状态属另一张表：一旦有人再拿它写/筛本列，本断言立刻显红
    assert!(
        !inventory_stock_status::ALL.contains(&master_data::ACTIVE),
        "台账状态取值域混入了通用主数据状态 {}",
        master_data::ACTIVE
    );
}

#[test]
fn test_quality_status_vocabulary_excludes_other_inspection_domains() {
    assert_eq!(
        inventory_stock_quality_status::ALL,
        &["合格", "待检", "不合格"]
    );
    // passed 属验布放行域、qualified 属委外收回域，两度都有写入方把它们的字面量落进本列
    for foreign in ["passed", "qualified", "unqualified", "pass", "fail"] {
        assert!(
            !inventory_stock_quality_status::ALL.contains(&foreign),
            "质量状态取值域混入了其他检验域的写法 {foreign}"
        );
    }
}

#[test]
fn test_validate_accepts_canonical_values_and_treats_blank_as_no_filter() {
    for value in inventory_stock_status::ALL {
        assert!(
            validate_stock_status_param(Some(value)).is_ok(),
            "合法台账状态被拒绝：{value}"
        );
    }
    assert!(validate_stock_status_param(None).is_ok(), "不筛选被拒绝");
    assert!(
        validate_stock_status_param(Some("  ")).is_ok(),
        "空白参数应视为不筛选而不是越界值"
    );
    assert!(
        validate_stock_status_param(Some(" 正常 ")).is_ok(),
        "首尾空白未归一，导致同一语义值被判定越界"
    );
}

#[test]
fn test_validate_rejects_cross_domain_values_and_lists_allowed() {
    for bad in [
        "active",
        "ACTIVE",
        "normal",
        "warning",
        "frozen",
        "pending",
        // 质量状态列的合法值，属另一列，不得当作台账状态提交
        "合格",
    ] {
        let err = validate_stock_status_param(Some(bad));
        assert!(err.is_err(), "越界台账状态被放行：{bad}");
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("正常") && msg.contains("报废") && msg.contains("已删除"),
            "错误信息未列出合法值，调用方无法自纠：{msg}"
        );
    }
}

#[test]
fn test_created_stock_writes_values_inside_each_column_domain() {
    let (stock_status, quality_status) = initial_stock_statuses();
    assert!(
        inventory_stock_status::ALL.contains(&stock_status.as_str()),
        "新建库存的台账状态越界：{stock_status}"
    );
    assert!(
        inventory_stock_quality_status::ALL.contains(&quality_status.as_str()),
        "新建库存的质量状态越界：{quality_status}"
    );
    // 显式钉住历史越界写法，防止"改用常量"时又选回跨域常量
    assert_ne!(stock_status, master_data::ACTIVE);
    assert_ne!(quality_status, "qualified");
}
