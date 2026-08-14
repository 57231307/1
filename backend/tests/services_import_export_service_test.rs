//! 安全漏洞 #8 修复配套单测
//!
//! 测试目标：
//! 1. 常量定义正确（避免被误改）
//! 2. service 层 import_data 在数据超过上限时立即拒绝（defense-in-depth 第四层）
//!
//! 备注：handler 层 DTO 校验 + 早期校验在路由层单测覆盖；
//! 本处只覆盖 service 层入口校验（最关键的 defense-in-depth 屏障）。
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::services::import_export_service::MAX_CELL_LEN;
use bingxi_backend::services::import_export_service::MAX_EXCEL_COLS;
use bingxi_backend::services::import_export_service::MAX_EXCEL_ROWS;
use std::sync::Arc;

/// 测试常量定义正确（防止误改后引发业务可用性问题）
#[test]
fn test_vuln8_constants_defined_correctly() {
    // CSV 10MB：业务上限
    assert_eq!(MAX_CSV_BYTES, 10 * 1024 * 1024, "MAX_CSV_BYTES 应为 10MB");
    // Excel 1 万行
    assert_eq!(MAX_EXCEL_ROWS, 10_000, "MAX_EXCEL_ROWS 应为 1 万行");
    // 100 列
    assert_eq!(MAX_EXCEL_COLS, 100, "MAX_EXCEL_COLS 应为 100 列");
    // 单元格 1024 字符
    assert_eq!(MAX_CELL_LEN, 1024, "MAX_CELL_LEN 应为 1024 字符");
}

/// 漏洞 #8 修复：service 层 import_data 行数上限校验
/// 超过 MAX_EXCEL_ROWS 行 → 立即拒绝（不进入 DB 查询）
#[tokio::test]
async fn test_import_data_rejects_exceeding_max_rows() {
    let db = Arc::new(setup_test_db().await);
    let service = ImportExportService::new(db);

    // 构造超过 MAX_EXCEL_ROWS + 1 行的数据
    let mut data = Vec::with_capacity(MAX_EXCEL_ROWS + 1);
    for _ in 0..=MAX_EXCEL_ROWS {
        data.push(vec!["P001".to_string(), "name".to_string()]);
    }

    // 调用 import_data，期望 ValidationError
    let result = service.import_data("products", &data, 1).await;
    assert!(
        result.is_err(),
        "漏洞 #8 单测：{} 行数据应被拒绝，但 import_data 返回成功",
        data.len()
    );
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("最大行数")
            || err_msg.contains("MAX_EXCEL_ROWS")
            || err_msg.contains("上限"),
        "漏洞 #8 单测：错误信息应包含'最大行数'或'上限'，实际：{}",
        err_msg
    );
}

/// 漏洞 #8 修复：service 层 import_data 列数上限校验
/// 单行列数超过 MAX_EXCEL_COLS → 立即拒绝
#[tokio::test]
async fn test_import_data_rejects_exceeding_max_cols() {
    let db = Arc::new(setup_test_db().await);
    let service = ImportExportService::new(db);

    // 构造 1 行 MAX_EXCEL_COLS + 1 列的数据
    let mut row = Vec::with_capacity(MAX_EXCEL_COLS + 1);
    for i in 0..=MAX_EXCEL_COLS {
        row.push(format!("col_{}", i));
    }
    let data = vec![row];

    let result = service.import_data("products", &data, 1).await;
    assert!(
        result.is_err(),
        "漏洞 #8 单测：{} 列数据应被拒绝，但 import_data 返回成功",
        data[0].len()
    );
}

/// 漏洞 #8 修复：service 层 import_data 单元格长度上限校验
/// 单个单元格超过 MAX_CELL_LEN 字符 → 立即拒绝
#[tokio::test]
async fn test_import_data_rejects_exceeding_max_cell_len() {
    let db = Arc::new(setup_test_db().await);
    let service = ImportExportService::new(db);

    // 构造 1 个超过 MAX_CELL_LEN 字符的单元格
    let long_cell = "A".repeat(MAX_CELL_LEN + 1);
    let data = vec![vec![long_cell.clone()]];

    let result = service.import_data("products", &data, 1).await;
    assert!(
        result.is_err(),
        "漏洞 #8 单测：{} 字符的单元格应被拒绝，但 import_data 返回成功",
        long_cell.len()
    );
}

/// 漏洞 #8 修复：service 层 import_data 正常数据不误拒
/// 边界值测试：在所有上限内的数据应通过校验（即使后续因 unknown import_type 失败）
#[tokio::test]
async fn test_import_data_allows_within_limits() {
    let db = Arc::new(setup_test_db().await);
    let service = ImportExportService::new(db);

    // 构造 1 行 100 列的合法数据
    let mut row = Vec::with_capacity(MAX_EXCEL_COLS);
    for i in 0..MAX_EXCEL_COLS {
        row.push(format!("val_{}", i));
    }
    let data = vec![row];

    // 使用 unknown import_type 触发 "不支持的导入类型" 错误（说明校验通过）
    let result = service.import_data("unknown_type", &data, 1).await;
    assert!(
        result.is_err(),
        "漏洞 #8 单测：边界内数据不应被 service 层校验拒绝"
    );
    let err_msg = result.err().unwrap().to_string();
    assert!(
        err_msg.contains("不支持的导入类型"),
        "漏洞 #8 单测：service 层应通过校验，仅在 import_type 校验处失败，实际：{}",
        err_msg
    );
}
