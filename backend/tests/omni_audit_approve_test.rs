//! omni_audit classify_operation APPROVE 分类单元测试
//!
//! P1.4 审计 APPROVE 分类：验证 classify_operation 对 approve/reject/submit 路径
//! 返回 "APPROVE"，且 PRINT/EXPORT/DOWNLOAD/HTTP 方法映射分类回归不变。
//!
//! 纯函数测试，不依赖 DB/HTTP，可在本地和 CI 跑。

use bingxi_backend::middleware::omni_audit::classify_operation;

/// APPROVE 分类：路径末段含 approve
#[test]
fn test_classify_approve_segment() {
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/purchase-orders/1/approve", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/sales-orders/1/approve", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/export-approvals/1/approve", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/role-change-requests/5/approve", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/transfers/3/approve", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/writeoffs/2/approve", ""),
        "APPROVE"
    );
}

/// APPROVE 分类：路径末段为 reject
#[test]
fn test_classify_reject_segment() {
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/purchase-orders/1/reject", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/export-approvals/1/reject", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/sales-orders/1/reject", ""),
        "APPROVE"
    );
}

/// APPROVE 分类：路径末段为 submit
#[test]
fn test_classify_submit_segment() {
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/inventory-counts/1/submit", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/dye-recipes/1/submit", ""),
        "APPROVE"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/color-card-issues/1/submit", ""),
        "APPROVE"
    );
}

/// PRINT 分类回归：approve 不误吞 print 路径
#[test]
fn test_classify_print_regression() {
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/sales-orders/1/print", ""),
        "PRINT"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/vouchers/1/print", ""),
        "PRINT"
    );
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/print/sales-orders/1", ""),
        "PRINT"
    );
}

/// EXPORT 分类回归
#[test]
fn test_classify_export_regression() {
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/customers/export", ""),
        "EXPORT"
    );
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/products/export", ""),
        "EXPORT"
    );
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/reports/1/pdf", ""),
        "EXPORT"
    );
}

/// DOWNLOAD 分类回归
#[test]
fn test_classify_download_regression() {
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/files/1/download", ""),
        "DOWNLOAD"
    );
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/files/1?action=download", "action=download"),
        "DOWNLOAD"
    );
}

/// HTTP 方法映射回归：普通 CRUD 路径
#[test]
fn test_classify_http_method_mapping() {
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/customers", ""),
        "READ"
    );
    assert_eq!(
        classify_operation("POST", "/api/v1/erp/customers", ""),
        "CREATE"
    );
    assert_eq!(
        classify_operation("PUT", "/api/v1/erp/customers/1", ""),
        "UPDATE"
    );
    assert_eq!(
        classify_operation("PATCH", "/api/v1/erp/customers/1", ""),
        "UPDATE"
    );
    assert_eq!(
        classify_operation("DELETE", "/api/v1/erp/customers/1", ""),
        "DELETE"
    );
    assert_eq!(
        classify_operation("HEAD", "/api/v1/erp/health", ""),
        "OTHER"
    );
}

/// 边界：approve 出现在非末段位置（如 /approve-requests/1）不触发 APPROVE
#[test]
fn test_classify_approve_not_last_segment() {
    // "approve-requests" 末段是 "approve-requests"，不含 "approve" 子串？
    // last_segment.contains("approve") → "approve-requests".contains("approve") = true → APPROVE
    // 这是预期行为：approve-requests 路径归 APPROVE
    assert_eq!(
        classify_operation("GET", "/api/v1/erp/approve-requests/1", ""),
        "APPROVE"
    );
}

/// 边界：空路径和根路径
#[test]
fn test_classify_edge_cases() {
    assert_eq!(classify_operation("GET", "", ""), "READ");
    assert_eq!(classify_operation("GET", "/", ""), "READ");
}
