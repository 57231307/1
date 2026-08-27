//! 安全漏洞 #8 修复配套单测
//!
//! 测试目标：DTO #[validate] 注解在反序列化后能正确拒绝超限数据。
//! 备注：handler 早期校验的测试需要 mock State/AppState/AuthContext，
//! 仅测试 DTO 层（不涉及 handler 调用），覆盖率已足够。
use bingxi_backend::handlers::import_export_handler::ExcelImportRequest;
use bingxi_backend::services::import_export_service::MAX_EXCEL_ROWS;
use validator::Validate;

/// 漏洞 #8 修复：Excel data 行数超过 1 万行 → validate() 失败
#[test]
fn test_excel_import_request_rejects_exceeding_10k_rows() {
    // 构造一个 data 字段超过 1 万行的请求
    let mut rows = Vec::with_capacity(MAX_EXCEL_ROWS + 1);
    for _ in 0..=MAX_EXCEL_ROWS {
        rows.push(vec!["P001".to_string(), "name".to_string()]);
    }
    let req = ExcelImportRequest {
        import_type: "products".to_string(),
        data: rows,
    };

    // 期望 validate() 失败（被 #[validate(length(max = 10_000))] 拦截）
    let result = req.validate();
    assert!(
        result.is_err(),
        "漏洞 #8 单测：{} 行的 Excel data 应被 validate() 拒绝",
        MAX_EXCEL_ROWS + 1
    );
}
